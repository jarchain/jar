//! RISC-V ELF to JAM PVM transpiler.
//!
//! Converts RISC-V rv32em/rv64em ELF binaries into PVM program blobs
//! suitable for execution by the Grey PVM (Appendix A of the Gray Paper).
//!
//! Also provides utilities to hand-assemble PVM programs directly.

pub mod elf;
pub mod riscv;
pub mod emitter;
pub mod assembler;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TranspileError {
    #[error("ELF parse error: {0}")]
    ElfParse(String),
    #[error("unsupported RISC-V instruction at offset {offset:#x}: {detail}")]
    UnsupportedInstruction { offset: usize, detail: String },
    #[error("unsupported relocation: {0}")]
    UnsupportedRelocation(String),
    #[error("register mapping error: RISC-V register {0} has no PVM equivalent")]
    RegisterMapping(u8),
    #[error("code too large: {0} bytes")]
    CodeTooLarge(usize),
    #[error("invalid section: {0}")]
    InvalidSection(String),
}

/// Transpile a RISC-V ELF binary into a PVM standard program blob.
///
/// The ELF must target rv32em or rv64em with no_std.
/// Returns the complete blob ready for `initialize_program()`.
pub fn transpile_elf(elf_data: &[u8]) -> Result<Vec<u8>, TranspileError> {
    let elf = elf::Elf::parse(elf_data)?;
    let mut ctx = riscv::TranslationContext::new(elf.is_64bit);

    // Translate all code sections
    for section in &elf.code_sections {
        ctx.translate_section(&section.data, section.address)?;
    }

    // Build the PVM blob with standard program header
    let blob = emitter::build_standard_program(
        &elf.ro_data,
        &elf.rw_data,
        elf.heap_pages,
        elf.stack_size,
        &ctx.code,
        &ctx.bitmask,
        &ctx.jump_table,
    );

    Ok(blob)
}
