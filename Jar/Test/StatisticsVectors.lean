import Jar.Test.Statistics

/-! Auto-generated statistics test vectors. Do not edit. -/

namespace Jar.Test.StatisticsVectors

open Jar.Test.Statistics

-- ============================================================================
-- stats_with_empty_extrinsic-1.json
-- ============================================================================

def stats_with_empty_extrinsic_1_pre_curr : Array FlatValidatorRecord := #[
    { blocks := 18093, tickets := 11785, preImages := 27, preImagesSize := 575, guarantees := 1363, assurances := 1359 },
    { blocks := 15610, tickets := 17650, preImages := 39, preImagesSize := 731, guarantees := 1319, assurances := 1379 },
    { blocks := 13127, tickets := 8155, preImages := 34, preImagesSize := 442, guarantees := 1882, assurances := 1586 },
    { blocks := 15860, tickets := 14436, preImages := 121, preImagesSize := 965, guarantees := 1457, assurances := 1749 },
    { blocks := 13377, tickets := 20301, preImages := 6, preImagesSize := 270, guarantees := 1310, assurances := 1734 },
    { blocks := 10894, tickets := 10806, preImages := 1, preImagesSize := 941, guarantees := 1441, assurances := 1541 }]

def stats_with_empty_extrinsic_1_pre_last : Array FlatValidatorRecord := #[
    { blocks := 13627, tickets := 17087, preImages := 88, preImagesSize := 504, guarantees := 1448, assurances := 1808 },
    { blocks := 11144, tickets := 2376, preImages := 86, preImagesSize := 926, guarantees := 1070, assurances := 1870 },
    { blocks := 8661, tickets := 8241, preImages := 98, preImagesSize := 826, guarantees := 1250, assurances := 1842 },
    { blocks := 6178, tickets := 19322, preImages := 107, preImagesSize := 271, guarantees := 1851, assurances := 1599 },
    { blocks := 8911, tickets := 5027, preImages := 53, preImagesSize := 465, guarantees := 1061, assurances := 1225 },
    { blocks := 6428, tickets := 10892, preImages := 65, preImagesSize := 365, guarantees := 1241, assurances := 1901 }]

def stats_with_empty_extrinsic_1_pre : FlatStatisticsState := {
  valsCurrStats := stats_with_empty_extrinsic_1_pre_curr,
  valsLastStats := stats_with_empty_extrinsic_1_pre_last,
  slot := 123456
}

def stats_with_empty_extrinsic_1_post_curr : Array FlatValidatorRecord := #[
    { blocks := 18093, tickets := 11785, preImages := 27, preImagesSize := 575, guarantees := 1363, assurances := 1359 },
    { blocks := 15611, tickets := 17650, preImages := 39, preImagesSize := 731, guarantees := 1319, assurances := 1379 },
    { blocks := 13127, tickets := 8155, preImages := 34, preImagesSize := 442, guarantees := 1882, assurances := 1586 },
    { blocks := 15860, tickets := 14436, preImages := 121, preImagesSize := 965, guarantees := 1457, assurances := 1749 },
    { blocks := 13377, tickets := 20301, preImages := 6, preImagesSize := 270, guarantees := 1310, assurances := 1734 },
    { blocks := 10894, tickets := 10806, preImages := 1, preImagesSize := 941, guarantees := 1441, assurances := 1541 }]

def stats_with_empty_extrinsic_1_post_last : Array FlatValidatorRecord := #[
    { blocks := 13627, tickets := 17087, preImages := 88, preImagesSize := 504, guarantees := 1448, assurances := 1808 },
    { blocks := 11144, tickets := 2376, preImages := 86, preImagesSize := 926, guarantees := 1070, assurances := 1870 },
    { blocks := 8661, tickets := 8241, preImages := 98, preImagesSize := 826, guarantees := 1250, assurances := 1842 },
    { blocks := 6178, tickets := 19322, preImages := 107, preImagesSize := 271, guarantees := 1851, assurances := 1599 },
    { blocks := 8911, tickets := 5027, preImages := 53, preImagesSize := 465, guarantees := 1061, assurances := 1225 },
    { blocks := 6428, tickets := 10892, preImages := 65, preImagesSize := 365, guarantees := 1241, assurances := 1901 }]

def stats_with_empty_extrinsic_1_post : FlatStatisticsState := {
  valsCurrStats := stats_with_empty_extrinsic_1_post_curr,
  valsLastStats := stats_with_empty_extrinsic_1_post_last,
  slot := 123456
}

def stats_with_empty_extrinsic_1_input : StatsInput := {
  slot := 123457,
  authorIndex := 1,
  extrinsic := {
    ticketCount := 0,
    preimageSizes := #[],
    guaranteeSigners := #[],
    assuranceValidators := #[]
  }
}

-- ============================================================================
-- stats_with_epoch_change-1.json
-- ============================================================================

