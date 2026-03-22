/-
  Genesis Protocol — Execution Model & State

  ## Variant System

  Protocol parameters are grouped in GenesisConfig/GenesisVariant.
  The active variant is selected by epoch via genesisSchedule, following
  the blockchain hard-fork pattern. Parameter changes are non-retroactive:
  each past index is processed under the variant active at its epoch,
  and each commit is scored under the variant active at its prCreatedAt.

  ## Spec Consistency Rule

  The current spec on master evaluates ALL past commits correctly.
  Spec changes (algorithms) must remain backward compatible.
  Parameter changes use the variant schedule — no backward compat needed.
  CI enforces via `genesis-replay.sh --verify`.
-/

import Genesis.Types
import Genesis.Scoring

/-! ### Genesis Constants -/

/-- GPG key fingerprints of trusted commit signers. -/
def trustedSigningKeys : Array String := #[
  "B5690EEEBB952194"  -- GitHub web-flow (2024-01-16, no expiry)
]

/-- The founding reviewer. -/
def founder : ContributorId := "sorpaas"

/-- The genesis commit. Scoring starts for commits AFTER this one. -/
def genesisCommit : CommitId := "4cc102a03d715c6bb2b119d8a3a1c49e4694751f"

/-- Initial weight for the founder. -/
def founderWeight : Nat := 1

/-! ### Activation Schedule -/

/-- Activation schedule. Each entry: (activationEpoch, variant).
    For a given epoch, the active variant is the last entry where
    activationEpoch ≤ epoch. Uses idx.epoch for state reconstruction,
    commit.prCreatedAt for scoring.

    To change a parameter:
    1. PR A: add new GenesisConfig + GenesisVariant instance. Safe (inactive).
    2. PR B: add entry here with a future activation epoch. Must merge before that date. -/
def genesisSchedule : List (Epoch × GenesisVariant) :=
  [ (0, GenesisVariant.v1)
  ]

/-- Resolve the active variant for a given epoch. -/
def activeVariant (epoch : Epoch) : GenesisVariant :=
  let applicable := genesisSchedule.filter (fun (e, _) => e ≤ epoch)
  match applicable.getLast? with
  | some (_, v) => v
  | none => GenesisVariant.v1

/-! ### CommitIndex — Output of evaluating one signed commit -/

/-- CommitIndex as stored in Genesis-Index: trailers.
    `globalRank` is `Option Nat` because old trailers predate this field.

    Contains only the raw facts needed for state reconstruction and
    future finalization. Token amounts are NOT stored here — they are
    computed during finalization using the current spec's parameters. -/
structure CommitIndex where
  /-- Hash of the signed commit that was evaluated. -/
  commitHash : CommitId
  /-- Epoch / timestamp of the commit. -/
  epoch : Epoch
  /-- The commit's score on each dimension. -/
  score : CommitScore
  /-- Who authored the commit. -/
  contributor : ContributorId
  /-- Weight change for the contributor (= score.weighted, 0-100).
      Needed at each step for reconstructing reviewer weights. -/
  weightDelta : Nat
  /-- Approved reviewers who participated. Their weights can be
      reconstructed from prior indices' weightDeltas. -/
  reviewers : List ContributorId
  /-- Meta-review results: who approved/rejected which reviews. -/
  metaReviews : List MetaReview
  /-- Reviewers who voted to merge. -/
  mergeVotes : List ContributorId
  /-- Reviewers who voted not to merge. -/
  rejectVotes : List ContributorId
  /-- Whether the founder used the escape hatch to force this merge. -/
  founderOverride : Bool
  /-- Position in the global quality ordering (0 = best).
      `none` for old trailers that predate this field. -/
  globalRank : Option Nat := none
  deriving Repr

/-- CachedCommitIndex as stored in genesis.json cache.
    `globalRank` is always present (computed during evaluate/rebuild). -/
structure CachedCommitIndex where
  commitHash : CommitId
  epoch : Epoch
  score : CommitScore
  contributor : ContributorId
  weightDelta : Nat
  reviewers : List ContributorId
  metaReviews : List MetaReview
  mergeVotes : List ContributorId
  rejectVotes : List ContributorId
  founderOverride : Bool
  globalRank : Nat
  deriving Repr

/-- Convert CachedCommitIndex to CommitIndex. -/
def CachedCommitIndex.toCommitIndex (c : CachedCommitIndex) : CommitIndex :=
  { commitHash := c.commitHash, epoch := c.epoch, score := c.score,
    contributor := c.contributor, weightDelta := c.weightDelta,
    reviewers := c.reviewers, metaReviews := c.metaReviews,
    mergeVotes := c.mergeVotes, rejectVotes := c.rejectVotes,
    founderOverride := c.founderOverride, globalRank := some c.globalRank }

/-! ### Intermediate State -/

/-- Intermediate state reconstructed from past indices. -/
structure EvalState where
  /-- Current contributor weights (for reviewer weight lookups). -/
  contributors : List Contributor
  /-- Scored commits with their merge epochs (for comparison target selection). -/
  scoredCommits : List (CommitId × Epoch)
  /-- Accumulated pairwise wins: (winner, list of losers). -/
  pairwiseWins : List (CommitId × List CommitId) := []

/-- Update or insert a contributor in a list. -/
private def upsertContributor (cs : List Contributor) (updated : Contributor) : List Contributor :=
  if cs.any (fun (c : Contributor) => c.id == updated.id) then
    cs.map (fun (c : Contributor) => if c.id == updated.id then updated else c)
  else
    cs ++ [updated]

/-- Initial evaluation state: founder with initial weight, no scored commits. -/
def initEvalState : EvalState := {
  contributors := [⟨founder, 0, founderWeight, true⟩],
  scoredCommits := []
}

