#!/usr/bin/env python3
"""Compare state KV pairs between two blocks to find divergent components.

Dumps state at two block boundaries and shows which KV pairs changed.
This helps identify which state component is causing a root mismatch.

Usage:
    python3 scripts/compare_states.py --before 8 --after 9
"""

import argparse
import hashlib
import os
import signal
import socket
import struct
import subprocess
import sys
import time
from pathlib import Path

DEFAULT_TRACE = "res/conformance/fuzz-proto/examples/0.7.2/no_forks"
BINARY = "target/release/grey-conform"

KEY_NAMES = {
    1: "alpha(auth_pool)",
    2: "phi(auth_queue)",
    3: "beta(recent_blocks)",
    4: "gamma(safrole)",
    5: "psi(judgments)",
    6: "eta(entropy)",
    7: "iota(pending_validators)",
    8: "kappa(current_validators)",
    9: "lambda(previous_validators)",
    10: "rho(pending_reports)",
    11: "tau(timeslot)",
    12: "chi(privileged)",
    13: "pi(statistics)",
    14: "omega(accum_queue)",
    15: "xi(accum_history)",
    16: "theta(accum_outputs)",
}


def send_msg(sock, data):
    sock.sendall(struct.pack("<I", len(data)) + data)


def recv_msg(sock, timeout=120):
    sock.settimeout(timeout)
    hdr = b""
    while len(hdr) < 4:
        chunk = sock.recv(4 - len(hdr))
        if not chunk:
            return None
        hdr += chunk
    length = struct.unpack("<I", hdr)[0]
    data = b""
    while len(data) < length:
        chunk = sock.recv(min(65536, length - len(data)))
        if not chunk:
            return None
        data += chunk
    return data


def read_compact(data, pos):
    if pos >= len(data):
        return 0, pos
    header = data[pos]
    pos += 1
    length = 0
    tmp = header
    while tmp & 0x80:
        length += 1
        tmp = (tmp << 1) & 0xFF
    if length == 8:
        val = int.from_bytes(data[pos:pos + 8], "little")
        pos += 8
        return val, pos
    threshold = 0 if length == 0 else 256 - (1 << (8 - length))
    header_value = header - threshold
    low = 0
    for i in range(length):
        low |= data[pos] << (8 * i)
        pos += 1
    return (header_value << (8 * length)) | low, pos


def parse_state_response(data):
    if not data or data[0] != 0x05:
        if data and data[0] == 0xFF:
            msg = data[1:].decode("utf-8", errors="replace")[:200]
            print(f"Error response: {msg}")
        return None
    pos = 1
    count, pos = read_compact(data, pos)
    kvs = {}
    for _ in range(count):
        key = bytes(data[pos:pos + 31])
        pos += 31
        vlen, pos = read_compact(data, pos)
        val = bytes(data[pos:pos + vlen])
        pos += vlen
        kvs[key] = val
    return kvs


def key_name(key_bytes):
    if key_bytes[1:] == b"\x00" * 30:
        idx = key_bytes[0]
        return KEY_NAMES.get(idx, f"C({idx})")
    if key_bytes[0] == 255:
        sid = (key_bytes[1] | (key_bytes[3] << 8) |
               (key_bytes[5] << 16) | (key_bytes[7] << 24))
        return f"C(255,{sid})"
    sid = (key_bytes[0] | (key_bytes[2] << 8) |
           (key_bytes[4] << 16) | (key_bytes[6] << 24))
    sub = key_bytes[1:]
    # Check if it's a storage/preimage key
    return f"C({sid},sub={sub[:8].hex()}...)"


def dump_state_at_block(sock_path, trace_dir, block_num):
    """Connect, replay trace up to block_num, request state dump."""
    fuzzer_files = sorted(trace_dir.glob("*_fuzzer_*.bin"))
    n_msgs = min(block_num + 2, len(fuzzer_files))

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(120)
    s.connect(sock_path)

    state_root = None
    for i, fuzz_file in enumerate(fuzzer_files[:n_msgs]):
        request = fuzz_file.read_bytes()
        send_msg(s, request)
        resp = recv_msg(s)
        if resp is None:
            s.close()
            return None, None
        if resp[0] == 0xFF:
            msg = resp[1:].decode("utf-8", errors="replace")[:200]
            print(f"  ERROR at msg {i}: {msg}")
            s.close()
            return None, None
        if resp[0] == 0x02:
            state_root = resp[1:33].hex()

    # Get header hash from next block's parent hash
    if n_msgs < len(fuzzer_files):
        next_block = fuzzer_files[n_msgs].read_bytes()
        if next_block[0] == 0x03:
            header_hash = next_block[1:33]
        else:
            s.close()
            return None, state_root
    else:
        s.close()
        return None, state_root

    # Request GetState
    send_msg(s, bytes([0x04]) + header_hash)
    resp = recv_msg(s, timeout=120)
    kvs = parse_state_response(resp)
    s.close()
    return kvs, state_root


