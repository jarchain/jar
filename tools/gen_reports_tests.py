#!/usr/bin/env python3
"""
Generate Lean test files from reports JSON test vectors.

Usage:
  python3 tools/gen_reports_tests.py <test_vectors_dir> <output_lean_file>
"""

import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from jam_codec import work_report_hash


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


def gen_work_result(r: dict) -> str:
    result = r["result"]
    if isinstance(result, dict):
        if "ok" in result:
            return f".ok ({hex_to_bytes(result['ok'])})"
        if "err" in result:
            err = result["err"]
            if err == "out_of_gas": return ".outOfGas"
            if err == "panic": return ".panic"
            if err == "bad_exports": return ".badExports"
            if err == "bad_code": return ".badCode"
            if err == "code_oversize": return ".codeOversize"
    return ".ok ByteArray.empty"


def gen_work_digest(d: dict, prefix: str, idx: int) -> (str, str):
    ref = f"{prefix}_digest_{idx}"
    rl = d.get("refine_load", {})
    defn = (
        f"def {ref} : TRWorkDigest := {{\n"
        f"  serviceId := {d['service_id']},\n"
        f"  codeHash := {hex_to_lean(d['code_hash'])},\n"
        f"  payloadHash := {hex_to_lean(d['payload_hash'])},\n"
        f"  accumulateGas := {d['accumulate_gas']},\n"
        f"  result := {gen_work_result(d)},\n"
        f"  gasUsed := {rl.get('gas_used', 0)},\n"
        f"  imports := {rl.get('imports', 0)},\n"
        f"  extrinsicCount := {rl.get('extrinsic_count', 0)},\n"
        f"  extrinsicSize := {rl.get('extrinsic_size', 0)},\n"
        f"  exports := {rl.get('exports', 0)} }}"
    )
    return defn, ref


def gen_avail_spec(spec: dict) -> str:
    return (
        f"{{ packageHash := {hex_to_lean(spec['hash'])},\n"
        f"      bundleLength := {spec['length']},\n"
        f"      erasureRoot := {hex_to_lean(spec['erasure_root'])},\n"
        f"      exportsRoot := {hex_to_lean(spec['exports_root'])},\n"
        f"      exportsCount := {spec['exports_count']} }}"
    )


def gen_context(ctx: dict) -> str:
    prereqs = ", ".join(hex_to_lean(p) for p in ctx.get("prerequisites", []))
    return (
        f"{{ anchor := {hex_to_lean(ctx['anchor'])},\n"
        f"      stateRoot := {hex_to_lean(ctx['state_root'])},\n"
        f"      beefyRoot := {hex_to_lean(ctx['beefy_root'])},\n"
        f"      lookupAnchor := {hex_to_lean(ctx['lookup_anchor'])},\n"
        f"      lookupAnchorSlot := {ctx['lookup_anchor_slot']},\n"
        f"      prerequisites := #[{prereqs}] }}"
    )


def gen_segment_root_lookup(srl: list) -> str:
    if not srl:
        return "#[]"
    items = ", ".join(
        f"({hex_to_lean(e[0] if isinstance(e, list) else e['work_package_hash'])}, "
        f"{hex_to_lean(e[1] if isinstance(e, list) else e['segment_tree_root'])})"
        for e in srl
    )
    return f"#[{items}]"


def gen_work_report(report: dict, prefix: str) -> (str, str):
    ref = f"{prefix}_report"
    lines = []

    # Digests
    digest_refs = []
    for i, d in enumerate(report["results"]):
        defn, dref = gen_work_digest(d, prefix, i)
        lines.append(defn)
        lines.append("")
        digest_refs.append(dref)
    digests_str = "#[" + ", ".join(digest_refs) + "]" if digest_refs else "#[]"

    srl = gen_segment_root_lookup(report.get("segment_root_lookup", []))

    # Extract packageSpec and context as separate defs to avoid nested struct parse issues
    spec = report['package_spec']
    spec_ref = f"{prefix}_spec"
    lines.append(f"def {spec_ref} : TRAvailSpec := {{")
    lines.append(f"  packageHash := {hex_to_lean(spec['hash'])},")
    lines.append(f"  bundleLength := {spec['length']},")
    lines.append(f"  erasureRoot := {hex_to_lean(spec['erasure_root'])},")
    lines.append(f"  exportsRoot := {hex_to_lean(spec['exports_root'])},")
    lines.append(f"  exportsCount := {spec['exports_count']} }}")
    lines.append("")

    ctx = report['context']
    ctx_ref = f"{prefix}_ctx"
    prereqs = ", ".join(hex_to_lean(p) for p in ctx.get("prerequisites", []))
    lines.append(f"def {ctx_ref} : TRContext := {{")
    lines.append(f"  anchor := {hex_to_lean(ctx['anchor'])},")
    lines.append(f"  stateRoot := {hex_to_lean(ctx['state_root'])},")
    lines.append(f"  beefyRoot := {hex_to_lean(ctx['beefy_root'])},")
    lines.append(f"  lookupAnchor := {hex_to_lean(ctx['lookup_anchor'])},")
    lines.append(f"  lookupAnchorSlot := {ctx['lookup_anchor_slot']},")
    lines.append(f"  prerequisites := #[{prereqs}] }}")
    lines.append("")

    lines.append(f"def {ref} : TRWorkReport := {{")
    lines.append(f"  packageSpec := {spec_ref},")
    lines.append(f"  context := {ctx_ref},")
    lines.append(f"  coreIndex := {report['core_index']},")
    lines.append(f"  authorizerHash := {hex_to_lean(report['authorizer_hash'])},")
    lines.append(f"  authGasUsed := {report['auth_gas_used']},")
    lines.append(f"  authOutput := {hex_to_bytes(report['auth_output'])},")
    lines.append(f"  segmentRootLookup := {srl},")
    lines.append(f"  results := {digests_str} }}")
    return "\n".join(lines), ref