def stats_with_epoch_change_1_pre_curr : Array FlatValidatorRecord := #[
    { blocks := 18093, tickets := 11785, preImages := 27, preImagesSize := 575, guarantees := 1363, assurances := 1359 },
    { blocks := 15610, tickets := 17650, preImages := 39, preImagesSize := 731, guarantees := 1319, assurances := 1379 },
    { blocks := 13127, tickets := 8155, preImages := 34, preImagesSize := 442, guarantees := 1882, assurances := 1586 },
    { blocks := 15860, tickets := 14436, preImages := 121, preImagesSize := 965, guarantees := 1457, assurances := 1749 },
    { blocks := 13377, tickets := 20301, preImages := 6, preImagesSize := 270, guarantees := 1310, assurances := 1734 },
    { blocks := 10894, tickets := 10806, preImages := 1, preImagesSize := 941, guarantees := 1441, assurances := 1541 }]

def stats_with_epoch_change_1_pre_last : Array FlatValidatorRecord := #[
    { blocks := 13627, tickets := 17087, preImages := 88, preImagesSize := 504, guarantees := 1448, assurances := 1808 },
    { blocks := 11144, tickets := 2376, preImages := 86, preImagesSize := 926, guarantees := 1070, assurances := 1870 },
    { blocks := 8661, tickets := 8241, preImages := 98, preImagesSize := 826, guarantees := 1250, assurances := 1842 },
    { blocks := 6178, tickets := 19322, preImages := 107, preImagesSize := 271, guarantees := 1851, assurances := 1599 },
    { blocks := 8911, tickets := 5027, preImages := 53, preImagesSize := 465, guarantees := 1061, assurances := 1225 },
    { blocks := 6428, tickets := 10892, preImages := 65, preImagesSize := 365, guarantees := 1241, assurances := 1901 }]

def stats_with_epoch_change_1_pre : FlatStatisticsState := {
  valsCurrStats := stats_with_epoch_change_1_pre_curr,
  valsLastStats := stats_with_epoch_change_1_pre_last,
  slot := 123455
}

def stats_with_epoch_change_1_post_curr : Array FlatValidatorRecord := #[
    { blocks := 0, tickets := 0, preImages := 0, preImagesSize := 0, guarantees := 1, assurances := 1 },
    { blocks := 1, tickets := 2, preImages := 3, preImagesSize := 51, guarantees := 1, assurances := 1 },
    { blocks := 0, tickets := 0, preImages := 0, preImagesSize := 0, guarantees := 1, assurances := 0 },
    { blocks := 0, tickets := 0, preImages := 0, preImagesSize := 0, guarantees := 1, assurances := 0 },
    { blocks := 0, tickets := 0, preImages := 0, preImagesSize := 0, guarantees := 0, assurances := 0 },
    { blocks := 0, tickets := 0, preImages := 0, preImagesSize := 0, guarantees := 0, assurances := 0 }]

def stats_with_epoch_change_1_post_last : Array FlatValidatorRecord := #[
    { blocks := 18093, tickets := 11785, preImages := 27, preImagesSize := 575, guarantees := 1363, assurances := 1359 },
    { blocks := 15610, tickets := 17650, preImages := 39, preImagesSize := 731, guarantees := 1319, assurances := 1379 },
    { blocks := 13127, tickets := 8155, preImages := 34, preImagesSize := 442, guarantees := 1882, assurances := 1586 },
    { blocks := 15860, tickets := 14436, preImages := 121, preImagesSize := 965, guarantees := 1457, assurances := 1749 },
    { blocks := 13377, tickets := 20301, preImages := 6, preImagesSize := 270, guarantees := 1310, assurances := 1734 },
    { blocks := 10894, tickets := 10806, preImages := 1, preImagesSize := 941, guarantees := 1441, assurances := 1541 }]

def stats_with_epoch_change_1_post : FlatStatisticsState := {
  valsCurrStats := stats_with_epoch_change_1_post_curr,
  valsLastStats := stats_with_epoch_change_1_post_last,
  slot := 123455
}

def stats_with_epoch_change_1_input : StatsInput := {
  slot := 123456,
  authorIndex := 1,
  extrinsic := {
    ticketCount := 2,
    preimageSizes := #[16, 17, 18],
    guaranteeSigners := #[#[0, 1], #[2, 3]],
    assuranceValidators := #[0, 1]
  }
}

-- ============================================================================
-- stats_with_some_extrinsic-1.json
-- ============================================================================

def stats_with_some_extrinsic_1_pre_curr : Array FlatValidatorRecord := #[
    { blocks := 18093, tickets := 11785, preImages := 27, preImagesSize := 575, guarantees := 1363, assurances := 1359 },
    { blocks := 15610, tickets := 17650, preImages := 39, preImagesSize := 731, guarantees := 1319, assurances := 1379 },
    { blocks := 13127, tickets := 8155, preImages := 34, preImagesSize := 442, guarantees := 1882, assurances := 1586 },
    { blocks := 15860, tickets := 14436, preImages := 121, preImagesSize := 965, guarantees := 1457, assurances := 1749 },
    { blocks := 13377, tickets := 20301, preImages := 6, preImagesSize := 270, guarantees := 1310, assurances := 1734 },
    { blocks := 10894, tickets := 10806, preImages := 1, preImagesSize := 941, guarantees := 1441, assurances := 1541 }]

