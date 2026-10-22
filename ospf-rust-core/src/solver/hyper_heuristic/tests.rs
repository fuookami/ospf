//! 超启发式迁移回归 / Hyper-heuristic migration regressions

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::model::callback::Solution;
use crate::solver::heuristic::session::{SearchBudget, SearchProblem, SearchRandom, SearchSlice, SearchTermination};
use crate::solver::hyper_heuristic::persistence::{FileSearchStore, RunCheckpoint, SearchStore};
use crate::solver::hyper_heuristic::problem::RandomHandle;
use crate::solver::hyper_heuristic::registry::BUILTIN_ALGORITHMS;
use crate::variable::{VariableId, new_standalone_id};
use super::*;

struct Quadratic {
    x: VariableId,
    calls: Arc<AtomicU64>,
    offset: f64,
    fingerprint: String,
}

impl SearchProblem<f64, f64> for Quadratic {
    fn id(&self) -> &str { "quadratic" }
    fn fingerprint(&self) -> &str { &self.fingerprint }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(-10.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(10.0) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount).map(|index| Solution::from([(self.x, index as f64 + 2.0)])).collect()
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Some((solution[&self.x] - self.offset).powi(2))
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn feasible(&self, solution: &Solution<f64>) -> bool { solution.get(&self.x).is_some_and(|value| value.is_finite()) }
    fn valid_objective(&self, objective: &f64) -> bool { objective.is_finite() }
    fn stateless(&self) -> bool { true }
}

fn registry(calls: Arc<AtomicU64>) -> SearchRegistry<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("quadratic", move |spec, _| {
        spec.validate_parameters(&["offset"])?;
        let offset = spec.number("offset", 0.0)?;
        Ok(Box::new(Quadratic { x: new_standalone_id(), calls: calls.clone(), offset, fingerprint: format!("quadratic:1:{offset}") }))
    }).unwrap();
    registry
}

fn config(kinds: &[&str], evaluations: u64) -> HyperHeuristicConfig {
    let mut config = HyperHeuristicConfig::new(ComponentSpec::new("quadratic"), evaluations);
    config.execution.mode = ExecutionMode::Serial;
    for (index, kind) in kinds.iter().enumerate() {
        let mut algorithm = AlgorithmConfig::new(format!("algorithm-{index}"), *kind);
        algorithm.size = 4;
        algorithm.import_mode = ImportMode::Disabled;
        config.algorithms.push(algorithm);
    }
    config
}

#[test]
fn native_checkpoints_keep_committed_members_and_causal_operator_events() {
    let mut config = config(&["pso"], 32);
    config.budget.slice_evaluations = 4;
    config.persistence.enabled = true;
    config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant")]);
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    let native = &saved.workers["algorithm-0"].session.state["session"];
    assert_eq!(native["archive"].as_array().unwrap().len(), 8);
    assert_eq!(saved.evaluations, 8);
    assert_eq!(run.transitions().len(), 3);
    assert_eq!(run.transitions()[1].scope, "algorithm-0/inertia");
    assert_eq!(run.transitions()[2].scope, "algorithm");
    assert_eq!(run.transitions()[1].decision, run.transitions()[2].decision);
    assert!(!run.transitions()[1].terminal);
    saved.validate_kotlin_v1().unwrap();
    let mut explicit_null = saved.clone();
    explicit_null.workers.get_mut("algorithm-0").unwrap().session.state["rewardState"] = serde_json::Value::Null;
    assert!(explicit_null.validate_kotlin_v1().is_err());
    let mut predecessor = saved.clone();
    predecessor.transitions[0].previous_action = Some("algorithm-0".into());
    assert!(predecessor.validate_kotlin_v1().is_err());
    let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); restored.run().unwrap();
    assert_eq!(run.transitions(), restored.transitions());
}

#[test]
fn scalar_memory_keeps_distinct_equal_fitness_solutions_and_counts_one_improvement() {
    use crate::solver::heuristic::Individual;
    use super::memory::{NotWorseAcceptance, SearchMemory};
    let x = new_standalone_id();
    let problem = Quadratic { x, calls: Arc::new(AtomicU64::new(0)), offset: 0.0, fingerprint: "memory:1".into() };
    let mut memory = SearchMemory::new(100, 20);
    let candidates = vec![
        Individual::new(Solution::from([(x, -1.0)]), 1.0),
        Individual::new(Solution::from([(x, 1.0)]), 1.0),
        Individual::new(Solution::from([(x, 2.0)]), 4.0),
        Individual::new(Solution::from([(x, 3.0)]), 9.0),
    ];
    assert_eq!(memory.contribution(&problem, &[], &candidates), (true, 0.25));
    assert!(memory.offer(&problem, candidates.clone(), &mut NotWorseAcceptance));
    assert_eq!(memory.archive().len(), 4);
    assert!(!memory.offer(&problem, candidates, &mut NotWorseAcceptance));
    assert_eq!(memory.archive().len(), 4);
    assert_eq!(problem.calls.load(Ordering::Relaxed), 0);
}

#[test]
fn partial_generation_credits_operator_invocations_without_committing_native_steps() {
    let mut config = config(&["genetic"], 6);
    config.budget.slice_evaluations = 8;
    config.algorithms[0].updates = vec![ComponentSpec::new("sbx")];
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap(); run.advance().unwrap();
    let event = run.transitions().iter().find(|event| event.scope == "algorithm-0/update").unwrap();
    assert_eq!(event.steps, 0);
    assert!(event.terminal);
    assert_eq!(run.transitions().last().unwrap().scope, "algorithm");
    assert_eq!(run.transitions().last().unwrap().steps, 0);
    assert_eq!(run.result().steps, 1);
    assert_eq!(run.result().evaluations, 6);
}

#[test]
fn synchronous_feedback_masks_workers_awaiting_ordered_commit() {
    let mut config = config(&["pso", "pso"], 32);
    config.execution.mode = ExecutionMode::Synchronous;
    config.execution.parallelism = 2;
    config.budget.slice_evaluations = 8;
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap();
    assert_eq!(run.transitions().len(), 2);
    assert_eq!(run.transitions()[0].next_available, ["algorithm-0"]);
    assert_eq!(run.transitions()[1].next_available, ["algorithm-0", "algorithm-1"]);
}

#[test]
fn steps_only_budget_stops_when_slices_cannot_commit_initialization() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous, ExecutionMode::Asynchronous] {
        let mut config = config(&["pso", "genetic"], 100);
        config.budget.evaluations = None;
        config.budget.steps = Some(2);
        config.budget.slice_evaluations = 1;
        config.execution.mode = mode;
        config.execution.parallelism = 2;
        config.persistence.enabled = true;
        let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
        let mut run = solver.open().unwrap();
        assert!(!run.advance().unwrap());
        let saved = run.checkpoint().unwrap();
        assert_eq!(saved.steps, 0);
        assert_eq!(saved.dispatched, saved.completed);
        assert!(saved.completed <= 2);
        assert!(saved.workers.values().filter(|worker| worker.disabled).count() >= 1);
        assert!(run.result().diagnostics.iter().any(|message| message.contains("cannot commit")));
        assert!(saved.transitions.iter().filter(|event| event.scope == "algorithm").all(|event| event.terminal));
    }
}

#[test]
fn shade_numerical_contracts_are_explicit_and_part_of_checkpoint_identity() {
    let mut config = config(&["shade"], 40);
    config.persistence.enabled = true;
    config.algorithms[0].algorithm = ComponentSpec::new("shade").parameter("numerics", "shade-fdlibm-v1");
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let solver = HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    assert!(saved.validate_kotlin_v1().unwrap_err().to_string().contains("numerics"));
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); resumed.run().unwrap();
    assert_eq!(run.transitions(), resumed.transitions());
    config.algorithms[0].algorithm.parameters.insert("numerics".into(), "native".into());
    let native = HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap();
    assert!(native.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).is_err());
    for unknown in [serde_json::json!("shade-fdlibm-v2"), serde_json::json!(true), serde_json::Value::Null] {
        config.algorithms[0].algorithm.parameters.insert("numerics".into(), unknown);
        let invalid = HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap();
        assert!(invalid.open().is_err());
    }
}

#[test]
fn native_population_import_preserves_elitist_order_not_replacement_positions() {
    for kind in ["genetic", "shade"] {
        let registry = registry(Arc::new(AtomicU64::new(0)));
        let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
        let x = problem.variable_order()[0];
        let mut config = AlgorithmConfig::new("search", kind); config.size = 4;
        let mut session = registry.session(AlgorithmContext {
            config, problem, random: RandomHandle::new(SearchRandom::new(1)),
        }).unwrap();
        session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 8, 1)).unwrap();
        session.import_solutions(vec![Solution::from([(x, 0.5)]), Solution::from([(x, 1.5)])],
            &mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 8, 0)).unwrap();
        let saved = session.snapshot().unwrap();
        let state = &saved.state["session"]["state"];
        let population = if kind == "shade" { &state["individuals"] } else { state };
        let fitness: Vec<f64> = population.as_array().unwrap().iter()
            .map(|candidate| candidate["fitness"].as_str().unwrap().parse().unwrap()).collect();
        assert_eq!(fitness, [0.25, 2.25, 4.0, 9.0], "{kind}");
    }
}

#[test]
fn cma_restart_import_uses_all_candidates_and_preserves_covariance_for_a_single_seed() {
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
    let x = problem.variable_order()[0];
    let mut config = AlgorithmConfig::new("search", "cma-es");
    config.algorithm = config.algorithm.parameter("restartOnImport", true).parameter("initialSigma", 0.5)
        .parameter("numerics", "cma-polar-fdlibm-v1");
    let mut session = registry.session(AlgorithmContext { config, problem, random: RandomHandle::new(SearchRandom::new(1)) }).unwrap();
    let budget = Arc::new(SearchBudget::new(100));
    session.step(&mut SearchSlice::new(budget.clone(), 1, 1)).unwrap();
    session.import_solutions(vec![Solution::from([(x, 1.0)]), Solution::from([(x, 3.0)])], &mut SearchSlice::new(budget.clone(), 2, 0)).unwrap();
    let saved = session.snapshot().unwrap();
    let state = &saved.state["session"]["state"];
    assert_eq!(state["vectors"][0], serde_json::json!([2.0]));
    assert_eq!(state["vectors"][4], serde_json::json!([8.0 + 1e-10]));
    assert_eq!(state["counters"], serde_json::json!([0]));
    session.import_solutions(vec![Solution::from([(x, 4.0)])], &mut SearchSlice::new(budget, 1, 0)).unwrap();
    let single = session.snapshot().unwrap();
    let single = &single.state["session"]["state"];
    assert_eq!(single["vectors"][0], serde_json::json!([4.0]));
    assert_eq!(single["vectors"][4], state["vectors"][4]);
    assert_eq!(single["individuals"]["fitness"], state["individuals"]["fitness"]);
}