def gen_guarantee(g: dict, prefix: str, idx: int) -> (str, str):
    ref = f"{prefix}_guarantee_{idx}"
    lines = []

    # Work report
    report_text, report_ref = gen_work_report(g["report"], f"{prefix}_g{idx}")
    lines.append(report_text)
    lines.append("")

    # Signatures
    sig_strs = []
    for s in g["signatures"]:
        sig_strs.append(
            f"{{ validatorIndex := {s['validator_index']}, "
            f"signature := {hex_to_lean(s['signature'])} }}"
        )
    sigs_str = "#[" + ",\n    ".join(sig_strs) + "]" if sig_strs else "#[]"

    # Pre-computed report hash
    rh = work_report_hash(g["report"])
    rh_lean = hex_to_lean("0x" + rh.hex())

    lines.append(f"def {ref} : TRGuarantee := {{")
    lines.append(f"  report := {report_ref},")
    lines.append(f"  slot := {g['slot']},")
    lines.append(f"  signatures := {sigs_str},")
    lines.append(f"  reportHash := {rh_lean} }}")
    return "\n".join(lines), ref


def gen_recent_block(b: dict, prefix: str, idx: int) -> (str, str):
    ref = f"{prefix}_block_{idx}"
    reported = []
    for r in b.get("reported", []):
        if isinstance(r, dict):
            h = r.get("hash", r.get("package_hash", "0x" + "00"*32))
            er = r.get("exports_root", "0x" + "00"*32)
            reported.append(f"({hex_to_lean(h)}, {hex_to_lean(er)})")
        elif isinstance(r, (list, tuple)):
            reported.append(f"({hex_to_lean(r[0])}, {hex_to_lean(r[1])})")
    reported_str = "#[" + ", ".join(reported) + "]" if reported else "#[]"
    defn = (
        f"def {ref} : TRRecentBlock := {{\n"
        f"  headerHash := {hex_to_lean(b['header_hash'])},\n"
        f"  stateRoot := {hex_to_lean(b['state_root'])},\n"
        f"  beefyRoot := {hex_to_lean(b['beefy_root'])},\n"
        f"  reported := {reported_str} }}"
    )
    return defn, ref


def gen_avail_assignment(a) -> str:
    if a is None:
        return "none"
    pkg_hash = a["report"]["package_spec"]["hash"]
    timeout = a["timeout"]
    return f"some {{ packageHash := {hex_to_lean(pkg_hash)}, timeout := {timeout} }}"


def gen_service_info(acct: dict) -> str:
    sid = acct["id"]
    data = acct.get("data", acct)
    svc = data.get("service", data)
    code_hash = svc.get("code_hash", "0x" + "00" * 32)
    min_gas = svc.get("min_item_gas", 0)
    return (
        f"{{ serviceId := {sid}, "
        f"codeHash := {hex_to_lean(code_hash)}, "
        f"minItemGas := {min_gas} }}"
    )


