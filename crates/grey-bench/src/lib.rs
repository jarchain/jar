//! PVM benchmark programs and helpers.
//!
//! Provides two guest programs for benchmarking:
//! - `fib`: compute-intensive iterative Fibonacci
//! - `hostcall`: host-call-heavy with many ecalli invocations
//!
//! Each program is available in both grey-pvm blob format and polkavm blob format.

use grey_transpiler::assembler::{Assembler, Reg};

/// Number of Fibonacci iterations for the compute benchmark.
pub const FIB_N: u64 = 1_000_000;

/// Number of host-call rounds for the host-call benchmark.
pub const HOSTCALL_N: u64 = 100_000;

// ---------------------------------------------------------------------------
// Grey-PVM blob builders (using grey-transpiler assembler)
// ---------------------------------------------------------------------------

/// Build a compute-intensive Fibonacci program as a grey-pvm standard blob.
///
/// Computes fib(N) iteratively:
///   T0=0, T1=1, T2=counter
///   loop: S0 = T0+T1; T0=T1; T1=S0; T2++; if T2<N goto loop
///   result in A0 = T1
///   halt
pub fn grey_fib_blob(n: u64) -> Vec<u8> {
    let mut asm = Assembler::new();
    asm.set_stack_size(4096);
    asm.set_heap_pages(0);

    asm.load_imm_64(Reg::T0, 0);          // fib_prev = 0
    asm.load_imm_64(Reg::T1, 1);          // fib_curr = 1
    asm.load_imm_64(Reg::T2, 0);          // counter = 0
    asm.load_imm_64(Reg::S1, n);          // N

    // Jump forward to the loop body — this is a terminator, so the next
    // instruction becomes a basic-block start that the backward branch can
    // target.
    let jump_pc = asm.current_offset();
    asm.jump(5); // jump offset = 5 bytes (size of the jump instruction itself)

    let loop_pc = asm.current_offset();
    assert_eq!(loop_pc, jump_pc + 5); // sanity check
    asm.add_64(Reg::S0, Reg::T0, Reg::T1);   // temp = prev + curr
    asm.move_reg(Reg::T0, Reg::T1);           // prev = curr
    asm.move_reg(Reg::T1, Reg::S0);           // curr = temp
    asm.add_imm_64(Reg::T2, Reg::T2, 1);     // counter++

    let branch_pc = asm.current_offset();
    let rel_offset = (loop_pc as i64) - (branch_pc as i64);
    emit_branch_lt_u(&mut asm, Reg::T2, Reg::S1, rel_offset as i32);

    asm.move_reg(Reg::A0, Reg::T1);
    // Halt: jump_ind RA, 0 (RA=0xFFFF0000 from standard init)
    asm.jump_ind(Reg::RA, 0);

    asm.build()
}

/// Build a host-call-heavy program as a grey-pvm standard blob.
///
/// Repeatedly calls ecalli(0) N times, then halts.
pub fn grey_hostcall_blob(n: u64) -> Vec<u8> {
    let mut asm = Assembler::new();
    asm.set_stack_size(4096);
    asm.set_heap_pages(0);

    asm.load_imm_64(Reg::T0, 0);
    asm.load_imm_64(Reg::S1, n);

    // Jump forward to create a BB boundary for the loop target
    let jump_pc = asm.current_offset();
    asm.jump(5);

    let loop_pc = asm.current_offset();
    assert_eq!(loop_pc, jump_pc + 5);
    asm.ecalli(0);
    asm.add_imm_64(Reg::T0, Reg::T0, 1);

    let branch_pc = asm.current_offset();
    let rel_offset = (loop_pc as i64) - (branch_pc as i64);
    emit_branch_lt_u(&mut asm, Reg::T0, Reg::S1, rel_offset as i32);

    asm.move_reg(Reg::A0, Reg::T0);
    asm.jump_ind(Reg::RA, 0);

    asm.build()
}

fn emit_branch_lt_u(asm: &mut Assembler, ra: Reg, rb: Reg, rel_offset: i32) {
    asm.emit_raw(172, true);
    asm.emit_raw((ra as u8) | ((rb as u8) << 4), false);
    let bytes = rel_offset.to_le_bytes();
    for &b in &bytes {
        asm.emit_raw(b, false);
    }
}

// ---------------------------------------------------------------------------
// PolkaVM blob builders (using polkavm-common ProgramBlobBuilder)
// ---------------------------------------------------------------------------

use polkavm_common::program::{Instruction as PInst, Reg as PReg};
use polkavm_common::writer::ProgramBlobBuilder;

fn pr(reg: PReg) -> polkavm_common::program::RawReg { reg.into() }

/// Build the same Fibonacci program as a polkavm blob.
pub fn polkavm_fib_blob(n: u64) -> Vec<u8> {
    let isa = polkavm_common::program::InstructionSetKind::JamV1;
    let mut builder = ProgramBlobBuilder::new(isa);
    builder.set_stack_size(4096);

    let code = vec![
        // BB0: init
        PInst::load_imm64(pr(PReg::T0), 0),
        PInst::load_imm64(pr(PReg::T1), 1),
        PInst::load_imm64(pr(PReg::T2), 0),
        PInst::load_imm64(pr(PReg::S1), n),
        PInst::jump(1),

        // BB1: loop body
        PInst::add_64(pr(PReg::S0), pr(PReg::T0), pr(PReg::T1)),
        PInst::move_reg(pr(PReg::T0), pr(PReg::T1)),
        PInst::move_reg(pr(PReg::T1), pr(PReg::S0)),
        PInst::add_imm_64(pr(PReg::T2), pr(PReg::T2), 1),
        PInst::branch_less_unsigned(pr(PReg::T2), pr(PReg::S1), 1),

        // BB2: done
        PInst::move_reg(pr(PReg::A0), pr(PReg::T1)),
        PInst::jump_indirect(pr(PReg::RA), 0),
    ];

    builder.set_code(&code, &[]);
    builder.add_export_by_basic_block(0, b"main");
    builder.to_vec().expect("failed to build polkavm fib blob")
}

/// Build the same host-call-heavy program as a polkavm blob.
pub fn polkavm_hostcall_blob(n: u64) -> Vec<u8> {
    let isa = polkavm_common::program::InstructionSetKind::JamV1;
    let mut builder = ProgramBlobBuilder::new(isa);
    builder.set_stack_size(4096);
    builder.add_import(b"host_gas");

    let code = vec![
        // BB0: init
        PInst::load_imm64(pr(PReg::T0), 0),
        PInst::load_imm64(pr(PReg::S1), n),
        PInst::jump(1),

        // BB1: loop
        PInst::ecalli(0),
        PInst::add_imm_64(pr(PReg::T0), pr(PReg::T0), 1),
        PInst::branch_less_unsigned(pr(PReg::T0), pr(PReg::S1), 1),

        // BB2: done
        PInst::move_reg(pr(PReg::A0), pr(PReg::T0)),
        PInst::jump_indirect(pr(PReg::RA), 0),
    ];

    builder.set_code(&code, &[]);
    builder.add_export_by_basic_block(0, b"main");
    builder.to_vec().expect("failed to build polkavm hostcall blob")
}
