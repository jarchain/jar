//! PVM blob emitter — produces JAR v1 and v2 program blobs.

use javm::program::{JAR_MAGIC, ProgramHeader};
use scale::Encode;

/// Pack a bitmask array (one byte per bit, 0 or 1) into packed bytes (LSB first).
pub fn pack_bitmask(bitmask: &[u8]) -> Vec<u8> {
    let packed_len = bitmask.len().div_ceil(8);
    let mut packed = vec![0u8; packed_len];
    for (i, &bit) in bitmask.iter().enumerate() {
        if bit != 0 {
            packed[i / 8] |= 1 << (i % 8);
        }
    }
    packed
}

/// Build a complete JAR v1 program blob.
///
/// Layout: header | ro_data | rw_data | jump_table | code | packed_bitmask
#[allow(clippy::too_many_arguments)]
pub fn build_standard_program(
    ro_data: &[u8],
    rw_data: &[u8],
    heap_pages: u32,
    max_heap_pages: u32,
    stack_pages: u32,
    code: &[u8],
    bitmask: &[u8],
    jump_table: &[u32],
) -> Vec<u8> {
    assert_eq!(
        code.len(),
        bitmask.len(),
        "code and bitmask must have same length"
    );

    // Determine jump table entry encoding size (z)
    let entry_size: u8 = if jump_table.is_empty() {
        1
    } else {
        let max_val = jump_table.iter().copied().max().unwrap_or(0);
        if max_val <= 0xFF {
            1
        } else if max_val <= 0xFFFF {
            2
        } else if max_val <= 0xFFFFFF {
            3
        } else {
            4
        }
    };

    let header = ProgramHeader {
        magic: JAR_MAGIC,
        ro_size: ro_data.len() as u32,
        rw_size: rw_data.len() as u32,
        heap_pages,
        max_heap_pages,
        stack_pages,
        jump_len: jump_table.len() as u32,
        entry_size,
        code_len: code.len() as u32,
    };

    let mut blob = header.encode();

    // ro_data
    blob.extend_from_slice(ro_data);

    // rw_data
    blob.extend_from_slice(rw_data);

    // jump table entries (entry_size bytes each, LE)
    for &entry in jump_table {
        let bytes = entry.to_le_bytes();
        blob.extend_from_slice(&bytes[..entry_size as usize]);
    }

    // code bytes
    blob.extend_from_slice(code);

    // packed bitmask
    blob.extend_from_slice(&pack_bitmask(bitmask));

    blob
}

