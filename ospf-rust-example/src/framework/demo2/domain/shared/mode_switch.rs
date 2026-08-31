use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

#[derive(Clone, Copy)]
pub enum ModeSelector {
    All,
    FullLoadOnly,
    PredistributionOnly,
    WeightRecommendationOnly,
    NotFullLoad,
    NotPredistribution,
    FullLoadOrWeightRecommendation,
}

impl ModeSelector {
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
