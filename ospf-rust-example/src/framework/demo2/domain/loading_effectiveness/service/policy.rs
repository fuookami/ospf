//! 装载效能流水线策略 / Loading effectiveness pipeline policy
//!
//! 定义各流水线步骤的优先级和模式选择器。

use crate::framework::demo2::domain::loading_effectiveness::service::limits;
use crate::framework::demo2::domain::loading_effectiveness::service::pipeline_list_generator::LoadingEffectivenessPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

/// 获取装载效能流水线规格列表 / Get loading effectiveness pipeline specifications
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
