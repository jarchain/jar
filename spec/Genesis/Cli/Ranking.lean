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
  -- Use a single global variant for ranking (ranking is a property of the
  -- state, not per-commit). When v3 activates, all commits are reranked
  -- under v3 rules — there are no "v2 commits" in v3's eyes.
  let globalV := forcedV.getD
    (activeVariant (indices.getLast?.map (·.epoch) |>.getD 0))
  -- Build per-commit contexts (global variant + per-commit weight function)
  let (contexts, _) := signedCommits.zip indices |>.foldl
    (fun (ctxs, state) (_, idx) =>
      let ctx : RankingCommitCtx := { variant := globalV, getWeight := state.reviewerWeight }
      let nextState := @stepState (activeVariant idx.epoch) state idx
      (ctxs ++ [ctx], nextState)
    ) (([] : List RankingCommitCtx), initEvalState)
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
