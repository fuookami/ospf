use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;
use crate::framework::demo2::domain::redundancy::service::limits;
use crate::framework::demo2::domain::redundancy::service::pipeline_list_generator::RedundancyPipelineStep;

pub fn pipeline_specs() -> Vec<PipelineSpec<RedundancyPipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::NotPredistribution,
            apply: limits::apply_destination_spread_limits,
        },
        PipelineSpec {
            priority: 20,
            mode_selector: ModeSelector::NotPredistribution,
            apply: limits::apply_experimental_longitudinal_balance_limits,
        },
        PipelineSpec {
            priority: 30,
            mode_selector: ModeSelector::NotPredistribution,
            apply: limits::apply_redundancy_limits,
        },
    ]
}