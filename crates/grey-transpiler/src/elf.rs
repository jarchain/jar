//! Minimal ELF parser for RISC-V binaries.
//!
//! Parses ELF32/ELF64 files targeting RISC-V and extracts
//! code sections, data sections, and relocations.

use crate::TranspileError;

/// Parsed ELF file contents.
pub struct Elf {
    /// Whether this is a 64-bit ELF.
    pub is_64bit: bool,
    /// Executable code sections.
    pub code_sections: Vec<Section>,
    /// Read-only data (concatenated .rodata sections).
    pub ro_data: Vec<u8>,
    /// Read-write data (concatenated .data + .bss sections).
    pub rw_data: Vec<u8>,
    /// Number of heap pages to allocate.
    pub heap_pages: u16,
    /// Stack size in bytes.
    pub stack_size: u32,
    /// Entry point address.
    pub entry_point: u64,
    /// Named symbols: (name, address).
    pub symbols: Vec<(String, u64)>,
}

impl Elf {
    /// Look up a symbol address by name.
    pub fn symbol_address(&self, name: &str) -> Option<u64> {
        self.symbols.iter().find(|(n, _)| n == name).map(|(_, a)| *a)
    }
}

/// A code section from the ELF.
pub struct Section {
    /// Virtual address.
    pub address: u64,
    /// Section data.
    pub data: Vec<u8>,
}

// ELF magic number
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
// RISC-V machine type
const EM_RISCV: u16 = 243;

/// Read a little-endian u16 from `data` at `offset`, returning an error if out of bounds.
fn read_u16(data: &[u8], offset: usize) -> Result<u16, TranspileError> {
    data.get(offset..offset + 2)
        .and_then(|s| s.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| TranspileError::ElfParse(format!("read u16 at offset {offset} out of bounds")))
}

/// Read a little-endian u32 from `data` at `offset`, returning an error if out of bounds.
fn read_u32(data: &[u8], offset: usize) -> Result<u32, TranspileError> {
    data.get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| TranspileError::ElfParse(format!("read u32 at offset {offset} out of bounds")))
}

/// Read a little-endian u64 from `data` at `offset`, returning an error if out of bounds.
fn read_u64(data: &[u8], offset: usize) -> Result<u64, TranspileError> {
    data.get(offset..offset + 8)
        .and_then(|s| s.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| TranspileError::ElfParse(format!("read u64 at offset {offset} out of bounds")))
}

/// Get a slice `data[offset..offset+len]`, returning an error if out of bounds.
fn get_slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8], TranspileError> {
    data.get(offset..offset.saturating_add(len))
        .ok_or_else(|| TranspileError::ElfParse(format!("slice [{offset}..+{len}] out of bounds (len={})", data.len())))
}

impl Elf {
    /// Parse an ELF binary.
    pub fn parse(data: &[u8]) -> Result<Self, TranspileError> {
        if data.len() < 64 {
            return Err(TranspileError::ElfParse("file too small".into()));
        }
        if data[0..4] != ELF_MAGIC {
            return Err(TranspileError::ElfParse("not an ELF file".into()));
        }

        let class = data[4]; // 1 = 32-bit, 2 = 64-bit
        let is_64bit = match class {
            1 => false,
            2 => true,
            _ => return Err(TranspileError::ElfParse(format!("unsupported ELF class: {}", class))),
        };

        // Verify little-endian
        if data[5] != 1 {
            return Err(TranspileError::ElfParse("not little-endian".into()));
        }

        // Verify RISC-V machine type
        let machine = u16::from_le_bytes([data[18], data[19]]);
        if machine != EM_RISCV {
            return Err(TranspileError::ElfParse(format!("not RISC-V: machine={}", machine)));
        }

        if is_64bit {
            Self::parse_elf64(data)
        } else {
            Self::parse_elf32(data)
        }
    }