#[test]
fn operator_slots_credit_used_actions_and_restore_complete_state() {
    let mut config = config(&["genetic", "pso", "hill-climbing"], 150);
    config.persistence.enabled = true;
    config.selection = ComponentSpec::new("round-robin");
    config.budget.slice_steps = 2;
    config.algorithms[0].updates = vec![ComponentSpec::new("single-point"), ComponentSpec::new("uniform")];
    config.algorithms[0].mutations = vec![ComponentSpec::new("none"), ComponentSpec::new("gaussian")];
    config.algorithms[1].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant").parameter("value", 0.4),
        ComponentSpec::new("linear").parameter("from", 0.9).parameter("to", 0.4)]);
    config.algorithms[2].neighborhoods = vec![ComponentSpec::new("random"), ComponentSpec::new("gaussian")];
    for algorithm in &mut config.algorithms { algorithm.selection = ComponentSpec::new("q-learning"); }
    let solver = HyperHeuristicSolver::new(config.clone(), registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap();
    for _ in 0..6 { run.advance().unwrap(); }
    let saved = run.checkpoint().unwrap();
    assert_eq!(saved.policy.operators.len(), 3);
    assert!(saved.policy.operators.values().flat_map(|slots| slots.values()).all(|policy| policy.revision > 0));
    assert!(run.transitions().iter().any(|event| event.scope.ends_with("/update")));
    assert!(run.transitions().iter().any(|event| event.scope.ends_with("/inertia")));
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); resumed.run().unwrap();
    assert_eq!(run.export_policy().unwrap(), resumed.export_policy().unwrap());
    assert_eq!(run.transitions(), resumed.transitions());
    assert_eq!(run.result().archive.iter().map(|value| value.fitness).collect::<Vec<_>>(),
        resumed.result().archive.iter().map(|value| value.fitness).collect::<Vec<_>>());
    let policy = run.export_policy().unwrap();
    config.learning.mode = LearningMode::Frozen;
    let frozen_solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut frozen = frozen_solver.open_with(RunOptions { checkpoint: None, policy: Some(&policy) }).unwrap();
    frozen.run().unwrap();
    assert_eq!(policy, frozen.export_policy().unwrap());
}

#[test]
fn unused_initialization_slots_do_not_receive_feedback_and_invalid_slots_fail_preflight() {
    let mut config = config(&["genetic"], 4);
    config.algorithms[0].updates = vec![ComponentSpec::new("uniform")];
    let solver = HyperHeuristicSolver::new(config.clone(), registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap(); run.run().unwrap();
    assert!(run.transitions().iter().all(|event| event.scope == "algorithm"));
    assert_eq!(run.export_policy().unwrap().operators["algorithm-0"]["update"].revision, 0);
    config.algorithms[0].algorithm = ComponentSpec::new("pso");
    assert!(HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap().open().is_err());
}

#[test]
fn kotlin_component_catalog_accepts_parameters_and_rejects_invalid_values() {
    let mut config = config(&["genetic", "hill-climbing", "pso"], 120);
    config.algorithms[0].updates = ["single-point", "arithmetic", "order", "cycle"].map(ComponentSpec::new).to_vec();
    config.algorithms[0].updates.push(ComponentSpec::new("sbx").parameter("eta", 15.0));
    config.algorithms[0].mutations = ["none", "polynomial", "swap", "inversion"].map(ComponentSpec::new).to_vec();
    config.algorithms[0].mutations.push(ComponentSpec::new("gaussian").parameter("sigma", 0.05));
    config.algorithms[1].neighborhoods = vec![ComponentSpec::new("perturbation").parameter("amount", 2)];
    config.algorithms[2].parameters.insert("inertia".into(), vec![ComponentSpec::new("linear")]);
    config.algorithms[2].parameters.insert("personalWeight".into(), vec![ComponentSpec::new("geometric").parameter("initial", -1.0).parameter("ratio", 1.1)]);
    config.memory.acceptance = ComponentSpec::new("always");
    config.memory.exchange = ComponentSpec::new("periodic");
    // 交换测试要求显式启用原生导入 / Exchange requires explicit native import support
    for algorithm in &mut config.algorithms { algorithm.import_mode = ImportMode::Native; }
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let solver = HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap();
    let mut run = solver.open().unwrap(); run.run().unwrap();
    let policy = run.export_policy().unwrap();
    assert_eq!(policy.operators["algorithm-0"]["update"].actions.len(), 5);
    assert_eq!(policy.operators["algorithm-0"]["mutation"].actions.len(), 5);
    assert!(policy.operators["algorithm-2"].contains_key("inertia"));
    assert!(run.transitions().iter().any(|event| event.scope == "algorithm-1/neighborhood"));
    for invalid in [ComponentSpec::new("sbx").parameter("eta", 0), ComponentSpec::new("sbx").parameter("unknown", 1)] {
        config.algorithms[0].updates = vec![invalid];
        assert!(HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap().open().is_err());
    }
    config.algorithms[0].updates.clear();
    config.algorithms[0].mutations = vec![ComponentSpec::new("gaussian").parameter("sigma", -0.1)];
    assert!(HyperHeuristicSolver::new(config, registry).unwrap().open().is_err());
}

struct PermutationProblem { order: Vec<VariableId> }

impl SearchProblem<f64, f64> for PermutationProblem {
    fn id(&self) -> &str { "permutation" }
    fn fingerprint(&self) -> &str { "permutation:1" }
    fn variable_order(&self) -> Vec<VariableId> { self.order.clone() }
    fn encoding(&self) -> crate::solver::heuristic::session::SearchEncoding { crate::solver::heuristic::session::SearchEncoding::Permutation }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount).map(|shift| self.order.iter().enumerate().map(|(index, id)| (*id, ((index + shift) % 4) as f64)).collect()).collect()
    }
    fn feasible(&self, solution: &Solution<f64>) -> bool {
        let mut values: Vec<_> = self.order.iter().filter_map(|id| solution.get(id).copied()).collect();
        values.sort_by(f64::total_cmp); values == vec![0.0, 1.0, 2.0, 3.0]
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        Some(self.order.iter().enumerate().map(|(index, id)| (index as f64 + 1.0) * solution[id]).sum())
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn stateless(&self) -> bool { true }
}

#[test]
fn permutation_native_slots_preserve_encoding_and_reject_continuous_defaults() {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("permutation", |_, _| Ok(Box::new(PermutationProblem { order: (0..4).map(|_| new_standalone_id()).collect() }))).unwrap();
    let mut config = config(&["genetic"], 48); config.problem = ComponentSpec::new("permutation"); config.persistence.enabled = true;
    assert!(HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap().open().is_err());
    config.algorithms[0].updates = vec![ComponentSpec::new("order"), ComponentSpec::new("cycle")];
    config.algorithms[0].mutations = vec![ComponentSpec::new("swap"), ComponentSpec::new("inversion")];
    let solver = HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    let expected = run.run().unwrap(); let actual = restored.run().unwrap();
    assert_eq!(actual.evaluations, 48); assert_eq!(actual.best().unwrap().fitness, expected.best().unwrap().fitness);
    for individual in actual.archive {
        let mut values: Vec<_> = individual.solution.values().copied().collect(); values.sort_by(f64::total_cmp);
        assert_eq!(values, vec![0.0, 1.0, 2.0, 3.0]);
    }
    config.algorithms[0].updates = vec![ComponentSpec::new("sbx")];
    assert!(HyperHeuristicSolver::new(config, registry).unwrap().open().is_err());
}

struct ScheduledProblem { inner: Quadratic, slow: bool, fail: bool, entered: Option<std::sync::mpsc::Sender<()>> }

impl SearchProblem<f64, f64> for ScheduledProblem {
    fn id(&self) -> &str { "scheduled" }
    fn fingerprint(&self) -> &str { "scheduled:1" }
    fn variable_order(&self) -> Vec<VariableId> { self.inner.variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.inner.lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.inner.upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        assert!(!self.fail, "scheduled worker failure");
        self.inner.initial_solutions(amount)
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        if self.slow { if let Some(entered) = &self.entered { let _ = entered.send(()); } }
        if self.slow { std::thread::sleep(std::time::Duration::from_millis(3)); }
        self.inner.objective(solution)
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn stateless(&self) -> bool { true }
}

fn schedule_solver(reverse: bool, fail: bool) -> HyperHeuristicSolver<f64, f64> {
    schedule_solver_on_entry(reverse, fail, None, ExecutionMode::Asynchronous)
}

fn schedule_solver_on_entry(
    reverse: bool,
    fail: bool,
    entered: Option<std::sync::mpsc::Sender<()>>,
    mode: ExecutionMode,
) -> HyperHeuristicSolver<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    let factories = Arc::new(AtomicU64::new(0));
    registry.register_problem("scheduled", move |_, _| {
        let role = factories.fetch_add(1, Ordering::SeqCst) % 3;
        Ok(Box::new(ScheduledProblem { inner: Quadratic { x: new_standalone_id(), calls: Arc::new(AtomicU64::new(0)), offset: 0.0, fingerprint: "scheduled:1".into() },
            slow: role == if reverse { 2 } else { 1 }, fail: fail && role == 1, entered: entered.clone() }))
    }).unwrap();
    let mut config = config(&["pso", "genetic"], 80);
    config.problem = ComponentSpec::new("scheduled"); config.execution.mode = mode;
    config.execution.parallelism = 2; config.budget.slice_evaluations = 4; config.persistence.enabled = true;
    HyperHeuristicSolver::new(config, registry).unwrap()
}

fn deterministic_checkpoint(saved: &RunCheckpoint) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                object.remove("activeNanos"); object.remove("elapsedNanos");
                for value in object.values_mut() { strip(value); }
            }
            serde_json::Value::Array(values) => for value in values { strip(value); },
            _ => {}
        }
    }
    let mut value = serde_json::to_value(saved).unwrap(); strip(&mut value); value
}

