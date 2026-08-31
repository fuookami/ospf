use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::soft_security::service::policy;

pub type SoftSecurityPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>>;

pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<SoftSecurityPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_security_pipeline_applies_in_all_modes() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 2);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 1);
        assert_eq!(
            pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(),
            2
        );
    }
}
