//! 软性安全管线步骤生成器 / Soft security pipeline step generator
use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::soft_security::service::policy;

/// 软安全管线步骤函数类型 / Soft security pipeline step function type
pub type SoftSecurityPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    aggregation: &mut SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>>;

/// 获取指定模式下的软安全管线步骤 / Get soft security pipeline steps for the specified mode
pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<SoftSecurityPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_security_pipeline_applies_in_all_modes() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 3);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 2);
        assert_eq!(
            pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(),
            3
        );
    }
}
