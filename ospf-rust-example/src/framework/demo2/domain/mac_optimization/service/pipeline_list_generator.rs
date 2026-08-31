//! 重心优化管线步骤生成器 / MAC optimization pipeline step generator
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::mac_optimization::service::policy;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// MAC 优化流水线步骤函数类型 / MAC optimization pipeline step function type
pub type MacOptimizationPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>>;

/// 获取指定模式下的流水线步骤列表 / Get pipeline steps for the specified mode
pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<MacOptimizationPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_pipeline_applies_mode_filter() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 1);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 2);
        assert_eq!(
            pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(),
            2
        );
    }
}
