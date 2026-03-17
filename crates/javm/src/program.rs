//! PVM program loading and initialization (JAR v0.8.0).
//!
//! Includes `deblob` for parsing program blobs and linear memory
//! initialization with basic block prevalidation.

use crate::instruction::Opcode;
use crate::memory::{Memory, PageAccess};
use crate::vm::Pvm;
use crate::{Gas, PVM_PAGE_SIZE};

/// Parse a program blob into (code, bitmask, jump_table) (eq A.2).
///
/// deblob(p) = (c, k, j) where:
///   p = E(|j|) ⌢ E₁(z) ⌢ E(|c|) ⌢ E_z(j) ⌢ E(c) ⌢ E(k), |k| = |c|
pub fn deblob(blob: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Vec<u32>)> {
    let mut offset = 0;

    // Read |j| (jump table length) as variable-length natural
    let (jt_len, n) = decode_natural(blob, offset)?;
    offset += n;

    // Read z (encoding size for jump table entries) as 1 byte
    if offset >= blob.len() {
        return None;
    }
    let z = blob[offset] as usize;
    offset += 1;

    // Read |c| (code length) as variable-length natural
    let (code_len, n) = decode_natural(blob, offset)?;
    offset += n;

    // Read jump table: jt_len entries, each z bytes LE
    let mut jump_table = Vec::with_capacity(jt_len);
    for _ in 0..jt_len {
        if offset + z > blob.len() {
            return None;
        }
        let mut val: u32 = 0;
        for i in 0..z {
            val |= (blob[offset + i] as u32) << (i * 8);
        }
        jump_table.push(val);
        offset += z;
    }

    // Read code: code_len bytes
    if offset + code_len > blob.len() {
        return None;
    }
    let code = blob[offset..offset + code_len].to_vec();
    offset += code_len;

    // Read bitmask: packed bitfield, ceil(code_len/8) bytes (eq C.9)
    let bitmask_bytes = (code_len + 7) / 8;
    if offset + bitmask_bytes > blob.len() {
        return None;
    }
    let packed_bitmask = &blob[offset..offset + bitmask_bytes];

    // Unpack packed bits to one byte per instruction (LSB first per byte)
    let mut bitmask = vec![0u8; code_len];
    for i in 0..code_len {
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        bitmask[i] = (packed_bitmask[byte_idx] >> bit_idx) & 1;
    }

    Some((code, bitmask, jump_table))
}

