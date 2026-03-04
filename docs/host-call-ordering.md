# Host-Call Check Ordering — Spec Ambiguity and Debugging Notes

This documents a conformance bug in Grey's accumulation host calls that caused
blocks 64-67 to fail. The root cause was incorrect ordering of checks in
`host_assign`, `host_designate`, and `host_eject`.

## The Bug

Grey's conformance target was producing mismatched state roots starting at block
64 (timeslot 63). Five KV pairs diverged between Grey and the Jamzig reference
implementation. The root cause: host calls were checking privilege/validation
conditions before attempting memory reads, when the Gray Paper requires memory
access first.

## Gray Paper Requirement

The Gray Paper defines host calls with an implicit ordering: memory reads come
first in the definition, which means if memory is inaccessible, the PVM should
PANIC (⚡) — regardless of whether other conditions (wrong core, wrong privilege,
invalid service) would also fail.

### `host_assign` (Ω_A, GP eq 4455-4468)

The GP defines the assignment host call as reading a queue of authorization hashes
from memory at `φ_8` (register ω8), then checking:
1. Memory read of `Q * 32` bytes from `φ_8` — if inaccessible → PANIC
2. Core index `φ_7 ≥ C` → return CORE sentinel
3. Caller is not the assigner for this core → return HUH
4. Target account doesn't exist → return WHO
5. Otherwise → OK, update auth queue

**Grey's bug**: Checked core index (CORE) and privilege (HUH) *before* reading
memory. When called with an unmapped address at `φ_8`, Grey returned HUH instead
of PANICking.

### `host_designate` (Ω_D, GP eq 4470-4482)

Same pattern: reads `V * 336` bytes of validator keys from `φ_7`, then checks
if the caller is the designator service.

**Grey's bug**: Checked `service_id != designate` before reading memory.

### `host_eject` (Ω_J, GP eq 4601-4621)

Reads a 32-byte hash from `φ_8`, then checks if the target service exists.

**Grey's bug**: Never read from `φ_8` at all. The memory read was completely
missing.

## Why This Matters

When a host call PANICs, the PVM rolls back to the "exceptional" context (the
last checkpoint). When it returns an error sentinel (HUH, WHO, CORE), the PVM
continues with the "regular" context. These produce completely different state
outcomes:

- **PANIC path**: All state changes since the last checkpoint are reverted. The
  guest program's error handler runs (or execution ends).
- **Error sentinel path**: State changes are preserved. The guest program sees
  the error code in ω7 and continues executing.

In the block 64 case, the guest program called `host_assign` with an intentionally
unmapped address (likely a programming pattern to test error handling). The correct
behavior was PANIC → rollback to checkpoint. Grey instead returned HUH → the
guest continued with incorrect state, causing 5 KV pair divergences that cascaded
through subsequent blocks.

## The Fix

For all three host calls, move the memory read before any other checks:

```rust
// CORRECT: read memory first, privilege checks second
fn host_assign(...) {
    let queue_bytes = match pvm.try_read_bytes(o_ptr, 32 * q_count) {
        Some(b) => b,
        None => return false, // page fault → PANIC
    };
    // Now check core, privilege, etc.
    if c >= config.core_count as u64 {
        pvm.set_reg(7, CORE);
        return true;
    }
    // ...
}
```

## General Rule

For any host call that reads from guest memory, the read MUST happen before all
other validation checks. The Gray Paper's definitions list the memory read first
in the equation, which establishes the evaluation order. This is consistent with
how physical hardware works: a memory access fault is detected at the hardware
level before any software logic runs.

**Pattern to follow**: In every `host_*` function, the first operation should be
`pvm.try_read_bytes()` (if the host call reads memory). If it returns `None`,
return `false` (PANIC). Only after successful memory read should you check
privileges, validate parameters, etc.

## Other Host Calls Checked

The following host calls were audited for the same pattern:

| Host Call     | Memory Read | Status |
|---------------|-------------|--------|
| gas (0)       | None        | OK — no memory access |
| fetch (1)     | Read+Write  | OK — reads/writes first |
| lookup (2)    | Write       | OK — writes first |
| read (3)      | Write       | OK — writes first |
| write (4)     | Read        | OK — reads first |
| info (5)      | Write       | OK — writes first |
| bless (10)    | Read        | OK — reads first |
| assign (15)   | Read        | **Fixed** |
| designate (16)| Read        | **Fixed** |
| checkpoint(17)| None        | OK — no memory access |
| new (18)      | Read        | OK — reads first |
| upgrade (19)  | None        | OK — no memory access |
| transfer (20) | Read        | OK — reads first |
| eject (21)    | Read        | **Fixed** (was missing entirely) |
| query (22)    | Write       | OK — writes first |
| solicit (23)  | None        | OK — no memory access |
| forget (24)   | None        | OK — no memory access |
| yield (25)    | Read        | OK — reads first |
| provide (26)  | Read        | Partial — early WHO return before read for invalid service |

## Impact

- Blocks affected: 64-67 (4 blocks recovered)
- Total conformance: 63/101 → 68/101 blocks passing
- Next failure: block 68 (uninvestigated, different root cause)
