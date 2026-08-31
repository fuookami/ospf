//! 冗余管线规格 / Redundancy pipeline specifications
use crate::framework::demo2::domain::redundancy::service::limits;
use crate::framework::demo2::domain::redundancy::service::pipeline_list_generator::RedundancyPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

/// 冗余管线规格列表 / Redundancy pipeline specification list
///
/// 定义冗余领域中各约束步骤的优先级和模式选择器。
/// Defines priority and mode selectors for each constraint step in the redundancy domain.
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
