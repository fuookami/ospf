use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;
use crate::framework::demo2::domain::stowage::service::limits;
use crate::framework::demo2::domain::stowage::service::pipeline_list_generator::StowagePipelineStep;

pub fn pipeline_specs() -> Vec<PipelineSpec<StowagePipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::FullLoadOnly,
            apply: limits::apply_assignment_limits,
        },
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::PredistributionOnly,
            apply: limits::apply_assignment_limits,
        },
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::WeightRecommendationOnly,
            apply: limits::apply_assignment_limits,
        },
    ]
}
