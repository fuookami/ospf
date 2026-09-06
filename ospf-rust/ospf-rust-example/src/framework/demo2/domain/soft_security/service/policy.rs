//! 软性安全管线规格 / Soft security pipeline specifications
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;
use crate::framework::demo2::domain::soft_security::service::limits;
use crate::framework::demo2::domain::soft_security::service::pipeline_list_generator::SoftSecurityPipelineStep;

/// 软安全管线规格列表 / Soft security pipeline specification list
///
/// 定义软安全领域中各约束步骤的优先级和模式选择器。
/// Defines priority and mode selectors for each constraint step in the soft security domain.
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
        PipelineSpec {
            priority: 30,
            mode_selector: ModeSelector::All,
            apply: limits::apply_divide_empty_loading_limits,
        },
    ]
}