#[test]
fn asynchronous_schedule_reexecutes_opposite_completion_order_and_resumes_quiescent_prefixes() {
    let solver = schedule_solver(false, false);
    let mut recorded = solver.open().unwrap(); recorded.record_schedule().unwrap();
    recorded.advance().unwrap(); recorded.advance().unwrap();
    let prefix = recorded.checkpoint().unwrap();
    assert_eq!(prefix.dispatched, prefix.completed);
    assert!(prefix.schedule.as_ref().unwrap().log.events.iter().any(|event| event.kind == "drain"));
    recorded.run().unwrap();
    let expected = recorded.checkpoint().unwrap();
    let log = recorded.export_schedule().unwrap(); log.validate().unwrap();
    assert_eq!(log.events.iter().find(|event| event.kind == "complete").unwrap().action, "algorithm-1");
    let opposite = schedule_solver(true, false);
    let mut replay = opposite.open().unwrap(); replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
    assert_eq!(replay.export_schedule().unwrap(), log);
    assert_eq!(deterministic_checkpoint(&replay.checkpoint().unwrap()), deterministic_checkpoint(&expected));
    let mut resumed = opposite.open_with(RunOptions { checkpoint: Some(&prefix), policy: None }).unwrap();
    assert!(resumed.advance().is_err());
    resumed.replay_schedule(&log).unwrap(); resumed.run().unwrap();
    assert_eq!(deterministic_checkpoint(&resumed.checkpoint().unwrap()), deterministic_checkpoint(&expected));
    let mut invalid = prefix.clone(); invalid.schedule.as_mut().unwrap().cursor += 1;
    assert!(opposite.open_with(RunOptions { checkpoint: Some(&invalid), policy: None }).is_err());
    for identity in [true, false] {
        let mut invalid = prefix.clone();
        let schedule = &mut invalid.schedule.as_mut().unwrap().log;
        if identity { schedule.problem.push_str("-other"); } else { schedule.events.iter_mut().find(|event| event.kind == "complete").unwrap().evaluations -= 1; }
        let mut value = serde_json::to_value(&schedule).unwrap(); value.as_object_mut().unwrap().remove("checksum");
        schedule.checksum = super::persistence::fingerprint(&value);
        assert!(opposite.open_with(RunOptions { checkpoint: Some(&invalid), policy: None }).is_err());
    }
    let mut changed = log.clone();
    changed.events.iter_mut().find(|event| event.kind == "dispatch").unwrap().random ^= 1;
    let mut value = serde_json::to_value(&changed).unwrap(); value.as_object_mut().unwrap().remove("checksum");
    changed.checksum = super::persistence::fingerprint(&value);
    let mut diverged = opposite.open().unwrap(); diverged.replay_schedule(&changed).unwrap();
    assert!(diverged.advance().is_err()); assert!(diverged.checkpoint().is_err());
    let mut corrupt = log.clone(); corrupt.events.pop();
    assert!(opposite.open().unwrap().replay_schedule(&corrupt).is_err());
    let mut value = serde_json::to_value(&corrupt).unwrap(); value.as_object_mut().unwrap().remove("checksum");
    corrupt.checksum = super::persistence::fingerprint(&value);
    assert!(corrupt.validate().is_err());
    let folder = std::env::temp_dir().join(format!("ospf-schedule-{}-{}", std::process::id(), new_standalone_id().group_id));
    let store = FileSearchStore::new(&folder).unwrap(); store.save_schedule("schedule.json", &log).unwrap();
    assert_eq!(store.load_schedule("schedule.json").unwrap(), log);
    assert!(store.save_schedule("schedule.json", &corrupt).is_err());
    assert_eq!(store.load_schedule("schedule.json").unwrap(), log);
    std::fs::remove_file(folder.join("schedule.json")).unwrap(); std::fs::remove_dir(folder).unwrap();
}

#[test]
fn asynchronous_schedule_replays_stop_and_failure_drains_without_duplicate_commits() {
    let solver = schedule_solver(false, false);
    let mut recorded = solver.open().unwrap(); recorded.record_schedule().unwrap(); recorded.advance().unwrap();
    recorded.request_stop(); let stopped = recorded.checkpoint().unwrap();
    let log = recorded.export_schedule().unwrap();
    let opposite = schedule_solver(true, false);
    let mut replay = opposite.open().unwrap(); replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
    assert_eq!(replay.result().termination, SearchTermination::Stopped);
    assert_eq!(deterministic_checkpoint(&replay.checkpoint().unwrap()), deterministic_checkpoint(&stopped));
    let mut resumed = opposite.open_with(RunOptions { checkpoint: Some(&stopped), policy: None }).unwrap();
    resumed.replay_schedule(&log).unwrap(); assert!(!resumed.advance().unwrap());
    let solver = schedule_solver(false, true);
    let mut failure = solver.open().unwrap(); failure.record_schedule().unwrap(); assert!(failure.run().is_err());
    let saved = failure.checkpoint().unwrap(); assert_eq!(saved.dispatched, saved.completed);
    let log = failure.export_schedule().unwrap();
    let mut replay = schedule_solver(true, true).open().unwrap(); replay.replay_schedule(&log).unwrap(); assert!(replay.run().is_err());
    assert_eq!(replay.export_schedule().unwrap(), log);
    assert_eq!(replay.checkpoint().unwrap().completed, saved.completed);
}

#[test]
fn asynchronous_schedule_rejects_wall_clock_and_non_threaded_modes() {
    for (mode, parallelism, time) in [(ExecutionMode::Serial, 1, None), (ExecutionMode::Synchronous, 2, None),
        (ExecutionMode::Asynchronous, 1, None), (ExecutionMode::Asynchronous, 2, Some(1000))] {
        let mut config = config(&["pso", "genetic"], 20);
        config.execution.mode = mode; config.execution.parallelism = parallelism; config.budget.time_millis = time;
        let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
        assert!(solver.open().unwrap().record_schedule().is_err());
    }
}

#[test]
fn asynchronous_schedule_replays_stop_observed_while_advance_is_waiting() {
    let (entered, receiver) = std::sync::mpsc::channel();
    let mut recorded = schedule_solver_on_entry(false, false, Some(entered), ExecutionMode::Asynchronous).open().unwrap();
    recorded.record_schedule().unwrap();
    let token = recorded.stop_token();
    let stopper = std::thread::spawn(move || {
        receiver.recv().unwrap();
        token.store(true, Ordering::Relaxed);
    });
    recorded.run().unwrap(); stopper.join().unwrap();
    let expected = recorded.checkpoint().unwrap();
    let log = recorded.export_schedule().unwrap();
    assert!(log.events.iter().any(|event| event.kind == "complete" && event.stopped));
    let mut replay = schedule_solver(true, false).open().unwrap();
    replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
    assert_eq!(deterministic_checkpoint(&replay.checkpoint().unwrap()), deterministic_checkpoint(&expected));
    assert_eq!(replay.export_schedule().unwrap(), log);
}

#[test]
fn synchronous_terminal_feedback_uses_committed_cost_not_racing_worker_counters() {
    let mut saved = Vec::new();
    for reverse in [false, true] {
        let mut run = schedule_solver_on_entry(reverse, false, None, ExecutionMode::Synchronous).open().unwrap();
        run.run().unwrap();
        let events: Vec<_> = run.transitions().iter().filter(|event| event.scope == "algorithm").collect();
        assert!(!events[..events.len() - 1].iter().any(|event| event.terminal));
        assert!(events.last().unwrap().terminal);
        saved.push(deterministic_checkpoint(&run.checkpoint().unwrap()));
    }
    assert_eq!(saved[0], saved[1]);
}

struct ThreadLocalProblem {
    x: VariableId,
    owner: std::thread::ThreadId,
    calls: std::rc::Rc<std::cell::Cell<u64>>,
    active: Arc<AtomicU64>,
    maximum: Arc<AtomicU64>,
    barrier: Arc<std::sync::Barrier>,
    slow: bool,
}

