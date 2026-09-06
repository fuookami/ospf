//! 模式选择器 / Mode selector
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

/// 模式选择器 / Mode selector
///
/// 用于管线步骤的模式过滤，决定步骤在哪些管线模式下生效。
/// Used for pipeline step mode filtering, determining which pipeline modes a step applies to.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum ModeSelector {
    /// 所有模式 / All modes
    All,
    /// 仅满载模式 / Full load mode only
    FullLoadOnly,
    /// 仅预分配模式 / Predistribution mode only
    PredistributionOnly,
    /// 仅重量推荐模式 / Weight recommendation mode only
    WeightRecommendationOnly,
    /// 排除满载模式 / Exclude full load mode
    NotFullLoad,
    /// 排除预分配模式 / Exclude predistribution mode
    NotPredistribution,
    /// 满载或重量推荐模式 / Full load or weight recommendation mode
    FullLoadOrWeightRecommendation,
}

impl ModeSelector {
    /// 判断指定模式是否被此选择器允许 / Check if the specified mode is allowed by this selector
    pub fn allows(self, mode: Demo2PipelineMode) -> bool {
        match self {
            ModeSelector::All => true,
            ModeSelector::FullLoadOnly => matches!(mode, Demo2PipelineMode::FullLoad),
            ModeSelector::PredistributionOnly => {
                matches!(mode, Demo2PipelineMode::Predistribution)
            }
            ModeSelector::WeightRecommendationOnly => {
                matches!(mode, Demo2PipelineMode::WeightRecommendation)
            }
            ModeSelector::NotFullLoad => !matches!(mode, Demo2PipelineMode::FullLoad),
            ModeSelector::NotPredistribution => !matches!(mode, Demo2PipelineMode::Predistribution),
            ModeSelector::FullLoadOrWeightRecommendation => matches!(
                mode,
                Demo2PipelineMode::FullLoad | Demo2PipelineMode::WeightRecommendation
            ),
        }
    }
}
