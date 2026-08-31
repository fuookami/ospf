use std::error::Error;

use ospf_rust_core::model::MetaModel;

use crate::framework_demo::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework_demo::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework_demo::demo2::domain::express_effectiveness::service::policy;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;

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
    use crate::framework_demo::demo2::domain::express_effectiveness::service::limits;

    #[test]
    fn express_pipeline_is_disabled_in_predistribution() {
        let predistribution = pipeline_steps(Demo2PipelineMode::Predistribution);
        let full_load = pipeline_steps(Demo2PipelineMode::FullLoad);
        let weight_recommendation = pipeline_steps(Demo2PipelineMode::WeightRecommendation);
        assert!(predistribution.is_empty());
        assert_eq!(full_load.len(), 1);
        assert_eq!(weight_recommendation.len(), 1);
        assert!(std::ptr::fn_addr_eq(
            full_load[0] as ExpressEffectivenessPipelineStep,
            limits::apply_must_ship_limits as ExpressEffectivenessPipelineStep
        ));
        assert!(std::ptr::fn_addr_eq(
            weight_recommendation[0] as ExpressEffectivenessPipelineStep,
            limits::apply_must_ship_limits as ExpressEffectivenessPipelineStep
        ));
    }
}
