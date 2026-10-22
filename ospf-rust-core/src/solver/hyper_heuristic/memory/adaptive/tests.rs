//! 接受历史、缩放不变性与坏状态回归 / Acceptance history, scale invariance, and invalid-state regressions

use crate::model::callback::Solution;
use crate::solver::heuristic::session::TextNumberCodec;
use crate::variable::{VariableId, new_standalone_id};
use super::*;

struct Problem { x: VariableId, maximize: bool }

impl SearchProblem<f64, f64> for Problem {
    fn id(&self) -> &str { "acceptance" }
    fn fingerprint(&self) -> &str { "acceptance:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn initial_solutions(&self, _: usize) -> Vec<Solution<f64>> { Vec::new() }
    fn objective(&self, _: &Solution<f64>) -> Option<f64> { panic!("acceptance must not evaluate an objective"); }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { if self.maximize { lhs > rhs } else { lhs < rhs } }
    fn has_objective_coordinates(&self) -> bool { true }
    fn objective_coordinates(&self, value: &f64) -> Result<Vec<f64>> { Ok(vec![if self.maximize { -*value } else { *value }]) }
}

fn problem() -> Problem { Problem { x: new_standalone_id(), maximize: false } }
fn point(problem: &Problem, value: f64) -> Individual<f64, f64> { Individual::new(Solution::from([(problem.x, value)]), value) }

#[test]
fn late_acceptance_advances_history_on_rejection_and_restores_cursor() {
    let problem = problem();
    let mut policy = LateAcceptance::new(Box::new(TextNumberCodec), 2).unwrap();
    assert!(policy.accept(&problem, &[point(&problem, 10.0)], &point(&problem, 8.0)));
    assert!(policy.accept(&problem, &[point(&problem, 8.0)], &point(&problem, 9.0)));
    assert!(!policy.accept(&problem, &[point(&problem, 9.0)], &point(&problem, 20.0)));
    let saved = policy.save().unwrap();
    assert_eq!(saved["cursor"], 1); assert_eq!(saved["history"], serde_json::json!(["9.0", "9.0"]));
    let mut restored = LateAcceptance::new(Box::new(TextNumberCodec), 2).unwrap(); restored.load(&saved).unwrap();
    for value in [11.0, 8.5, -2.0, 8.0] {
        assert_eq!(restored.accept(&problem, &[point(&problem, 9.0)], &point(&problem, value)),
            policy.accept(&problem, &[point(&problem, 9.0)], &point(&problem, value)));
    }
    assert_eq!(restored.save().unwrap(), policy.save().unwrap());
}

#[test]
fn record_threshold_tightens_to_zero_and_preserves_negative_maximization_coordinates() {
    let problem = problem();
    let mut policy = RecordToRecordAcceptance::new(0.05, 2).unwrap();
    assert!(policy.accept(&problem, &[point(&problem, 100.0)], &point(&problem, 104.0)));
    let saved = policy.save().unwrap();
    let mut restored = RecordToRecordAcceptance::new(0.05, 2).unwrap(); restored.load(&saved).unwrap();
    for value in [104.0, 100.1, 99.0] {
        assert_eq!(restored.accept(&problem, &[point(&problem, 100.0)], &point(&problem, value)),
            policy.accept(&problem, &[point(&problem, 100.0)], &point(&problem, value)));
    }
    assert_eq!(policy.save().unwrap()["clock"], 4); assert_eq!(policy.save().unwrap()["best"], 99.0);
    assert_eq!(restored.save().unwrap(), policy.save().unwrap());
    let mut policy = RecordToRecordAcceptance::new(0.05, 2).unwrap();
    assert!(policy.accept(&problem, &[point(&problem, -100.0)], &point(&problem, -96.0)));
    let problem = Problem { maximize: true, ..problem };
    let mut policy = RecordToRecordAcceptance::new(0.05, 2).unwrap();
    assert!(policy.accept(&problem, &[point(&problem, 100.0)], &point(&problem, 96.0)));
}

#[test]
fn automatic_capacity_matches_shared_square_root_boundary_vectors() {
    for (horizon, capacity) in [(0, 8), (63, 8), (64, 8), (80, 8), (81, 9), (99, 9), (100, 10),
        (65535, 255), (65536, 256), (65537, 256), (i64::MAX as u64, 256)] {
        let policy = AutoAcceptance::new(Box::new(TextNumberCodec), horizon).unwrap();
        assert_eq!(policy.save().unwrap()["capacity"], capacity);
        assert_eq!(policy.history_span(), capacity as usize / 2);
    }
}

#[test]
fn automatic_span_counts_only_strict_improvements_and_does_not_stop_at_horizon() {
    let problem = problem();
    let mut policy = AutoAcceptance::new(Box::new(TextNumberCodec), 64).unwrap();
    assert_eq!(policy.history_span(), 4);
    for _ in 0..8 { assert!(policy.accept(&problem, &[point(&problem, 10.0)], &point(&problem, 10.0))); }
    assert_eq!(policy.history_span(), 8);
    for _ in 0..8 { assert!(policy.accept(&problem, &[point(&problem, 10.0)], &point(&problem, 9.0))); }
    assert_eq!(policy.history_span(), 4);
    for _ in 0..56 { assert!(policy.accept(&problem, &[point(&problem, 10.0)], &point(&problem, 9.0))); }
    assert_eq!(policy.save().unwrap()["decisions"], 72);
    assert_eq!(policy.history_span(), 2);
}

#[test]
fn automatic_acceptance_is_positive_scale_and_translation_invariant_with_exact_resume() {
    let problem = problem();
    let mut reference = None;
    for (scale, offset) in [(1.0, 0.0), (4.0, 100.0), (0.5, -1000.0)] {
        let mut policy = AutoAcceptance::new(Box::new(TextNumberCodec), 64).unwrap();
        let mut restored = AutoAcceptance::new(Box::new(TextNumberCodec), 64).unwrap();
        let mut decisions = Vec::new();
        for index in 0..30 {
            let current = (10 - (index % 5)) as f64 * scale + offset;
            let value = (8 + (index % 7)) as f64 * scale + offset;
            decisions.push(policy.accept(&problem, &[point(&problem, current)], &point(&problem, value)));
            if index == 11 { restored.load(&policy.save().unwrap()).unwrap(); }
            if index > 11 { assert_eq!(restored.accept(&problem, &[point(&problem, current)], &point(&problem, value)), *decisions.last().unwrap()); }
        }
        assert_eq!(restored.save().unwrap(), policy.save().unwrap());
        if let Some(expected) = &reference { assert_eq!(&decisions, expected); } else { reference = Some(decisions); }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Business { cost: i64, label: String }

struct BusinessCodec;

impl SearchCodec<Business> for BusinessCodec {
    fn encode(&self, value: &Business) -> Result<Value> { Ok(serde_json::json!({ "cost": value.cost.to_string(), "label": value.label })) }
    fn decode(&self, value: &Value) -> Result<Business> {
        let cost = value["cost"].as_str().and_then(|value| value.parse().ok()).ok_or_else(|| invalid("bad business cost"))?;
        let label = value["label"].as_str().ok_or_else(|| invalid("bad business label"))?.into();
        Ok(Business { cost, label })
    }
}

struct BusinessProblem;

impl SearchProblem<Business, f64> for BusinessProblem {
    fn id(&self) -> &str { "business" }
    fn fingerprint(&self) -> &str { "business:1" }
    fn variable_order(&self) -> Vec<VariableId> { Vec::new() }
    fn initial_solutions(&self, _: usize) -> Vec<Solution<f64>> { Vec::new() }
    fn objective(&self, _: &Solution<f64>) -> Option<Business> { None }
    fn dominates(&self, lhs: &Business, rhs: &Business) -> bool { lhs.cost < rhs.cost }
}

#[test]
fn acceptance_history_uses_custom_objective_codec_without_serde_or_scalar_projection() {
    let current = Individual::new(Solution::new(), Business { cost: i64::MAX - 1, label: "original".into() });
    let candidate = Individual::new(Solution::new(), Business { cost: i64::MAX - 2, label: "candidate".into() });
    let mut policy = AutoAcceptance::new(Box::new(BusinessCodec), 64).unwrap();
    assert!(policy.accept(&BusinessProblem, &[current], &candidate));
    let saved = policy.save().unwrap();
    assert_eq!(saved["history"][0]["cost"], (i64::MAX - 2).to_string());
    assert_eq!(saved["history"][0]["label"], "candidate");
    let mut restored = AutoAcceptance::new(Box::new(BusinessCodec), 64).unwrap(); restored.load(&saved).unwrap();
    assert_eq!(restored.save().unwrap(), saved);
}

#[test]
fn bad_acceptance_state_is_rejected_atomically() {
    let mut late = LateAcceptance::new(Box::new(TextNumberCodec), 2).unwrap();
    let saved = late.save().unwrap(); let mut bad = saved.clone(); bad["cursor"] = 1.into();
    assert!(late.load(&bad).is_err()); assert_eq!(late.save().unwrap(), saved);
    let mut auto = AutoAcceptance::new(Box::new(TextNumberCodec), 64).unwrap();
    let saved = auto.save().unwrap(); let mut bad = saved.clone(); bad["decisions"] = 1.into();
    assert!(auto.load(&bad).is_err()); assert_eq!(auto.save().unwrap(), saved);
    let mut bad = saved.clone(); bad["improvements"] = 1.into(); assert!(auto.load(&bad).is_err());
    let mut record = RecordToRecordAcceptance::new(0.05, 2).unwrap();
    let saved = record.save().unwrap(); let mut bad = saved.clone(); bad["best"] = 2.0.into();
    assert!(record.load(&bad).is_err()); assert_eq!(record.save().unwrap(), saved);
}

#[test]
fn acceptance_factories_validate_definitions_and_require_original_objective_codecs() {
    for spec in [ComponentSpec::new("late").parameter("length", 0),
        ComponentSpec::new("record-to-record").parameter("initialRatio", -0.1),
        ComponentSpec::new("record-to-record").parameter("horizon", 0),
        ComponentSpec::new("auto").parameter("span", 2)] {
        assert!(builtin_acceptance::<f64, f64>(&spec, Some(Box::new(TextNumberCodec)), 64).is_err());
    }
    for kind in ["auto", "late"] { assert!(builtin_acceptance::<f64, f64>(&ComponentSpec::new(kind), None, 64).is_err()); }
    assert!(builtin_acceptance::<f64, f64>(&ComponentSpec::new("record-to-record"), None, 64).is_ok());
}

#[test]
fn rejecting_late_history_keeps_valid_unaccepted_archive_candidates() {
    let problem = problem();
    let mut memory = crate::solver::hyper_heuristic::memory::SearchMemory::new(10, 10);
    let mut policy = LateAcceptance::new(Box::new(TextNumberCodec), 1).unwrap();
    memory.offer(&problem, vec![point(&problem, 10.0)], &mut policy);
    memory.offer(&problem, vec![point(&problem, 8.0)], &mut policy);
    memory.offer(&problem, vec![point(&problem, 9.0)], &mut policy);
    assert_eq!(memory.working()[0].fitness, 8.0);
    assert!(memory.archive().iter().any(|value| value.fitness == 9.0));
    assert_eq!(memory.archive()[0].fitness, 8.0);
    assert_eq!(policy.save().unwrap()["history"], serde_json::json!(["8.0"]));
}
