use crate::framework::demo2::domain::mac_optimization::service::limits;
use crate::framework::demo2::domain::mac_optimization::service::pipeline_list_generator::MacOptimizationPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

pub fn pipeline_specs() -> Vec<PipelineSpec<MacOptimizationPipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::PredistributionOnly,
            apply: limits::apply_longitudinal_balance_limits,
        },
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::WeightRecommendationOnly,
            apply: limits::apply_longitudinal_balance_limits,
        },
        PipelineSpec {
            priority: 20,
            mode_selector: ModeSelector::All,
            apply: limits::apply_lateral_balance_limits,
        },
    ]
}
