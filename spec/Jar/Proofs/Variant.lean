import Jar.Variant

/-!
# Variant Config Proofs — compile-time regression tests

These theorems assert the configuration fields of each variant.
If someone accidentally changes a variant definition, these proofs
break at compile time — serving as a lightweight regression harness.
-/

namespace Jar.Proofs

-- ============================================================================
-- jar1 config assertions (v2 capability model)
-- ============================================================================

theorem jar1_capabilityModel_v2 :
    @JarConfig.capabilityModel JarVariant.jar1.toJarConfig = .v2 := by rfl

theorem jar1_memoryModel_linear :
    @JarConfig.memoryModel JarVariant.jar1.toJarConfig = .linear := by rfl

theorem jar1_gasModel_singlePass :
    @JarConfig.gasModel JarVariant.jar1.toJarConfig = .basicBlockSinglePass := by rfl

theorem jar1_variableValidators :
    @JarConfig.variableValidators JarVariant.jar1.toJarConfig = true := by rfl

-- ============================================================================
-- gp072_tiny config assertions (contrast)
-- ============================================================================

theorem gp072_tiny_memoryModel_segmented :
    @JarConfig.memoryModel JarVariant.gp072_tiny.toJarConfig = .segmented := by rfl

theorem gp072_tiny_gasModel_perInstruction :
    @JarConfig.gasModel JarVariant.gp072_tiny.toJarConfig = .perInstruction := by rfl

theorem gp072_tiny_variableValidators_false :
    @JarConfig.variableValidators JarVariant.gp072_tiny.toJarConfig = false := by rfl

-- ============================================================================
-- Validator count consistency (isValidValCount returns true for config V)
-- ============================================================================

/-- Params.full has a valid validator count (V=1023, C=341).
    1023 ≥ 6, 1023 ≤ 3*(341+1) = 1026, 1023 % 3 = 0. -/
theorem full_validValCount :
    Params.full.isValidValCount Params.full.V = true := by decide

/-- Params.tiny has a valid validator count (V=6, C=2).
    6 ≥ 6, 6 ≤ 3*(2+1) = 9, 6 % 3 = 0. -/
theorem tiny_validValCount :
    Params.tiny.isValidValCount Params.tiny.V = true := by decide

-- ============================================================================
-- Economic model assertions (jar1 = coinless, gp072 = token-based)
-- ============================================================================

/-- jar1 uses the coinless QuotaEcon model. -/
theorem jar1_econType_quota :
    @JarConfig.EconType JarVariant.jar1.toJarConfig = QuotaEcon := by rfl

/-- gp072_full uses the token-based BalanceEcon model. -/
theorem gp072_full_econType_balance :
    @JarConfig.EconType JarVariant.gp072_full.toJarConfig = BalanceEcon := by rfl

/-- gp072_tiny uses the token-based BalanceEcon model. -/
theorem gp072_tiny_econType_balance :
    @JarConfig.EconType JarVariant.gp072_tiny.toJarConfig = BalanceEcon := by rfl

-- ============================================================================
-- Variable validators: jar1 vs gp072 contrast
-- ============================================================================

/-- jar1 enables variable validator sets (GP#514). -/
theorem jar1_variableValidators_true :
    @JarConfig.variableValidators JarVariant.jar1.toJarConfig = true := by rfl

/-- gp072_full uses fixed validator sets. -/
theorem gp072_full_variableValidators_false :
    @JarConfig.variableValidators JarVariant.gp072_full.toJarConfig = false := by rfl

-- ============================================================================
-- Compact deblob: jar1 disables, gp072 enables
-- ============================================================================

/-- jar1 uses fixed-width blob headers (not compact). -/
theorem jar1_useCompactDeblob_false :
    @JarConfig.useCompactDeblob JarVariant.jar1.toJarConfig = false := by rfl

/-- gp072_full uses compact (variable-length) blob headers. -/
theorem gp072_full_useCompactDeblob_true :
    @JarConfig.useCompactDeblob JarVariant.gp072_full.toJarConfig = true := by rfl

end Jar.Proofs