/// Build a JAR v2 capability manifest blob from components.
///
/// This is the v2 equivalent of `build_standard_program`. It takes
/// code/data as separate pieces and assembles a capability manifest.
///
/// The simplest v2 blob has one CODE cap and one DATA cap (stack).
/// More complex blobs have separate ro_data, rw_data, heap DATA caps.
#[allow(clippy::too_many_arguments)]
pub fn build_v2_service_program(
    code: &[u8],
    bitmask: &[u8],
    jump_table: &[u32],
    ro_data: &[u8],
    rw_data: &[u8],
    stack_pages: u32,
    heap_pages: u32,
    memory_pages: u32,
) -> Vec<u8> {
    use javm::cap::Access;
    use javm::program_v2::{build_v2_blob, CapEntryType, CapManifestEntry};

    // Build the CODE blob (jump_table + code + packed_bitmask) as a sub-blob
    // that will be the data for the CODE cap.
    let entry_size: u8 = if jump_table.is_empty() {
        1
    } else {
        let max_val = jump_table.iter().copied().max().unwrap_or(0);
        if max_val <= 0xFF { 1 } else if max_val <= 0xFFFF { 2 } else if max_val <= 0xFFFFFF { 3 } else { 4 }
    };

    let mut code_blob = Vec::new();
    // Jump table entries
    for &entry in jump_table {
        code_blob.extend_from_slice(&entry.to_le_bytes()[..entry_size as usize]);
    }
    // Code bytes
    code_blob.extend_from_slice(code);
    // Packed bitmask
    code_blob.extend_from_slice(&pack_bitmask(bitmask));

    // Build data section: code_blob + ro_data + rw_data
    let mut data_section = Vec::new();
    let code_offset = 0u32;
    let code_len = code_blob.len() as u32;
    data_section.extend_from_slice(&code_blob);

    let ro_offset = data_section.len() as u32;
    let ro_len = ro_data.len() as u32;
    data_section.extend_from_slice(ro_data);

    let rw_offset = data_section.len() as u32;
    let rw_len = rw_data.len() as u32;
    data_section.extend_from_slice(rw_data);

    // Build cap manifest
    // Layout: cap[64]=CODE, cap[65]=stack(RW), cap[66]=ro(RO), cap[67]=rw(RW)
    let mut caps = Vec::new();
    let mut next_page = 0u32;

    // CODE cap
    caps.push(CapManifestEntry {
        cap_index: 64,
        cap_type: CapEntryType::Code,
        base_page: 0,
        page_count: 0,
        init_access: Access::RO,
        data_offset: code_offset,
        data_len: code_len,
    });

    // Stack DATA cap (zero-filled)
    caps.push(CapManifestEntry {
        cap_index: 65,
        cap_type: CapEntryType::Data,
        base_page: next_page,
        page_count: stack_pages,
        init_access: Access::RW,
        data_offset: 0,
        data_len: 0,
    });
    next_page += stack_pages;

    // RO DATA cap (if non-empty)
    if !ro_data.is_empty() {
        let ro_pages = (ro_data.len() as u32).div_ceil(4096);
        caps.push(CapManifestEntry {
            cap_index: 66,
            cap_type: CapEntryType::Data,
            base_page: next_page,
            page_count: ro_pages,
            init_access: Access::RO,
            data_offset: ro_offset,
            data_len: ro_len,
        });
        next_page += ro_pages;
    }

    // RW DATA cap (if non-empty)
    if !rw_data.is_empty() {
        let rw_pages = (rw_data.len() as u32).div_ceil(4096);
        caps.push(CapManifestEntry {
            cap_index: 67,
            cap_type: CapEntryType::Data,
            base_page: next_page,
            page_count: rw_pages,
            init_access: Access::RW,
            data_offset: rw_offset,
            data_len: rw_len,
        });
        next_page += rw_pages;
    }

    // Heap DATA cap (zero-filled)
    if heap_pages > 0 {
        caps.push(CapManifestEntry {
            cap_index: 68,
            cap_type: CapEntryType::Data,
            base_page: next_page,
            page_count: heap_pages,
            init_access: Access::RW,
            data_offset: 0,
            data_len: 0,
        });
    }

    let total = memory_pages.max(next_page + heap_pages);
    build_v2_blob(total, 64, 65, &caps, &data_section)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_bitmask() {
        assert_eq!(pack_bitmask(&[1, 1, 1]), vec![0x07]);
        assert_eq!(pack_bitmask(&[1, 0, 1, 0, 1, 0, 1, 0]), vec![0x55]);
        assert_eq!(pack_bitmask(&[1, 0, 1, 0, 1, 0, 1, 0, 1]), vec![0x55, 0x01]);
    }

    #[test]
    fn test_build_minimal() {
        let code = vec![0, 1, 0]; // trap, fallthrough, trap
        let bitmask = vec![1, 1, 1];
        let blob = build_standard_program(&[], &[], 0, 0, 1, &code, &bitmask, &[]);

        // Should be loadable by PVM
        let pvm = javm::program::initialize_program(&blob, &[], 100_000);
        assert!(pvm.is_some(), "JAR blob should be loadable");
    }

    #[test]
    fn test_round_trip_with_data() {
        let ro = vec![0xDE, 0xAD];
        let rw = vec![0xBE, 0xEF];
        let code = vec![0, 1, 0];
        let bitmask = vec![1, 1, 1];
        let blob = build_standard_program(&ro, &rw, 1, 1, 1, &code, &bitmask, &[]);

        let pvm = javm::program::initialize_program(&blob, &[], 100_000);
        assert!(pvm.is_some());
    }
}
