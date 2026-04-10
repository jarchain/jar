/-
  genesis_validate CLI

  Input:  {"indices": [...], "signedCommits": [...],
           "rankings": {...} (required for v2+),
           "scores": {...} (required for v3)}
  Output: {"valid": bool, "errors": [...]}

  Re-evaluates each signed commit against prior indices and checks
  that the stored CommitIndex matches. For v2 commits, the rankings
  map is REQUIRED for target validation. For v3 commits, the scores
  map is also REQUIRED (variances are derived from it).
-/

import Genesis.Cli.Common

open Lean (Json ToJson toJson fromJson? FromJson)
open Genesis.Cli

/-- Look up the ranking snapshot for a commit: find the latest prior commit
    in the rankings map (by walking pastIndices in reverse). -/
def lookupRanking (pastIndices : List CommitIndex) (rankings : Lean.Json)
    (prCreatedAt : Epoch) : Option (List CommitId) :=
  let prior := pastIndices.filter (fun idx => idx.epoch < prCreatedAt)
  match prior.getLast? with
  | none => none
  | some lastIdx =>
    (rankings.getObjValAs? (List CommitId) (toString lastIdx.commitHash)).toOption

/-- Look up variances from the scores map: find the latest prior commit,
    extract (commitId, sigma2) pairs from its scores entry. -/
def lookupVariances (pastIndices : List CommitIndex) (scoresJson : Lean.Json)
    (prCreatedAt : Epoch) : Option (List (CommitId × Nat)) :=
  let prior := pastIndices.filter (fun idx => idx.epoch < prCreatedAt)
  match prior.getLast? with
  | none => none
  | some lastIdx =>
    match scoresJson.getObjValAs? (List Json) (toString lastIdx.commitHash) with
    | .ok scores =>
      some (scores.filterMap fun s =>
        match s.getObjValAs? CommitId "commit", s.getObjValAs? Nat "sigma2" with
        | .ok c, .ok v => some (c, v)
        | _, _ => none)
    | .error _ => none

def validateMain : IO UInt32 := runJsonPipe fun j => do
  let indices ← IO.ofExcept (j.getObjValAs? (List CommitIndex) "indices")
  let signedCommits ← IO.ofExcept (j.getObjValAs? (List SignedCommit) "signedCommits")
  let rankingsJson := j.getObjVal? "rankings" |>.toOption |>.getD (Lean.Json.mkObj [])
  let scoresJson := j.getObjVal? "scores" |>.toOption |>.getD (Lean.Json.mkObj [])
  if indices.length != signedCommits.length then
    return Json.mkObj [
      ("valid", toJson false),
      ("errors", Json.arr #[Json.str s!"index count ({indices.length}) != commit count ({signedCommits.length})"])
    ]
  let mut errors : Array Json := #[]
  let mut pastIndices : List CommitIndex := []
  for (idx, commit) in indices.zip signedCommits do
    let v := activeVariant commit.prCreatedAt
    -- Look up ranking for v2+ commits
    let ranking := lookupRanking pastIndices rankingsJson commit.prCreatedAt
    -- Error if v2+ requires ranking but it's missing
    if v.useRankedTargets && ranking.isNone then
      errors := errors.push (Json.str s!"commit {idx.commitHash}: v2 active but ranking not found in rankings map")
      pastIndices := pastIndices ++ [idx]
      continue
    -- Look up variances for v3 commits
    let variances := if v.useBradleyTerry then
      lookupVariances pastIndices scoresJson commit.prCreatedAt
    else none
    if v.useBradleyTerry && variances.isNone then
      errors := errors.push (Json.str s!"commit {idx.commitHash}: v3 active but variances not found in scores map")
      pastIndices := pastIndices ++ [idx]
      continue
    let expected := evaluate pastIndices commit ranking variances
    -- Compare key fields
    if expected.commitHash != idx.commitHash then
      errors := errors.push (Json.str s!"commit {idx.commitHash}: hash mismatch")
    if expected.score != idx.score then
      errors := errors.push (Json.str s!"commit {idx.commitHash}: score mismatch (expected {repr expected.score}, got {repr idx.score})")
    if expected.weightDelta != idx.weightDelta then
      errors := errors.push (Json.str s!"commit {idx.commitHash}: weightDelta mismatch (expected {expected.weightDelta}, got {idx.weightDelta})")
    if expected.contributor != idx.contributor then
      errors := errors.push (Json.str s!"commit {idx.commitHash}: contributor mismatch")
    pastIndices := pastIndices ++ [idx]
  return Json.mkObj [
    ("valid", toJson errors.isEmpty),
    ("errors", Json.arr errors)
  ]
