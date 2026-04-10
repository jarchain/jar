/-
  genesis_evaluate CLI

  Input:  {"commit": {...}, "pastIndices": [...],
           "ranking": [...] (required for v2+),
           "variances": [...] (required for v3)}
  Output: CommitIndex JSON

  For v2 (useRankedTargets), the "ranking" field is REQUIRED.
  For v3 (useBradleyTerry), the "variances" field is also REQUIRED.
  Missing fields for the active variant are fatal errors.
-/

import Genesis.Cli.Common

open Lean (Json ToJson toJson fromJson? FromJson)
open Genesis.Cli

def evaluateMain : IO UInt32 := runJsonPipe fun j => do
  let commit ← IO.ofExcept (j.getObjValAs? SignedCommit "commit")
  let pastIndices ← IO.ofExcept (j.getObjValAs? (List CommitIndex) "pastIndices")
  let v := activeVariant commit.prCreatedAt
  let ranking ← if v.useRankedTargets then
    IO.ofExcept (j.getObjValAs? (List CommitId) "ranking"
      |>.mapError (s!"v2 variant active (useRankedTargets=true) but ranking field missing: " ++ ·))
    |>.map some
  else
    pure none
  let variances ← if v.useBradleyTerry then
    IO.ofExcept (j.getObjValAs? (List (CommitId × Nat)) "variances"
      |>.mapError (s!"v3 variant active (useBradleyTerry=true) but variances field missing: " ++ ·))
    |>.map some
  else
    pure none
  let (idx, warnings) := evaluateWithWarnings pastIndices commit ranking variances
  let baseJson := toJson idx
  match baseJson with
  | .obj kvs => return .obj (kvs.insert "warnings" (toJson warnings))
  | other => return other
