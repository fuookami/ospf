//! 诊断信息常量与工具 / Diagnostic note constants and utilities
use crate::framework::demo2::infrastructure::dto::DiagnosticNote;

/// 诊断级别：诊断 / Note level: diagnostic
pub const NOTE_LEVEL_DIAGNOSTIC: &str = "diagnostic";
/// 诊断级别：严重 / Note level: critical
pub const NOTE_LEVEL_CRITICAL: &str = "critical";

/// 诊断分组：适航性 / Note group: airworthiness
pub const NOTE_GROUP_AIRWORTHINESS: &str = "airworthiness";
/// 诊断分组：载量 / Note group: payload
pub const NOTE_GROUP_PAYLOAD: &str = "payload";
/// 诊断分组：MAC 优化 / Note group: MAC optimization
pub const NOTE_GROUP_MAC_OPTIMIZATION: &str = "mac_optimization";
/// 诊断分组：冗余 / Note group: redundancy
pub const NOTE_GROUP_REDUNDANCY: &str = "redundancy";
/// 诊断分组：求解器 / Note group: solver
pub const NOTE_GROUP_SOLVER: &str = "solver";

/// 诊断码：包络范围无效 / Note code: envelope range invalid
pub const NOTE_CODE_ENVELOPE_RANGE_INVALID: &str = "envelope_range_invalid";
/// 诊断码：载量上限为负 / Note code: payload upper negative
pub const NOTE_CODE_PAYLOAD_UPPER_NEGATIVE: &str = "payload_upper_negative";
/// 诊断码：最小载量比率越界 / Note code: min payload ratio out of range
pub const NOTE_CODE_MIN_PAYLOAD_RATIO_OUT_OF_RANGE: &str = "min_payload_ratio_out_of_range";
/// 诊断码：最小载量超过上限 / Note code: min payload greater than upper
pub const NOTE_CODE_MIN_PAYLOAD_GT_UPPER: &str = "min_payload_gt_upper";
/// 诊断码：最小载量超过总容量 / Note code: min payload greater than total capacity
pub const NOTE_CODE_MIN_PAYLOAD_GT_TOTAL_CAPACITY: &str = "min_payload_gt_total_capacity";
/// 诊断码：货物超过所有舱位容量 / Note code: cargo exceeds all positions
pub const NOTE_CODE_CARGO_EXCEEDS_ALL_POSITIONS: &str = "cargo_exceeds_all_positions";

/// 诊断码：容量利用率高 / Note code: capacity utilization high
pub const NOTE_CODE_CAPACITY_UTILIZATION_HIGH: &str = "capacity_utilization_high";
/// 诊断码：载量上限利用率高 / Note code: payload upper utilization high
pub const NOTE_CODE_PAYLOAD_UPPER_UTILIZATION_HIGH: &str = "payload_upper_utilization_high";
/// 诊断码：载量下限接近 / Note code: payload lower close
pub const NOTE_CODE_PAYLOAD_LOWER_CLOSE: &str = "payload_lower_close";
/// 诊断码：纵向包络上限接近 / Note code: envelope longitudinal max close
pub const NOTE_CODE_ENVELOPE_LONGITUDINAL_MAX_CLOSE: &str = "envelope_longitudinal_max_close";
/// 诊断码：纵向包络下限接近 / Note code: envelope longitudinal min close
pub const NOTE_CODE_ENVELOPE_LONGITUDINAL_MIN_CLOSE: &str = "envelope_longitudinal_min_close";
/// 诊断码：横向不平衡接近 / Note code: lateral imbalance close
pub const NOTE_CODE_LATERAL_IMBALANCE_CLOSE: &str = "lateral_imbalance_close";
/// 诊断码：目的地集中度高 / Note code: destination concentration high
pub const NOTE_CODE_REDUNDANCY_DESTINATION_CONCENTRATION_HIGH: &str =
    "destination_concentration_high";
/// 诊断码：Benders 迭代次数 / Note code: Benders iterations
pub const NOTE_CODE_BENDERS_ITERATIONS: &str = "benders_iterations";
/// 诊断码：Benders 间隙 / Note code: Benders gap
pub const NOTE_CODE_BENDERS_GAP: &str = "benders_gap";
/// 诊断码：Benders 耗时（毫秒） / Note code: Benders time in milliseconds
pub const NOTE_CODE_BENDERS_TIME_MS: &str = "benders_time_ms";
/// 诊断码：Benders 自适应生效 / Note code: Benders adaptive effective
pub const NOTE_CODE_BENDERS_ADAPTIVE_EFFECTIVE: &str = "benders_adaptive_effective";
/// 诊断码：Benders 问题规模二元变量数 / Note code: Benders problem size binary variables
pub const NOTE_CODE_BENDERS_PROBLEM_SIZE_BINARY_VARIABLES: &str =
    "benders_problem_size_binary_variables";
