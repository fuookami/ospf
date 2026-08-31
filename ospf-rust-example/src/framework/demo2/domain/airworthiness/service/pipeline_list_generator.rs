use std::error::Error;

use ospf_rust_core::model::MetaModel;

use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use crate::framework::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework::demo2::domain::airworthiness::service::policy;

pub type AirworthinessPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>>;

pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<AirworthinessPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airworthiness_pipeline_applies_in_all_modes() {
        assert_eq!(pipeline_steps(Demo2PipelineMode::FullLoad).len(), 5);
        assert_eq!(pipeline_steps(Demo2PipelineMode::Predistribution).len(), 5);
        assert_eq!(pipeline_steps(Demo2PipelineMode::WeightRecommendation).len(), 5);
    }
}
