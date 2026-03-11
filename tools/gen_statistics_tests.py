#!/usr/bin/env python3
"""
Generate Lean test files from statistics JSON test vectors.

Usage:
  python3 tools/gen_statistics_tests.py <test_vectors_dir> <output_lean_file>

Example:
  python3 tools/gen_statistics_tests.py \
    ../grey/res/testvectors/stf/statistics/tiny/ \
    Jar/Test/StatisticsVectors.lean
"""

import json
import os
import sys
from pathlib import Path


def gen_validator_record(r: dict) -> str:
    """Generate FlatValidatorRecord literal."""
    return (
        f"{{ blocks := {r['blocks']}, tickets := {r['tickets']}, "
        f"preImages := {r['pre_images']}, preImagesSize := {r['pre_images_size']}, "
        f"guarantees := {r['guarantees']}, assurances := {r['assurances']} }}"
    )


def gen_record_array(records: list, name: str) -> str:
    """Generate an Array FlatValidatorRecord definition."""
    items = ",\n    ".join(gen_validator_record(r) for r in records)
    return f"def {name} : Array FlatValidatorRecord := #[\n    {items}]"


def gen_state(s: dict, name: str) -> str:
    """Generate FlatStatisticsState definition."""
    lines = []
    lines.append(gen_record_array(s["vals_curr_stats"], f"{name}_curr"))
    lines.append("")
    lines.append(gen_record_array(s["vals_last_stats"], f"{name}_last"))
    lines.append("")
    lines.append(f"def {name} : FlatStatisticsState := {{")
    lines.append(f"  valsCurrStats := {name}_curr,")
    lines.append(f"  valsLastStats := {name}_last,")
    lines.append(f"  slot := {s['slot']}")
    lines.append("}")
    return "\n".join(lines)


def gen_input(inp: dict, name: str) -> str:
    """Generate StatsInput definition."""
    ext = inp["extrinsic"]

    ticket_count = len(ext["tickets"])

    # Preimage blob sizes
    preimage_sizes = []
    for p in ext["preimages"]:
        blob = p["blob"]
        size = (len(blob) - 2) // 2 if blob.startswith("0x") else len(blob) // 2
        preimage_sizes.append(str(size))

    # Guarantee signer validator indices (array of arrays)
    guarantee_signers = []
    for g in ext["guarantees"]:
        signers = [str(s["validator_index"]) for s in g["signatures"]]
        guarantee_signers.append(f"#[{', '.join(signers)}]")

    # Assurance validator indices
    assurance_validators = [str(a["validator_index"]) for a in ext["assurances"]]

    lines = []
    lines.append(f"def {name} : StatsInput := {{")
    lines.append(f"  slot := {inp['slot']},")
    lines.append(f"  authorIndex := {inp['author_index']},")
    lines.append(f"  extrinsic := {{")
    lines.append(f"    ticketCount := {ticket_count},")
    lines.append(f"    preimageSizes := #[{', '.join(preimage_sizes)}],")
    lines.append(f"    guaranteeSigners := #[{', '.join(guarantee_signers)}],")
    lines.append(f"    assuranceValidators := #[{', '.join(assurance_validators)}]")
    lines.append(f"  }}")
    lines.append("}")
    return "\n".join(lines)


def sanitize_name(filename: str) -> str:
    """Convert filename to a valid Lean identifier."""
    name = Path(filename).stem
    return name.replace("-", "_")


def generate_test_file(test_dir: str, output_file: str):
    """Generate the complete Lean test file."""
    json_files = sorted(f for f in os.listdir(test_dir) if f.endswith(".json"))

    if not json_files:
        print(f"No JSON files found in {test_dir}")
        sys.exit(1)

    print(f"Generating tests for {len(json_files)} test vectors...")

    lines = []
    lines.append("import Jar.Test.Statistics")
    lines.append("")
    lines.append("/-! Auto-generated statistics test vectors. Do not edit. -/")
    lines.append("")
    lines.append("namespace Jar.Test.StatisticsVectors")
    lines.append("")
    lines.append("open Jar.Test.Statistics")
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

        # Generate pre_state
        lines.append(gen_state(pre, f"{test_name}_pre"))
        lines.append("")

        # Generate post_state
        lines.append(gen_state(post, f"{test_name}_post"))
        lines.append("")

        # Generate input
        lines.append(gen_input(inp, f"{test_name}_input"))
        lines.append("")

    # Generate main runner
    lines.append("-- ============================================================================")
    lines.append("-- Test Runner")
    lines.append("-- ============================================================================")
    lines.append("")
    lines.append("end Jar.Test.StatisticsVectors")
    lines.append("")
    lines.append("open Jar.Test.Statistics Jar.Test.StatisticsVectors in")
    lines.append("def main : IO Unit := do")
    lines.append('  IO.println "Running statistics test vectors..."')
    lines.append("  let mut passed := (0 : Nat)")
    lines.append("  let mut failed := (0 : Nat)")

    for name in test_names:
        lines.append(f'  if (← runTest "{name}" {name}_pre {name}_input {name}_post)')
        lines.append(f"  then passed := passed + 1")
        lines.append(f"  else failed := failed + 1")

    lines.append(f'  IO.println s!"Statistics: {{passed}} passed, {{failed}} failed out of {len(test_names)}"')
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
