use crate::framework_demo::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::PipelineSpec;
use crate::framework_demo::demo2::domain::soft_security::service::limits;
use crate::framework_demo::demo2::domain::soft_security::service::pipeline_list_generator::SoftSecurityPipelineStep;

pub fn pipeline_specs() -> Vec<PipelineSpec<SoftSecurityPipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::All,
            apply: limits::apply_separation_limits,
        },
        PipelineSpec {
            priority: 20,
            mode_selector: ModeSelector::NotPredistribution,
            apply: limits::apply_adjacent_separation_limits,
        },
    ]
}
