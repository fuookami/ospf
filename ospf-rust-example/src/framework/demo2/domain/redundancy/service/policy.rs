use crate::framework::demo2::domain::redundancy::service::limits;
use crate::framework::demo2::domain::redundancy::service::pipeline_list_generator::RedundancyPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

pub fn pipeline_specs() -> Vec<PipelineSpec<RedundancyPipelineStep>> {
    vec![PipelineSpec {
        priority: 10,
        mode_selector: ModeSelector::NotPredistribution,
        apply: limits::apply_destination_spread_limits,
    }]
}

