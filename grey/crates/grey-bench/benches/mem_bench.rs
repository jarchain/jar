//! Memory cache pressure benchmark.
//!
//! Measures how PVM load instruction throughput degrades as the working set
//! grows beyond L1 → L2 → L3 → DRAM. Two access patterns:
//!   - `mem_seq`: sequential sweep (prefetch-friendly, best case)
//!   - `mem_rand`: pseudo-random stride (cache-hostile, worst case)
//!
//! Run: `cargo bench -p grey-bench --features javm/signals --bench mem_bench`

use criterion::{Criterion, criterion_group, criterion_main};
use grey_bench::mem::*;

/// Compute gas limit proportional to working set size.
fn gas_for_size(size_bytes: u64) -> u64 {
    let n_elems = size_bytes / 4;
    let loads = n_elems * 15; // SWEEPS
    loads * 100 + 10_000_000
}

const SIZES: &[(&str, u64)] = &[
    ("4K", 4 * 1024),
    ("32K", 32 * 1024),
    ("256K", 256 * 1024),
    ("1M", 1024 * 1024),
    ("8M", 8 * 1024 * 1024),
    ("32M", 32 * 1024 * 1024),
    ("128M", 128 * 1024 * 1024),
    ("256M", 256 * 1024 * 1024),
    ("1G", 1024 * 1024 * 1024),
    ("2G", 2 * 1024 * 1024 * 1024),
    ("3G", 3 * 1024 * 1024 * 1024), // ~3GB (leave room for stack + guard zones)
];

/// Initialize a recompiler PVM for the given blob + size.
/// For sizes > u16::MAX pages (256MB), expands heap_top after init.
fn init_pvm(blob: &[u8], size_bytes: u64) -> javm::recompiler::RecompiledPvm {
    let gas = gas_for_size(size_bytes);
    let mut pvm = javm::recompiler::initialize_program_recompiled(blob, &[], gas).unwrap();
    let desired_top = (HEAP_BASE + size_bytes) as u32;
    if desired_top > pvm.heap_top() {
        pvm.set_heap_top(desired_top);
    }
    pvm
}

fn bench_mem_seq(c: &mut Criterion) {
    for &(label, size) in SIZES {
        let blob = grey_mem_seq_blob(size);

        let mut group = c.benchmark_group(format!("mem_seq/{label}"));
        if size >= 8 * 1024 * 1024 {
            group.sample_size(10);
        }
        group.bench_function("grey-recompiler-exec", |b| {
            b.iter_batched(
                || init_pvm(&blob, size),
                |mut pvm| {
                    loop {
                        match pvm.run() {
                            javm::ExitReason::Halt => break,
                            javm::ExitReason::HostCall(_) => continue,
                            other => panic!("unexpected exit: {:?}", other),
                        }
                    }
                    pvm.registers()[7]
                },
                criterion::BatchSize::LargeInput,
            );
        });
        group.finish();
    }
}

fn bench_mem_rand(c: &mut Criterion) {
    for &(label, size) in SIZES {
        let blob = grey_mem_rand_blob(size);

        let mut group = c.benchmark_group(format!("mem_rand/{label}"));
        if size >= 8 * 1024 * 1024 {
            group.sample_size(10);
        }
        group.bench_function("grey-recompiler-exec", |b| {
            b.iter_batched(
                || init_pvm(&blob, size),
                |mut pvm| {
                    loop {
                        match pvm.run() {
                            javm::ExitReason::Halt => break,
                            javm::ExitReason::HostCall(_) => continue,
                            other => panic!("unexpected exit: {:?}", other),
                        }
                    }
                    pvm.registers()[7]
                },
                criterion::BatchSize::LargeInput,
            );
        });
        group.finish();
    }
}

criterion_group!(mem_benches, bench_mem_seq, bench_mem_rand);
criterion_main!(mem_benches);
