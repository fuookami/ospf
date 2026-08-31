use std::error::Error;

use ospf_rust_core::model::MetaModel;

use crate::framework::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;
use crate::framework::demo2::domain::express_effectiveness::service::policy;

pub type ExpressEffectivenessPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &ExpressEffectivenessContext<'_>,
    aggregation: &ExpressEffectivenessAggregation,
) -> Result<(), Box<dyn Error>>;

pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<ExpressEffectivenessPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::demo2::domain::express_effectiveness::service::limits;

    #[test]
    fn express_pipeline_is_disabled_in_predistribution() {
        let predistribution = pipeline_steps(Demo2PipelineMode::Predistribution);
        let full_load = pipeline_steps(Demo2PipelineMode::FullLoad);
        let weight_recommendation = pipeline_steps(Demo2PipelineMode::WeightRecommendation);
        assert!(predistribution.is_empty());
        assert_eq!(full_load.len(), 1);
        assert_eq!(weight_recommendation.len(), 1);
        assert_eq!(full_load[0] as usize, limits::apply_must_ship_limits as usize);
        assert_eq!(
            weight_recommendation[0] as usize,
            limits::apply_must_ship_limits as usize
        );
    }
}