    fn parse_elf32(data: &[u8]) -> Result<Self, TranspileError> {
        let entry_point = read_u32(data, 24)? as u64;
        let sh_offset = read_u32(data, 32)? as usize;
        let sh_size = read_u16(data, 46)? as usize;
        let sh_count = read_u16(data, 48)? as usize;
        let sh_strndx = read_u16(data, 50)? as usize;

        let mut code_sections = Vec::new();
        let mut symbols = Vec::new();

        // Collect data section info: (virtual_addr, size, section_data_or_zeros)
        let mut ro_sections: Vec<(u64, usize, Option<Vec<u8>>)> = Vec::new();
        let mut rw_sections: Vec<(u64, usize, Option<Vec<u8>>)> = Vec::new();

        // Get section-name string table
        let strtab = if sh_strndx < sh_count {
            let str_sh = sh_offset + sh_strndx * sh_size;
            let str_off = read_u32(data, str_sh + 16)? as usize;
            let str_sz = read_u32(data, str_sh + 20)? as usize;
            get_slice(data, str_off, str_sz)?
        } else {
            &[]
        };

        // Track symtab/strtab sections for symbol parsing
        let mut symtab_off = 0usize;
        let mut symtab_sz = 0usize;
        let mut symtab_link = 0usize; // index of associated string table

        for i in 0..sh_count {
            let sh = sh_offset + i * sh_size;
            if sh + sh_size > data.len() { break; }

            let name_off = read_u32(data, sh)? as usize;
            let sh_type = read_u32(data, sh + 4)?;
            let sh_flags = read_u32(data, sh + 8)?;
            let sh_addr = read_u32(data, sh + 12)? as u64;
            let sh_off = read_u32(data, sh + 16)? as usize;
            let sh_sz = read_u32(data, sh + 20)? as usize;
            let sh_link = read_u32(data, sh + 24)? as usize;

            // SHT_SYMTAB = 2
            if sh_type == 2 {
                symtab_off = sh_off;
                symtab_sz = sh_sz;
                symtab_link = sh_link;
            }

            let name = get_string(strtab, name_off);

            let is_alloc = sh_flags & 0x2 != 0; // SHF_ALLOC
            let is_exec = sh_flags & 0x4 != 0;  // SHF_EXECINSTR
            let is_write = sh_flags & 0x1 != 0;  // SHF_WRITE

            if !is_alloc || sh_type == 0 { continue; } // Skip non-allocated sections

            if is_exec && sh_off + sh_sz <= data.len() {
                code_sections.push(Section {
                    address: sh_addr,
                    data: data[sh_off..sh_off + sh_sz].to_vec(),
                });
            } else if !is_write && !is_exec {
                if name.starts_with(".rodata") || name == ".srodata" {
                    if sh_off + sh_sz <= data.len() {
                        ro_sections.push((sh_addr, sh_sz, Some(data[sh_off..sh_off + sh_sz].to_vec())));
                    }
                }
            } else if is_write {
                if sh_type == 8 { // SHT_NOBITS (.bss) — no file data, just virtual size
                    rw_sections.push((sh_addr, sh_sz, None)); // None = zero-filled
                } else if sh_off + sh_sz <= data.len() {
                    rw_sections.push((sh_addr, sh_sz, Some(data[sh_off..sh_off + sh_sz].to_vec())));
                }
            }
        }

        // Build address-aware data blobs: place sections at correct offsets
        // relative to the PVM base address (ZZ=0x10000 for RO, computed for RW).
        let ro_data = build_data_blob(&ro_sections, 0x10000); // PVM RO base = ZZ
        let rw_data = build_data_blob_rw(&rw_sections, ro_data.len());

        // Parse symbol table
        if symtab_sz > 0 && symtab_link < sh_count {
            // Get the symbol string table
            let sym_strtab_sh = sh_offset + symtab_link * sh_size;
            let sym_strtab_off = read_u32(data, sym_strtab_sh + 16)? as usize;
            let sym_strtab_sz = read_u32(data, sym_strtab_sh + 20)? as usize;
            let sym_strtab = get_slice(data, sym_strtab_off, sym_strtab_sz)?;

            // ELF32 symbol entry is 16 bytes
            let sym_count = symtab_sz / 16;
            for j in 0..sym_count {
                let sym = symtab_off + j * 16;
                if sym + 16 > data.len() { break; }
                let st_name = read_u32(data, sym)? as usize;
                let st_value = read_u32(data, sym + 4)? as u64;
                let st_info = data[sym + 12];
                // STT_FUNC=2, STT_NOTYPE=0; STB_GLOBAL=1, STB_WEAK=2
                let st_type = st_info & 0xF;
                let st_bind = st_info >> 4;
                if (st_type == 2 || st_type == 0) && (st_bind == 1 || st_bind == 2) && st_value != 0 {
                    let name = get_string(sym_strtab, st_name);
                    if !name.is_empty() && !name.starts_with('$') {
                        symbols.push((name.to_string(), st_value));
                    }
                }
            }
        }

        Ok(Elf {
            is_64bit: false,
            code_sections,
            ro_data,
            rw_data,
            heap_pages: 4,
            stack_size: 4096,
            entry_point,
            symbols,
        })
    }