impl SearchProblem<f64, f64> for ThreadLocalProblem {
    fn id(&self) -> &str { "thread-local" }
    fn fingerprint(&self) -> &str { "thread-local:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(-10.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(10.0) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        assert_eq!(self.owner, std::thread::current().id());
        (0..amount).map(|index| Solution::from([(self.x, index as f64 + 1.0)])).collect()
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        assert_eq!(self.owner, std::thread::current().id());
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.maximum.fetch_max(active, Ordering::SeqCst);
        if self.calls.get() == 0 { self.barrier.wait(); }
        self.calls.set(self.calls.get() + 1);
        if self.slow { std::thread::sleep(std::time::Duration::from_millis(8)); }
        self.active.fetch_sub(1, Ordering::SeqCst);
        Some(solution[&self.x].powi(2))
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
}

#[test]
fn threaded_workers_own_non_send_models_and_merge_in_the_requested_order() {
    for mode in [ExecutionMode::Synchronous, ExecutionMode::Asynchronous] {
        let maximum = Arc::new(AtomicU64::new(0));
        let active = Arc::new(AtomicU64::new(0));
        let factories = Arc::new(AtomicU64::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let mut registry = SearchRegistry::with_builtins();
        let peak = maximum.clone();
        registry.register_problem("thread-local", move |_, _| {
            let role = factories.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(ThreadLocalProblem { x: new_standalone_id(), owner: std::thread::current().id(),
                calls: std::rc::Rc::new(std::cell::Cell::new(0)), active: active.clone(), maximum: peak.clone(),
                barrier: barrier.clone(), slow: role == 1 }))
        }).unwrap();
        let mut config = config(&["pso", "genetic"], 16);
        config.problem = ComponentSpec::new("thread-local");
        config.execution.mode = mode; config.execution.parallelism = 2;
        config.budget.slice_evaluations = 4;
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap();
        run.run().unwrap();
        assert_eq!(run.result().evaluations, 16);
        assert_eq!(run.result().objective_calls, 16);
        assert_eq!(run.result().decisions, 4);
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
        let first = &run.transitions()[0];
        if mode == ExecutionMode::Synchronous { assert_eq!(first.action, "algorithm-0"); assert_eq!(first.decision, 0); }
        else { assert_eq!(first.action, "algorithm-1"); assert_eq!(first.decision, 1); }
        let mut decisions: Vec<_> = run.transitions().iter().map(|event| event.decision).collect();
        decisions.sort(); assert_eq!(decisions, vec![0, 1, 2, 3]);
        run.close().unwrap(); run.close().unwrap();
    }
}

#[test]
fn parallel_checkpoints_drain_and_preserve_native_random_policy_and_budget() {
    let mut config = config(&["pso", "genetic", "hill-climbing"], 77);
    config.execution.mode = ExecutionMode::Synchronous; config.execution.parallelism = 3;
    config.persistence.enabled = true; config.budget.slice_evaluations = 8;
    config.selection = ComponentSpec::new("ucb1");
    config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant").parameter("value", 0.4)]);
    let solver = HyperHeuristicSolver::new(config.clone(), registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    assert_eq!(saved.dispatched, saved.completed);
    assert_eq!(saved.completed, 3);
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); resumed.run().unwrap();
    assert_eq!(run.result().evaluations, 77);
    assert_eq!(run.result().best().unwrap().fitness, resumed.result().best().unwrap().fitness);
    assert_eq!(run.transitions(), resumed.transitions());
    assert_eq!(run.export_policy().unwrap(), resumed.export_policy().unwrap());
    config.budget.steps = Some(5); config.budget.decisions = Some(5);
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let result = solver.solve().unwrap();
    assert!(result.steps <= 5); assert!(result.decisions <= 5); assert!(result.evaluations <= 77);
}

struct GatedProblem {
    x: VariableId,
    role: u64,
    calls: std::cell::Cell<u64>,
    gate: Arc<std::sync::Mutex<std::sync::mpsc::Receiver<()>>>,
    release: std::sync::mpsc::Sender<()>,
    refill: bool,
    observed: Arc<AtomicU64>,
}

impl SearchProblem<f64, f64> for GatedProblem {
    fn id(&self) -> &str { "gated" }
    fn fingerprint(&self) -> &str { "gated:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount).map(|index| Solution::from([(self.x, index as f64 + 1.0)])).collect()
    }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(-10.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(10.0) }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if self.role == 1 && call == 1 {
            // 超时只用来终止错误实现，不参与完成顺序判断 / Timeout only bounds a broken implementation, never determines ordering
            self.gate.lock().unwrap().recv_timeout(std::time::Duration::from_secs(10)).expect("slow worker must be released");
        }
        if self.refill && self.role == 2 && call == 5 {
            self.observed.fetch_add(1, Ordering::SeqCst);
            self.release.send(()).unwrap();
        }
        Some(solution[&self.x].powi(2))
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn save_state(&self) -> crate::solver::heuristic::result::Result<serde_json::Value> { Ok(self.calls.get().into()) }
    fn load_state(&mut self, state: &serde_json::Value) -> crate::solver::heuristic::result::Result<()> {
        self.calls.set(state.as_u64().ok_or_else(|| invalid("invalid gated count"))?); Ok(())
    }
}

fn gated_solver(refill: bool) -> (HyperHeuristicSolver<f64, f64>, std::sync::mpsc::Sender<()>, Arc<AtomicU64>) {
    let (release, gate) = std::sync::mpsc::channel();
    let gate = Arc::new(std::sync::Mutex::new(gate));
    let factories = Arc::new(AtomicU64::new(0));
    let observed = Arc::new(AtomicU64::new(0));
    let mut registry = SearchRegistry::with_builtins();
    let send = release.clone(); let count = observed.clone();
    registry.register_problem("gated", move |_, _| Ok(Box::new(GatedProblem {
        x: new_standalone_id(), role: factories.fetch_add(1, Ordering::SeqCst), calls: std::cell::Cell::new(0),
        gate: gate.clone(), release: send.clone(), refill, observed: count.clone(),
    }))).unwrap();
    let mut config = config(&["pso", "genetic"], 20);
    config.problem = ComponentSpec::new("gated");
    config.execution.mode = ExecutionMode::Asynchronous; config.execution.parallelism = 2;
    config.budget.slice_evaluations = 4; config.persistence.enabled = true;
    (HyperHeuristicSolver::new(config, registry).unwrap(), release, observed)
}

#[test]
fn asynchronous_scheduler_refills_before_the_slow_first_slice_completes() {
    let (solver, _, observed) = gated_solver(true);
    let mut run = solver.open().unwrap();
    assert!(run.advance().unwrap());
    assert_eq!(run.result().decisions, 1);
    assert_eq!(run.transitions()[0].action, "algorithm-1");
    assert_eq!(observed.load(Ordering::SeqCst), 0);
    run.run().unwrap();
    assert_eq!(observed.load(Ordering::SeqCst), 1);
    assert_eq!(run.result().evaluations, 20);
    let mut sequence: Vec<_> = run.transitions().iter().map(|event| event.decision).collect();
    sequence.sort_unstable(); assert_eq!(sequence, vec![0, 1, 2, 3, 4]);
    run.close().unwrap();
}

#[test]
fn asynchronous_pending_work_is_drained_by_checkpoint_policy_stop_and_close() {
    for operation in ["checkpoint", "policy", "stop", "close"] {
        let (solver, release, _) = gated_solver(false);
        let mut run = solver.open().unwrap();
        run.advance().unwrap();
        assert_eq!(run.result().decisions, 1);
        release.send(()).unwrap();
        match operation {
            "checkpoint" => {
                let saved = run.checkpoint().unwrap();
                assert_eq!(saved.dispatched, 2); assert_eq!(saved.completed, 2);
                assert_eq!(saved.evaluations, 8);
                let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
                assert_eq!(restored.run().unwrap().evaluations, 20);
                restored.close().unwrap();
            }
            "policy" => { run.export_policy().unwrap(); }
            "stop" => { run.request_stop(); assert!(!run.advance().unwrap()); }
            _ => { run.close().unwrap(); }
        }
        assert_eq!(run.result().decisions, 2, "{operation}");
        assert_eq!(run.result().evaluations, 8, "{operation}");
        assert_eq!(run.transitions().len(), 2, "{operation}");
        run.close().unwrap();
    }
}

#[test]
fn asynchronous_reservations_respect_decision_step_and_evaluation_limits() {
    for (evaluations, steps, decisions) in [(19, 100, 100), (100, 3, 100), (100, 100, 3)] {
        let mut config = config(&["pso", "genetic", "hill-climbing"], evaluations);
        config.execution.mode = ExecutionMode::Asynchronous; config.execution.parallelism = 3;
        config.budget.slice_evaluations = 8; config.budget.slice_steps = 2;
        config.budget.steps = Some(steps); config.budget.decisions = Some(decisions);
        config.persistence.enabled = true;
        let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
        let mut run = solver.open().unwrap(); run.run().unwrap();
        let saved = run.checkpoint().unwrap();
        assert_eq!(saved.dispatched, saved.completed);
        assert!(saved.steps <= steps); assert!(saved.completed <= decisions); assert!(saved.evaluations <= evaluations);
        assert_eq!(saved.transitions.iter().map(|event| event.evaluations).sum::<u64>(), saved.evaluations);
        let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        restored.close().unwrap(); run.close().unwrap();
    }
}

struct CountReward(u64);
impl RecoverableComponent for CountReward {
    fn save(&self) -> crate::solver::heuristic::result::Result<serde_json::Value> { Ok(self.0.into()) }
    fn load(&mut self, state: &serde_json::Value) -> crate::solver::heuristic::result::Result<()> {
        self.0 = state.as_u64().ok_or_else(|| invalid("invalid reward count"))?; Ok(())
    }
}
impl RewardPolicy for CountReward {
    fn reward(&mut self, _: &RewardContext) -> crate::solver::heuristic::result::Result<f64> { self.0 += 1; Ok(self.0 as f64) }
}

struct ConstantFeatures;
impl RecoverableComponent for ConstantFeatures { fn stateless(&self) -> bool { true } }
impl FeatureExtractor for ConstantFeatures {
    fn version(&self) -> &str { "constant:1" }
    fn extract(&self, progress: &SearchProgress) -> crate::solver::heuristic::result::Result<SelectionContext> {
        Ok(SelectionContext { decision: progress.decision, features: vec![1.0], ..SelectionContext::default() })
    }
}

struct SmallAllocator(bool);
impl RecoverableComponent for SmallAllocator { fn stateless(&self) -> bool { true } }
impl BudgetAllocator for SmallAllocator {
    fn allocate(&mut self, ceiling: SliceAllocation, _: &SelectionContext) -> crate::solver::heuristic::result::Result<SliceAllocation> {
        Ok(if self.0 { SliceAllocation { evaluations: ceiling.evaluations + 1, steps: 1 } }
            else { SliceAllocation { evaluations: ceiling.evaluations.min(4), steps: 1 } })
    }
}

struct NoExchange;
impl RecoverableComponent for NoExchange { fn stateless(&self) -> bool { true } }
impl ExchangePolicy for NoExchange {
    fn requires_import(&self) -> bool { false }
    fn amount(&mut self, _: u64) -> crate::solver::heuristic::result::Result<usize> { Ok(0) }
}

struct RejectWorking;
impl RecoverableComponent for RejectWorking { fn stateless(&self) -> bool { true } }
impl super::memory::AcceptancePolicy<f64, f64> for RejectWorking {
    fn accept(
        &mut self,
        _: &dyn SearchProblem<f64, f64>,
        _: &[crate::solver::heuristic::Individual<f64, f64>],
        _: &crate::solver::heuristic::Individual<f64, f64>,
    ) -> bool { false }
}

#[test]
fn independent_custom_components_restore_without_modifying_the_search_loop() {
    let mut registry = registry(Arc::new(AtomicU64::new(0)));
    registry.components.register_reward("count", |_| Ok(Box::new(CountReward(0)))).unwrap();
    registry.components.register_features("constant", |_| Ok(Box::new(ConstantFeatures))).unwrap();
    registry.components.register_allocator("small", |_| Ok(Box::new(SmallAllocator(false)))).unwrap();
    registry.components.register_exchange("no-exchange", |_| Ok(Box::new(NoExchange))).unwrap();
    registry.components.register_acceptance("reject", |_| Ok(Box::new(RejectWorking))).unwrap();
    registry.components.register_selector("custom-q", |_, actions, learning| {
        Ok(Box::new(TabularSelectionPolicy::new(&ComponentSpec::new("q-learning"), actions, learning)?))
    }).unwrap();
    let mut config = config(&["pso", "genetic"], 36);
    config.persistence.enabled = true;
    config.learning.reward = ComponentSpec::new("count"); config.learning.features = ComponentSpec::new("constant");
    config.budget.allocator = ComponentSpec::new("small"); config.memory.exchange = ComponentSpec::new("no-exchange");
    config.memory.acceptance = ComponentSpec::new("reject"); config.selection = ComponentSpec::new("custom-q");
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    assert_eq!(saved.components["reward"], serde_json::json!(2));
    assert_eq!(saved.working, serde_json::json!([]));
    assert!(!run.result().archive.is_empty());
    let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); restored.run().unwrap();
    assert_eq!(run.transitions(), restored.transitions());
    assert_eq!(run.export_policy().unwrap(), restored.export_policy().unwrap());
    assert_eq!(run.checkpoint().unwrap().components, restored.checkpoint().unwrap().components);
}

