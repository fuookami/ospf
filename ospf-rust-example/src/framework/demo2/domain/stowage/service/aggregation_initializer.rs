//! 装载聚合初始化器 / Stowage aggregation initializer
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;

/// 装载聚合初始化器 / Stowage aggregation initializer
/// 对齐 Kotlin stowage AggregationInitializer
pub struct StowageAggregationInitializer;

impl StowageAggregationInitializer {
    /// 从上下文初始化装载聚合参数 / Initialize stowage aggregation parameters from context
    pub fn initialize(context: &StowageContext<'_>) -> StowageAggregation {
        StowageAggregation::from_context(context)
    }
}
