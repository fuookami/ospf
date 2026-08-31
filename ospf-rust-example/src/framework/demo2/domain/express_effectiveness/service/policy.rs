//! 快递效能流水线策略 / Express effectiveness pipeline policy
//!
//! 定义各流水线步骤的优先级和模式选择器。

use crate::framework::demo2::domain::express_effectiveness::service::limits;
use crate::framework::demo2::domain::express_effectiveness::service::pipeline_list_generator::ExpressEffectivenessPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

/// 获取快递效能流水线规格列表 / Get express effectiveness pipeline specifications
pub fn pipeline_specs() -> Vec<PipelineSpec<ExpressEffectivenessPipelineStep>> {
    vec![PipelineSpec {
        priority: 10,
        mode_selector: ModeSelector::FullLoadOrWeightRecommendation,
        apply: limits::apply_must_ship_limits,
    }]
}
