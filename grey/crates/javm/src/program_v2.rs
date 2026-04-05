//! JAR v2 program blob format — capability manifest.
//!
//! The blob is a capability manifest: a list of initial capabilities
//! (CODE and DATA) with their contents, plus invocation directives.
//!
//! Layout:
//! ```text
//! Header:
//!   magic: u32              'JAR\x02'
//!   memory_pages: u32       total Untyped budget
//!   cap_count: u8           number of initial capabilities
//!   invoke_cap: u8          cap_index of CODE cap to execute first
//!   args_cap: u8            cap_index of DATA cap for arguments (0xFF = none)
//!
//! Capabilities[cap_count]:
//!   cap[i]: {
//!     cap_index: u8         slot in VM's cap table
//!     cap_type: u8          0 = CODE, 1 = DATA
//!     base_page: u32        starting page in address space (DATA only)
//!     page_count: u32       number of pages (DATA only)
//!     init_access: u8       0 = RO, 1 = RW (DATA only)
//!     data_offset: u32      offset into blob's data section
//!     data_len: u32         bytes of initial data (0 = zero-filled)
//!   }
//!
//! Data section:
//!   (variable-length, referenced by capabilities)
//! ```

use alloc::{vec, vec::Vec};

use crate::cap::Access;

/// JAR v2 magic: 'J','A','R', 0x02.
pub const JAR_V2_MAGIC: u32 = u32::from_le_bytes([b'J', b'A', b'R', 0x02]);

/// Header size: magic(4) + memory_pages(4) + cap_count(1) + invoke_cap(1) = 10.
const HEADER_SIZE: usize = 10;

/// Per-cap entry size: cap_index(1) + cap_type(1) + base_page(4) + page_count(4)
///   + init_access(1) + data_offset(4) + data_len(4) = 19.
const CAP_ENTRY_SIZE: usize = 19;

/// Cap type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CapEntryType {
    Code = 0,
    Data = 1,
}

/// A single capability entry in the manifest.
#[derive(Debug, Clone)]
pub struct CapManifestEntry {
    /// Slot in the VM's cap table.
    pub cap_index: u8,
    /// Capability type.
    pub cap_type: CapEntryType,
    /// Starting page in address space (DATA only, ignored for CODE).
    pub base_page: u32,
    /// Number of pages (DATA only, ignored for CODE).
    pub page_count: u32,
    /// Initial access mode for MAP at program init (DATA only).
    pub init_access: Access,
    /// Offset into the blob's data section (0 = no data).
    pub data_offset: u32,
    /// Bytes of initial data (0 = zero-filled for DATA, empty for CODE).
    pub data_len: u32,
}

/// Parsed JAR v2 header.
#[derive(Debug, Clone)]
pub struct ProgramHeaderV2 {
    /// Total Untyped page budget.
    pub memory_pages: u32,
    /// Number of capabilities in the manifest.
    pub cap_count: u8,
    /// Cap index of the CODE cap to execute first.
    pub invoke_cap: u8,
}

/// Parsed JAR v2 blob.
#[derive(Debug)]
pub struct ParsedBlobV2<'a> {
    /// Header fields.
    pub header: ProgramHeaderV2,
    /// Capability manifest entries.
    pub caps: Vec<CapManifestEntry>,
    /// Data section (referenced by capabilities via data_offset + data_len).
    pub data_section: &'a [u8],
}

fn read_u8(blob: &[u8], offset: &mut usize) -> Option<u8> {
    if *offset >= blob.len() {
        return None;
    }
    let v = blob[*offset];
    *offset += 1;
    Some(v)
}

fn read_u32_le(blob: &[u8], offset: &mut usize) -> Option<u32> {
    if *offset + 4 > blob.len() {
        return None;
    }
    let v = u32::from_le_bytes([
        blob[*offset],
        blob[*offset + 1],
        blob[*offset + 2],
        blob[*offset + 3],
    ]);
    *offset += 4;
    Some(v)
}

