//! 管线模式定义 / Pipeline mode definitions
/// Demo2 管线模式 / Demo2 pipeline mode
///
/// 定义飞机装载优化的三种运行模式。
/// Defines three operational modes for aircraft loading optimization.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Demo2PipelineMode {
    /// 满载模式 / Full load mode
    FullLoad,
    /// 预分配模式 / Predistribution mode
    Predistribution,
    /// 重量推荐模式 / Weight recommendation mode
    WeightRecommendation,
}

/// 获取管线模式名称 / Get pipeline mode name
pub fn mode_name(mode: Demo2PipelineMode) -> &'static str {
    match mode {
        Demo2PipelineMode::FullLoad => "full_load",
        Demo2PipelineMode::Predistribution => "predistribution",
        Demo2PipelineMode::WeightRecommendation => "weight_recommendation",
    }
}
