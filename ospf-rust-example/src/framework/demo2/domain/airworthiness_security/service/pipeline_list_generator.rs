use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::airworthiness_security::service::policy;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;

pub type AirworthinessPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
    estimate_load_weight_idx: &[usize],
    estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>>;

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
