// ============================================================================
// 需求影子价格键 / Demand shadow price key
// ============================================================================

/// 需求影子价格键 / Demand shadow price key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DemandShadowPriceKey {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
}

// ============================================================================
// SolutionAnalyzer - 解分析器 / Solution analyzer
// ============================================================================

/// 解分析器 / Solution analyzer
///
/// 分析层分配求解结果。
/// Analyzes layer assignment solving results.
#[derive(Debug, Clone, Default)]
pub struct SolutionAnalyzer;

