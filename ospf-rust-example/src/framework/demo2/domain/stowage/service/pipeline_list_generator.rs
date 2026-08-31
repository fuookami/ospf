use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use crate::framework_demo::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework_demo::demo2::domain::stowage::context::StowageContext;
use crate::framework_demo::demo2::domain::stowage::service::policy;

pub type StowagePipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    aggregation: &StowageAggregation,
) -> Result<(), Box<dyn Error>>;

pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<StowagePipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stowage_pipeline_applies_in_all_modes() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 1);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 1);
        assert_eq!(
            pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(),
            1
        );
    }
}
