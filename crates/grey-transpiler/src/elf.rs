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
        let entry_point = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as u64;
        let sh_offset = u32::from_le_bytes([data[32], data[33], data[34], data[35]]) as usize;
        let sh_size = u16::from_le_bytes([data[46], data[47]]) as usize;
        let sh_count = u16::from_le_bytes([data[48], data[49]]) as usize;
        let sh_strndx = u16::from_le_bytes([data[50], data[51]]) as usize;

        let mut code_sections = Vec::new();
        let mut ro_data = Vec::new();
        let mut rw_data = Vec::new();

        // Get string table
        let strtab = if sh_strndx < sh_count {
            let str_sh = sh_offset + sh_strndx * sh_size;
            let str_off = u32::from_le_bytes([data[str_sh + 16], data[str_sh + 17], data[str_sh + 18], data[str_sh + 19]]) as usize;
            let str_sz = u32::from_le_bytes([data[str_sh + 20], data[str_sh + 21], data[str_sh + 22], data[str_sh + 23]]) as usize;
            &data[str_off..str_off + str_sz]
        } else {
            &[]
        };

        for i in 0..sh_count {
            let sh = sh_offset + i * sh_size;
            if sh + sh_size > data.len() { break; }

            let name_off = u32::from_le_bytes([data[sh], data[sh + 1], data[sh + 2], data[sh + 3]]) as usize;
            let sh_type = u32::from_le_bytes([data[sh + 4], data[sh + 5], data[sh + 6], data[sh + 7]]);
            let sh_flags = u32::from_le_bytes([data[sh + 8], data[sh + 9], data[sh + 10], data[sh + 11]]);
            let sh_addr = u32::from_le_bytes([data[sh + 12], data[sh + 13], data[sh + 14], data[sh + 15]]) as u64;
            let sh_off = u32::from_le_bytes([data[sh + 16], data[sh + 17], data[sh + 18], data[sh + 19]]) as usize;
            let sh_sz = u32::from_le_bytes([data[sh + 20], data[sh + 21], data[sh + 22], data[sh + 23]]) as usize;

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
            } else if !is_write && !is_exec && sh_off + sh_sz <= data.len() {
                // Read-only data
                if name.starts_with(".rodata") || name == ".srodata" {
                    ro_data.extend_from_slice(&data[sh_off..sh_off + sh_sz]);
                }
            } else if is_write && sh_off + sh_sz <= data.len() {
                if sh_type == 8 { // SHT_NOBITS (.bss)
                    rw_data.extend(std::iter::repeat(0).take(sh_sz));
                } else {
                    rw_data.extend_from_slice(&data[sh_off..sh_off + sh_sz]);
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
        })
    }

    fn parse_elf64(data: &[u8]) -> Result<Self, TranspileError> {
        if data.len() < 64 {
            return Err(TranspileError::ElfParse("ELF64 header too small".into()));
        }

        let entry_point = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let sh_offset = u64::from_le_bytes(data[40..48].try_into().unwrap()) as usize;
        let sh_size = u16::from_le_bytes([data[58], data[59]]) as usize;
        let sh_count = u16::from_le_bytes([data[60], data[61]]) as usize;
        let sh_strndx = u16::from_le_bytes([data[62], data[63]]) as usize;

        let mut code_sections = Vec::new();
        let mut ro_data = Vec::new();
        let mut rw_data = Vec::new();

        // Get string table
        let strtab = if sh_strndx < sh_count && sh_offset + sh_strndx * sh_size + sh_size <= data.len() {
            let str_sh = sh_offset + sh_strndx * sh_size;
            let str_off = u64::from_le_bytes(data[str_sh + 24..str_sh + 32].try_into().unwrap()) as usize;
            let str_sz = u64::from_le_bytes(data[str_sh + 32..str_sh + 40].try_into().unwrap()) as usize;
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

            let name_off = u32::from_le_bytes(data[sh..sh + 4].try_into().unwrap()) as usize;
            let sh_type = u32::from_le_bytes(data[sh + 4..sh + 8].try_into().unwrap());
            let sh_flags = u64::from_le_bytes(data[sh + 8..sh + 16].try_into().unwrap());
            let sh_addr = u64::from_le_bytes(data[sh + 16..sh + 24].try_into().unwrap());
            let sh_off = u64::from_le_bytes(data[sh + 24..sh + 32].try_into().unwrap()) as usize;
            let sh_sz = u64::from_le_bytes(data[sh + 32..sh + 40].try_into().unwrap()) as usize;

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
            } else if !is_write && !is_exec && sh_off + sh_sz <= data.len() {
                if name.starts_with(".rodata") || name == ".srodata" {
                    ro_data.extend_from_slice(&data[sh_off..sh_off + sh_sz]);
                }
            } else if is_write && sh_off + sh_sz <= data.len() {
                if sh_type == 8 {
                    rw_data.extend(std::iter::repeat(0).take(sh_sz));
                } else {
                    rw_data.extend_from_slice(&data[sh_off..sh_off + sh_sz]);
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
