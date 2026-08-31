use crate::framework_demo::demo2::domain::express_effectiveness::service::limits;
use crate::framework_demo::demo2::domain::express_effectiveness::service::pipeline_list_generator::ExpressEffectivenessPipelineStep;
use crate::framework_demo::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework_demo::demo2::domain::shared::pipeline_policy::PipelineSpec;

pub fn pipeline_specs() -> Vec<PipelineSpec<ExpressEffectivenessPipelineStep>> {
    vec![PipelineSpec {
        priority: 10,
        mode_selector: ModeSelector::FullLoadOrWeightRecommendation,
        apply: limits::apply_must_ship_limits,
    }]
}