    fn parse_elf64(data: &[u8]) -> Result<Self, TranspileError> {
        if data.len() < 64 {
            return Err(TranspileError::ElfParse("ELF64 header too small".into()));
        }

        let entry_point = read_u64(data, 24)?;
        let sh_offset = read_u64(data, 40)? as usize;
        let sh_size = read_u16(data, 58)? as usize;
        let sh_count = read_u16(data, 60)? as usize;
        let sh_strndx = read_u16(data, 62)? as usize;

        let mut code_sections = Vec::new();
        let mut symbols = Vec::new();

        // Collect data section info: (virtual_addr, size, section_data_or_zeros)
        let mut ro_sections: Vec<(u64, usize, Option<Vec<u8>>)> = Vec::new();
        let mut rw_sections: Vec<(u64, usize, Option<Vec<u8>>)> = Vec::new();

        // Track symtab for symbol parsing
        let mut symtab_off = 0usize;
        let mut symtab_sz = 0usize;
        let mut symtab_link = 0usize;

        // Get string table
        let strtab = if sh_strndx < sh_count && sh_offset + sh_strndx * sh_size + sh_size <= data.len() {
            let str_sh = sh_offset + sh_strndx * sh_size;
            let str_off = read_u64(data, str_sh + 24)? as usize;
            let str_sz = read_u64(data, str_sh + 32)? as usize;
            if str_off + str_sz <= data.len() {
                &data[str_off..str_off + str_sz]
            } else {
                &[]
            }
        } else {
            &[]
        };

        for i in 0..sh_count {
            let sh = sh_offset + i * sh_size;
            if sh + sh_size > data.len() { break; }

            let name_off = read_u32(data, sh)? as usize;
            let sh_type = read_u32(data, sh + 4)?;
            let sh_flags = read_u64(data, sh + 8)?;
            let sh_addr = read_u64(data, sh + 16)?;
            let sh_off = read_u64(data, sh + 24)? as usize;
            let sh_sz = read_u64(data, sh + 32)? as usize;
            let sh_link_val = read_u32(data, sh + 40)? as usize;

            // SHT_SYMTAB = 2
            if sh_type == 2 {
                symtab_off = sh_off;
                symtab_sz = sh_sz;
                symtab_link = sh_link_val;
            }

            let name = get_string(strtab, name_off);

            let is_alloc = sh_flags & 0x2 != 0;
            let is_exec = sh_flags & 0x4 != 0;
            let is_write = sh_flags & 0x1 != 0;

            if !is_alloc || sh_type == 0 { continue; }

            if is_exec && sh_off + sh_sz <= data.len() {
                code_sections.push(Section {
                    address: sh_addr,
                    data: data[sh_off..sh_off + sh_sz].to_vec(),
                });
            } else if !is_write && !is_exec {
                if name.starts_with(".rodata") || name == ".srodata" {
                    if sh_off + sh_sz <= data.len() {
                        ro_sections.push((sh_addr, sh_sz, Some(data[sh_off..sh_off + sh_sz].to_vec())));
                    }
                }
            } else if is_write {
                if sh_type == 8 { // SHT_NOBITS (.bss)
                    rw_sections.push((sh_addr, sh_sz, None));
                } else if sh_off + sh_sz <= data.len() {
                    rw_sections.push((sh_addr, sh_sz, Some(data[sh_off..sh_off + sh_sz].to_vec())));
                }
            }
        }

        let ro_data = build_data_blob(&ro_sections, 0x10000);
        let rw_data = build_data_blob_rw(&rw_sections, ro_data.len());

        // Parse ELF64 symbol table (24-byte entries)
        if symtab_sz > 0 && symtab_link < sh_count {
            let sym_strtab_sh = sh_offset + symtab_link * sh_size;
            if sym_strtab_sh + sh_size <= data.len() {
                let sym_strtab_off = read_u64(data, sym_strtab_sh + 24)? as usize;
                let sym_strtab_sz = read_u64(data, sym_strtab_sh + 32)? as usize;
                if sym_strtab_off + sym_strtab_sz <= data.len() {
                    let sym_strtab = &data[sym_strtab_off..sym_strtab_off + sym_strtab_sz];
                    // ELF64 symbol entry: 24 bytes
                    // [0..4] st_name, [4] st_info, [5] st_other, [6..8] st_shndx,
                    // [8..16] st_value, [16..24] st_size
                    let sym_count = symtab_sz / 24;
                    for j in 0..sym_count {
                        let sym = symtab_off + j * 24;
                        if sym + 24 > data.len() { break; }
                        let st_name = read_u32(data, sym)? as usize;
                        let st_info = data[sym + 4];
                        let st_value = read_u64(data, sym + 8)?;
                        let st_type = st_info & 0xF;
                        let st_bind = st_info >> 4;
                        if (st_type == 2 || st_type == 0) && (st_bind == 1 || st_bind == 2) && st_value != 0 {
                            let name = get_string(sym_strtab, st_name);
                            if !name.is_empty() && !name.starts_with('$') {
                                symbols.push((name.to_string(), st_value));
                            }
                        }
                    }
                }
            }
        }

        Ok(Elf {
            is_64bit: true,
            code_sections,
            ro_data,
            rw_data,
            heap_pages: 4,
            stack_size: 4096,
            entry_point,
            symbols,
        })
    }
}