#[test]
fn kotlin_export_rejects_stateful_inner_rewards_without_losing_rust_state() {
    let mut registry = registry(Arc::new(AtomicU64::new(0)));
    registry.components.register_reward("count", |_| Ok(Box::new(CountReward(0)))).unwrap();
    let mut config = config(&["pso"], 24);
    config.persistence.enabled = true;
    config.learning.reward = ComponentSpec::new("count");
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    let error = saved.validate_kotlin_v1().unwrap_err().to_string();
    assert!(error.contains("workers/algorithm-0/rewardState"));

    let directory = std::env::temp_dir().join(format!("ospf-kotlin-state-{}-{}", std::process::id(), new_standalone_id()));
    let store = FileSearchStore::new(&directory).unwrap();
    store.save_checkpoint("state.json", &saved).unwrap();
    let bytes = std::fs::read(directory.join("state.json")).unwrap();
    assert!(store.save_kotlin_checkpoint("state.json", &saved).is_err());
    assert_eq!(std::fs::read(directory.join("state.json")).unwrap(), bytes);
    assert_eq!(store.load_checkpoint("state.json").unwrap(), saved);

    let mut nested = saved.clone();
    let worker = nested.workers.get_mut("algorithm-0").unwrap();
    worker.session.state = serde_json::json!({ "children": { "nested": worker.session.clone() } });
    assert!(nested.validate_kotlin_v1().unwrap_err().to_string().contains("children/nested/rewardState"));

    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); resumed.run().unwrap();
    assert_eq!(run.transitions(), resumed.transitions());
    assert_eq!(run.export_policy().unwrap(), resumed.export_policy().unwrap());
    assert_eq!(run.checkpoint().unwrap().components, resumed.checkpoint().unwrap().components);
    std::fs::remove_file(directory.join("state.json")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn inner_rewards_credit_only_used_slots_and_restore_shared_and_legacy_formats() {
    let mut registry = registry(Arc::new(AtomicU64::new(0)));
    registry.components.register_reward("count", |_| Ok(Box::new(CountReward(0)))).unwrap();
    let mut config = config(&["pso", "pso"], 80);
    config.persistence.enabled = true;
    config.learning.reward = ComponentSpec::new("count");
    for algorithm in &mut config.algorithms {
        for name in ["personalWeight", "inertia"] {
            algorithm.parameters.insert(name.into(), vec![ComponentSpec::new("constant")]);
        }
    }
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap(); run.advance().unwrap();
    let initialized = run.checkpoint().unwrap();
    for worker in initialized.workers.values() {
        assert_eq!(worker.session.state["rewardState"], 0);
        assert_eq!(worker.session.state["rewardStateVersion"], 1);
        assert!(worker.session.state.get("reward").is_none());
    }
    run.advance().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    assert_eq!(saved.components["reward"], 4);
    for worker in saved.workers.values() { assert_eq!(worker.session.state["rewardState"], 2); }
    let mut continuations = vec![saved.clone()];
    let mut legacy_rust = saved.clone();
    let mut legacy_kotlin = saved.clone();
    for worker in legacy_rust.workers.values_mut() {
        let state = worker.session.state.as_object_mut().unwrap();
        let reward = state.remove("rewardState").unwrap();
        state.insert("reward".into(), reward); state.remove("rewardStateVersion");
    }
    for worker in legacy_kotlin.workers.values_mut() { worker.session.state.as_object_mut().unwrap().remove("rewardStateVersion"); }
    continuations.extend([legacy_rust, legacy_kotlin]);
    run.run().unwrap();
    let final_state = run.checkpoint().unwrap();
    for saved in continuations {
        let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        restored.run().unwrap();
        assert_eq!(run.transitions(), restored.transitions());
        assert_eq!(run.export_policy().unwrap(), restored.export_policy().unwrap());
        let resumed = restored.checkpoint().unwrap();
        assert_eq!(final_state.components, resumed.components);
        for (id, worker) in &final_state.workers {
            let mut expected = worker.session.state.clone();
            let mut actual = resumed.workers[id].session.state.clone();
            expected["session"].as_object_mut().unwrap().remove("activeNanos");
            actual["session"].as_object_mut().unwrap().remove("activeNanos");
            assert_eq!(expected, actual);
        }
    }
    for invalid in ["version", "missing", "ambiguous"] {
        let mut corrupt = saved.clone();
        let state = corrupt.workers.get_mut("algorithm-0").unwrap().session.state.as_object_mut().unwrap();
        match invalid {
            "version" => { state.insert("rewardStateVersion".into(), 99.into()); }
            "missing" => { state.remove("rewardState"); }
            _ => { state.insert("reward".into(), 2.into()); }
        }
        assert!(solver.open_with(RunOptions { checkpoint: Some(&corrupt), policy: None }).is_err());
    }
}

#[test]
fn imported_transition_metadata_is_preserved_without_becoming_required() {
    let original = transition("a", 1.0);
    let mut json = serde_json::to_value(&original).unwrap();
    assert!(json.get("elapsedNanos").is_none());
    assert!(json.get("previousAction").is_none());
    assert_eq!(serde_json::from_value::<SelectionTransition>(json.clone()).unwrap(), original);
    json["elapsedNanos"] = 123456.into();
    json["previousAction"] = "b".into();
    let imported: SelectionTransition = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(imported.elapsed_nanos, 123456);
    assert_eq!(imported.previous_action.as_deref(), Some("b"));
    assert_eq!(serde_json::to_value(&imported).unwrap(), json);
}

struct FailingFeedbackSelector {
    delegate: TabularSelectionPolicy,
    panic: bool,
}

impl SelectionPolicy for FailingFeedbackSelector {
    fn revision(&self) -> u64 { self.delegate.revision() }
    fn select(
        &mut self,
        context: &SelectionContext,
        available: &[String],
        random: &mut SearchRandom,
    ) -> crate::solver::heuristic::result::Result<String> {
        self.delegate.select(context, available, random)
    }
    fn observe(&mut self, _: &SelectionTransition) -> crate::solver::heuristic::result::Result<()> {
        if self.panic { panic!("operator feedback panicked"); }
        Err(invalid("operator feedback rejected"))
    }
    fn snapshot(&self) -> crate::solver::heuristic::result::Result<PolicySnapshot> { self.delegate.snapshot() }
    fn restore(&mut self, snapshot: &PolicySnapshot) -> crate::solver::heuristic::result::Result<()> {
        self.delegate.restore(snapshot)
    }
}

#[test]
fn failing_slot_feedback_preserves_candidates_steps_and_remaining_slot_credit() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous, ExecutionMode::Asynchronous] {
        for panic in [false, true] {
            let mut registry = registry(Arc::new(AtomicU64::new(0)));
            registry.components.register_reward("count", |_| Ok(Box::new(CountReward(0)))).unwrap();
            registry.components.register_selector("fail-feedback", move |_, actions, learning| {
                Ok(Box::new(FailingFeedbackSelector {
                    delegate: TabularSelectionPolicy::new(&ComponentSpec::new("ucb1"), actions, learning)?, panic,
                }))
            }).unwrap();
            let mut config = config(&["pso"], 32);
            config.persistence.enabled = true;
            config.execution.mode = mode;
            config.budget.slice_evaluations = 4;
            config.learning.reward = ComponentSpec::new("count");
            config.algorithms[0].selection = ComponentSpec::new("fail-feedback");
            for name in ["inertia", "personalWeight"] {
                config.algorithms[0].parameters.insert(name.into(), vec![ComponentSpec::new("constant")]);
            }
            let solver = HyperHeuristicSolver::new(config, registry).unwrap();
            let mut run = solver.open().unwrap();
            run.advance().unwrap();
            assert!(run.advance().is_err());
            let saved = run.checkpoint().unwrap();
            assert_eq!(saved.steps, 2);
            assert_eq!(saved.evaluations, 8);
            assert_eq!(saved.completed, 2);
            assert_eq!(saved.components["reward"], 2);
            let state = &saved.workers["algorithm-0"].session.state;
            assert_eq!(state["rewardState"], 2);
            // 最优粒子不移动，局部/全局档案按解身份去重；原生档案保留全部成员。
            // The best particle stays put; local/global memory deduplicates identities, while native archives retain all members.
            assert_eq!(state["localArchive"].as_array().unwrap().len(), 7);
            assert_eq!(state["session"]["archive"].as_array().unwrap().len(), 8);
            assert_eq!(state["session"]["iteration"], 2);
            assert_eq!(saved.transitions.len(), 4);
            assert_eq!(saved.transitions[1].scope, "algorithm-0/inertia");
            assert_eq!(saved.transitions[2].scope, "algorithm-0/personalWeight");
            assert!(saved.transitions[3].failed);
            assert_eq!(saved.transitions[3].steps, 1);
            assert_eq!(saved.archive.as_array().unwrap().len(), 7);
        }
    }
}

struct InvalidReward;
impl RecoverableComponent for InvalidReward { fn stateless(&self) -> bool { true } }
impl RewardPolicy for InvalidReward {
    fn reward(&mut self, _: &RewardContext) -> crate::solver::heuristic::result::Result<f64> { Ok(f64::NAN) }
}

#[test]
fn invalid_extensions_fail_without_overspending_or_losing_executed_feedback() {
    let calls = Arc::new(AtomicU64::new(0));
    let mut registry = registry(calls.clone());
    registry.components.register_allocator("overspend", |_| Ok(Box::new(SmallAllocator(true)))).unwrap();
    registry.components.register_reward("invalid", |_| Ok(Box::new(InvalidReward))).unwrap();
    let mut config = config(&["pso"], 20);
    config.budget.allocator = ComponentSpec::new("overspend");
    let solver = HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap();
    let mut run = solver.open().unwrap();
    assert!(run.advance().is_err()); assert_eq!(calls.load(Ordering::Relaxed), 0);
    config.budget.allocator = ComponentSpec::new("fixed"); config.learning.reward = ComponentSpec::new("invalid");
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap();
    assert!(run.advance().is_err());
    assert_eq!(run.transitions().len(), 1); assert!(run.transitions()[0].failed);
    assert_eq!(run.transitions()[0].reward, 0.0); assert!(!run.result().archive.is_empty());
}

struct CountingSource(u64);
impl RecoverableComponent for CountingSource {
    fn save(&self) -> crate::solver::heuristic::result::Result<serde_json::Value> { Ok(self.0.into()) }
    fn load(&mut self, state: &serde_json::Value) -> crate::solver::heuristic::result::Result<()> {
        self.0 = state.as_u64().ok_or_else(|| invalid("invalid source count"))?; Ok(())
    }
}
impl crate::solver::heuristic::operator::evolution::ParameterSource for CountingSource {
    fn value(&mut self, _: &crate::solver::heuristic::Iteration, _: &mut dyn crate::solver::heuristic::entropy::Entropy) -> f64 {
        self.0 += 1; 0.4 + (self.0 % 3) as f64 * 0.05
    }
}

#[test]
fn custom_stateful_operator_components_restore_in_serial_and_threaded_runs() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let mut registry = registry(Arc::new(AtomicU64::new(0)));
        registry.operators.register_parameter("count", |_| Ok(Box::new(CountingSource(0)))).unwrap();
        let mut config = config(&["pso", "genetic"], 80);
        config.execution.mode = mode; config.execution.parallelism = 2;
        config.persistence.enabled = true; config.budget.slice_evaluations = 8;
        config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("count")]);
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap();
        assert!(saved.workers["algorithm-0"].session.state["slots"]["inertia"]["components"].as_object().unwrap().values()
            .any(|value| value.as_u64().is_some_and(|count| count > 0)));
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        run.run().unwrap(); resumed.run().unwrap();
        assert_eq!(run.result().best().unwrap().fitness, resumed.result().best().unwrap().fitness);
        assert_eq!(run.transitions(), resumed.transitions());
        assert_eq!(run.export_policy().unwrap(), resumed.export_policy().unwrap());
    }
}

