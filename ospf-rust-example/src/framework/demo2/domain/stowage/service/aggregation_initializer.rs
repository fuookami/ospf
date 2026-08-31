use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;

/// 装载聚合初始化器 / Stowage aggregation initializer
/// 对齐 Kotlin stowage AggregationInitializer
pub struct StowageAggregationInitializer;

impl StowageAggregationInitializer {
    pub fn initialize(context: &StowageContext<'_>) -> StowageAggregation {
        StowageAggregation::from_context(context)
    }
}
