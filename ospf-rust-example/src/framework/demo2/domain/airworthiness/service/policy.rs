use crate::framework_demo::demo2::domain::airworthiness::service::limits;
use crate::framework_demo::demo2::domain::airworthiness::service::pipeline_list_generator::AirworthinessPipelineStep;
use crate::framework_demo::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::PipelineSpec;

pub fn pipeline_specs() -> Vec<PipelineSpec<AirworthinessPipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::All,
            apply: limits::apply_capacity_limits,
        },
        PipelineSpec {
            priority: 20,
            mode_selector: ModeSelector::All,
            apply: limits::apply_payload_limits,
        },
        PipelineSpec {
            priority: 30,
            mode_selector: ModeSelector::All,
            apply: limits::apply_cumulative_limits,
        },
        PipelineSpec {
            priority: 40,
            mode_selector: ModeSelector::All,
            apply: limits::apply_envelope_limits,
        },
        PipelineSpec {
            priority: 50,
            mode_selector: ModeSelector::All,
            apply: limits::apply_adjacent_gap_limits,
        },
    ]
}
