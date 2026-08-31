//! 适航性安全管线规格 / Airworthiness security pipeline specifications
use crate::framework::demo2::domain::airworthiness_security::service::limits;
use crate::framework::demo2::domain::airworthiness_security::service::pipeline_list_generator::AirworthinessPipelineStep;
use crate::framework::demo2::domain::shared::mode_switch::ModeSelector;
use crate::framework::demo2::domain::shared::pipeline_policy::PipelineSpec;

/// 适航性安全管道规格定义 / Airworthiness security pipeline specification
///
/// 定义适航性约束的执行顺序和模式选择。
/// Defines the execution order and mode selection for airworthiness constraints.
pub fn pipeline_specs() -> Vec<PipelineSpec<AirworthinessPipelineStep>> {
    vec![
        PipelineSpec {
            priority: 10,
            mode_selector: ModeSelector::All,
            apply: limits::apply_payload_limits,
        },
        PipelineSpec {
            priority: 30,
            mode_selector: ModeSelector::All,
            apply: limits::apply_cumulative_load_weight_limits,
        },
        PipelineSpec {
            priority: 40,
            mode_selector: ModeSelector::All,
            apply: limits::apply_envelope_limits,
        },
        PipelineSpec {
            priority: 50,
            mode_selector: ModeSelector::All,
            apply: limits::apply_adjacent_gap_limits,
        },
    ]
}
