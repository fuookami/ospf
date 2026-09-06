//! 管线策略框架 / Pipeline policy framework
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

/// 管线步骤规格 / Pipeline step specification
///
/// 定义管线步骤的优先级、模式选择器和执行函数。
/// Defines priority, mode selector, and execution function for a pipeline step.
pub struct PipelineSpec<T> {
    /// 优先级（数值越小越先执行） / Priority (lower values execute first)
    pub priority: u8,
    /// 模式选择器 / Mode selector
    pub mode_selector: ModeSelector,
    /// 执行函数 / Apply function
    pub apply: T,
}

/// 收集管线步骤 / Collect pipeline steps
///
/// 按优先级排序并过滤出指定模式允许的步骤。
/// Sorts by priority and filters steps allowed by the specified mode.
pub fn collect_pipeline_steps<T: Copy>(
    mode: Demo2PipelineMode,
    mut specs: Vec<PipelineSpec<T>>,
) -> Vec<T> {
    specs.sort_by_key(|spec| spec.priority);
    specs
        .into_iter()
        .filter(|spec| spec.mode_selector.allows(mode))
        .map(|spec| spec.apply)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

    #[test]
    fn collect_pipeline_steps_filters_and_sorts() {
        let steps = collect_pipeline_steps(
            Demo2PipelineMode::FullLoad,
            vec![
                PipelineSpec {
                    priority: 20,
                    mode_selector: ModeSelector::All,
                    apply: 2_u8,
                },
                PipelineSpec {
                    priority: 10,
                    mode_selector: ModeSelector::FullLoadOrWeightRecommendation,
                    apply: 1_u8,
                },
                PipelineSpec {
                    priority: 5,
                    mode_selector: ModeSelector::NotFullLoad,
                    apply: 9_u8,
                },
            ],
        );
        assert_eq!(steps, vec![1_u8, 2_u8]);
    }
}
