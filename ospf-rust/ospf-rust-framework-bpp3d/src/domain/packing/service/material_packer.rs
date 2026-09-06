// ============================================================================
// MaterialPacker - 物料装箱器 / Material packer
// ============================================================================

/// 物料装箱器 / Material packer
///
/// 对物料维度执行装箱汇总。
/// Performs packing summarization on the material dimension.
#[derive(Debug, Clone, Default)]
pub struct MaterialPacker;

impl MaterialPacker {
    /// 创建物料装箱器 / Create a material packer
    pub fn new() -> Self {
        Self
    }

    /// 汇总物料装箱结果 / Summarize material packing results
    pub fn invoke<V, U>(&self, result: &PackingResult<V, U>) -> Vec<MaterialSummary>
    where
        U: UnitTrait,
    {
        result.material_summaries.clone()
    }
}