struct FallibleEvaluation { x: VariableId, calls: std::cell::Cell<u64> }
impl SearchProblem<f64, f64> for FallibleEvaluation {
    fn id(&self) -> &str { "fallible" }
    fn fingerprint(&self) -> &str { "fallible:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { vec![Solution::from([(self.x, 1.0)]); amount] }
    fn objective(&self, _: &Solution<f64>) -> Option<f64> { Some(1.0) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn evaluate(&self, solution: Solution<f64>, slice: &SearchSlice) -> crate::solver::heuristic::result::Result<crate::solver::heuristic::session::SearchEvaluationResult<f64, f64>> {
        if !slice.acquire_shared() { return Ok(crate::solver::heuristic::session::SearchEvaluationResult { admitted: false, candidate: None }); }
        if self.calls.get() > 0 { return Err(invalid("evaluation callback rejected request")); }
        self.calls.set(1); slice.global().objective_called();
        Ok(crate::solver::heuristic::session::SearchEvaluationResult { admitted: true,
            candidate: Some(crate::solver::heuristic::Individual::new(solution, 1.0)) })
    }
}

#[test]
fn metered_evaluation_errors_propagate_and_keep_completed_candidates() {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("fallible", |_, _| Ok(Box::new(FallibleEvaluation { x: new_standalone_id(), calls: std::cell::Cell::new(0) }))).unwrap();
    let mut config = config(&["pso"], 20); config.problem = ComponentSpec::new("fallible");
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap();
    assert!(run.advance().is_err());
    assert_eq!(run.result().evaluations, 2); assert_eq!(run.result().objective_calls, 1);
    assert_eq!(run.result().archive.len(), 1); assert_eq!(run.transitions().len(), 1);
    assert!(run.result().diagnostics.iter().any(|message| message.contains("evaluation callback rejected request")));
}

fn transition(action: &str, reward: f64) -> SelectionTransition {
    SelectionTransition { episode: "test".into(), decision: 0, action: action.into(),
        before: SelectionContext::default(), after: SelectionContext { state: 1, ..SelectionContext::default() },
        available: vec!["a".into(), "b".into()], next_available: vec!["a".into(), "b".into()], reward,
        evaluations: 1, terminal: false, policy_version: 0, feature_version: "progress:1".into(),
        scope: "algorithm".into(), steps: 1, failed: false, merged_improvement: false, elapsed_nanos: 0, previous_action: None }
}

#[test]
fn q_learning_credits_executed_action_and_masked_successor() {
    let mut policy = TabularSelectionPolicy::new(&ComponentSpec::new("q-learning").parameter("alpha", 0.5).parameter("gamma", 0.9),
        vec!["a".into(), "b".into()], true).unwrap();
    let mut saved = policy.snapshot().unwrap();
    saved.table[1] = vec![2.0, 10.0];
    policy.restore(&saved).unwrap();
    let mut event = transition("a", 1.0);
    event.next_available = vec!["a".into()];
    policy.observe(&event).unwrap();
    assert!((policy.snapshot().unwrap().table[0][0] - 1.4).abs() < 1e-12);
    assert_eq!(policy.snapshot().unwrap().table[0][1], 0.0);
    event.terminal = true;
    policy.observe(&event).unwrap();
    assert!((policy.snapshot().unwrap().table[0][0] - 1.2).abs() < 1e-12);
}

#[test]
fn frozen_learning_and_action_id_restore_are_stable() {
    let spec = ComponentSpec::new("epsilon-greedy").parameter("epsilon", 1.0);
    let mut training = TabularSelectionPolicy::new(&spec, vec!["a".into(), "b".into()], true).unwrap();
    training.observe(&transition("b", 10.0)).unwrap();
    let saved = training.snapshot().unwrap();
    let mut frozen = TabularSelectionPolicy::new(&spec, vec!["b".into(), "a".into()], false).unwrap();
    frozen.restore(&saved).unwrap();
    let before = frozen.snapshot().unwrap();
    for seed in 0..20 {
        assert_eq!(frozen.select(&SelectionContext::default(), &["a".into(), "b".into()], &mut SearchRandom::new(seed)).unwrap(), "b");
        frozen.observe(&transition("a", 100.0)).unwrap();
    }
    assert_eq!(frozen.snapshot().unwrap(), before);
}

#[test]
fn ucb_explores_unobserved_actions_then_uses_observed_rewards() {
    let mut policy = TabularSelectionPolicy::new(&ComponentSpec::new("ucb1"), vec!["a".into(), "b".into()], true).unwrap();
    let mut random = SearchRandom::new(1);
    let actions = vec!["a".into(), "b".into()];
    assert_eq!(policy.select(&SelectionContext::default(), &actions, &mut random).unwrap(), "a");
    policy.observe(&transition("a", 0.0)).unwrap();
    assert_eq!(policy.select(&SelectionContext::default(), &actions, &mut random).unwrap(), "b");
    policy.observe(&transition("b", 2.0)).unwrap();
    assert_eq!(policy.select(&SelectionContext::default(), &actions, &mut random).unwrap(), "b");
}

#[test]
fn invalid_masks_and_snapshots_do_not_mutate_policy() {
    let mut policy = TabularSelectionPolicy::new(&ComponentSpec::new("q-learning"), vec!["a".into(), "b".into()], true).unwrap();
    let before = policy.snapshot().unwrap();
    let mut invalid = before.clone();
    invalid.table[0][0] = f64::NAN;
    assert!(policy.restore(&invalid).is_err());
    assert!(policy.select(&SelectionContext::default(), &["unknown".into()], &mut SearchRandom::new(1)).is_err());
    assert!(policy.select(&SelectionContext::default(), &["a".into(), "a".into()], &mut SearchRandom::new(1)).is_err());
    assert_eq!(policy.snapshot().unwrap(), before);
}

#[test]
fn json_yaml_and_builder_share_the_same_configuration() {
    let config = config(&["pso", "genetic"], 100);
    let json = ConfigurationFormats::encode(&config).unwrap();
    let yaml = ConfigurationFormats::encode_format(&config, ConfigurationFormat::Yaml).unwrap();
    assert_eq!(ConfigurationFormats::decode(&json).unwrap(), config);
    assert_eq!(ConfigurationFormats::decode_format(&yaml, ConfigurationFormat::Yaml).unwrap(), config);
    assert!(ConfigurationFormats::decode(&json.replace("\"evaluations\": 100", "\"evaluations\": -1")).is_err());
    assert!(ConfigurationFormats::decode(&json.replace("\"version\": 1", "\"version\": 2")).is_err());
    assert!(ConfigurationFormats::decode(&json.replace("\"parallelism\": 1", "\"parallelism\": 1, \"unknown\": 2")).is_err());
    let mut no_limit = config.clone();
    no_limit.budget = BudgetConfig::default();
    assert!(no_limit.validate().is_err());
}

#[test]
fn all_native_algorithms_charge_exact_request_budgets() {
    for kind in BUILTIN_ALGORITHMS {
        let calls = Arc::new(AtomicU64::new(0));
        let solver = HyperHeuristicSolver::new(config(&[kind], 37), registry(calls.clone())).unwrap();
        let result = solver.solve().unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert_eq!(result.evaluations, 37, "{kind}");
        assert_eq!(result.objective_calls, calls.load(Ordering::Relaxed), "{kind}");
        assert_eq!(result.evaluations, result.objective_calls, "{kind}");
        assert!(!result.archive.is_empty(), "{kind}");
        assert_eq!(result.termination, SearchTermination::EvaluationLimit, "{kind}");
    }
}

#[test]
fn zero_budget_and_zero_steps_never_call_objectives() {
    for step_limit in [None, Some(0)] {
        let calls = Arc::new(AtomicU64::new(0));
        let mut config = config(&["pso"], if step_limit.is_some() { 100 } else { 0 });
        config.budget.steps = step_limit;
        let result = HyperHeuristicSolver::new(config, registry(calls.clone())).unwrap().solve().unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(result.evaluations, 0);
        assert!(result.archive.is_empty());
    }
}

#[test]
fn repeated_runs_are_independent_and_deterministic() {
    let solver = HyperHeuristicSolver::new(config(&["pso", "genetic"], 200), registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let first = solver.solve().unwrap();
    let second = solver.solve().unwrap();
    assert_eq!(first.best().unwrap().fitness, second.best().unwrap().fitness);
    assert_eq!(first.evaluations, second.evaluations);
    assert_eq!(first.steps, second.steps);
    assert_ne!(first.best().unwrap().solution.keys().next(), second.best().unwrap().solution.keys().next());
}

#[test]
fn native_snapshots_resume_actual_random_streams() {
    fn without_timing(mut value: serde_json::Value) -> serde_json::Value {
        match &mut value {
            serde_json::Value::Object(fields) => {
                fields.remove("activeNanos");
                for value in fields.values_mut() { *value = without_timing(std::mem::take(value)); }
            }
            serde_json::Value::Array(values) => {
                for value in values { *value = without_timing(std::mem::take(value)); }
            }
            _ => {}
        }
        value
    }
    for kind in ["pso", "apso", "clpso", "shade", "l-shade", "cma-es", "hill-climbing", "genetic"] {
        let registry = registry(Arc::new(AtomicU64::new(0)));
        let algorithm = config(&[kind], 1000).algorithms.remove(0);
        let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
        let random = RandomHandle::new(SearchRandom::new(42));
        let mut session = registry.session(AlgorithmContext { config: algorithm.clone(), problem, random: random.clone() }).unwrap();
        session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(1000)), 500, 3)).unwrap();
        let saved = session.snapshot().unwrap();
        let random_saved = random.state();
        let expected = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(1000)), 500, 3)).unwrap();
        let expected_state = session.snapshot().unwrap().state;
        let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
        let random = RandomHandle::new(SearchRandom::new(99));
        let mut restored = registry.session(AlgorithmContext { config: algorithm, problem, random: random.clone() }).unwrap();
        restored.restore(&saved).unwrap();
        random.restore(random_saved);
        let actual = restored.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(1000)), 500, 3)).unwrap();
        assert_eq!(actual.evaluations, expected.evaluations, "{kind}");
        // 比较所有原生与受控状态，但不比较真实墙钟耗时 / Compare all native and controlled state, excluding real wall-clock timing
        assert_eq!(without_timing(restored.snapshot().unwrap().state), without_timing(expected_state), "{kind}");
    }
}

#[test]
fn partial_generations_keep_native_state_and_return_completed_candidates() {
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
    let mut session = registry.session(AlgorithmContext { config: config(&["pso"], 100).algorithms.remove(0), problem,
        random: RandomHandle::new(SearchRandom::new(42)) }).unwrap();
    session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 4, 1)).unwrap();
    let before = session.snapshot().unwrap();
    let report = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 2, 1)).unwrap();
    assert_eq!(report.evaluations, 2);
    assert_eq!(report.steps, 0);
    assert_eq!(report.candidates.len(), 2);
    let after = session.snapshot().unwrap();
    assert_eq!(before.state["session"]["state"], after.state["session"]["state"]);
    assert_eq!(before.iteration, after.iteration);
}

#[test]
fn invalid_dimensions_consume_requests_without_calling_objectives() {
    let calls = Arc::new(AtomicU64::new(0));
    let problem = Quadratic { x: new_standalone_id(), calls: calls.clone(), offset: 0.0, fingerprint: "quadratic:1".into() };
    let global = Arc::new(SearchBudget::new(3));
    let slice = SearchSlice::new(global.clone(), 3, 1);
    let invalid = problem.evaluate(Solution::from([(new_standalone_id(), 1.0)]), &slice).unwrap();
    assert!(invalid.admitted && invalid.candidate.is_none());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(global.used(), 1);
    assert_eq!(global.objective_calls(), 0);
}

#[test]
fn stop_and_close_are_idempotent_without_extra_evaluations() {
    let calls = Arc::new(AtomicU64::new(0));
    let solver = HyperHeuristicSolver::new(config(&["pso"], 100), registry(calls.clone())).unwrap();
    let mut run = solver.open().unwrap();
    run.advance().unwrap();
    let used = calls.load(Ordering::Relaxed);
    run.request_stop();
    assert!(!run.advance().unwrap());
    assert_eq!(run.result().termination, SearchTermination::Stopped);
    run.close().unwrap(); run.close().unwrap();
    assert!(run.advance().is_err());
    assert_eq!(calls.load(Ordering::Relaxed), used);
}

