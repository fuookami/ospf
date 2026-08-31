use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework_demo::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework_demo::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework_demo::demo2::domain::redundancy::service::policy;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;

pub type RedundancyPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &RedundancyContext<'_>,
    aggregation: &RedundancyAggregation,
) -> Result<(), Box<dyn Error>>;

pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<RedundancyPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redundancy_pipeline_applies_mode_filter() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 1);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 0);
        assert_eq!(
            pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(),
            1
        );
    }
}
