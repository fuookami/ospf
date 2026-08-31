#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Demo2PipelineMode {
    FullLoad,
    Predistribution,
    WeightRecommendation,
}

pub fn mode_name(mode: Demo2PipelineMode) -> &'static str {
    match mode {
        Demo2PipelineMode::FullLoad => "full_load",
        Demo2PipelineMode::Predistribution => "predistribution",
        Demo2PipelineMode::WeightRecommendation => "weight_recommendation",
    }
}
