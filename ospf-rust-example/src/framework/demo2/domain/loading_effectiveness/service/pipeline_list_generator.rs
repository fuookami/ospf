use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework_demo::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework_demo::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework_demo::demo2::domain::loading_effectiveness::service::policy;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::collect_pipeline_steps;

pub type LoadingEffectivenessPipelineStep = fn(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>>;

pub fn pipeline_steps(mode: Demo2PipelineMode) -> Vec<LoadingEffectivenessPipelineStep> {
    collect_pipeline_steps(mode, policy::pipeline_specs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework_demo::demo2::domain::loading_effectiveness::service::limits;

    #[test]
    fn loading_pipeline_applies_mode_filter() {
        let predistribution = pipeline_steps(Demo2PipelineMode::Predistribution);
        let full_load = pipeline_steps(Demo2PipelineMode::FullLoad);
        let weight_recommendation = pipeline_steps(Demo2PipelineMode::WeightRecommendation);
        assert_eq!(predistribution.len(), 1);
        assert_eq!(full_load.len(), 2);
        assert_eq!(weight_recommendation.len(), 2);
        assert!(std::ptr::fn_addr_eq(
            predistribution[0] as LoadingEffectivenessPipelineStep,
            limits::apply_priority_order_limits as LoadingEffectivenessPipelineStep
        ));
        assert!(std::ptr::fn_addr_eq(
            full_load[0] as LoadingEffectivenessPipelineStep,
            limits::apply_priority_order_limits as LoadingEffectivenessPipelineStep
        ));
        assert!(std::ptr::fn_addr_eq(
            full_load[1] as LoadingEffectivenessPipelineStep,
            limits::apply_source_early_limits as LoadingEffectivenessPipelineStep
        ));
        assert!(std::ptr::fn_addr_eq(
            weight_recommendation[0] as LoadingEffectivenessPipelineStep,
            limits::apply_priority_order_limits as LoadingEffectivenessPipelineStep
        ));
        assert!(std::ptr::fn_addr_eq(
            weight_recommendation[1] as LoadingEffectivenessPipelineStep,
            limits::apply_source_early_limits as LoadingEffectivenessPipelineStep
        ));
    }
}
