import Jar.JAVM.Memory

/-!
# JAVM Memory Proofs — sbrk edge cases

Properties of the `sbrk` (heap growth) function.
These verify the critical edge cases: query mode (size=0),
oversized requests, and memory preservation.
-/

namespace Jar.Proofs

-- ============================================================================
-- sbrk query mode (size = 0)
-- ============================================================================

/-- sbrk with zero size is a query: returns unchanged memory and current heap top. -/
theorem sbrk_zero (m : Jar.JAVM.Memory) :
    Jar.JAVM.sbrk m 0 = (m, UInt64.ofNat m.heapTop) := by
  unfold Jar.JAVM.sbrk
  simp

/-- sbrk with zero size preserves memory state. -/
theorem sbrk_zero_preserves (m : Jar.JAVM.Memory) :
    (Jar.JAVM.sbrk m 0).1 = m := by
  rw [sbrk_zero]

/-- sbrk with zero size returns the current heap top. -/
theorem sbrk_zero_returns_top (m : Jar.JAVM.Memory) :
    (Jar.JAVM.sbrk m 0).2 = UInt64.ofNat m.heapTop := by
  rw [sbrk_zero]

-- ============================================================================
-- sbrk oversized request
-- ============================================================================

/-- sbrk rejects requests larger than 2^32 bytes by returning 0.
    This ensures the 32-bit address space bound is enforced. -/
theorem sbrk_too_large (m : Jar.JAVM.Memory) (size : UInt64)
    (h : size.toNat > 2^32) :
    Jar.JAVM.sbrk m size = (m, 0) := by
  unfold Jar.JAVM.sbrk
  simp [h]

/-- sbrk returns 0 (failure) for oversized requests. -/
theorem sbrk_too_large_fails (m : Jar.JAVM.Memory) (size : UInt64)
    (h : size.toNat > 2^32) :
    (Jar.JAVM.sbrk m size).2 = 0 := by
  rw [sbrk_too_large m size h]

/-- sbrk preserves memory for oversized requests. -/
theorem sbrk_too_large_preserves (m : Jar.JAVM.Memory) (size : UInt64)
    (h : size.toNat > 2^32) :
    (Jar.JAVM.sbrk m size).1 = m := by
  rw [sbrk_too_large m size h]

end Jar.Proofs
