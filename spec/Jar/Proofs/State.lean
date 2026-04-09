import Jar.State
import Jar.Variant

/-!
# State Transition Proofs

Theorems about the state transition functions and protocol constants
defined in `Jar.State` and `Jar.Types.Constants`.
-/

namespace Jar.Proofs.State

-- ============================================================================
-- Protocol Constant Relationships
-- ============================================================================

/-- PVM page size is 2^12. -/
theorem Z_P_eq_pow2_12 : Z_P = 2 ^ 12 := by rfl

/-- PVM initialization zone is 2^16. -/
theorem Z_Z_eq_pow2_16 : Z_Z = 2 ^ 16 := by rfl

/-- PVM input size limit is 2^24. -/
theorem Z_I_eq_pow2_24 : Z_I = 2 ^ 24 := by rfl

/-- Slot period is 6 seconds. -/
theorem P_value : P = 6 := by rfl

/-- Max segment imports equals max segment exports. -/
theorem W_M_eq_W_X : W_M = W_X := by rfl

/-- PVM register count is 13. -/
theorem PVM_REGISTERS_value : PVM_REGISTERS = 13 := by rfl

/-- Min public service index is 256. -/
theorem S_MIN_value : S_MIN = 256 := by rfl

/-- Audit tranche period is 8. -/
theorem A_TRANCHE_value : A_TRANCHE = 8 := by rfl

/-- Transfer memo size is 128 bytes. -/
theorem W_T_value : W_T = 128 := by rfl

-- ============================================================================
-- Zero Records
-- ============================================================================

/-- ValidatorRecord.zero has all fields set to 0. -/
theorem ValidatorRecord_zero_blocks :
    ValidatorRecord.zero.blocks = 0 := by rfl

theorem ValidatorRecord_zero_tickets :
    ValidatorRecord.zero.tickets = 0 := by rfl

theorem ValidatorRecord_zero_guarantees :
    ValidatorRecord.zero.guarantees = 0 := by rfl

theorem ValidatorRecord_zero_assurances :
    ValidatorRecord.zero.assurances = 0 := by rfl

/-- CoreStatistics.zero has all fields set to 0. -/
theorem CoreStatistics_zero_gasUsed :
    CoreStatistics.zero.gasUsed = 0 := by rfl

theorem CoreStatistics_zero_popularity :
    CoreStatistics.zero.popularity = 0 := by rfl

theorem CoreStatistics_zero_exports :
    CoreStatistics.zero.exports = 0 := by rfl

-- ============================================================================
-- Timekeeping (§6.1)
-- ============================================================================

/-- newTimeslot extracts the header's timeslot. GP eq (28). -/
theorem newTimeslot_eq [JarVariant] (h : Header) :
    newTimeslot h = h.timeslot := by rfl

-- ============================================================================
-- Validation on Empty Inputs
-- ============================================================================

/-- Assurance ordering is trivially valid for empty assurances. -/
theorem validateAssuranceOrder_empty [JarVariant] :
    validateAssuranceOrder #[] = true := by rfl

/-- Assurance ordering is trivially valid for a single assurance. -/
theorem validateAssuranceOrder_singleton [JarVariant] (a : Assurance) :
    validateAssuranceOrder #[a] = true := by rfl

/-- Empty guarantees yield empty reported packages. -/
theorem collectReportedPackages_empty [JarVariant] :
    collectReportedPackages #[] = Dict.empty := by rfl

end Jar.Proofs.State
