// ============================================================================
// PackingGeometryContract - 装箱几何契约 / Packing geometry contract
// ============================================================================

/// 装箱几何契约 / Packing geometry contract
///
/// 定义装箱几何验证的核心接口。
/// Defines the core interface for packing geometry verification.
pub trait PackingGeometryContract<V, U>: Debug + Send + Sync
where
    V: Debug + Clone + Send + Sync + num_traits::Float + Field + num_traits::FloatConst + PartialOrd + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    /// 验证装箱几何是否合法 / Validate packing geometry
    fn validate(&self, packed_bin: &PackedBin<V, U>) -> Result<(), Vec<String>>;
}

