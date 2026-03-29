//! Sieve of Eratosthenes benchmark — counts primes up to N.
//!
//! Compiled to RISC-V, then transpiled to PVM bytecode for both grey and
//! polkavm benchmarks. Exercises memory-heavy access patterns with
//! data-dependent branching.

#![no_std]
#![no_main]

const N: usize = 100_000;

// Entry point for grey PVM (starts execution at PC=0).
#[cfg(not(target_env = "polkavm"))]
core::arch::global_asm!(
    ".global _start",
    "_start:",
    "call prime_sieve",
    "lui t0, 0xffff0",
    "jr t0",
);

// polkavm needs a _start symbol to link but doesn't use it.
#[cfg(target_env = "polkavm")]
core::arch::global_asm!(
    ".global _start",
    "_start:",
    "unimp",
);

/// Sieve of Eratosthenes: count primes up to N.
/// Returns the prime count (π(100000) = 9592).
#[cfg_attr(target_env = "polkavm", polkavm_derive::polkavm_export)]
#[no_mangle]
pub extern "C" fn prime_sieve() -> u32 {
    // Use static mut to avoid 100KB stack allocation — PVM stack is limited.
    // Initialized to 1 (prime) — reset to 0 for composites during sieve.
    static mut IS_PRIME: [u8; N] = [1; N];

    let is_prime = unsafe { &mut IS_PRIME };
    // Re-initialize each call (static persists across calls in the same instance)
    let mut i: usize = 0;
    while i < N {
        is_prime[i] = 1;
        i += 1;
    }
    is_prime[0] = 0;
    if N > 1 {
        is_prime[1] = 0;
    }

    // Sieve: mark multiples of each prime starting from p*p
    let mut p: usize = 2;
    while p * p < N {
        if is_prime[p] != 0 {
            let mut j = p * p;
            while j < N {
                is_prime[j] = 0;
                j += p;
            }
        }
        p += 1;
    }

    // Count primes
    let mut count: u32 = 0;
    let mut i: usize = 0;
    while i < N {
        count += is_prime[i] as u32;
        i += 1;
    }
    count
}

// ---------------------------------------------------------------------------
// Compiler builtins required for no_std RISC-V
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn memset(dst: *mut u8, val: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        unsafe { *dst.add(i) = val as u8; }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        unsafe { *dst.add(i) = *src.add(i); }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = unsafe { *s1.add(i) };
        let b = unsafe { *s2.add(i) };
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!(
            "li a0, 0xDEAD",
            "unimp",
            options(noreturn),
        );
    }
}