/// Extract a null-terminated string from a string table.
fn get_string(strtab: &[u8], offset: usize) -> &str {
    if offset >= strtab.len() {
        return "";
    }
    let end = strtab[offset..].iter().position(|&b| b == 0).unwrap_or(strtab.len() - offset);
    std::str::from_utf8(&strtab[offset..offset + end]).unwrap_or("")
}

/// PVM zone size (ZZ = 2^16 = 65536).
const PVM_ZONE_SIZE: u64 = 1 << 16;

/// Zone-round: round up to next multiple of ZZ.
fn zone_round(x: u64) -> u64 {
    ((x + PVM_ZONE_SIZE - 1) / PVM_ZONE_SIZE) * PVM_ZONE_SIZE
}

/// Build a data blob for RO sections, placed at correct offsets relative to PVM base.
///
/// The PVM places RO data at ZZ (0x10000). Each section's virtual address in the ELF
/// must map to the same address in PVM. We build a byte array where index 0 corresponds
/// to PVM address `pvm_base`, and each section is placed at `section_addr - pvm_base`.
fn build_data_blob(sections: &[(u64, usize, Option<Vec<u8>>)], pvm_base: u64) -> Vec<u8> {
    if sections.is_empty() {
        return Vec::new();
    }

    // Find the address range spanned by all sections
    let min_addr = sections.iter().map(|(a, _, _)| *a).min().unwrap();
    let max_end = sections.iter().map(|(a, sz, _)| *a + *sz as u64).max().unwrap();

    // The blob covers [pvm_base .. max_end], but sections may start above pvm_base
    let blob_start = pvm_base.min(min_addr);
    let blob_size = (max_end - blob_start) as usize;
    let mut blob = vec![0u8; blob_size];

    for (addr, sz, data) in sections {
        let offset = (*addr - blob_start) as usize;
        match data {
            Some(d) => blob[offset..offset + sz].copy_from_slice(d),
            None => {} // BSS: already zero
        }
    }

    blob
}

/// Build a data blob for RW sections, placed at correct offsets relative to PVM RW base.
///
/// The PVM places RW data at `2*ZZ + Z(ro_size)`. We compute this base and place
/// sections relative to it.
fn build_data_blob_rw(sections: &[(u64, usize, Option<Vec<u8>>)], ro_size: usize) -> Vec<u8> {
    if sections.is_empty() {
        return Vec::new();
    }

    let rw_base = 2 * PVM_ZONE_SIZE + zone_round(ro_size as u64);
    build_data_blob(sections, rw_base)
}