/// Program initialization with JAR v0.8.0 linear memory layout.
///
/// Contiguous layout: [stack | args | roData | rwData | heap | unmapped...]
/// All mapped pages are read-write. No guard zones.
pub fn initialize_program(program_blob: &[u8], arguments: &[u8], gas: Gas) -> Option<Pvm> {
    let blob = skip_metadata(program_blob);

    // Parse the standard program blob header:
    // E₃(|o|) ⌢ E₃(|w|) ⌢ E₂(z) ⌢ E₃(s) ⌢ o ⌢ w ⌢ E₄(|c|) ⌢ c
    if blob.len() < 15 {
        return None;
    }

    let mut offset = 0;

    let ro_size = read_le_u24(blob, &mut offset)? as u32;
    let rw_size = read_le_u24(blob, &mut offset)? as u32;
    let heap_pages = read_le_u16(blob, &mut offset)? as u32;
    let stack_size = read_le_u24(blob, &mut offset)? as u32;

    // Read read-only data
    if offset + ro_size as usize > blob.len() {
        return None;
    }
    let ro_data = &blob[offset..offset + ro_size as usize];
    offset += ro_size as usize;

    // Read read-write data
    if offset + rw_size as usize > blob.len() {
        return None;
    }
    let rw_data = &blob[offset..offset + rw_size as usize];
    offset += rw_size as usize;

    // Read E₄(|c|) — 4-byte LE code blob length
    let code_len = read_le_u32(blob, &mut offset)? as usize;
    if offset + code_len > blob.len() {
        return None;
    }
    let program_data = &blob[offset..offset + code_len];
    let (code, bitmask, jump_table) = deblob(program_data)?;

    // JAR v0.8.0: basic block prevalidation
    if !validate_basic_blocks(&code, &bitmask, &jump_table) {
        return None;
    }

    let page_round = |x: u32| -> u32 {
        ((x + PVM_PAGE_SIZE - 1) / PVM_PAGE_SIZE) * PVM_PAGE_SIZE
    };

    // Linear layout: stack | args | roData | rwData | heap
    let s = page_round(stack_size);              // stack: [0, s)
    let arg_start = s;                            // args:  [s, s + P(|a|))
    let ro_start = arg_start + page_round(arguments.len() as u32);
    let rw_start = ro_start + page_round(ro_size);
    let heap_start = rw_start + page_round(rw_size);
    let heap_end = heap_start + heap_pages * PVM_PAGE_SIZE;
    let mem_size = heap_end;

    // Check total fits in 32-bit address space
    if (mem_size as u64) > (1u64 << 32) {
        return None;
    }

    // Build memory — all mapped pages are ReadWrite
    let mut memory = Memory::new();
    map_region(&mut memory, 0, mem_size, PageAccess::ReadWrite);

    // Copy data into memory
    copy_data(&mut memory, arg_start, arguments);
    copy_data(&mut memory, ro_start, ro_data);
    copy_data(&mut memory, rw_start, rw_data);

    // Registers (JAR v0.8.0 linear)
    let mut registers = [0u64; 13];
    registers[0] = s as u64;                  // φ[0]: SP (top of stack)
    registers[1] = s as u64;                  // φ[1]: stack top
    registers[7] = arg_start as u64;          // φ[7]: argument base
    registers[8] = arguments.len() as u64;    // φ[8]: argument length

    tracing::info!(
        "PVM init (linear): stack=[0,{:#x}), args={:#x}+{}, ro={:#x}+{}, rw={:#x}+{}, heap={:#x}..{:#x}, SP={:#x}",
        s, arg_start, arguments.len(), ro_start, ro_size, rw_start, rw_size, heap_start, heap_end, registers[0]
    );

    let mut pvm = Pvm::new(code, bitmask, jump_table, registers, memory, gas);
    pvm.heap_base = heap_start;
    pvm.heap_top = heap_end;

    Some(pvm)
}

/// JAR v0.8.0 basic block prevalidation.
/// 1. Last instruction must be a terminator
/// 2. All jump table entries must point to valid instruction boundaries
fn validate_basic_blocks(code: &[u8], bitmask: &[u8], jump_table: &[u32]) -> bool {
    if code.is_empty() {
        return false;
    }
    // Find the last instruction start (scan backwards through bitmask)
    let mut last = code.len() - 1;
    while last > 0 && (last >= bitmask.len() || bitmask[last] != 1) {
        last -= 1;
    }
    // Check it's a valid terminator
    if last >= bitmask.len() || bitmask[last] != 1 {
        return false;
    }
    match Opcode::from_byte(code[last]) {
        Some(op) if op.is_terminator() => {}
        _ => return false,
    }
    // All jump table entries must point to instruction boundaries
    for &target in jump_table {
        let t = target as usize;
        if t != 0 && (t >= bitmask.len() || bitmask[t] != 1) {
            return false;
        }
    }
    true
}

/// Copy data bytes into memory at the given base address.
fn copy_data(memory: &mut Memory, base: u32, data: &[u8]) {
    for (i, &byte) in data.iter().enumerate() {
        memory.write_u8(base + i as u32, byte);
    }
}

/// Decode a variable-length natural number (JAM codec format).
/// Returns (value, bytes_consumed) or None.
fn decode_natural(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset >= data.len() {
        return None;
    }

    let first = data[offset];
    if first < 128 {
        // Single byte
        Some((first as usize, 1))
    } else if first < 192 {
        // Two bytes
        if offset + 2 > data.len() {
            return None;
        }
        let val = ((first as usize & 0x3F) << 8) | data[offset + 1] as usize;
        Some((val, 2))
    } else if first < 224 {
        // Three bytes: remaining 2 bytes in LE order
        if offset + 3 > data.len() {
            return None;
        }
        let val = ((first as usize & 0x1F) << 16)
            | ((data[offset + 2] as usize) << 8)
            | data[offset + 1] as usize;
        Some((val, 3))
    } else {
        // Four bytes: remaining 3 bytes in LE order
        if offset + 4 > data.len() {
            return None;
        }
        let val = ((first as usize & 0x0F) << 24)
            | ((data[offset + 3] as usize) << 16)
            | ((data[offset + 2] as usize) << 8)
            | data[offset + 1] as usize;
        Some((val, 4))
    }
}

