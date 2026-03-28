/-
  genesis — unified CLI for Genesis Proof-of-Intelligence protocol.

  Subcommands:
    select-targets  Select comparison targets for a PR
    evaluate        Score a single signed commit
    check-merge     Check merge readiness (weighted vote tally)
    finalize        Compute final weights from all indices
    validate        Verify index consistency
    ranking         Compute global quality ranking

  All subcommands read JSON from stdin and write JSON to stdout.
-/

import Cli
import Genesis.Cli.SelectTargets
import Genesis.Cli.Evaluate
import Genesis.Cli.CheckMerge
import Genesis.Cli.Finalize
import Genesis.Cli.Validate
import Genesis.Cli.Ranking

open Cli

def runSelectTargets (_ : Parsed) : IO UInt32 := selectTargetsMain
def runEvaluate (_ : Parsed) : IO UInt32 := evaluateMain
def runCheckMerge (_ : Parsed) : IO UInt32 := checkMergeMain
def runFinalize (_ : Parsed) : IO UInt32 := finalizeMain
def runValidate (_ : Parsed) : IO UInt32 := validateMain
def runRanking (p : Parsed) : IO UInt32 :=
  let fv := if p.hasFlag "force-variant" then some (p.flag! "force-variant" |>.as! String) else none
  rankingMainWith fv

def selectTargetsCmd := `[Cli|
  "select-targets" VIA runSelectTargets;
  "Select comparison targets for a PR. Input: {prId, prCreatedAt, indices, ranking?, variances?}"
]

def evaluateCmd := `[Cli|
  evaluate VIA runEvaluate;
  "Score a single signed commit. Input: {commit, pastIndices, ranking?}"
]

def checkMergeCmd := `[Cli|
  "check-merge" VIA runCheckMerge;
  "Check merge readiness. Input: {reviews, metaReviews, indices}"
]

def finalizeCmd := `[Cli|
  finalize VIA runFinalize;
  "Compute final weights. Input: {indices}"
]

def validateCmd := `[Cli|
  validate VIA runValidate;
  "Verify index consistency. Input: {indices, signedCommits, rankings?}"
]

def rankingCmd := `[Cli|
  ranking VIA runRanking;
  "Compute global quality ranking. Input: {signedCommits, indices}"

  FLAGS:
    "force-variant" : String; "Override variant for all commits (v1, v2, v3)."
]

def genesis := `[Cli|
  genesis NOOP; ["0.1.0"]
  "Genesis Proof-of-Intelligence CLI. All subcommands read JSON from stdin."

  SUBCOMMANDS:
    selectTargetsCmd;
    evaluateCmd;
    checkMergeCmd;
    finalizeCmd;
    validateCmd;
    rankingCmd
]

def main (args : List String) : IO UInt32 :=
  genesis.validate args