/// 诊断码：Benders 间隙守卫超限 / Note code: Benders gap guard exceeded
pub const NOTE_CODE_BENDERS_GAP_GUARD_EXCEEDED: &str = "benders_gap_guard_exceeded";
/// 诊断码：Benders 时间守卫超限 / Note code: Benders time guard exceeded
pub const NOTE_CODE_BENDERS_TIME_GUARD_EXCEEDED: &str = "benders_time_guard_exceeded";
/// 诊断码：Benders 进度守卫触发 / Note code: Benders progress guard triggered
pub const NOTE_CODE_BENDERS_PROGRESS_GUARD_TRIGGERED: &str = "benders_progress_guard_triggered";
/// 诊断码：Benders 割效率低 / Note code: Benders cut efficiency low
pub const NOTE_CODE_BENDERS_CUT_EFFICIENCY_LOW: &str = "benders_cut_efficiency_low";
/// 诊断码：Benders 轨迹弱 / Note code: Benders trajectory weak
pub const NOTE_CODE_BENDERS_TRAJECTORY_WEAK: &str = "benders_trajectory_weak";
/// 诊断码：Benders 质量守卫生效 / Note code: Benders quality guard effective
pub const NOTE_CODE_BENDERS_QUALITY_GUARD_EFFECTIVE: &str = "benders_quality_guard_effective";
/// 诊断码：Benders 质量评分 / Note code: Benders quality score
pub const NOTE_CODE_BENDERS_QUALITY_SCORE: &str = "benders_quality_score";
/// 诊断码：Benders 质量动作 / Note code: Benders quality action
pub const NOTE_CODE_BENDERS_QUALITY_ACTION: &str = "benders_quality_action";
/// 诊断码：求解器路径 / Note code: solver path
pub const NOTE_CODE_SOLVER_PATH: &str = "solver_path";
/// 诊断码：Benders 求解失败 / Note code: Benders failed
pub const NOTE_CODE_BENDERS_FAILED: &str = "benders_failed";

/// 添加分组诊断信息 / Push a grouped diagnostic note
pub fn push_grouped_note(
    notes: &mut Vec<String>,
    level: &str,
    group: &str,
    code: &str,
    message: &str,
) {
    notes.push(format!(
        "{}|group={}|code={}|msg={}",
        level, group, code, message
    ));
}

fn parse_grouped_note(note: &str) -> Option<DiagnosticNote> {
    let mut segments = note.split("|");
    let level = segments.next()?.trim();
    if level.is_empty() {
        return None;
    }

    let mut group: Option<String> = None;
    let mut code: Option<String> = None;
    let mut message: Option<String> = None;
    for segment in segments {
        let mut kv = segment.splitn(2, '=');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        match key {
            "group" => group = Some(value.to_string()),
            "code" => code = Some(value.to_string()),
            "msg" => message = Some(value.to_string()),
            _ => {}
        }
    }

    message.map(|msg| DiagnosticNote {
        level: level.to_string(),
        group,
        code,
        message: msg,
    })
}

/// 构建结构化诊断信息列表 / Build structured diagnostics from raw notes
pub fn build_structured_diagnostics(notes: &[String]) -> Vec<DiagnosticNote> {
    notes
        .iter()
        .map(|note| {
            parse_grouped_note(note).unwrap_or_else(|| DiagnosticNote {
                level: String::from(NOTE_LEVEL_DIAGNOSTIC),
                group: None,
                code: None,
                message: note.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_structured_diagnostics_parses_grouped_note() {
        let notes = vec![String::from(
            "critical|group=payload|code=payload_upper_utilization_high|msg=payload near upper",
        )];
        let diagnostics = build_structured_diagnostics(&notes);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].level, "critical");
        assert_eq!(diagnostics[0].group.as_deref(), Some("payload"));
        assert_eq!(
            diagnostics[0].code.as_deref(),
            Some("payload_upper_utilization_high")
        );
    }

    #[test]
    fn build_structured_diagnostics_fallbacks_plain_note() {
        let notes = vec![String::from("unsupported aircraft type")];
        let diagnostics = build_structured_diagnostics(&notes);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].level, NOTE_LEVEL_DIAGNOSTIC);
        assert!(diagnostics[0].group.is_none());
        assert!(diagnostics[0].code.is_none());
        assert_eq!(diagnostics[0].message, "unsupported aircraft type");
    }
}
