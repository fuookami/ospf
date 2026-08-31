use crate::framework_demo::demo2::domain::loading_effectiveness::service::limits;
use crate::framework_demo::demo2::domain::loading_effectiveness::service::pipeline_list_generator::LoadingEffectivenessPipelineStep;
use crate::framework_demo::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::PipelineSpec;

pub fn pipeline_specs() -> Vec<PipelineSpec<LoadingEffectivenessPipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::All,
            apply: limits::apply_priority_order_limits,
        },
        PipelineSpec {
            priority: 20,
            mode_selector: ModeSelector::NotPredistribution,
            apply: limits::apply_source_early_limits,
        },
    ]
}
