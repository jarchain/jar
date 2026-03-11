#!/usr/bin/env python3
"""
Generate Lean test files from authorizations JSON test vectors.

Usage:
  python3 tools/gen_authorizations_tests.py <test_vectors_dir> <output_lean_file>
"""

import json
import os
import sys
from pathlib import Path


def hex_to_lean(hex_str: str) -> str:
    """Convert 0x... hex string to Lean hexSeq call."""
    h = hex_str.removeprefix("0x")
    return f'hexSeq "{h}"'


def gen_hash_array(hashes: list, name: str) -> str:
    """Generate an Array Hash definition from hex strings."""
    if not hashes:
        return f"def {name} : Array Hash := #[]"
    items = ",\n    ".join(hex_to_lean(h) for h in hashes)
    return f"def {name} : Array Hash := #[\n    {items}]"


def gen_state(s: dict, name: str) -> str:
    """Generate FlatAuthState definition."""
    lines = []

    # Generate pool arrays
    for c, pool in enumerate(s["auth_pools"]):
        lines.append(gen_hash_array(pool, f"{name}_pool{c}"))
        lines.append("")

    # Generate queue arrays
    for c, queue in enumerate(s["auth_queues"]):
        lines.append(gen_hash_array(queue, f"{name}_queue{c}"))
        lines.append("")

    # State struct
    num_cores = len(s["auth_pools"])
    pool_refs = ", ".join(f"{name}_pool{c}" for c in range(num_cores))
    queue_refs = ", ".join(f"{name}_queue{c}" for c in range(num_cores))
    lines.append(f"def {name} : FlatAuthState := {{")
    lines.append(f"  authPools := #[{pool_refs}],")
    lines.append(f"  authQueues := #[{queue_refs}]")
    lines.append("}")
    return "\n".join(lines)


def gen_input(inp: dict, name: str) -> str:
    """Generate AuthInput definition."""
    lines = []
    lines.append(f"def {name} : AuthInput := {{")
    lines.append(f"  slot := {inp['slot']},")

    if not inp["auths"]:
        lines.append(f"  auths := #[]")
    else:
        auth_items = []
        for a in inp["auths"]:
            auth_items.append(
                f"{{ core := {a['core']}, authHash := {hex_to_lean(a['auth_hash'])} }}"
            )
        items = ",\n    ".join(auth_items)
        lines.append(f"  auths := #[\n    {items}]")

    lines.append("}")
    return "\n".join(lines)


def sanitize_name(filename: str) -> str:
    name = Path(filename).stem
    return name.replace("-", "_")


def check_arrays_equal(a: list, b: list) -> bool:
    return a == b


def generate_test_file(test_dir: str, output_file: str):
    json_files = sorted(f for f in os.listdir(test_dir) if f.endswith(".json"))

    if not json_files:
        print(f"No JSON files found in {test_dir}")
        sys.exit(1)

    print(f"Generating tests for {len(json_files)} test vectors...")

    lines = []
    lines.append("import Jar.Test.Authorizations")
    lines.append("")
    lines.append("/-! Auto-generated authorizations test vectors. Do not edit. -/")
    lines.append("")
    lines.append("namespace Jar.Test.AuthorizationsVectors")
    lines.append("")
    lines.append("open Jar Jar.Test.Authorizations")
    lines.append("")

    # hexSeq helper
    lines.append("def hexToBytes (s : String) : ByteArray :=")
    lines.append("  let chars := s.data")
    lines.append("  let nibble (c : Char) : UInt8 :=")
    lines.append("    if c.toNat >= 48 && c.toNat <= 57 then (c.toNat - 48).toUInt8")
    lines.append("    else if c.toNat >= 97 && c.toNat <= 102 then (c.toNat - 87).toUInt8")
    lines.append("    else if c.toNat >= 65 && c.toNat <= 70 then (c.toNat - 55).toUInt8")
    lines.append("    else 0")
    lines.append("  let rec go (cs : List Char) (acc : ByteArray) : ByteArray :=")
    lines.append("    match cs with")
    lines.append("    | hi :: lo :: rest => go rest (acc.push ((nibble hi <<< 4) ||| nibble lo))")
    lines.append("    | _ => acc")
    lines.append("  go chars ByteArray.empty")
    lines.append("")
    lines.append("def hexSeq (s : String) : OctetSeq n := ⟨hexToBytes s, sorry⟩")
    lines.append("")

    test_names = []
    for json_file in json_files:
        with open(os.path.join(test_dir, json_file)) as f:
            data = json.load(f)

        test_name = sanitize_name(json_file)
        test_names.append(test_name)

        lines.append(f"-- ============================================================================")
        lines.append(f"-- {json_file}")
        lines.append(f"-- ============================================================================")
        lines.append("")

        pre = data["pre_state"]
        post = data["post_state"]
        inp = data["input"]

        # Pre state
        lines.append(gen_state(pre, f"{test_name}_pre"))
        lines.append("")

        # Post state — check if queues are unchanged (they always are for this sub-transition)
        # For pools, check if any are the same as pre
        post_lines = []
        for c, pool in enumerate(post["auth_pools"]):
            if check_arrays_equal(pre["auth_pools"][c], pool):
                post_lines.append(
                    f"def {test_name}_post_pool{c} : Array Hash := {test_name}_pre_pool{c}"
                )
            else:
                post_lines.append(
                    gen_hash_array(pool, f"{test_name}_post_pool{c}")
                )
            post_lines.append("")

        lines.extend(post_lines)

        # Queue refs for post (always same as pre)
        num_cores = len(post["auth_pools"])
        pool_refs = ", ".join(f"{test_name}_post_pool{c}" for c in range(num_cores))
        queue_refs = ", ".join(f"{test_name}_pre_queue{c}" for c in range(num_cores))
        lines.append(f"def {test_name}_post : FlatAuthState := {{")
        lines.append(f"  authPools := #[{pool_refs}],")
        lines.append(f"  authQueues := #[{queue_refs}]")
        lines.append("}")
        lines.append("")

        # Input
        lines.append(gen_input(inp, f"{test_name}_input"))
        lines.append("")

    # Test runner
    lines.append("-- ============================================================================")
    lines.append("-- Test Runner")
    lines.append("-- ============================================================================")
    lines.append("")
    lines.append("end Jar.Test.AuthorizationsVectors")
    lines.append("")
    lines.append("open Jar.Test.Authorizations Jar.Test.AuthorizationsVectors in")
    lines.append("def main : IO Unit := do")
    lines.append('  IO.println "Running authorizations test vectors..."')
    lines.append("  let mut passed := (0 : Nat)")
    lines.append("  let mut failed := (0 : Nat)")

    for name in test_names:
        lines.append(
            f'  if (← runTest "{name}" {name}_pre {name}_input {name}_post)'
        )
        lines.append(f"  then passed := passed + 1")
        lines.append(f"  else failed := failed + 1")

    lines.append(
        f'  IO.println s!"Authorizations: {{passed}} passed, {{failed}} failed out of {len(test_names)}"'
    )
    lines.append("  if failed > 0 then")
    lines.append("    IO.Process.exit 1")

    with open(output_file, "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"Generated {output_file} with {len(test_names)} test cases")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <test_vectors_dir> <output_lean_file>")
        sys.exit(1)
    generate_test_file(sys.argv[1], sys.argv[2])
