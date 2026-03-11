#!/usr/bin/env python3
"""
Generate Lean test files from history JSON test vectors.

Usage:
  python3 tools/gen_history_tests.py <test_vectors_dir> <output_lean_file>
"""

import json
import os
import sys
from pathlib import Path


def hex_to_lean(hex_str: str) -> str:
    h = hex_str.removeprefix("0x")
    return f'hexSeq "{h}"'


def gen_reported_pkg(wp: dict) -> str:
    return (
        f'{{ hash := {hex_to_lean(wp["hash"])}, '
        f'exportsRoot := {hex_to_lean(wp["exports_root"])} }}'
    )


def gen_history_entry(entry: dict) -> str:
    reported = entry["reported"]
    if not reported:
        rep_str = "#[]"
    else:
        items = ", ".join(gen_reported_pkg(wp) for wp in reported)
        rep_str = f"#[{items}]"
    return (
        f'{{ headerHash := {hex_to_lean(entry["header_hash"])},\n'
        f'       beefyRoot := {hex_to_lean(entry["beefy_root"])},\n'
        f'       stateRoot := {hex_to_lean(entry["state_root"])},\n'
        f'       reported := {rep_str} }}'
    )


def gen_mmr_peak(peak) -> str:
    if peak is None:
        return "none"
    return f"some ({hex_to_lean(peak)})"


def gen_state(beta: dict, name: str) -> str:
    lines = []
    # History entries
    history = beta["history"]
    if not history:
        lines.append(f"def {name}_history : Array HistoryEntry := #[]")
    else:
        items = ",\n    ".join(gen_history_entry(e) for e in history)
        lines.append(f"def {name}_history : Array HistoryEntry := #[\n    {items}]")
    lines.append("")

    # MMR peaks
    peaks = beta["mmr"]["peaks"]
    if not peaks:
        lines.append(f"def {name}_peaks : Array (Option Hash) := #[]")
    else:
        items = ", ".join(gen_mmr_peak(p) for p in peaks)
        lines.append(f"def {name}_peaks : Array (Option Hash) := #[{items}]")
    lines.append("")

    lines.append(f"def {name} : FlatHistoryState := {{")
    lines.append(f"  history := {name}_history,")
    lines.append(f"  mmrPeaks := {name}_peaks")
    lines.append("}")
    return "\n".join(lines)


def gen_input(inp: dict, name: str) -> str:
    lines = []
    lines.append(f"def {name} : HistoryInput := {{")
    lines.append(f"  headerHash := {hex_to_lean(inp['header_hash'])},")
    lines.append(f"  parentStateRoot := {hex_to_lean(inp['parent_state_root'])},")
    lines.append(f"  accumulateRoot := {hex_to_lean(inp['accumulate_root'])},")
    if not inp["work_packages"]:
        lines.append(f"  workPackages := #[]")
    else:
        items = ",\n    ".join(gen_reported_pkg(wp) for wp in inp["work_packages"])
        lines.append(f"  workPackages := #[\n    {items}]")
    lines.append("}")
    return "\n".join(lines)


def sanitize_name(filename: str) -> str:
    name = Path(filename).stem
    return name.replace("-", "_")


def generate_test_file(test_dir: str, output_file: str):
    json_files = sorted(f for f in os.listdir(test_dir) if f.endswith(".json"))

    if not json_files:
        print(f"No JSON files found in {test_dir}")
        sys.exit(1)

    print(f"Generating tests for {len(json_files)} test vectors...")

    lines = []
    lines.append("import Jar.Test.History")
    lines.append("")
    lines.append("/-! Auto-generated history test vectors. Do not edit. -/")
    lines.append("")
    lines.append("namespace Jar.Test.HistoryVectors")
    lines.append("")
    lines.append("open Jar Jar.Test.History")
    lines.append("")

    # hexSeq helper
    lines.append("def hexToBytes (s : String) : ByteArray :=")
    lines.append("  let chars := s.toList")
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

        pre = data["pre_state"]["beta"]
        post = data["post_state"]["beta"]
        inp = data["input"]

        lines.append(gen_state(pre, f"{test_name}_pre"))
        lines.append("")
        lines.append(gen_state(post, f"{test_name}_post"))
        lines.append("")
        lines.append(gen_input(inp, f"{test_name}_input"))
        lines.append("")

    # Test runner
    lines.append("-- ============================================================================")
    lines.append("-- Test Runner")
    lines.append("-- ============================================================================")
    lines.append("")
    lines.append("end Jar.Test.HistoryVectors")
    lines.append("")
    lines.append("open Jar.Test.History Jar.Test.HistoryVectors in")
    lines.append("def main : IO Unit := do")
    lines.append('  IO.println "Running history test vectors..."')
    lines.append("  let mut passed := (0 : Nat)")
    lines.append("  let mut failed := (0 : Nat)")

    for name in test_names:
        lines.append(
            f'  if (← runTest "{name}" {name}_pre {name}_input {name}_post)'
        )
        lines.append(f"  then passed := passed + 1")
        lines.append(f"  else failed := failed + 1")

    lines.append(
        f'  IO.println s!"History: {{passed}} passed, {{failed}} failed out of {len(test_names)}"'
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
