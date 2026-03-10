//! PVM benchmark: grey interpreter vs recompiler vs polkavm.
//!
//! Two workloads:
//!   - fib: compute-intensive iterative Fibonacci (1M iterations)
//!   - hostcall: host-call-heavy (100K ecalli invocations)

use criterion::{criterion_group, criterion_main, Criterion};
use grey_bench::*;

const GAS_LIMIT: u64 = 100_000_000;

// ---------------------------------------------------------------------------
// Grey-PVM runners
// ---------------------------------------------------------------------------

fn run_grey_interpreter(blob: &[u8]) -> (u64, u64) {
    let mut pvm = grey_pvm::program::initialize_program(blob, &[], GAS_LIMIT).unwrap();
    loop {
        let (exit, _) = pvm.run();
        match exit {
            grey_pvm::ExitReason::Halt => break,
            grey_pvm::ExitReason::HostCall(_) => continue,
            other => panic!("unexpected exit: {:?}", other),
        }
    }
    let result = pvm.registers[7]; // A0
    let consumed = GAS_LIMIT - pvm.gas;
    (result, consumed)
}

fn run_grey_recompiler(blob: &[u8]) -> (u64, u64) {
    let mut rpvm =
        grey_pvm::recompiler::initialize_program_recompiled(blob, &[], GAS_LIMIT).unwrap();
    loop {
        let exit = rpvm.run();
        match exit {
            grey_pvm::ExitReason::Halt => break,
            grey_pvm::ExitReason::HostCall(_) => continue,
            other => panic!("unexpected exit: {:?}", other),
        }
    }
    let result = rpvm.registers()[7]; // A0
    let consumed = GAS_LIMIT - rpvm.gas();
    (result, consumed)
}

// ---------------------------------------------------------------------------
// PolkaVM runners
// ---------------------------------------------------------------------------

use polkavm::{BackendKind, Config, Engine, GasMeteringKind, InterruptKind, Module, ModuleConfig};
use polkavm_common::program::Reg as PReg;

fn try_make_polkavm_module(blob: &[u8], backend: BackendKind) -> Option<(Engine, Module)> {
    let mut config = Config::new();
    config.set_backend(Some(backend));
    config.set_allow_experimental(true);
    config.set_sandboxing_enabled(false);
    let engine = Engine::new(&config).ok()?;

    let mut mc = ModuleConfig::new();
    mc.set_gas_metering(Some(GasMeteringKind::Sync));
    let module = Module::new(&engine, &mc, blob.to_vec().into()).ok()?;
    Some((engine, module))
}

fn run_polkavm_module(module: &Module) -> (u64, i64) {
    let mut inst = module.instantiate().unwrap();
    inst.set_gas(GAS_LIMIT as i64);
    if let Some(export) = module.exports().next() {
        inst.set_next_program_counter(export.program_counter());
    }
    inst.set_reg(PReg::RA, 0xFFFF0000u64);
    loop {
        match inst.run().unwrap() {
            InterruptKind::Finished => break,
            InterruptKind::Ecalli(_) => continue,
            InterruptKind::Trap => panic!("polkavm trap"),
            InterruptKind::NotEnoughGas => panic!("polkavm out of gas"),
            other => panic!("polkavm unexpected: {:?}", other),
        }
    }
    (inst.reg(PReg::A0), inst.gas())
}

// ---------------------------------------------------------------------------
// Correctness validation
// ---------------------------------------------------------------------------

fn validate(name: &str, grey_blob: &[u8], pvm_blob: &[u8]) {
    let (gi_result, gi_gas) = run_grey_interpreter(grey_blob);
    let (gr_result, gr_gas) = run_grey_recompiler(grey_blob);
    assert_eq!(
        gi_result, gr_result,
        "{name}: interpreter/recompiler result mismatch"
    );
    assert_eq!(
        gi_gas, gr_gas,
        "{name}: interpreter/recompiler gas mismatch"
    );

    if let Some((_, pvm_module)) = try_make_polkavm_module(pvm_blob, BackendKind::Interpreter) {
        let (pvm_result, pvm_remaining) = run_polkavm_module(&pvm_module);
        let pvm_gas = GAS_LIMIT as i64 - pvm_remaining;
        eprintln!(
            "{name}: grey result={gi_result} gas={gi_gas}, polkavm result={pvm_result} gas={pvm_gas}"
        );
        assert_eq!(
            gi_result, pvm_result,
            "{name}: grey/polkavm result mismatch"
        );
        if gi_gas as i64 != pvm_gas {
            eprintln!(
                "  WARNING: gas mismatch grey={gi_gas} polkavm={pvm_gas} (delta={})",
                gi_gas as i64 - pvm_gas
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_fib(c: &mut Criterion) {
    let grey_blob = grey_fib_blob(FIB_N);
    let pvm_blob = polkavm_fib_blob(FIB_N);

    validate("fib", &grey_blob, &pvm_blob);

    let pvm_interp = try_make_polkavm_module(&pvm_blob, BackendKind::Interpreter);
    let pvm_compiler = try_make_polkavm_module(&pvm_blob, BackendKind::Compiler);

    let mut group = c.benchmark_group("fib");

    group.bench_function("grey-interpreter", |b| {
        b.iter(|| run_grey_interpreter(&grey_blob))
    });

    group.bench_function("grey-recompiler", |b| {
        b.iter(|| run_grey_recompiler(&grey_blob))
    });

    if let Some((_, ref m)) = pvm_interp {
        group.bench_function("polkavm-interpreter", |b| {
            b.iter(|| run_polkavm_module(m))
        });
    }

    if let Some((_, ref m)) = pvm_compiler {
        group.bench_function("polkavm-compiler", |b| {
            b.iter(|| run_polkavm_module(m))
        });
    }

    group.finish();
}

fn bench_hostcall(c: &mut Criterion) {
    let grey_blob = grey_hostcall_blob(HOSTCALL_N);
    let pvm_blob = polkavm_hostcall_blob(HOSTCALL_N);

    validate("hostcall", &grey_blob, &pvm_blob);

    let pvm_interp = try_make_polkavm_module(&pvm_blob, BackendKind::Interpreter);
    let pvm_compiler = try_make_polkavm_module(&pvm_blob, BackendKind::Compiler);

    let mut group = c.benchmark_group("hostcall");

    group.bench_function("grey-interpreter", |b| {
        b.iter(|| run_grey_interpreter(&grey_blob))
    });

    group.bench_function("grey-recompiler", |b| {
        b.iter(|| run_grey_recompiler(&grey_blob))
    });

    if let Some((_, ref m)) = pvm_interp {
        group.bench_function("polkavm-interpreter", |b| {
            b.iter(|| run_polkavm_module(m))
        });
    }

    if let Some((_, ref m)) = pvm_compiler {
        group.bench_function("polkavm-compiler", |b| {
            b.iter(|| run_polkavm_module(m))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_fib, bench_hostcall);
criterion_main!(benches);