/// Parse a JAR v2 program blob.
pub fn parse_v2_blob(blob: &[u8]) -> Option<ParsedBlobV2<'_>> {
    if blob.len() < HEADER_SIZE {
        return None;
    }

    let mut offset = 0;

    // Header
    let magic = read_u32_le(blob, &mut offset)?;
    if magic != JAR_V2_MAGIC {
        return None;
    }
    let memory_pages = read_u32_le(blob, &mut offset)?;
    let cap_count = read_u8(blob, &mut offset)?;
    let invoke_cap = read_u8(blob, &mut offset)?;

    // Capability entries
    let entries_size = cap_count as usize * CAP_ENTRY_SIZE;
    if offset + entries_size > blob.len() {
        return None;
    }

    let mut caps = Vec::with_capacity(cap_count as usize);
    for _ in 0..cap_count {
        let cap_index = read_u8(blob, &mut offset)?;
        let cap_type_raw = read_u8(blob, &mut offset)?;
        let cap_type = match cap_type_raw {
            0 => CapEntryType::Code,
            1 => CapEntryType::Data,
            _ => return None,
        };
        let base_page = read_u32_le(blob, &mut offset)?;
        let page_count = read_u32_le(blob, &mut offset)?;
        let init_access_raw = read_u8(blob, &mut offset)?;
        let init_access = match init_access_raw {
            0 => Access::RO,
            1 => Access::RW,
            _ => return None,
        };
        let data_offset = read_u32_le(blob, &mut offset)?;
        let data_len = read_u32_le(blob, &mut offset)?;

        caps.push(CapManifestEntry {
            cap_index,
            cap_type,
            base_page,
            page_count,
            init_access,
            data_offset,
            data_len,
        });
    }

    // Data section = everything after the cap entries
    let data_section = &blob[offset..];

    // Validate data references
    for cap in &caps {
        if cap.data_len > 0 {
            let end = cap.data_offset as usize + cap.data_len as usize;
            if end > data_section.len() {
                return None;
            }
        }
    }

    Some(ParsedBlobV2 {
        header: ProgramHeaderV2 {
            memory_pages,
            cap_count,
            invoke_cap,
        },
        caps,
        data_section,
    })
}

/// Parsed code sub-blob (within a CODE cap's data section).
#[derive(Debug)]
pub struct ParsedCodeBlob {
    pub jump_table: Vec<u32>,
    pub code: Vec<u8>,
    pub bitmask: Vec<u8>,
}

/// Parse a CODE cap's data section into jump table, code, and bitmask.
/// Format: jump_len(4) + entry_size(1) + code_len(4) + jump_entries + code + packed_bitmask
pub fn parse_code_blob(data: &[u8]) -> Option<ParsedCodeBlob> {
    if data.len() < 9 {
        return None;
    }
    let mut offset = 0;
    let jump_len = read_u32_le(data, &mut offset)? as usize;
    let entry_size = read_u8(data, &mut offset)? as usize;
    let code_len = read_u32_le(data, &mut offset)? as usize;

    if entry_size == 0 || entry_size > 4 {
        return None;
    }

    // Read jump table
    let jt_bytes = jump_len * entry_size;
    if offset + jt_bytes > data.len() {
        return None;
    }
    let mut jump_table = Vec::with_capacity(jump_len);
    for _ in 0..jump_len {
        let mut val: u32 = 0;
        for i in 0..entry_size {
            val |= (data[offset + i] as u32) << (i * 8);
        }
        jump_table.push(val);
        offset += entry_size;
    }

    // Read code
    if offset + code_len > data.len() {
        return None;
    }
    let code = data[offset..offset + code_len].to_vec();
    offset += code_len;

    // Read packed bitmask
    let bitmask_bytes = code_len.div_ceil(8);
    if offset + bitmask_bytes > data.len() {
        return None;
    }
    let bitmask = unpack_bitmask(&data[offset..offset + bitmask_bytes], code_len);

    Some(ParsedCodeBlob {
        jump_table,
        code,
        bitmask,
    })
}

/// Unpack a packed bitmask (1 bit per byte) into one byte per code position.
fn unpack_bitmask(packed: &[u8], code_len: usize) -> Vec<u8> {
    let mut bitmask = vec![0u8; code_len];
    for i in 0..code_len {
        bitmask[i] = (packed[i / 8] >> (i % 8)) & 1;
    }
    bitmask
}

/// Build a JAR v2 blob from components.
pub fn build_v2_blob(
    memory_pages: u32,
    invoke_cap: u8,
    caps: &[CapManifestEntry],
    data_section: &[u8],
) -> Vec<u8> {
    let cap_count = caps.len() as u8;
    let total_size = HEADER_SIZE + caps.len() * CAP_ENTRY_SIZE + data_section.len();
    let mut blob = vec![0u8; total_size];
    let mut offset = 0;

    // Header (10 bytes: magic + memory_pages + cap_count + invoke_cap)
    write_u32_le(&mut blob, &mut offset, JAR_V2_MAGIC);
    write_u32_le(&mut blob, &mut offset, memory_pages);
    write_u8(&mut blob, &mut offset, cap_count);
    write_u8(&mut blob, &mut offset, invoke_cap);

    // Cap entries
    for cap in caps {
        write_u8(&mut blob, &mut offset, cap.cap_index);
        write_u8(&mut blob, &mut offset, cap.cap_type as u8);
        write_u32_le(&mut blob, &mut offset, cap.base_page);
        write_u32_le(&mut blob, &mut offset, cap.page_count);
        write_u8(&mut blob, &mut offset, cap.init_access as u8);
        write_u32_le(&mut blob, &mut offset, cap.data_offset);
        write_u32_le(&mut blob, &mut offset, cap.data_len);
    }

    // Data section
    blob[offset..].copy_from_slice(data_section);

    blob
}

fn write_u8(buf: &mut [u8], offset: &mut usize, v: u8) {
    buf[*offset] = v;
    *offset += 1;
}

fn write_u32_le(buf: &mut [u8], offset: &mut usize, v: u32) {
    buf[*offset..*offset + 4].copy_from_slice(&v.to_le_bytes());
    *offset += 4;
}

