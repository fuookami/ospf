//! 适航性安全管线步骤生成器 / Airworthiness security pipeline step generator
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::airworthiness_security::service::policy;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 适航性管道步骤类型 / Airworthiness pipeline step type
///
/// 每个步骤接收模型、上下文、聚合以及估算载荷变量索引，
/// 向模型中注册适航性约束。
/// Each step receives the model, context, aggregation, and estimated load
/// variable indices, registering airworthiness constraints into the model.
pub type AirworthinessPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
    estimate_load_weight_idx: &[usize],
    estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>>;

/// 获取指定模式下的管道步骤列表 / Get pipeline steps for the specified mode
pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<AirworthinessPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airworthiness_security_pipeline_applies_in_all_modes() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 4);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 4);
        assert_eq!(
            pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(),
            4
        );
    }
}
