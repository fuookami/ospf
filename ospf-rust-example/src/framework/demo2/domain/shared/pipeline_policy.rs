use crate::framework_demo::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

pub struct PipelineSpec<T> {
    pub priority: u8,
    pub mode_selector: ModeSelector,
    pub apply: T,
}

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
    use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

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