def gen_result(output: dict, name: str) -> str:
    if "err" in output:
        return f'def {name} : TRResult := .err "{output["err"]}"'
    return f"def {name} : TRResult := .ok"


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
    lines.append("import Jar.Test.Reports")
    lines.append("")
    lines.append("/-! Auto-generated reports test vectors. Do not edit. -/")
    lines.append("")
    lines.append("namespace Jar.Test.ReportsVectors")
    lines.append("")
    lines.append("open Jar.Test.Reports")
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

        # --- Pre state ---

        # Avail assignments
        avail_items = ", ".join(gen_avail_assignment(a) for a in pre["avail_assignments"])
        lines.append(f"def {test_name}_pre_avail : Array (Option TRAvailAssignment) := #[{avail_items}]")
        lines.append("")

        # Validators
        vk_items = ",\n    ".join(gen_validator_key(vk) for vk in pre["curr_validators"])
        lines.append(f"def {test_name}_pre_curr_vk : Array ValidatorKey := #[\n    {vk_items}]")
        lines.append("")

        pvk_items = ",\n    ".join(gen_validator_key(vk) for vk in pre["prev_validators"])
        lines.append(f"def {test_name}_pre_prev_vk : Array ValidatorKey := #[\n    {pvk_items}]")
        lines.append("")

        # Entropy (4 values, could be array or list)
        entropy = pre["entropy"]
        ent_items = ", ".join(hex_to_lean(e) for e in entropy)
        lines.append(f"def {test_name}_pre_entropy : Array Hash := #[{ent_items}]")
        lines.append("")

        # Offenders
        off_items = ", ".join(hex_to_lean(o) for o in pre.get("offenders", []))
        lines.append(f"def {test_name}_pre_offenders : Array Ed25519PublicKey := #[{off_items}]")
        lines.append("")

        # Recent blocks (stored as {"history": [...], "mmr": [...]})
        recent = pre["recent_blocks"]
        history = recent["history"] if isinstance(recent, dict) else recent
        block_refs = []
        for i, b in enumerate(history):
            defn, ref = gen_recent_block(b, f"{test_name}_pre", i)
            lines.append(defn)
            lines.append("")
            block_refs.append(ref)
        blocks_str = "#[" + ", ".join(block_refs) + "]" if block_refs else "#[]"

        # Auth pools
        pool_items = []
        for pool in pre["auth_pools"]:
            items = ", ".join(hex_to_lean(h) for h in pool)
            pool_items.append(f"#[{items}]")
        pools_str = "#[" + ", ".join(pool_items) + "]"
        lines.append(f"def {test_name}_pre_auth_pools : Array (Array Hash) := {pools_str}")
        lines.append("")

        # Service accounts
        svc_items = ", ".join(gen_service_info(a) for a in pre.get("accounts", []))
        lines.append(f"def {test_name}_pre_accounts : Array TRServiceInfo := #[{svc_items}]")
        lines.append("")

        # Pre state struct
        lines.append(f"def {test_name}_pre : TRState := {{")
        lines.append(f"  availAssignments := {test_name}_pre_avail,")
        lines.append(f"  currValidators := {test_name}_pre_curr_vk,")
        lines.append(f"  prevValidators := {test_name}_pre_prev_vk,")
        lines.append(f"  entropy := {test_name}_pre_entropy,")
        lines.append(f"  offenders := {test_name}_pre_offenders,")
        lines.append(f"  recentBlocks := {blocks_str},")
        lines.append(f"  authPools := {test_name}_pre_auth_pools,")
        lines.append(f"  accounts := {test_name}_pre_accounts")
        lines.append("}")
        lines.append("")

        # --- Post state (avail only) ---
        post_avail_items = ", ".join(gen_avail_assignment(a) for a in post["avail_assignments"])
        lines.append(f"def {test_name}_post_avail : Array (Option TRAvailAssignment) := #[{post_avail_items}]")
        lines.append("")

        # --- Input ---
        guarantee_refs = []
        for i, g in enumerate(inp["guarantees"]):
            defn, ref = gen_guarantee(g, f"{test_name}_input", i)
            lines.append(defn)
            lines.append("")
            guarantee_refs.append(ref)
        guarantees_str = "#[" + ", ".join(guarantee_refs) + "]" if guarantee_refs else "#[]"

        kp_items = ", ".join(hex_to_lean(p) for p in inp.get("known_packages", []))
        lines.append(f"def {test_name}_input : TRInput := {{")
        lines.append(f"  guarantees := {guarantees_str},")
        lines.append(f"  knownPackages := #[{kp_items}],")
        lines.append(f"  slot := {inp['slot']}")
        lines.append("}")
        lines.append("")

        # --- Expected result ---
        lines.append(gen_result(output, f"{test_name}_result"))
        lines.append("")

    # Test runner
    lines.append("-- ============================================================================")
    lines.append("-- Test Runner")
    lines.append("-- ============================================================================")
    lines.append("")
    lines.append("end Jar.Test.ReportsVectors")
    lines.append("")
    lines.append("open Jar.Test.Reports Jar.Test.ReportsVectors in")
    lines.append("def main : IO Unit := do")
    lines.append('  IO.println "Running reports test vectors..."')
    lines.append("  let mut passed := (0 : Nat)")
    lines.append("  let mut failed := (0 : Nat)")

    for name in test_names:
        lines.append(
            f'  if (← runTest "{name}" {name}_pre {name}_input {name}_result {name}_post_avail)'
        )
        lines.append(f"  then passed := passed + 1")
        lines.append(f"  else failed := failed + 1")

    lines.append(
        f'  IO.println s!"Reports: {{passed}} passed, {{failed}} failed out of {len(test_names)}"'
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
