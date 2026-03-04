# Conformance Testing

Grey validates correctness against the JAM specification by replaying pre-recorded
block traces through its conformance target binary (`grey-conform`) and comparing
state roots with expected values.

## Architecture

```
┌────────────┐     Unix socket      ┌──────────────┐
│  Fuzzer /   │ ──── fuzz-proto ──── │ grey-conform │
│  Replayer   │  (length-prefixed)   │  (target)    │
└────────────┘                       └──────────────┘
```

### Fuzz-proto v1 protocol

Messages are length-prefixed (4-byte LE length + body). The first byte of each
message is a discriminant:

| Disc | Name        | Direction       | Description                              |
|------|-------------|-----------------|------------------------------------------|
| 0x00 | PeerInfo    | Fuzzer → Target | Exchange capabilities (features, version) |
| 0x01 | Initialize  | Fuzzer → Target | Load initial state from KV pairs          |
| 0x02 | StateRoot   | Target → Fuzzer | Return state Merkle root (32 bytes)       |
| 0x03 | ImportBlock | Fuzzer → Target | Import a JAM-encoded block                |
| 0x04 | GetState    | Fuzzer → Target | Request full state dump for a header hash |
| 0x05 | State       | Target → Fuzzer | Return ancestry + state KV pairs          |
| 0xFF | Error       | Either          | Error message (UTF-8)                     |

### Configuration

By default, `grey-conform` uses **tiny configuration** (V=6, C=2, E=12).
Set `JAM_CONSTANTS=full` for full configuration (V=1023, C=341, E=600).

## Quick Start

```bash
# Build the conformance target
cargo build --release --bin grey-conform

# Start the server
./target/release/grey-conform /tmp/jam_target.sock &

# Replay a trace (basic, shows PASS/FAIL per block)
python3 scripts/replay_trace.py \
  res/conformance/fuzz-proto/examples/0.7.2/no_forks \
  /tmp/jam_target.sock

# All-in-one: starts server, replays trace, captures logs
python3 scripts/run_conform.py \
  res/conformance/fuzz-proto/examples/0.7.2/no_forks

# Compare state KV pairs at a specific block for debugging
python3 scripts/dump_state.py --block 8 \
  res/conformance/fuzz-proto/examples/0.7.2/no_forks
```

## Trace Files

Conformance traces live under `res/conformance/fuzz-proto/examples/`. Each trace
is a directory containing numbered message pairs:

```
00000000_fuzzer_peer_info.bin     00000000_target_peer_info.bin
00000001_fuzzer_initialize.bin    00000001_target_state_root.bin
00000002_fuzzer_import_block.bin  00000002_target_state_root.bin
...
```

Each `*_fuzzer_*.bin` file is sent to the target, and the response is compared
against the matching `*_target_*.bin` file.

JSON versions of each message are also provided (`*.json`) for human inspection.
These are useful for understanding block contents (guarantees, assurances,
prerequisites, work results).

## Scripts

### `scripts/replay_trace.py`

The main conformance replay tool. Sends all messages from a trace directory and
compares responses with expected values.

```bash
# Replay against a running server
python3 scripts/replay_trace.py [TRACE_DIR] [SOCKET_PATH]

# Defaults: res/conformance/.../no_forks, /tmp/jam_target.sock
```

### `scripts/run_conform.py`

All-in-one script that starts `grey-conform`, replays a trace, and captures the
server log. Useful for quick iteration:

```bash
python3 scripts/run_conform.py [TRACE_DIR] [--blocks N] [--log FILE]
```

Options:
- `--blocks N`: Stop after N blocks (default: all)
- `--log FILE`: Write server log to FILE (default: /tmp/grey_conform.log)

### `scripts/dump_state.py`

Dumps and compares state KV pairs at a specific block boundary. Sends messages
up to the given block, then requests a GetState dump over the protocol. Shows
each state component with its size and hash for debugging.

```bash
python3 scripts/dump_state.py --block 8 [TRACE_DIR]
```

## Debugging Process

### Step 1: Identify the Failing Block

Run the full trace to find where mismatches start:

```bash
python3 scripts/run_conform.py res/conformance/.../no_forks
```

Output shows PASS/FAIL per block. The first FAIL is the block to investigate.

### Step 2: Read the Block JSON

Examine the failing block's JSON trace to understand what the block contains:

```bash
cat res/conformance/.../no_forks/00000009_fuzzer_import_block.json | python3 -m json.tool
```

Key fields to check:
- `header.slot` — the timeslot
- `extrinsic.guarantees` — new work reports entering pending
- `extrinsic.assurances` — availability votes making reports accumulate-ready
- Report `context.prerequisites` — dependency hashes
- Report `segment_root_lookup` — segment import dependencies
- Report `results[].accumulate_gas` — gas budget per work item

### Step 3: Enable Debug Logging

Run with `RUST_LOG=debug` or `RUST_LOG=grey_state=info` to see accumulation
details (dependency resolution, PVM host calls, gas usage):

```bash
RUST_LOG=grey_state=info python3 scripts/run_conform.py ...
```

Key log messages to look for:
- `run_accumulation: N available reports` — how many reports are being accumulated
- `run_accumulate_pvm: service=X, gas=Y` — PVM invocation parameters
- `PVM HALT/PANIC: gas_used=X` — PVM execution result
- `host_call write/transfer/read` — state-mutating host calls

### Step 4: Dump State for Comparison