#[test]
fn unsupported_definitions_fail_before_evaluation() {
    let calls = Arc::new(AtomicU64::new(0));
    for variant in 0..4 {
        let mut config = config(&["pso"], 100);
        match variant {
            0 => config.algorithms[0].neighborhoods.push(ComponentSpec::new("random")),
            1 => { config.algorithms[0].algorithm.parameters.insert("unknown".into(), 1.into()); }
            2 => config.selection = ComponentSpec::new("unknown"),
            _ => config.execution.parallelism = 0,
        }
        assert!(HyperHeuristicSolver::new(config, registry(calls.clone())).and_then(|solver| solver.open()).is_err());
    }
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn complete_checkpoints_preserve_budget_policy_memory_and_random_streams() {
    for kind in ["pso", "clpso", "shade", "cma-es", "genetic"] {
        let mut config = config(&[kind], 100);
        config.persistence.enabled = true;
        config.budget.slice_evaluations = 20;
        config.selection = ComponentSpec::new("q-learning");
        let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
        let mut run = solver.open().unwrap();
        run.advance().unwrap();
        let checkpoint = run.checkpoint().unwrap();
        let expected = run.run().unwrap();
        let expected_policy = run.export_policy().unwrap();
        let mut restored = solver.open_with(RunOptions { checkpoint: Some(&checkpoint), policy: None }).unwrap();
        let actual = restored.run().unwrap();
        assert_eq!(expected.evaluations, actual.evaluations, "{kind}");
        assert_eq!(expected.steps, actual.steps, "{kind}");
        assert_eq!(expected.best().unwrap().fitness, actual.best().unwrap().fitness, "{kind}");
        assert_eq!(run.transitions(), restored.transitions(), "{kind}");
        assert_eq!(expected_policy, restored.export_policy().unwrap(), "{kind}");
    }
}

#[test]
fn policy_transfer_freezes_updates_and_checkpoint_inputs_are_exclusive() {
    let mut training_config = config(&["pso", "genetic"], 100);
    training_config.selection = ComponentSpec::new("ucb1");
    training_config.persistence.enabled = true;
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let solver = HyperHeuristicSolver::new(training_config.clone(), registry.clone()).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap();
    let policy = run.export_policy().unwrap(); let checkpoint = run.checkpoint().unwrap();
    assert!(solver.open_with(RunOptions { policy: Some(&policy), checkpoint: Some(&checkpoint) }).is_err());
    training_config.learning.mode = LearningMode::Frozen;
    let frozen = HyperHeuristicSolver::new(training_config, registry).unwrap();
    let mut run = frozen.open_with(RunOptions { policy: Some(&policy), checkpoint: None }).unwrap();
    run.run().unwrap();
    assert_eq!(run.export_policy().unwrap(), policy);
}

#[test]
fn file_storage_replaces_atomically_and_rejects_corruption_and_unsafe_keys() {
    let root = std::env::temp_dir().join(format!("ospf-hyper-store-{}-{}", std::process::id(), new_standalone_id().group_id));
    let store = FileSearchStore::new(&root).unwrap();
    let mut config = config(&["pso"], 100); config.persistence.enabled = true;
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap();
    let checkpoint = run.checkpoint().unwrap();
    store.save_checkpoint("checkpoint.json", &checkpoint).unwrap();
    store.save_checkpoint("checkpoint.json", &checkpoint).unwrap();
    assert_eq!(store.load_checkpoint("checkpoint.json").unwrap(), checkpoint);
    let policy = run.export_policy().unwrap();
    store.save_policy("policy.json", &policy).unwrap();
    assert_eq!(store.load_policy("policy.json").unwrap(), policy);
    for key in ["../escape", "..", "C:escape", "a/b", "a\\b", "CON", "nul.json"] {
        assert!(store.save_policy(key, &policy).is_err(), "{key}");
    }
    std::fs::write(root.join("policy.json"), "{\"version\":1,\"kind\":\"policy\",\"checksum\":\"broken\",\"payload\":{}}").unwrap();
    assert!(store.load_policy("policy.json").is_err());
    // 仅删除本测试精确创建的文件，不递归删除目录 / Remove only exact test-owned files, never recursively delete directories
    std::fs::remove_file(root.join("policy.json")).unwrap();
    std::fs::remove_file(root.join("checkpoint.json")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn checkpoint_mismatches_and_counter_corruption_are_rejected() {
    let mut config = config(&["pso"], 100); config.persistence.enabled = true;
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap();
    let checkpoint = run.checkpoint().unwrap();
    for variant in 0..4 {
        let mut broken = checkpoint.clone();
        match variant {
            0 => broken.problem = "different".into(),
            1 => broken.evaluations = 101,
            2 => broken.version = 99,
            _ => broken.workers.clear(),
        }
        assert!(solver.open_with(RunOptions { checkpoint: Some(&broken), policy: None }).is_err());
    }
}

#[test]
fn island_and_memetic_children_remain_persistent_and_recoverable() {
    for kind in ["island", "memetic"] {
        let mut config = config(&[kind], 180);
        config.algorithms[0].import_mode = ImportMode::Native;
        config.algorithms[0].children = ["pso", "genetic"].iter().enumerate().map(|(index, kind)| {
            let mut child = AlgorithmConfig::new(format!("child-{index}"), *kind); child.size = 4; child
        }).collect();
        config.budget.slice_evaluations = 24;
        config.persistence.enabled = true;
        let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap();
        let expected = run.run().unwrap();
        let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        let actual = restored.run().unwrap();
        assert_eq!(actual.evaluations, 180);
        assert_eq!(actual.evaluations, expected.evaluations);
        assert_eq!(actual.best().unwrap().fitness, expected.best().unwrap().fitness);
        assert_eq!(actual.steps, expected.steps);
        assert_eq!(restored.transitions(), run.transitions());
    }
}

#[test]
fn memetic_phases_advance_once_and_import_only_into_the_current_child() {
    let mut config = config(&["memetic"], 96);
    config.persistence.enabled = true;
    config.budget.slice_evaluations = 24;
    let composite = &mut config.algorithms[0];
    composite.import_mode = ImportMode::Native;
    composite.algorithm = ComponentSpec::new("memetic").parameter("phaseSteps", 1);
    composite.children = ["global", "local"].into_iter().map(|id| {
        let mut child = AlgorithmConfig::new(id, "pso"); child.size = 4; child
    }).collect();
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap();
    for _ in 0..4 { run.advance().unwrap(); }
    let saved = run.checkpoint().unwrap();
    let state = &saved.workers["algorithm-0"].session.state;
    assert_eq!(state["cursor"], 1);
    assert_eq!(state["children"]["global"]["iteration"], 1);
    assert_eq!(state["children"]["local"]["iteration"], 3);
    let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); restored.run().unwrap();
    assert_eq!(run.transitions(), restored.transitions());
    let final_state = run.checkpoint().unwrap().workers["algorithm-0"].session.state.clone();
    assert_eq!(final_state["cursor"], 1);
    assert_eq!(final_state["children"]["global"], state["children"]["global"]);
}

struct VectorProblem { x: VariableId }

impl SearchProblem<Vec<f64>, f64> for VectorProblem {
    fn id(&self) -> &str { "vector" }
    fn fingerprint(&self) -> &str { "vector:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(0.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(2.0) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount).map(|index| Solution::from([(self.x, index as f64 / amount.max(1) as f64 * 2.0)])).collect()
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> { let value = solution[&self.x]; Some(vec![value.powi(2), (value - 2.0).powi(2)]) }
    fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool {
        lhs.iter().zip(rhs).all(|(lhs, rhs)| lhs <= rhs) && lhs.iter().zip(rhs).any(|(lhs, rhs)| lhs < rhs)
    }
    fn equivalent(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { lhs == rhs }
    fn valid_objective(&self, objective: &Vec<f64>) -> bool { objective.len() == 2 && objective.iter().all(|value| value.is_finite()) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { solution.get(&self.x).is_some_and(|value| value.is_finite()) }
    fn multi_objective(&self) -> bool { true }
    fn stateless(&self) -> bool { true }
}

#[test]
fn local_vector_memory_retains_arrival_order_while_global_memory_preserves_extremes() {
    use crate::solver::heuristic::Individual;
    use super::memory::{NotWorseAcceptance, SearchMemory};
    let problem = VectorProblem { x: new_standalone_id() };
    let candidates: Vec<_> = (0..=100).map(|index| {
        let solution = Solution::from([(problem.x, index as f64 / 50.0)]);
        let fitness = problem.objective(&solution).unwrap();
        Individual::new(solution, fitness)
    }).collect();
    let mut local = SearchMemory::local(100, 20);
    let mut global = SearchMemory::new(100, 20);
    local.offer(&problem, candidates.clone(), &mut NotWorseAcceptance);
    global.offer(&problem, candidates.clone(), &mut NotWorseAcceptance);
    let values = |entries: &[Individual<Vec<f64>, f64>]| entries.iter()
        .map(|value| (value.solution.clone(), value.fitness.clone())).collect::<Vec<_>>();
    assert_eq!(values(local.archive()), values(&candidates[..100]));
    assert_eq!(global.archive().len(), 100);
    assert_eq!(global.archive().first().unwrap().solution, candidates.first().unwrap().solution);
    assert_eq!(global.archive().last().unwrap().solution, candidates.last().unwrap().solution);
    let mut restored = SearchMemory::local(100, 20);
    restored.restore(&problem, local.archive().to_vec(), local.working().to_vec()).unwrap();
    for memory in [&mut local, &mut restored] {
        memory.offer(&problem, vec![candidates[100].clone()], &mut NotWorseAcceptance);
        assert_eq!(values(memory.archive()), values(&candidates[..100]));
    }
    assert_eq!(values(local.working()), values(restored.working()));
}

#[test]
fn nsga_keeps_original_objective_vectors_and_has_no_unique_best() {
    for kind in ["nsga-ii", "nsga-iii"] {
        let mut registry = SearchRegistry::with_multi_objective_builtins();
        registry.register_problem("vector", |_, _| Ok(Box::new(VectorProblem { x: new_standalone_id() }))).unwrap();
        let mut config = config(&[kind], 80); config.problem = ComponentSpec::new("vector"); config.persistence.enabled = true;
        config.budget.slice_evaluations = 20;
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap();
        let checkpoint = run.checkpoint().unwrap();
        let expected = run.run().unwrap();
        let mut restored = solver.open_with(RunOptions { checkpoint: Some(&checkpoint), policy: None }).unwrap();
        let result = restored.run().unwrap();
        assert_eq!(result.evaluations, 80);
        assert!(result.best().is_none());
        assert!(result.archive.iter().all(|value| value.fitness.len() == 2));
        assert_eq!(result.archive.iter().map(|value| &value.fitness).collect::<Vec<_>>(), expected.archive.iter().map(|value| &value.fitness).collect::<Vec<_>>());
    }
}

#[test]
fn instance_training_freezes_holdouts_and_rejects_leaking_fingerprints() {
    use super::learning::*;
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let trainer = PolicyTrainer::new(registry.clone());
    let case = |id: &str, offset: f64| TrainingCase { id: id.into(), problem: ComponentSpec::new("quadratic").parameter("offset", offset) };
    let split = TrainingSplit { training: vec![case("train", 0.0)], validation: vec![case("validation", 1.0)], test: vec![case("test", 2.0)] };
    let mut config = config(&["pso", "genetic"], 60); config.selection = ComponentSpec::new("ucb1");
    let report = trainer.train(&config, &split, &[1, 2], None).unwrap();
    assert_eq!(report.training.len(), 2); assert_eq!(report.validation.len(), 2); assert_eq!(report.test.len(), 2);
    assert!(report.training.iter().chain(&report.validation).chain(&report.test).all(|observation| observation.failure.is_none()));
    let evaluator = PolicyEvaluator::new(registry);
    let frozen = evaluator.evaluate(&config, &split.validation, &[1], Some(&report.policy)).unwrap();
    assert!(frozen[0].failure.is_none());
    let mut leaked = split;
    leaked.validation[0].problem = ComponentSpec::new("quadratic");
    // offset=0 的显式与省略配置不同，但问题指纹相同 / Explicit offset=0 and omitted offset differ as configs but share a problem fingerprint
    assert!(trainer.train(&config, &leaked, &[1], None).is_err());
}

#[test]
fn replay_and_jsonl_round_trip_match_online_q_updates() {
    use super::learning::{PolicyTrainer, TrajectoryFiles};
    let mut config = config(&["pso", "genetic"], 100); config.selection = ComponentSpec::new("q-learning");
    config.budget.slice_evaluations = 12;
    let solver = HyperHeuristicSolver::new(config.clone(), registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap(); run.run().unwrap();
    let mut replay = TabularSelectionPolicy::new(&config.selection, vec!["algorithm-0".into(), "algorithm-1".into()], true).unwrap();
    PolicyTrainer::<f64, f64>::replay(&mut replay, run.transitions(), "algorithm").unwrap();
    assert_eq!(replay.snapshot().unwrap(), run.export_policy().unwrap().root);
    let path = std::env::temp_dir().join(format!("ospf-trajectory-{}-{}.jsonl", std::process::id(), new_standalone_id().group_id));
    TrajectoryFiles::write(&path, run.transitions()).unwrap();
    assert_eq!(TrajectoryFiles::read(&path).unwrap(), run.transitions());
    std::fs::write(&path, "{\"truncated\":").unwrap();
    assert!(TrajectoryFiles::read(&path).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn baseline_comparison_reports_quality_costs_and_rejects_unequal_budgets() {
    use super::learning::{PolicyEvaluator, TrainingCase};
    use std::collections::BTreeMap;
    let evaluator = PolicyEvaluator::new(registry(Arc::new(AtomicU64::new(0))));
    let mut baselines = BTreeMap::from([("pso".into(), config(&["pso"], 40)), ("ga".into(), config(&["genetic"], 40))]);
    let cases = vec![TrainingCase { id: "test".into(), problem: ComponentSpec::new("quadratic") }];
    let reports = evaluator.compare(&baselines, &cases, &[1, 2], |result| result.best().unwrap().fitness).unwrap();
    for report in reports.values() {
        assert_eq!(report.quality.len(), 2); assert_eq!(report.evaluations, 80); assert_eq!(report.failure_rate, 0.0);
    }
    baselines.get_mut("ga").unwrap().budget.evaluations = Some(41);
    assert!(evaluator.compare(&baselines, &cases, &[1], |_| 0.0).is_err());
}

#[test]
fn explicit_scalarization_retains_canonical_vectors_and_survives_checkpoints() {
    let mut registry = SearchRegistry::with_multi_objective_builtins();
    registry.register_problem("vector", |_, _| Ok(Box::new(VectorProblem { x: new_standalone_id() }))).unwrap();
    registry.register_scalarization("weighted", |values| Ok(values[0] * 0.25 + values[1] * 0.75)).unwrap();
    let mut config = config(&["nsga-ii", "pso"], 100); config.problem = ComponentSpec::new("vector");
    config.budget.slice_evaluations = 12; config.persistence.enabled = true;
    assert!(HyperHeuristicSolver::new(config.clone(), registry.clone()).unwrap().open().is_err());
    config.algorithms[1].scalarization = Some("weighted".into());
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    let local = saved.workers["algorithm-1"].session.state["localArchive"].as_array().unwrap();
    assert!(!local.is_empty());
    assert!(local.iter().all(|candidate| candidate["fitness"].as_array().is_some_and(|values| values.len() == 2)));
    assert!(saved.workers["algorithm-1"].session.state["session"]["state"]["individuals"]
        .as_array().unwrap().iter().all(|candidate| candidate["fitness"].is_string()));
    let expected = run.run().unwrap();
    let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    let result = restored.run().unwrap();
    assert_eq!(result.evaluations, 100); assert_eq!(result.objective_calls, 100);
    assert!(result.best().is_none()); assert!(result.archive.iter().all(|value| value.fitness.len() == 2));
    assert_eq!(result.archive.iter().map(|value| &value.fitness).collect::<Vec<_>>(), expected.archive.iter().map(|value| &value.fitness).collect::<Vec<_>>());
}

#[test]
fn scalarization_failure_keeps_the_already_evaluated_canonical_candidate() {
    let mut registry = SearchRegistry::with_multi_objective_builtins();
    registry.register_problem("vector", |_, _| Ok(Box::new(VectorProblem { x: new_standalone_id() }))).unwrap();
    registry.register_scalarization("invalid", |_| Err(invalid("projection callback failed"))).unwrap();
    let mut config = config(&["pso"], 20); config.problem = ComponentSpec::new("vector");
    config.algorithms[0].scalarization = Some("invalid".into());
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap();
    assert!(run.advance().is_err());
    assert_eq!(run.result().evaluations, 1); assert_eq!(run.result().objective_calls, 1);
    assert_eq!(run.result().archive.len(), 1); assert_eq!(run.result().archive[0].fitness.len(), 2);
    assert_eq!(run.transitions().len(), 1); assert!(run.transitions()[0].failed);
}

#[test]
fn configuration_rejects_duplicate_keys_and_migrates_only_explicit_versions() {
    use std::collections::BTreeMap;
    use super::persistence::ConfigurationMigration;
    let config = config(&["pso"], 100);
    let text = ConfigurationFormats::encode(&config).unwrap();
    let duplicated = text.replacen("\"parameters\": {}", "\"parameters\": {\"x\": 1, \"x\": 2}", 1);
    assert!(ConfigurationFormats::decode(&duplicated).is_err());
    let mut document = serde_json::to_value(&config).unwrap(); document["version"] = 2.into();
    let text = serde_json::to_string(&document).unwrap();
    assert!(ConfigurationFormats::decode(&text).is_err());
    let mut migrations: BTreeMap<u32, ConfigurationMigration> = BTreeMap::new();
    migrations.insert(2, Box::new(|mut value| { value["version"] = 1.into(); Ok(value) }));
    assert_eq!(ConfigurationFormats::decode_migrating(&text, ConfigurationFormat::Json, &migrations).unwrap(), config);
    let yaml = "problem: {type: quadratic}\nalgorithms: [{id: pso, algorithm: {type: pso}}]\nbudget: {evaluations: 1, evaluations: 2}";
    assert!(ConfigurationFormats::decode_format(yaml, ConfigurationFormat::Yaml).is_err());
}

#[test]
fn checkpoint_configuration_matches_kotlin_default_omission_shape() {
    let mut config = HyperHeuristicConfig::new(ComponentSpec::new("quadratic"), 100).algorithm(AlgorithmConfig::new("swarm", "pso"));
    config.execution = ExecutionConfig::default();
    let compact = super::config::checkpoint_configuration(&config).unwrap();
    assert_eq!(compact, serde_json::json!({
        "problem": {"type": "quadratic"}, "algorithms": [{"id": "swarm", "algorithm": {"type": "pso"}}], "budget": {"evaluations": 100}
    }));
    let snapshot: PolicySnapshot = serde_json::from_value(serde_json::json!({"type":"round-robin","actions":["a"]})).unwrap();
    assert_eq!(snapshot.version, 1); assert_eq!(snapshot.revision, 0);
}

#[test]
fn kotlin_policy_order_and_empty_controlled_sessions_remain_portable() {
    let mut config = config(&["pso", "genetic"], 32);
    config.algorithms[0].id = "z-swarm".into(); config.algorithms[1].id = "a-genetic".into();
    config.persistence.enabled = true;
    let seed = config.execution.seed;
    let solver = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap();
    let saved = run.checkpoint().unwrap();
    assert_eq!(saved.policy.root.actions, vec!["z-swarm", "a-genetic"]);
    assert_eq!(saved.policy.operators.len(), 2);
    for (id, worker) in &saved.workers {
        assert_eq!(worker.random, SearchRandom::new(seed as u64).split(&format!("algorithm:{id}")).state() as i64);
        assert!(worker.session.state.get("session").is_some());
        assert_eq!(worker.session.state["slots"], serde_json::json!({}));
        assert!(worker.session.state.get("reward").is_none());
    }
    run.advance().unwrap();
    assert_eq!(run.transitions()[0].action, "z-swarm");
    assert_eq!(run.transitions()[0].before.decision, 0);
    assert_eq!(saved.problem, "quadratic:quadratic:1:0");
}

struct FailingProblem { x: VariableId, counter: std::cell::Cell<u64> }

impl SearchProblem<f64, f64> for FailingProblem {
    fn id(&self) -> &str { "failing" }
    fn fingerprint(&self) -> &str { "failing:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { (0..amount).map(|index| Solution::from([(self.x, index as f64)])).collect() }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        let next = self.counter.get() + 1; self.counter.set(next);
        assert_ne!(next, 2, "injected callback failure");
        Some(solution[&self.x])
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
}

#[test]
fn callback_failure_keeps_completed_candidates_and_records_exactly_one_feedback() {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("failing", |_, _| Ok(Box::new(FailingProblem { x: new_standalone_id(), counter: std::cell::Cell::new(0) }))).unwrap();
    let mut config = config(&["pso"], 100); config.problem = ComponentSpec::new("failing");
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap();
    assert!(run.advance().is_err());
    let result = run.result();
    assert_eq!(result.evaluations, 2); assert_eq!(result.objective_calls, 2);
    assert_eq!(result.archive.len(), 1); assert_eq!(result.steps, 0);
    assert_eq!(run.transitions().len(), 1); assert!(run.transitions()[0].failed);
    assert_eq!(run.transitions()[0].evaluations, 2);
}

#[test]
fn parallel_failures_drain_every_dispatch_and_credit_each_exactly_once() {
    for mode in [ExecutionMode::Synchronous, ExecutionMode::Asynchronous] {
        let mut registry = SearchRegistry::with_builtins();
        registry.register_problem("failing", |_, _| Ok(Box::new(FailingProblem { x: new_standalone_id(), counter: std::cell::Cell::new(0) }))).unwrap();
        let mut config = config(&["pso", "genetic"], 16);
        config.problem = ComponentSpec::new("failing"); config.execution.mode = mode; config.execution.parallelism = 2;
        config.budget.slice_evaluations = 4;
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap();
        assert!(run.advance().is_err());
        assert_eq!(run.result().evaluations, 4); assert_eq!(run.result().objective_calls, 4);
        assert_eq!(run.result().decisions, 2); assert_eq!(run.transitions().len(), 2);
        assert!(run.transitions().iter().all(|event| event.failed && event.evaluations == 2));
        assert!(run.export_policy().is_ok());
        run.close().unwrap(); run.close().unwrap();
    }
}

#[test]
fn nonrecoverable_callbacks_fail_before_search_when_persistence_is_enabled() {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("failing", |_, _| Ok(Box::new(FailingProblem { x: new_standalone_id(), counter: std::cell::Cell::new(0) }))).unwrap();
    let mut config = config(&["pso"], 100); config.problem = ComponentSpec::new("failing"); config.persistence.enabled = true;
    assert!(HyperHeuristicSolver::new(config, registry).unwrap().open().is_err());
}

#[test]
fn paused_iteration_excludes_idle_time_and_restores_saved_active_time() {
    let mut iteration = crate::solver::heuristic::Iteration::new(); iteration.pause();
    let paused = iteration.time(); std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(iteration.time(), paused);
    iteration.restore_time(std::time::Duration::from_nanos(123));
    assert_eq!(iteration.time().as_nanos(), 123);
    iteration.resume(); iteration.pause(); assert!(iteration.time().as_nanos() >= 123);
}
