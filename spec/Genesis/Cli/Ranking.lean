/-
  genesis_ranking CLI

  Computes the global quality ranking using 1/3 quantile reviewer selection
  (Sybil-resistant: same model as score derivation).

  Input:  {"signedCommits": [...], "indices": [...]}
  Output: {"ranking": ["hash1", "hash2", ...]}  (best to worst)

  Reviewer weights are reconstructed from indices at each step.
-/

import Genesis.Cli.Common

open Lean (Json ToJson toJson fromJson? FromJson)
open Genesis.Cli

def rankingMain : IO UInt32 := runJsonPipe fun j => do
  let signedCommits ← IO.ofExcept (j.getObjValAs? (List SignedCommit) "signedCommits")
  let indices ← IO.ofExcept (j.getObjValAs? (List CommitIndex) "indices")
  -- Build per-commit contexts (variant + weight function)
  let (contexts, _) := signedCommits.zip indices |>.foldl
    (fun (ctxs, pastIndices) (commit, idx) =>
      let state := reconstructState pastIndices
      let v := activeVariant commit.prCreatedAt
      let ctx : RankingCommitCtx := { variant := v, getWeight := state.reviewerWeight }
      (ctxs ++ [ctx], pastIndices ++ [idx])
    ) (([] : List RankingCommitCtx), ([] : List CommitIndex))
  -- Check if BT is active (v3+) — if so, output variances too
  let useBT := contexts.getLast?.map (·.variant.useBradleyTerry) |>.getD false
  if useBT then
    let (ranking, btState) := computeRankingBTWithState signedCommits contexts
    let variances := btState.map fun (c, e) => (c, e.sigma2)
    return Json.mkObj [
      ("ranking", toJson ranking),
      ("variances", toJson variances)
    ]
  else
    let ranking := computeRanking signedCommits contexts
    return Json.mkObj [("ranking", toJson ranking)]