/// Map a memory region with zero-filled pages.
fn map_region(memory: &mut Memory, base: u32, size: u32, access: PageAccess) {
    if size == 0 {
        return;
    }
    let start_page = base / PVM_PAGE_SIZE;
    let num_pages = (size + PVM_PAGE_SIZE - 1) / PVM_PAGE_SIZE;
    for i in 0..num_pages {
        memory.map_page(start_page + i, access);
    }
}

/// Map a memory region and copy data into it.
fn map_region_with_data(memory: &mut Memory, base: u32, data: &[u8], size: u32, access: PageAccess) {
    if size == 0 {
        return;
    }
    let start_page = base / PVM_PAGE_SIZE;
    let num_pages = (size + PVM_PAGE_SIZE - 1) / PVM_PAGE_SIZE;
    let page_size = PVM_PAGE_SIZE as usize;

    for i in 0..num_pages {
        let data_offset = i as usize * page_size;
        if data_offset < data.len() {
            let end = (data_offset + page_size).min(data.len());
            memory.map_page_with_data(start_page + i, access, &data[data_offset..end]);
        } else {
            memory.map_page(start_page + i, access);
        }
    }
}

fn read_le_u16(data: &[u8], offset: &mut usize) -> Option<u16> {
    if *offset + 2 > data.len() {
        return None;
    }
    let val = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    Some(val)
}

/// Skip metadata prefix from polkavm-linker output.
/// Detects metadata by checking if the first 3 bytes as E3(ro_size) would be too large.
fn skip_metadata(blob: &[u8]) -> &[u8] {
    if blob.len() < 14 {
        return blob;
    }
    // Try parsing as standard program header (first 3 bytes = E3(ro_size) LE)
    let ro_size = blob[0] as u32 | ((blob[1] as u32) << 8) | ((blob[2] as u32) << 16);
    if (ro_size as usize) + 14 <= blob.len() {
        // Looks like a valid standard program header
        return blob;
    }
    // Assume metadata: varint(length) prefix + metadata bytes
    if let Some((meta_len, consumed)) = decode_natural(blob, 0) {
        let skip = consumed + meta_len;
        if skip < blob.len() {
            return &blob[skip..];
        }
    }
    blob
}

fn read_le_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
    if *offset + 4 > data.len() {
        return None;
    }
    let val = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    Some(val)
}

fn read_le_u24(data: &[u8], offset: &mut usize) -> Option<u32> {
    if *offset + 3 > data.len() {
        return None;
    }
    let val = data[*offset] as u32 | ((data[*offset + 1] as u32) << 8) | ((data[*offset + 2] as u32) << 16);
    *offset += 3;
    Some(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deblob_simple() {
        // Build a simple blob: |j|=0, z=1, |c|=3, code=[0,1,0], bitmask packed
        let mut blob = Vec::new();
        blob.push(0); // |j| = 0 (single byte natural)
        blob.push(1); // z = 1
        blob.push(3); // |c| = 3
        // no jump table entries
        blob.extend_from_slice(&[0, 1, 0]); // code: trap, fallthrough, trap
        blob.push(0x07); // packed bitmask: bits 0,1,2 set = 0b00000111
        let (code, bitmask, jt) = deblob(&blob).unwrap();
        assert_eq!(code, vec![0, 1, 0]);
        assert_eq!(bitmask, vec![1, 1, 1]);
        assert!(jt.is_empty());
    }

    #[test]
    fn test_deblob_with_jump_table() {
        let mut blob = Vec::new();
        blob.push(2); // |j| = 2
        blob.push(2); // z = 2 (2-byte entries)
        blob.push(2); // |c| = 2
        blob.extend_from_slice(&[0, 0]); // j[0] = 0
        blob.extend_from_slice(&[1, 0]); // j[1] = 1
        blob.extend_from_slice(&[0, 1]); // code: trap, fallthrough
        blob.push(0x03); // packed bitmask: bits 0,1 set = 0b00000011
        let (code, bitmask, jt) = deblob(&blob).unwrap();
        assert_eq!(code, vec![0, 1]);
        assert_eq!(bitmask, vec![1, 1]);
        assert_eq!(jt, vec![0, 1]);
    }

    #[test]
    fn test_invalid_blob() {
        assert!(deblob(&[]).is_none());
        assert!(deblob(&[0]).is_none()); // missing z
    }
}
