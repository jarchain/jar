#!/usr/bin/env python3
"""
Generate Lean test files from assurances JSON test vectors.

Usage:
  python3 tools/gen_assurances_tests.py <test_vectors_dir> <output_lean_file>
"""

import json
import os
import sys
from pathlib import Path


def hex_to_lean(hex_str: str) -> str:
    h = hex_str.removeprefix("0x")
    return f'hexSeq "{h}"'


def hex_to_bytes(hex_str: str) -> str:
    h = hex_str.removeprefix("0x")
    return f'hexToBytes "{h}"'


def gen_validator_key(vk: dict) -> str:
    bs = vk["bandersnatch"].removeprefix("0x")
    ed = vk["ed25519"].removeprefix("0x")
    bl = vk["bls"].removeprefix("0x")
    mt = vk["metadata"].removeprefix("0x")
    return f'mkVK "{bs}" "{ed}" "{bl}" "{mt}"'


def gen_avail_assignment(a) -> str:
    if a is None:
        return "none"
    pkg_hash = a["report"]["package_spec"]["hash"]
    core = a["report"]["core_index"]
    timeout = a["timeout"]
    return f"some {{ reportPackageHash := {hex_to_lean(pkg_hash)}, coreIndex := {core}, timeout := {timeout} }}"


def gen_avail_array(assignments: list, name: str) -> str:
    items = ",\n    ".join(gen_avail_assignment(a) for a in assignments)
    return f"def {name} : Array (Option TAAvailAssignment) := #[\n    {items}]"


def gen_assurance(a: dict, name_prefix: str, idx: int) -> (str, str):
    ref = f"{name_prefix}_assurance_{idx}"
    defn = (
        f"def {ref} : TAAssurance := {{\n"
        f"  anchor := {hex_to_lean(a['anchor'])},\n"
        f"  bitfield := {hex_to_bytes(a['bitfield'])},\n"
        f"  validatorIndex := {a['validator_index']},\n"
        f"  signature := {hex_to_lean(a['signature'])} }}"
    )
    return defn, ref


def gen_result(output: dict, name: str) -> str:
    if "err" in output:
        return f'def {name} : TAResult := .err "{output["err"]}"'
    if "ok" in output:
        reported = output["ok"].get("reported", [])
        cores = [str(r["core_index"]) for r in reported]
        if not cores:
            return f"def {name} : TAResult := .ok #[]"
        return f"def {name} : TAResult := .ok #[{', '.join(cores)}]"
    return f"def {name} : TAResult := .ok #[]"


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
    lines.append("import Jar.Test.Assurances")
    lines.append("")
    lines.append("/-! Auto-generated assurances test vectors. Do not edit. -/")
    lines.append("")
    lines.append("namespace Jar.Test.AssurancesVectors")
    lines.append("")
    lines.append("open Jar.Test.Assurances")
    lines.append("")

    # Helpers
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
    lines.append("def mkVK (bs ed bl mt : String) : ValidatorKey := {")
    lines.append("  bandersnatch := hexSeq bs,")
    lines.append("  ed25519 := hexSeq ed,")
    lines.append("  bls := hexSeq bl,")
    lines.append("  metadata := hexSeq mt }")
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
        output = data["output"]

        # Pre state
        lines.append(gen_avail_array(pre["avail_assignments"], f"{test_name}_pre_avail"))
        lines.append("")

        validators = pre["curr_validators"]
        vk_items = ",\n    ".join(gen_validator_key(vk) for vk in validators)
        lines.append(f"def {test_name}_pre_validators : Array ValidatorKey := #[\n    {vk_items}]")
        lines.append("")

        lines.append(f"def {test_name}_pre : TAState := {{")
        lines.append(f"  availAssignments := {test_name}_pre_avail,")
        lines.append(f"  currValidators := {test_name}_pre_validators")
        lines.append("}")
        lines.append("")

        # Post state (avail only)
        lines.append(gen_avail_array(post["avail_assignments"], f"{test_name}_post_avail"))
        lines.append("")

        # Input
        assurance_refs = []
        for i, a in enumerate(inp["assurances"]):
            defn, ref = gen_assurance(a, f"{test_name}_input", i)
            lines.append(defn)
            lines.append("")
            assurance_refs.append(ref)

        assurances_str = "#[" + ", ".join(assurance_refs) + "]" if assurance_refs else "#[]"
        lines.append(f"def {test_name}_input : TAInput := {{")
        lines.append(f"  assurances := {assurances_str},")
        lines.append(f"  slot := {inp['slot']},")
        lines.append(f"  parent := {hex_to_lean(inp['parent'])}")
        lines.append("}")
        lines.append("")

        # Expected result
        lines.append(gen_result(output, f"{test_name}_result"))
        lines.append("")

    # Test runner
    lines.append("-- ============================================================================")
    lines.append("-- Test Runner")
    lines.append("-- ============================================================================")
    lines.append("")
    lines.append("end Jar.Test.AssurancesVectors")
    lines.append("")
    lines.append("open Jar.Test.Assurances Jar.Test.AssurancesVectors in")
    lines.append("def main : IO Unit := do")
    lines.append('  IO.println "Running assurances test vectors..."')
    lines.append("  let mut passed := (0 : Nat)")
    lines.append("  let mut failed := (0 : Nat)")

    for name in test_names:
        lines.append(
            f'  if (← runTest "{name}" {name}_pre {name}_input {name}_result {name}_post_avail)'
        )
        lines.append(f"  then passed := passed + 1")
        lines.append(f"  else failed := failed + 1")

    lines.append(
        f'  IO.println s!"Assurances: {{passed}} passed, {{failed}} failed out of {len(test_names)}"'
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
