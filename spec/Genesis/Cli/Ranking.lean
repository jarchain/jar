/-
  genesis ranking CLI

  Computes the global quality ranking using 1/3 quantile reviewer selection
  (Sybil-resistant: same model as score derivation).

  Input:  {"signedCommits": [...], "indices": [...]}
  Output: {"ranking": ["hash1", "hash2", ...]}  (best to worst)
  For v3 (BT): also outputs "scores" with per-commit μ and σ².

  Reviewer weights are reconstructed from indices at each step.
  Use --force-variant to override the variant for all commits.
-/

import Genesis.Cli.Common

open Lean (Json ToJson toJson fromJson? FromJson)
open Genesis.Cli

/-- Resolve a variant name string to a GenesisVariant instance. -/
def resolveVariantName (name : String) : Option GenesisVariant :=
  match name with
  | "v1" => some GenesisVariant.v1
  | "v2" => some GenesisVariant.v2
  | "v3" => some GenesisVariant.v3
  | _ => none

/-- Core ranking logic. forceVariant overrides activeVariant when set. -/
def rankingMainWith (forceVariant : Option String := none) : IO UInt32 := runJsonPipe fun j => do
  let signedCommits ← IO.ofExcept (j.getObjValAs? (List SignedCommit) "signedCommits")
  let indices ← IO.ofExcept (j.getObjValAs? (List CommitIndex) "indices")
  -- Resolve forced variant (if any)
  let forcedV := forceVariant.bind resolveVariantName
  -- Build per-commit contexts (variant + weight function)
  let (contexts, _) := signedCommits.zip indices |>.foldl
    (fun (ctxs, pastIndices) (commit, idx) =>
      let state := reconstructState pastIndices
      let v := forcedV.getD (activeVariant commit.prCreatedAt)
      let ctx : RankingCommitCtx := { variant := v, getWeight := state.reviewerWeight }
      (ctxs ++ [ctx], pastIndices ++ [idx])
    ) (([] : List RankingCommitCtx), ([] : List CommitIndex))
  -- Check if BT is active (v3+) — if so, output scores with μ and σ²
  let useBT := contexts.getLast?.map (·.variant.useBradleyTerry) |>.getD false
  if useBT then
    let (ranking, btState) := computeRankingBTWithState signedCommits contexts
    let scores := ranking.map fun c =>
      let entry := btLookup btState c
      Json.mkObj [("commit", toJson c), ("mu", toJson entry.mu), ("sigma2", toJson entry.sigma2)]
    return Json.mkObj [
      ("ranking", toJson ranking),
      ("scores", Json.arr scores.toArray)
    ]
  else
    let ranking := computeRanking signedCommits contexts
    return Json.mkObj [("ranking", toJson ranking)]

def rankingMain : IO UInt32 := rankingMainWith