def main():
    parser = argparse.ArgumentParser(description="Compare state between blocks")
    parser.add_argument("--before", type=int, required=True, help="Block number for 'before' state")
    parser.add_argument("--after", type=int, required=True, help="Block number for 'after' state")
    parser.add_argument("--trace", default=DEFAULT_TRACE, help="Trace directory")
    parser.add_argument("--hex", action="store_true", help="Show hex of changed values")
    args = parser.parse_args()

    trace_dir = Path(args.trace)
    if not trace_dir.exists():
        print(f"Error: trace dir {trace_dir} not found", file=sys.stderr)
        sys.exit(1)

    # Read expected state roots from target files
    target_files = sorted(trace_dir.glob("*_target_*.bin"))
    expected_before = None
    expected_after = None
    for tf in target_files:
        seq = int(tf.name[:8])
        data = tf.read_bytes()
        if data[0] == 0x02:
            if seq == args.before + 1:  # +1 because msg 0=peer_info, 1=initialize
                expected_before = data[1:33].hex()
            elif seq == args.after + 1:
                expected_after = data[1:33].hex()

    if expected_before:
        print(f"Block {args.before} expected root: {expected_before}")
    if expected_after:
        print(f"Block {args.after} expected root:  {expected_after}")

    # Build binary
    binary = BINARY
    if not os.path.exists(binary):
        print("Building grey-conform...")
        subprocess.run(
            ["cargo", "build", "--release", "--bin", "grey-conform"],
            check=True, capture_output=True)

    # We need two separate server instances since each starts from genesis
    results = {}
    for label, block_num in [("before", args.before), ("after", args.after)]:
        sock_path = f"/tmp/grey_cmp_{label}_{os.getpid()}.sock"
        try:
            os.unlink(sock_path)
        except FileNotFoundError:
            pass

        proc = subprocess.Popen(
            [binary, sock_path],
            stdout=subprocess.DEVNULL,
            stderr=open(f"/tmp/grey_cmp_{label}.log", "w"))
        for _ in range(20):
            if os.path.exists(sock_path):
                break
            time.sleep(0.1)

        print(f"\nDumping state at block {block_num}...")
        try:
            kvs, our_root = dump_state_at_block(sock_path, trace_dir, block_num)
            results[label] = (kvs, our_root)
            if our_root:
                match = "MATCH" if our_root == (expected_before if label == "before" else expected_after) else "MISMATCH"
                print(f"  Our root:      {our_root} [{match}]")
        finally:
            proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
            try:
                os.unlink(sock_path)
            except FileNotFoundError:
                pass

    kvs_before, _ = results.get("before", (None, None))
    kvs_after, _ = results.get("after", (None, None))

    if kvs_before is None or kvs_after is None:
        print("\nFailed to get state for one or both blocks")
        sys.exit(1)

    # Compare
    all_keys = sorted(set(list(kvs_before.keys()) + list(kvs_after.keys())))

    print(f"\n{'='*80}")
    print(f"State comparison: block {args.before} → block {args.after}")
    print(f"Before: {len(kvs_before)} KV pairs, After: {len(kvs_after)} KV pairs")
    print(f"{'='*80}")

    added = []
    removed = []
    changed = []
    unchanged = []

    for key in all_keys:
        name = key_name(key)
        in_before = key in kvs_before
        in_after = key in kvs_after

        if not in_before and in_after:
            added.append((name, key, kvs_after[key]))
        elif in_before and not in_after:
            removed.append((name, key, kvs_before[key]))
        elif kvs_before[key] != kvs_after[key]:
            changed.append((name, key, kvs_before[key], kvs_after[key]))
        else:
            unchanged.append(name)

    if unchanged:
        print(f"\nUnchanged ({len(unchanged)}):")
        for name in unchanged:
            print(f"  {name}")

    if added:
        print(f"\nAdded ({len(added)}):")
        for name, key, val in added:
            h = hashlib.blake2b(val, digest_size=32).hexdigest()[:32]
            print(f"  + {name}: {len(val)} bytes, hash={h}")
            if args.hex and len(val) <= 256:
                print(f"    hex: {val.hex()}")

    if removed:
        print(f"\nRemoved ({len(removed)}):")
        for name, key, val in removed:
            print(f"  - {name}: {len(val)} bytes")

    if changed:
        print(f"\nChanged ({len(changed)}):")
        for name, key, old, new in changed:
            h_old = hashlib.blake2b(old, digest_size=32).hexdigest()[:32]
            h_new = hashlib.blake2b(new, digest_size=32).hexdigest()[:32]
            print(f"  ~ {name}: {len(old)} → {len(new)} bytes")
            print(f"    old hash: {h_old}")
            print(f"    new hash: {h_new}")
            if args.hex:
                if len(old) <= 512 and len(new) <= 512:
                    print(f"    old: {old.hex()}")
                    print(f"    new: {new.hex()}")
                elif name.startswith("C(255"):
                    # Always show service accounts
                    print(f"    old: {old.hex()}")
                    print(f"    new: {new.hex()}")
                else:
                    # Show first differing byte
                    for i in range(min(len(old), len(new))):
                        if old[i] != new[i]:
                            print(f"    first diff at byte {i}: 0x{old[i]:02x} → 0x{new[i]:02x}")
                            ctx = max(0, i - 8)
                            print(f"    old[{ctx}:{i+16}]: {old[ctx:i+16].hex()}")
                            print(f"    new[{ctx}:{i+16}]: {new[ctx:i+16].hex()}")
                            break

    print(f"\nSummary: {len(unchanged)} unchanged, {len(added)} added, {len(removed)} removed, {len(changed)} changed")


if __name__ == "__main__":
    main()