def stats_with_some_extrinsic_1_pre_last : Array FlatValidatorRecord := #[
    { blocks := 13627, tickets := 17087, preImages := 88, preImagesSize := 504, guarantees := 1448, assurances := 1808 },
    { blocks := 11144, tickets := 2376, preImages := 86, preImagesSize := 926, guarantees := 1070, assurances := 1870 },
    { blocks := 8661, tickets := 8241, preImages := 98, preImagesSize := 826, guarantees := 1250, assurances := 1842 },
    { blocks := 6178, tickets := 19322, preImages := 107, preImagesSize := 271, guarantees := 1851, assurances := 1599 },
    { blocks := 8911, tickets := 5027, preImages := 53, preImagesSize := 465, guarantees := 1061, assurances := 1225 },
    { blocks := 6428, tickets := 10892, preImages := 65, preImagesSize := 365, guarantees := 1241, assurances := 1901 }]

def stats_with_some_extrinsic_1_pre : FlatStatisticsState := {
  valsCurrStats := stats_with_some_extrinsic_1_pre_curr,
  valsLastStats := stats_with_some_extrinsic_1_pre_last,
  slot := 123456
}

def stats_with_some_extrinsic_1_post_curr : Array FlatValidatorRecord := #[
    { blocks := 18093, tickets := 11785, preImages := 27, preImagesSize := 575, guarantees := 1364, assurances := 1360 },
    { blocks := 15611, tickets := 17652, preImages := 42, preImagesSize := 782, guarantees := 1320, assurances := 1380 },
    { blocks := 13127, tickets := 8155, preImages := 34, preImagesSize := 442, guarantees := 1883, assurances := 1586 },
    { blocks := 15860, tickets := 14436, preImages := 121, preImagesSize := 965, guarantees := 1458, assurances := 1749 },
    { blocks := 13377, tickets := 20301, preImages := 6, preImagesSize := 270, guarantees := 1310, assurances := 1734 },
    { blocks := 10894, tickets := 10806, preImages := 1, preImagesSize := 941, guarantees := 1441, assurances := 1541 }]

def stats_with_some_extrinsic_1_post_last : Array FlatValidatorRecord := #[
    { blocks := 13627, tickets := 17087, preImages := 88, preImagesSize := 504, guarantees := 1448, assurances := 1808 },
    { blocks := 11144, tickets := 2376, preImages := 86, preImagesSize := 926, guarantees := 1070, assurances := 1870 },
    { blocks := 8661, tickets := 8241, preImages := 98, preImagesSize := 826, guarantees := 1250, assurances := 1842 },
    { blocks := 6178, tickets := 19322, preImages := 107, preImagesSize := 271, guarantees := 1851, assurances := 1599 },
    { blocks := 8911, tickets := 5027, preImages := 53, preImagesSize := 465, guarantees := 1061, assurances := 1225 },
    { blocks := 6428, tickets := 10892, preImages := 65, preImagesSize := 365, guarantees := 1241, assurances := 1901 }]

def stats_with_some_extrinsic_1_post : FlatStatisticsState := {
  valsCurrStats := stats_with_some_extrinsic_1_post_curr,
  valsLastStats := stats_with_some_extrinsic_1_post_last,
  slot := 123456
}

def stats_with_some_extrinsic_1_input : StatsInput := {
  slot := 123457,
  authorIndex := 1,
  extrinsic := {
    ticketCount := 2,
    preimageSizes := #[16, 17, 18],
    guaranteeSigners := #[#[0, 1], #[2, 3]],
    assuranceValidators := #[0, 1]
  }
}

-- ============================================================================
-- Test Runner
-- ============================================================================

end Jar.Test.StatisticsVectors

open Jar.Test.Statistics Jar.Test.StatisticsVectors in
def main : IO Unit := do
  IO.println "Running statistics test vectors..."
  let mut passed := (0 : Nat)
  let mut failed := (0 : Nat)
  if (← runTest "stats_with_empty_extrinsic_1" stats_with_empty_extrinsic_1_pre stats_with_empty_extrinsic_1_input stats_with_empty_extrinsic_1_post)
  then passed := passed + 1
  else failed := failed + 1
  if (← runTest "stats_with_epoch_change_1" stats_with_epoch_change_1_pre stats_with_epoch_change_1_input stats_with_epoch_change_1_post)
  then passed := passed + 1
  else failed := failed + 1
  if (← runTest "stats_with_some_extrinsic_1" stats_with_some_extrinsic_1_pre stats_with_some_extrinsic_1_input stats_with_some_extrinsic_1_post)
  then passed := passed + 1
  else failed := failed + 1
  IO.println s!"Statistics: {passed} passed, {failed} failed out of 3"
  if failed > 0 then
    IO.Process.exit 1
