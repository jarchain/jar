import Jar.Variant

/-!
# Variant Config Proofs — compile-time regression tests

These theorems assert the configuration fields of each variant.
If someone accidentally changes a variant definition, these proofs
break at compile time — serving as a lightweight regression harness.
-/

namespace Jar.Proofs

-- ============================================================================
-- jar080_tiny config assertions
-- ============================================================================

theorem jar080_tiny_memoryModel_linear :
    @JamConfig.memoryModel JamVariant.jar080_tiny.toJamConfig = .linear := by rfl

theorem jar080_tiny_gasModel_singlePass :
    @JamConfig.gasModel JamVariant.jar080_tiny.toJamConfig = .basicBlockSinglePass := by rfl

theorem jar080_tiny_heapModel_growHeap :
    @JamConfig.heapModel JamVariant.jar080_tiny.toJamConfig = .growHeap := by rfl

theorem jar080_tiny_hostcallVersion_1 :
    @JamConfig.hostcallVersion JamVariant.jar080_tiny.toJamConfig = 1 := by rfl

-- ============================================================================
-- gp072_tiny config assertions (contrast)
-- ============================================================================

theorem gp072_tiny_memoryModel_segmented :
    @JamConfig.memoryModel JamVariant.gp072_tiny.toJamConfig = .segmented := by rfl

theorem gp072_tiny_gasModel_perInstruction :
    @JamConfig.gasModel JamVariant.gp072_tiny.toJamConfig = .perInstruction := by rfl

theorem gp072_tiny_hostcallVersion_0 :
    @JamConfig.hostcallVersion JamVariant.gp072_tiny.toJamConfig = 0 := by rfl

end Jar.Proofs
