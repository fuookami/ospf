//! 重心优化管线规格 / MAC optimization pipeline specifications
use crate::framework::demo2::domain::mac_optimization::service::limits;
use crate::framework::demo2::domain::mac_optimization::service::pipeline_list_generator::MacOptimizationPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

/// 返回 MAC 优化流水线规格列表 / Return MAC optimization pipeline specification list
///
/// 定义各约束限制步骤的优先级和适用模式。
/// Defines priority and applicable mode for each constraint limit step.
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