Use `dump_state.py` to see the state KV pairs at the failing block. Compare
component hashes between the last passing block and the failing block to
narrow down which state component diverged.

State components (keyed by first byte of 31-byte key):

| Key | Component           | Description                            |
|-----|---------------------|----------------------------------------|
|  1  | auth_pool           | Authorization pool (O entries per core)|
|  2  | auth_queue           | Authorization queue (Q entries)        |
|  3  | recent_blocks       | Recent block history (H entries)       |
|  4  | safrole             | Safrole consensus state                |
|  5  | judgments           | Dispute judgments                      |
|  6  | entropy             | Entropy accumulator (η)                |
|  7  | pending_validators  | Next epoch validator keys              |
|  8  | current_validators  | Current epoch validator keys           |
|  9  | previous_validators | Previous epoch validator keys          |
| 10  | pending_reports     | Guaranteed reports awaiting assurance  |
| 11  | timeslot            | Current timeslot (τ)                   |
| 12  | privileged          | Privileged service IDs (χ)             |
| 13  | statistics          | Validator activity statistics (π)      |
| 14  | accumulation_queue  | Ready queue for deferred reports (ω)   |
| 15  | accumulation_history| Accumulated package hashes (ξ)         |
| 16  | accumulation_outputs| Per-service yield outputs (θ)          |
| 255 | service_account(S)  | Service S account data                 |

### Step 5: PVM Execution Debugging

For PVM-level issues, capture a detailed host-call trace:

```bash
RUST_LOG=grey_state::accumulate=info python3 scripts/run_conform.py ...
```

Each host call is logged with register values (ω7-ω12), gas before/after,
and return values. Compare with the Gray Paper's host-call specifications
(Appendix B) to verify correctness.

For instruction-level debugging, the PVM supports trace mode that logs every
instruction executed. See [docs/pvm-sbrk.md](pvm-sbrk.md) for an example of
how instruction-level tracing was used to find a 4-gas discrepancy.

### `scripts/compare_with_ref.py`

Compares Grey's state KV pairs against a reference implementation (Jamzig) at a
specific block boundary. Runs both targets in parallel, dumps state from each,
and shows mismatched/added/removed keys with decoded service account fields.

```bash
python3 scripts/compare_with_ref.py 64 --trace res/conformance/.../no_forks
```

This is the most powerful debugging tool for finding exactly which state components
diverge. For service accounts, it decodes all fields (balance, gas limits, items,
creation_slot, etc.) and highlights individual field differences.

### `scripts/compare_states.py`

Compares Grey's own state between two block boundaries to see what changed in a
single transition. Useful for understanding what a specific block's accumulation did.

```bash
python3 scripts/compare_states.py --before 63 --after 64 --hex
```

## Advanced Debugging: Host-Call Tracing

When the state divergence is caused by PVM execution differences, use detailed
host-call tracing to compare the execution sequence with a reference.

### Capturing Host-Call Traces

Run with verbose accumulation logging:

```bash
RUST_LOG=grey_state::accumulate=debug python3 scripts/run_conform.py \
  --blocks 65 res/conformance/.../no_forks
```

Each host call is logged with:
- Host call ID and name
- Input registers (ω7-ω12)
- Output register (ω7) / return value
- Gas before and after

### Interpreting Host-Call Sequences

A typical accumulation host-call sequence:

1. `gas(0)` — query remaining gas
2. `fetch(1)` — read config values (timeslot, core count, etc.)
3. `info(5)` — query service account info
4. `read(3)` / `write(4)` — storage operations
5. `checkpoint(17)` — save state snapshot (regular ← exceptional)
6. `assign(15)` / `designate(16)` — privileged operations
7. PVM HALT or PANIC

When a PANIC occurs after a checkpoint, the exceptional context (saved at
checkpoint) is restored. This means all state changes between the last checkpoint
and the PANIC are reverted.

### Key Insight: Check Ordering Matters

Host calls that read from guest memory MUST attempt the read before any validation
checks. If memory is inaccessible, the PVM PANICs — and this takes priority over
returning error sentinels like HUH or CORE. See [host-call-ordering.md](host-call-ordering.md)
for a detailed case study.

## Common Failure Modes

### "No accumulation" (output_hash = 0x000...0)

Reports with prerequisites or segment_root_lookup entries are "queued" rather
than "immediate." Their dependencies must be resolved against the accumulated
history ⊜(ξ) before they can be accumulated. If dependency resolution fails,
no PVM runs and the output hash is all zeros.

Fixed in commit `d086ddb`: apply `E(R^Q, ⊜(ξ))` to strip already-satisfied
dependencies from new queued reports per Gray Paper eq 12.5.

### PVM PANIC at a specific PC

A PVM PANIC uses the "exceptional" context (rolls back to the last checkpoint).
This is normal behavior — guest programs may deliberately panic for error
handling. Check whether the PANIC is expected by examining the host-call
sequence leading up to it. If a host call returns an unexpected value, the
guest program may trap.

### Gas mismatch

Small gas differences (1-10 instructions) usually indicate a PVM instruction
implementation bug. Use instruction-level tracing to compare execution paths.
See [docs/pvm-sbrk.md](pvm-sbrk.md) for a detailed case study.

### State root mismatch with correct accumulation

If the PVM runs and produces correct side-effects but the state root doesn't
match, the issue is likely in state serialization (`T(σ)`) or Merklization.
Use `dump_state.py` to compare individual state components.

## Conformance Status

See [README.md](../README.md) for current pass/fail counts.
