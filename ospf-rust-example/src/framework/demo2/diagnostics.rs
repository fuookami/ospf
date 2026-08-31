use crate::framework_demo::demo2::infrastructure::dto::DiagnosticNote;

pub const NOTE_LEVEL_DIAGNOSTIC: &str = "diagnostic";
pub const NOTE_LEVEL_CRITICAL: &str = "critical";

pub const NOTE_GROUP_AIRWORTHINESS: &str = "airworthiness";
pub const NOTE_GROUP_PAYLOAD: &str = "payload";
pub const NOTE_GROUP_MAC_OPTIMIZATION: &str = "mac_optimization";
pub const NOTE_GROUP_REDUNDANCY: &str = "redundancy";
pub const NOTE_GROUP_SOLVER: &str = "solver";

pub const NOTE_CODE_ENVELOPE_RANGE_INVALID: &str = "envelope_range_invalid";
pub const NOTE_CODE_PAYLOAD_UPPER_NEGATIVE: &str = "payload_upper_negative";
pub const NOTE_CODE_MIN_PAYLOAD_RATIO_OUT_OF_RANGE: &str = "min_payload_ratio_out_of_range";
pub const NOTE_CODE_MIN_PAYLOAD_GT_UPPER: &str = "min_payload_gt_upper";
pub const NOTE_CODE_MIN_PAYLOAD_GT_TOTAL_CAPACITY: &str = "min_payload_gt_total_capacity";
pub const NOTE_CODE_CARGO_EXCEEDS_ALL_POSITIONS: &str = "cargo_exceeds_all_positions";

pub const NOTE_CODE_CAPACITY_UTILIZATION_HIGH: &str = "capacity_utilization_high";
pub const NOTE_CODE_PAYLOAD_UPPER_UTILIZATION_HIGH: &str = "payload_upper_utilization_high";
pub const NOTE_CODE_PAYLOAD_LOWER_CLOSE: &str = "payload_lower_close";
pub const NOTE_CODE_ENVELOPE_LONGITUDINAL_MAX_CLOSE: &str = "envelope_longitudinal_max_close";
pub const NOTE_CODE_ENVELOPE_LONGITUDINAL_MIN_CLOSE: &str = "envelope_longitudinal_min_close";
pub const NOTE_CODE_LATERAL_IMBALANCE_CLOSE: &str = "lateral_imbalance_close";
pub const NOTE_CODE_REDUNDANCY_DESTINATION_CONCENTRATION_HIGH: &str =
    "destination_concentration_high";
pub const NOTE_CODE_BENDERS_ITERATIONS: &str = "benders_iterations";
pub const NOTE_CODE_BENDERS_GAP: &str = "benders_gap";
pub const NOTE_CODE_BENDERS_TIME_MS: &str = "benders_time_ms";
pub const NOTE_CODE_BENDERS_ADAPTIVE_EFFECTIVE: &str = "benders_adaptive_effective";
pub const NOTE_CODE_BENDERS_PROBLEM_SIZE_BINARY_VARIABLES: &str =
    "benders_problem_size_binary_variables";
pub const NOTE_CODE_BENDERS_GAP_GUARD_EXCEEDED: &str = "benders_gap_guard_exceeded";
pub const NOTE_CODE_BENDERS_TIME_GUARD_EXCEEDED: &str = "benders_time_guard_exceeded";
pub const NOTE_CODE_BENDERS_PROGRESS_GUARD_TRIGGERED: &str = "benders_progress_guard_triggered";
pub const NOTE_CODE_BENDERS_CUT_EFFICIENCY_LOW: &str = "benders_cut_efficiency_low";
pub const NOTE_CODE_BENDERS_TRAJECTORY_WEAK: &str = "benders_trajectory_weak";
pub const NOTE_CODE_BENDERS_QUALITY_GUARD_EFFECTIVE: &str = "benders_quality_guard_effective";
pub const NOTE_CODE_BENDERS_QUALITY_SCORE: &str = "benders_quality_score";
pub const NOTE_CODE_BENDERS_QUALITY_ACTION: &str = "benders_quality_action";
pub const NOTE_CODE_SOLVER_PATH: &str = "solver_path";
pub const NOTE_CODE_BENDERS_FAILED: &str = "benders_failed";

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