/-! ### Inner functions (use [GenesisVariant] typeclass) -/

section VariantScoped
variable [gv : GenesisVariant]

/-- Process one past cached index under the current variant's parameters.
    Updates contributor weights, reviewer status, and ranking state. -/
def stepState (state : EvalState) (idx : CachedCommitIndex) : EvalState :=
  let contributors :=
    if idx.weightDelta == 0 then state.contributors
    else
      let existing := state.contributors.find? (fun (c : Contributor) => c.id == idx.contributor)
      let c := existing.getD ⟨idx.contributor, 0, 0, false⟩
      let newWeight := c.weight + idx.weightDelta
      let meetsThreshold := newWeight ≥ gv.reviewerThreshold
      let updated : Contributor := ⟨c.id, c.balance, newWeight, c.isReviewer || meetsThreshold⟩
      upsertContributor state.contributors updated
  let scoredCommits := state.scoredCommits ++ [(idx.commitHash, idx.epoch)]
  { contributors := contributors, scoredCommits := scoredCommits,
    pairwiseWins := state.pairwiseWins }

/-- Get reviewer weight from an EvalState. -/
def EvalState.reviewerWeight (s : EvalState) (id : ContributorId) : Nat :=
  match s.contributors.find? (fun (c : Contributor) => c.id == id) with
  | some c => if c.isReviewer then c.weight else 0
  | none => 0

/-- Evaluate a single signed commit given pre-built state.
    Uses the current [GenesisVariant] for scoring parameters.
    Returns CachedCommitIndex with computed globalRank, and updated EvalState
    (with accumulated pairwise evidence). -/
def evaluateWithState (state : EvalState) (commit : SignedCommit) : CachedCommitIndex × EvalState :=
  let score := commitScore commit
    state.scoredCommits (state.reviewerWeight ·)
  let approved := filterReviews commit.reviews commit.metaReviews (state.reviewerWeight ·)
  let approvedReviewers := approved
    |>.filter (fun (r : EmbeddedReview) => state.reviewerWeight r.reviewer > 0)
    |>.map (fun (r : EmbeddedReview) => r.reviewer)
  let mergeVoters := commit.reviews
    |>.filter (fun (r : EmbeddedReview) => r.verdict == .merge)
    |>.map (fun (r : EmbeddedReview) => r.reviewer)
  let rejectVoters := commit.reviews
    |>.filter (fun (r : EmbeddedReview) => r.verdict == .notMerge)
    |>.map (fun (r : EmbeddedReview) => r.reviewer)
  -- Compute globalRank via full net-wins recomputation
  let updatedWins := accumulatePairwise commit.reviews state.pairwiseWins
  let pastCommitIds := state.scoredCommits.map (·.1)
  let allCommits := pastCommitIds ++ [commit.id]
  let netWins := computeNetWins allCommits updatedWins
  let indexed := netWins.zip (List.range netWins.length)
  let sorted := indexed.toArray.qsort (fun ((_, nw1), i1) ((_, nw2), i2) =>
    if nw1 != nw2 then nw1 > nw2 else i1 < i2
  ) |>.toList
  let globalRank := sorted.findIdx? (fun ((c, _), _) => c == commit.id) |>.getD allCommits.length
  let idx : CachedCommitIndex :=
    { commitHash := commit.id,
      epoch := commit.mergeEpoch,
      score := score,
      contributor := commit.author,
      weightDelta := score.weighted,
      reviewers := approvedReviewers,
      metaReviews := commit.metaReviews,
      mergeVotes := mergeVoters,
      rejectVotes := rejectVoters,
      founderOverride := commit.founderOverride,
      globalRank := globalRank }
  let newState := stepState state idx
  (idx, { newState with pairwiseWins := updatedWins })

end VariantScoped

/-! ### Outer dispatch (resolves variant per-commit via schedule) -/

/-- Reconstruct state from past cached indices. Each index is processed under
    the variant active at its epoch (idx.epoch). -/
def reconstructState (pastIndices : List CachedCommitIndex) : EvalState :=
  pastIndices.foldl (fun state idx =>
    letI := activeVariant idx.epoch
    stepState state idx
  ) initEvalState

/-- Evaluate a single signed commit.
    State reconstruction uses per-index variants.
    Scoring uses the variant active at commit.prCreatedAt. -/
def evaluate (pastIndices : List CachedCommitIndex) (commit : SignedCommit) : CachedCommitIndex :=
  let state := reconstructState pastIndices
  letI := activeVariant commit.prCreatedAt
  (evaluateWithState state commit).1

/-- Evaluate a full sequence of signed commits. -/
def evaluateAll (signedCommits : List SignedCommit) : List CachedCommitIndex :=
  signedCommits.foldl (fun indices commit =>
    indices ++ [evaluate indices commit]
  ) []

/-- Final weight for each contributor, computed from all indices.
    Weight = founderWeight + Σ weightDelta for authored commits. -/
def finalWeights (indices : List CachedCommitIndex) : List (ContributorId × Nat) :=
  let addToWeight (acc : List (ContributorId × Nat))
      (id : ContributorId) (amount : Nat) : List (ContributorId × Nat) :=
    if amount == 0 then acc
    else
      match acc.find? (fun (cid, _) => cid == id) with
      | some _ => acc.map (fun (cid, w) => if cid == id then (cid, w + amount) else (cid, w))
      | none => acc ++ [(id, amount)]
  let init := [(founder, founderWeight)]
  indices.foldl (fun acc (idx : CachedCommitIndex) =>
    if idx.weightDelta == 0 then acc
    else addToWeight acc idx.contributor idx.weightDelta
  ) init
