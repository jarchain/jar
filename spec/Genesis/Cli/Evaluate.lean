/-
  genesis_evaluate CLI

  Input:  {"commit": {...}, "pastIndices": [...]}
  Output: CachedCommitIndex JSON (includes globalRank)
-/

import Genesis.Cli.Common

open Lean (Json ToJson toJson fromJson? FromJson)
open Genesis.Cli

def main : IO UInt32 := runJsonPipe fun j => do
  let commit ← IO.ofExcept (j.getObjValAs? SignedCommit "commit")
  let pastIndices ← IO.ofExcept (j.getObjValAs? (List CachedCommitIndex) "pastIndices")
  let idx := evaluate pastIndices commit
  return toJson idx