/// Get the data slice for a capability entry from the data section.
pub fn cap_data<'a>(entry: &CapManifestEntry, data_section: &'a [u8]) -> &'a [u8] {
    if entry.data_len == 0 {
        return &[];
    }
    &data_section[entry.data_offset as usize..entry.data_offset as usize + entry.data_len as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_blob() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        // CODE blob: 4 bytes of PVM code
        let code_data = vec![0x00, 0x01, 0x02, 0x03]; // trap, fallthrough, unlikely, ...
        // RO data: 8 bytes
        let ro_data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];

        // Combined data section: code_data + ro_data
        let mut data_section = Vec::new();
        data_section.extend_from_slice(&code_data);
        data_section.extend_from_slice(&ro_data);

        (code_data, ro_data, data_section)
    }

    #[test]
    fn test_roundtrip() {
        let (_code_data, _ro_data, data_section) = make_test_blob();

        let caps = vec![
            CapManifestEntry {
                cap_index: 64,
                cap_type: CapEntryType::Code,
                base_page: 0,
                page_count: 0,
                init_access: Access::RO,
                data_offset: 0,
                data_len: 4, // code blob
            },
            CapManifestEntry {
                cap_index: 65,
                cap_type: CapEntryType::Data,
                base_page: 0,
                page_count: 1,
                init_access: Access::RW,
                data_offset: 0,
                data_len: 0, // zero-filled stack
            },
            CapManifestEntry {
                cap_index: 66,
                cap_type: CapEntryType::Data,
                base_page: 1,
                page_count: 1,
                init_access: Access::RO,
                data_offset: 4,
                data_len: 8, // ro_data
            },
        ];

        let blob = build_v2_blob(10, 64, &caps, &data_section);
        let parsed = parse_v2_blob(&blob).expect("parse failed");

        assert_eq!(parsed.header.memory_pages, 10);
        assert_eq!(parsed.header.cap_count, 3);
        assert_eq!(parsed.header.invoke_cap, 64);
        assert_eq!(parsed.caps.len(), 3);

        // CODE cap
        assert_eq!(parsed.caps[0].cap_index, 64);
        assert_eq!(parsed.caps[0].cap_type, CapEntryType::Code);
        assert_eq!(parsed.caps[0].data_len, 4);
        let code = cap_data(&parsed.caps[0], parsed.data_section);
        assert_eq!(code, &[0x00, 0x01, 0x02, 0x03]);

        // Stack DATA cap (zero-filled)
        assert_eq!(parsed.caps[1].cap_index, 65);
        assert_eq!(parsed.caps[1].cap_type, CapEntryType::Data);
        assert_eq!(parsed.caps[1].base_page, 0);
        assert_eq!(parsed.caps[1].page_count, 1);
        assert_eq!(parsed.caps[1].init_access, Access::RW);
        assert_eq!(parsed.caps[1].data_len, 0);

        // RO DATA cap
        assert_eq!(parsed.caps[2].cap_index, 66);
        assert_eq!(parsed.caps[2].cap_type, CapEntryType::Data);
        assert_eq!(parsed.caps[2].base_page, 1);
        assert_eq!(parsed.caps[2].page_count, 1);
        assert_eq!(parsed.caps[2].init_access, Access::RO);
        let ro = cap_data(&parsed.caps[2], parsed.data_section);
        assert_eq!(ro, &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn test_bad_magic() {
        let blob = build_v2_blob(10, 64, &[], &[]);
        let mut bad = blob.clone();
        bad[3] = 0x99; // corrupt version byte
        assert!(parse_v2_blob(&bad).is_none());
    }

    #[test]
    fn test_truncated_blob() {
        // Too short for header
        assert!(parse_v2_blob(&[0; 5]).is_none());

        // Header says 1 cap but blob is too short
        let blob = build_v2_blob(10, 64, &[], &[]);
        let mut bad = blob;
        bad[8] = 1; // cap_count = 1 but no cap entries follow
        assert!(parse_v2_blob(&bad).is_none());
    }

    #[test]
    fn test_bad_data_reference() {
        let caps = vec![CapManifestEntry {
            cap_index: 64,
            cap_type: CapEntryType::Code,
            base_page: 0,
            page_count: 0,
            init_access: Access::RO,
            data_offset: 0,
            data_len: 100, // references 100 bytes but data section is empty
        }];
        let blob = build_v2_blob(10, 64, &caps, &[]);
        assert!(parse_v2_blob(&blob).is_none());
    }

    #[test]
    fn test_no_args_cap() {
        let blob = build_v2_blob(5, 64, &[], &[]);
        let _parsed = parse_v2_blob(&blob).unwrap();
    }

    #[test]
    fn test_empty_manifest() {
        let blob = build_v2_blob(0, 0, &[], &[]);
        let parsed = parse_v2_blob(&blob).unwrap();
        assert_eq!(parsed.caps.len(), 0);
        assert_eq!(parsed.data_section.len(), 0);
    }
}
