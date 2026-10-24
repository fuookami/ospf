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

mod ruin;

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
fn pso_and_clpso_real_imports_warm_nearest_original_motion_and_preserve_true_best() {
    for kind in ["pso", "apso", "clpso"] {
        let calls = Arc::new(AtomicU64::new(0)); let registry = registry(calls.clone());
        let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
        let x = problem.variable_order()[0];
        let mut config = AlgorithmConfig::new("search", kind); config.size = 4;
        config.algorithm = config.algorithm.parameter("maxVelocity", 0.25);
        let random = RandomHandle::new(SearchRandom::new(1));
        let mut session = registry.session(AlgorithmContext { config, problem, random: random.clone() }).unwrap();
        let budget = Arc::new(SearchBudget::new(5));
        session.step(&mut SearchSlice::new(budget.clone(), 4, 1)).unwrap();
        let before_random = random.state();
        let report = session.import_solutions(vec![Solution::from([(x, 4.5)])], &mut SearchSlice::new(budget.clone(), 1, 0)).unwrap();
        assert_eq!(report.evaluations, 1); assert_eq!(report.steps, 0); assert_eq!(random.state(), before_random);
        assert_eq!(budget.used(), 5); assert_eq!(calls.load(Ordering::Relaxed), 5);
        let saved = session.snapshot().unwrap();
        let state = &saved.state["session"]["state"];
        assert_eq!(state["individuals"][6]["fitness"], "20.25");
        assert_eq!(state["individuals"][7]["fitness"], "16.0");
        assert_eq!(state["vectors"][3], serde_json::json!([0.25]));
        if kind == "clpso" {
            use crate::solver::heuristic::session::{ClpsoStateCodec, SearchCodec};
            let particles = ClpsoStateCodec::new(vec![x]).decode(state).unwrap();
            for (index, particle) in particles.iter().enumerate() { assert_eq!(particle.exemplars, vec![index]); assert_eq!(particle.stagnation, 0); }
        }
    }
}

#[test]
fn shade_real_import_archives_displaced_population_without_overwriting_learned_parameter_memory() {
    let registry = registry(Arc::new(AtomicU64::new(0)));
    let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
    let x = problem.variable_order()[0];
    let mut config = AlgorithmConfig::new("search", "shade"); config.size = 4;
    config.algorithm = config.algorithm.parameter("memorySize", 2);
    let mut session = registry.session(AlgorithmContext { config, problem, random: RandomHandle::new(SearchRandom::new(1)) }).unwrap();
    session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(4)), 4, 1)).unwrap();
    let mut saved = session.snapshot().unwrap();
    use crate::solver::heuristic::session::{SearchCodec, ShadeStateCodec};
    let codec = ShadeStateCodec::new(vec![x]);
    let mut native = codec.decode(&saved.state["session"]["state"]).unwrap();
    native.memory = crate::solver::heuristic::runner::ShadeMemory { memory_f: vec![0.21, 0.73], memory_cr: vec![0.31, 0.82], cursor: 1 };
    saved.state["session"]["state"] = codec.encode(&native).unwrap(); session.restore(&saved).unwrap();
    session.import_solutions(vec![Solution::from([(x, 0.5)])], &mut SearchSlice::new(Arc::new(SearchBudget::new(1)), 1, 0)).unwrap();
    let after = codec.decode(&session.snapshot().unwrap().state["session"]["state"]).unwrap();
    assert_eq!(after.memory, native.memory);
    assert_eq!(after.population[0].fitness, 0.25);
    assert!(after.archive.contains(&Solution::from([(x, 5.0)])));
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
        if self.slow && let Some(entered) = &self.entered { let _ = entered.send(()); }
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

fn assert_deterministic_checkpoint_eq(actual: &RunCheckpoint, expected: &RunCheckpoint, label: &str) {
    fn compare(actual: &serde_json::Value, expected: &serde_json::Value, path: &str) {
        match (actual, expected) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{path}: keys");
                for (key, value) in a { compare(value, &b[key], &format!("{path}/{key}")); }
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "{path}: length");
                for (index, (a, b)) in a.iter().zip(b).enumerate() { compare(a, b, &format!("{path}/{index}")); }
            }
            _ => assert_eq!(actual, expected, "{path}"),
        }
    }
    compare(&deterministic_checkpoint(actual), &deterministic_checkpoint(expected), label);
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
    fn has_constraint_violation(&self) -> bool { true }
    fn constraint_violation(&self, _: &Solution<f64>) -> crate::solver::heuristic::result::Result<f64> { Ok(0.0) }
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

#[test]
fn constraint_view_preserves_custom_evaluation_errors_and_charges_exactly_once() {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("fallible", |_, _| Ok(Box::new(FallibleEvaluation { x: new_standalone_id(), calls: std::cell::Cell::new(0) }))).unwrap();
    let mut config = config(&["pso"], 20);
    config.problem = ComponentSpec::new("fallible");
    config.algorithms[0].constraints = Some(crate::solver::heuristic::session::ConstraintHandlingConfig::default());
    let mut run = HyperHeuristicSolver::new(config, registry).unwrap().open().unwrap();
    assert!(run.advance().is_err());
    assert_eq!(run.result().evaluations, 2);
    assert_eq!(run.result().objective_calls, 1);
    assert_eq!(run.result().archive.len(), 1);
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

fn non_stationary_config(kind: &str, mode: ExecutionMode) -> HyperHeuristicConfig {
    let mut config = config(&["pso", "genetic"], 128);
    config.selection = ComponentSpec::new(kind);
    config.execution.mode = mode;
    config.execution.parallelism = 2;
    config.budget.slice_evaluations = 12;
    config.persistence.enabled = true;
    for algorithm in &mut config.algorithms { algorithm.selection = ComponentSpec::new(kind); }
    config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant").parameter("value", 0.2),
        ComponentSpec::new("constant").parameter("value", 0.8)]);
    config.algorithms[1].updates = vec![ComponentSpec::new("single-point"), ComponentSpec::new("blend")];
    config
}

#[test]
fn non_stationary_both_levels_restore_complete_credit_and_discard_unused_slots() {
    for kind in ["frr-mab", "choice-function", "adaptive-pursuit"] {
        for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
            let mut config = non_stationary_config(kind, mode);
            for algorithm in &mut config.algorithms {
                algorithm.constraints = Some(crate::solver::heuristic::session::ConstraintHandlingConfig {
                    mode: crate::solver::heuristic::session::ConstraintMode::StochasticRanking, ..Default::default()
                });
            }
            let solver = HyperHeuristicSolver::new(config, constraint_registry(Arc::new(std::sync::atomic::AtomicBool::new(false)))).unwrap();
            let mut run = solver.open().unwrap();
            run.advance().unwrap(); run.advance().unwrap();
            let saved = run.checkpoint().unwrap();
            assert!(saved.validate_kotlin_v1().is_err());
            let mut restored = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
            run.run().unwrap(); restored.run().unwrap();
            let expected = run.checkpoint().unwrap();
            let actual = restored.checkpoint().unwrap();
            assert_eq!(deterministic_checkpoint(&actual), deterministic_checkpoint(&expected), "{kind}/{mode:?}");
            assert!(expected.policy.root.revision > 0);
            assert_eq!(expected.policy.root.extension["clock"].as_u64(), Some(expected.completed));
            assert!(expected.policy.root.extension["pending"].as_object().unwrap().is_empty());
            for policies in expected.policy.operators.values() {
                for policy in policies.values() { assert!(policy.extension["pending"].as_object().unwrap().is_empty()); }
            }
            assert!(expected.policy.operators.values().flat_map(|policies| policies.values()).any(|policy| policy.revision > 0));
        }
    }
}

#[test]
fn non_stationary_choice_replay_learns_frozen_predecessors_and_frozen_runs_do_not_learn() {
    let config = non_stationary_config("choice-function", ExecutionMode::Serial);
    let solver = HyperHeuristicSolver::new(config.clone(), registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open().unwrap(); run.run().unwrap();
    let policy = run.export_policy().unwrap();
    let mut replay = NonStationarySelector::new(&config.selection, policy.root.actions.clone(), true).unwrap();
    super::learning::PolicyTrainer::<f64, f64>::replay(&mut replay, run.transitions(), "algorithm").unwrap();
    assert_eq!(replay.snapshot().unwrap(), policy.root);
    let mut frozen = config.clone(); frozen.learning.mode = LearningMode::Frozen;
    let solver = HyperHeuristicSolver::new(frozen, registry(Arc::new(AtomicU64::new(0)))).unwrap();
    let mut run = solver.open_with(RunOptions { checkpoint: None, policy: Some(&policy) }).unwrap();
    let before = run.export_policy().unwrap(); run.run().unwrap();
    assert_eq!(run.export_policy().unwrap(), before);
    assert!(run.transitions().iter().all(|event| event.previous_action.is_none()));
}

struct FailingPreparationExchange { calls: u64 }

impl RecoverableComponent for FailingPreparationExchange {
    fn save(&self) -> crate::solver::heuristic::result::Result<serde_json::Value> { Ok(self.calls.into()) }
    fn load(&mut self, value: &serde_json::Value) -> crate::solver::heuristic::result::Result<()> {
        self.calls = value.as_u64().ok_or_else(|| invalid("invalid exchange calls"))?;
        Ok(())
    }
}

impl ExchangePolicy for FailingPreparationExchange {
    fn requires_import(&self) -> bool { false }
    fn amount(&mut self, _: u64) -> crate::solver::heuristic::result::Result<usize> {
        self.calls += 1;
        if self.calls == 2 { panic!("second preparation failed"); }
        Ok(0)
    }
}

#[test]
fn choice_discards_unexecuted_decisions_when_serial_or_partial_batch_preparation_panics() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let mut config = non_stationary_config("choice-function", mode);
        config.memory.exchange = ComponentSpec::new("failing-preparation");
        let mut registry = registry(Arc::new(AtomicU64::new(0)));
        registry.components.register_exchange("failing-preparation", |_| Ok(Box::new(FailingPreparationExchange { calls: 0 }))).unwrap();
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap();
        if mode == ExecutionMode::Serial { run.advance().unwrap(); }
        assert!(run.advance().is_err());
        let saved = run.checkpoint().unwrap();
        assert!(saved.policy.root.extension["pending"].as_object().unwrap().is_empty());
        let completed = if mode == ExecutionMode::Serial { 1 } else { 0 };
        assert_eq!(saved.dispatched, completed);
        assert_eq!(saved.completed, completed);
        assert_eq!(saved.policy.root.revision, completed);
        assert_eq!(saved.transitions.iter().filter(|event| event.scope == "algorithm").count() as u64, completed);
    }
}

fn non_stationary_schedule_solver(kind: &str, reverse: bool) -> HyperHeuristicSolver<f64, f64> {
    non_stationary_schedule_solver_with_failure(kind, reverse, false)
}

fn non_stationary_schedule_solver_with_failure(kind: &str, reverse: bool, fail: bool) -> HyperHeuristicSolver<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    let factories = Arc::new(AtomicU64::new(0));
    registry.register_problem("scheduled", move |_, _| {
        let role = factories.fetch_add(1, Ordering::SeqCst) % 3;
        Ok(Box::new(ScheduledProblem { inner: Quadratic { x: new_standalone_id(), calls: Arc::new(AtomicU64::new(0)),
            offset: 0.0, fingerprint: "scheduled:1".into() }, slow: role == if reverse { 2 } else { 1 }, fail: fail && role == 1, entered: None }))
    }).unwrap();
    let mut config = non_stationary_config(kind, ExecutionMode::Asynchronous);
    config.problem = ComponentSpec::new("scheduled");
    config.budget.slice_evaluations = 8;
    HyperHeuristicSolver::new(config, registry).unwrap()
}

#[test]
fn non_stationary_schedule_stop_and_failure_drain_credit_exactly_once() {
    for kind in ["frr-mab", "choice-function", "adaptive-pursuit"] {
        for fail in [false, true] {
            let solver = non_stationary_schedule_solver_with_failure(kind, false, fail);
            let mut run = solver.open().unwrap(); run.record_schedule().unwrap();
            if fail { assert!(run.run().is_err()); }
            else { run.advance().unwrap(); run.request_stop(); }
            let expected = run.checkpoint().unwrap();
            let log = run.export_schedule().unwrap();
            assert_eq!(expected.dispatched, expected.completed);
            assert_eq!(expected.policy.root.revision, expected.completed);
            assert!(expected.policy.root.extension["pending"].as_object().unwrap().is_empty());
            let decisions: std::collections::BTreeSet<_> = expected.transitions.iter()
                .filter(|event| event.scope == "algorithm").map(|event| event.decision).collect();
            assert_eq!(decisions.len() as u64, expected.completed);
            let solver = non_stationary_schedule_solver_with_failure(kind, true, fail);
            let mut replay = solver.open().unwrap(); replay.replay_schedule(&log).unwrap();
            assert_eq!(replay.run().is_err(), fail);
            assert_eq!(replay.export_schedule().unwrap(), log);
            assert_deterministic_checkpoint_eq(&replay.checkpoint().unwrap(), &expected, &format!("{kind}/{fail}"));
        }
    }
}

#[test]
fn non_stationary_async_schedule_reexecutes_and_restores_dispatch_frozen_credit() {
    for kind in ["frr-mab", "choice-function", "adaptive-pursuit"] {
        let solver = non_stationary_schedule_solver(kind, false);
        let mut recorded = solver.open().unwrap(); recorded.record_schedule().unwrap();
        recorded.advance().unwrap(); recorded.advance().unwrap();
        let prefix = recorded.checkpoint().unwrap();
        recorded.run().unwrap();
        let expected = recorded.checkpoint().unwrap();
        let log = recorded.export_schedule().unwrap();
        let solver = non_stationary_schedule_solver(kind, true);
        let mut replay = solver.open().unwrap(); replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
        assert_eq!(replay.export_schedule().unwrap(), log);
        assert_deterministic_checkpoint_eq(&replay.checkpoint().unwrap(), &expected, kind);
        let mut restored = solver.open_with(RunOptions { checkpoint: Some(&prefix), policy: None }).unwrap();
        restored.replay_schedule(&log).unwrap(); restored.run().unwrap();
        assert_deterministic_checkpoint_eq(&restored.checkpoint().unwrap(), &expected, &format!("{kind}/prefix"));
        if kind == "choice-function" {
            let transitions: Vec<_> = expected.transitions.iter().filter(|event| event.scope == "algorithm").collect();
            assert!(transitions.windows(2).any(|pair| pair[1].decision < pair[0].decision));
            assert!(transitions.iter().any(|event| event.previous_action.is_some()));
            let mut learned = NonStationarySelector::new(&ComponentSpec::new(kind), expected.policy.root.actions.clone(), true).unwrap();
            super::learning::PolicyTrainer::<f64, f64>::replay(&mut learned, &expected.transitions, "algorithm").unwrap();
            assert_eq!(learned.snapshot().unwrap(), expected.policy.root);
        }
    }
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

struct ConstrainedQuadratic {
    x: VariableId,
    bad_violation: Arc<std::sync::atomic::AtomicBool>,
}

impl SearchProblem<f64, f64> for ConstrainedQuadratic {
    fn id(&self) -> &str { "quadratic" }
    fn fingerprint(&self) -> &str { "constrained-quadratic:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(-10.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(10.0) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        (0..amount).map(|index| Solution::from([(self.x, index as f64 - 2.0)])).collect()
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { Some((solution[&self.x] + 1.0).powi(2)) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn feasible(&self, solution: &Solution<f64>) -> bool { solution[&self.x] >= 0.0 }
    fn admissible(&self, solution: &Solution<f64>) -> bool { solution[&self.x].is_finite() && solution[&self.x] >= -8.0 }
    fn has_constraint_violation(&self) -> bool { true }
    fn constraint_violation(&self, solution: &Solution<f64>) -> crate::solver::heuristic::result::Result<f64> {
        if self.bad_violation.load(Ordering::Relaxed) { Err(invalid("domain violation callback failed")) }
        else { Ok((-solution[&self.x]).max(0.0)) }
    }
    fn stateless(&self) -> bool { true }
}

fn constraint_registry(bad: Arc<std::sync::atomic::AtomicBool>) -> SearchRegistry<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("quadratic", move |_, _| Ok(Box::new(ConstrainedQuadratic {
        x: new_standalone_id(), bad_violation: bad.clone(),
    }))).unwrap();
    registry
}

fn without_constraint_timing(mut value: serde_json::Value) -> serde_json::Value {
    match &mut value {
        serde_json::Value::Object(fields) => {
            fields.remove("activeNanos");
            for value in fields.values_mut() { *value = without_constraint_timing(std::mem::take(value)); }
        }
        serde_json::Value::Array(values) => {
            for value in values { *value = without_constraint_timing(std::mem::take(value)); }
        }
        _ => {}
    }
    value
}

#[test]
fn constraint_modes_resume_complete_native_and_random_states_across_families() {
    use crate::solver::heuristic::session::{ConstraintHandlingConfig, ConstraintMode};
    let registry = constraint_registry(Arc::new(std::sync::atomic::AtomicBool::new(false)));
    for mode in [ConstraintMode::FeasibilityFirst, ConstraintMode::Epsilon, ConstraintMode::StochasticRanking] {
        for kind in ["pso", "apso", "clpso", "shade", "l-shade", "cma-es", "hill-climbing", "genetic"] {
            let mut algorithm = config(&[kind], 1000).algorithms.remove(0);
            algorithm.constraints = Some(ConstraintHandlingConfig { mode, horizon: 5, ..Default::default() });
            let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
            let random = RandomHandle::new(SearchRandom::new(42));
            let mut session = registry.session(AlgorithmContext { config: algorithm.clone(), problem, random: random.clone() }).unwrap();
            let report = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(1000)), 500, 2)).unwrap();
            assert_eq!(report.steps, 2, "{mode:?}/{kind}: {:?}", report.diagnostics);
            let saved = session.snapshot().unwrap();
            assert_eq!(saved.constraints["steps"], 2);
            assert_eq!(saved.constraints["scale"], 2.0);
            let stream = random.state();
            let expected = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(1000)), 500, 2)).unwrap();
            let expected_state = without_constraint_timing(serde_json::to_value(session.snapshot().unwrap()).unwrap());
            let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
            let random = RandomHandle::new(SearchRandom::new(99));
            let mut restored = registry.session(AlgorithmContext { config: algorithm, problem, random: random.clone() }).unwrap();
            restored.restore(&saved).unwrap();
            random.restore(stream);
            let actual = restored.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(1000)), 500, 2)).unwrap();
            assert_eq!(actual.evaluations, expected.evaluations, "{mode:?}/{kind}");
            assert_eq!(without_constraint_timing(serde_json::to_value(restored.snapshot().unwrap()).unwrap()), expected_state, "{mode:?}/{kind}");
        }
    }
}

#[test]
fn constraint_budget_interruptions_and_failed_imports_roll_back_without_refunding() {
    use crate::solver::heuristic::session::ConstraintHandlingConfig;
    let bad = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let registry = constraint_registry(bad.clone());
    let mut algorithm = AlgorithmConfig::new("leaf", "pso");
    algorithm.size = 4;
    algorithm.constraints = Some(ConstraintHandlingConfig::default());
    let problem = registry.problem(&ComponentSpec::new("quadratic"), SearchRandom::new(1)).unwrap();
    let x = problem.variable_order()[0];
    let mut session = registry.session(AlgorithmContext { config: algorithm, problem, random: RandomHandle::new(SearchRandom::new(42)) }).unwrap();
    let budget = Arc::new(SearchBudget::new(100));
    let empty = session.snapshot().unwrap();
    let partial = session.step(&mut SearchSlice::new(budget.clone(), 2, 1)).unwrap();
    assert_eq!((partial.steps, partial.evaluations, partial.candidates.len()), (0, 2, 2));
    assert_eq!(session.snapshot().unwrap().constraints, empty.constraints);
    assert!(!session.snapshot().unwrap().initialized);
    session.step(&mut SearchSlice::new(budget.clone(), 4, 1)).unwrap();
    let before = session.snapshot().unwrap();
    let partial = session.step(&mut SearchSlice::new(budget.clone(), 2, 1)).unwrap();
    let after = session.snapshot().unwrap();
    assert_eq!((partial.steps, partial.evaluations), (0, 2));
    assert_eq!(before.constraints, after.constraints);
    assert_eq!(before.state["session"]["state"], after.state["session"]["state"]);
    assert_eq!(before.state["session"]["archive"], after.state["session"]["archive"]);
    let used = budget.used();
    let calls = budget.objective_calls();
    bad.store(true, Ordering::Relaxed);
    let failure = session.import_solutions(vec![Solution::from([(x, -0.5)])], &mut SearchSlice::new(budget.clone(), 4, 1)).unwrap();
    assert_eq!(failure.termination, SearchTermination::Failed);
    assert_eq!(budget.used(), used + 1);
    assert_eq!(budget.objective_calls(), calls);
    let after = session.snapshot().unwrap();
    assert_eq!(before.constraints, after.constraints);
    assert_eq!(before.state["session"]["state"], after.state["session"]["state"]);
    bad.store(false, Ordering::Relaxed);
    let imported = session.import_solutions(vec![Solution::from([(x, 0.25)])], &mut SearchSlice::new(budget.clone(), 4, 1)).unwrap();
    assert_eq!(imported.termination, SearchTermination::SliceComplete);
    assert_eq!(after.constraints["steps"], session.snapshot().unwrap().constraints["steps"]);
}

#[test]
fn constraint_search_can_visit_infeasible_points_but_global_archives_stay_strict() {
    let mut config = config(&["genetic", "pso"], 100);
    for algorithm in &mut config.algorithms {
        algorithm.constraints = Some(crate::solver::heuristic::session::ConstraintHandlingConfig::default());
    }
    config.persistence.enabled = true;
    let solver = HyperHeuristicSolver::new(config, constraint_registry(Arc::new(std::sync::atomic::AtomicBool::new(false)))).unwrap();
    let mut run = solver.open().unwrap();
    run.run().unwrap();
    let result = run.result();
    assert_eq!(result.evaluations, 100);
    assert!(!result.archive.is_empty());
    assert!(result.archive.iter().all(|individual| individual.solution.values().all(|value| *value >= 0.0)));
    let working = run.checkpoint().unwrap().working;
    assert!(working.as_array().unwrap().iter().all(|individual| individual["solution"].as_array().unwrap().iter()
        .all(|value| value.as_str().unwrap().parse::<f64>().unwrap() >= 0.0)));
}

#[test]
fn constraint_preflight_rejects_missing_callback_composite_root_and_undeclared_factory() {
    let mut requested = config(&["pso"], 100);
    requested.algorithms[0].constraints = Some(crate::solver::heuristic::session::ConstraintHandlingConfig::default());
    let calls = Arc::new(AtomicU64::new(0));
    assert!(HyperHeuristicSolver::new(requested.clone(), registry(calls.clone())).unwrap().open().is_err());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let mut registry = constraint_registry(Arc::new(std::sync::atomic::AtomicBool::new(false)));
    registry.register_algorithm("custom", |_| panic!("unsupported factory must not run")).unwrap();
    requested.algorithms[0].algorithm = ComponentSpec::new("custom");
    assert!(HyperHeuristicSolver::new(requested.clone(), registry.clone()).unwrap().open().is_err());
    requested.algorithms[0].algorithm = ComponentSpec::new("island");
    requested.algorithms[0].children = vec![AlgorithmConfig::new("child", "pso")];
    requested.algorithms[0].import_mode = ImportMode::Native;
    assert!(HyperHeuristicSolver::new(requested, registry).is_err());
}

struct ConstrainedVector(VectorProblem);

impl SearchProblem<Vec<f64>, f64> for ConstrainedVector {
    fn id(&self) -> &str { "vector" }
    fn fingerprint(&self) -> &str { "constrained-vector:1" }
    fn variable_order(&self) -> Vec<VariableId> { self.0.variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.0.lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.0.upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.0.initial_solutions(amount) }
    fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> { self.0.objective(solution) }
    fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { self.0.dominates(lhs, rhs) }
    fn equivalent(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { lhs == rhs }
    fn valid_objective(&self, objective: &Vec<f64>) -> bool { self.0.valid_objective(objective) }
    fn admissible(&self, solution: &Solution<f64>) -> bool { self.0.feasible(solution) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { solution[&self.0.x] >= 1.0 }
    fn has_constraint_violation(&self) -> bool { true }
    fn constraint_violation(&self, solution: &Solution<f64>) -> crate::solver::heuristic::result::Result<f64> {
        Ok((1.0 - solution[&self.0.x]).max(0.0))
    }
    fn multi_objective(&self) -> bool { true }
    fn stateless(&self) -> bool { true }
}

#[test]
fn constraint_nsga_and_scalarization_preserve_vectors_tiers_and_shared_state() {
    use crate::solver::heuristic::session::{ConstraintHandlingConfig, ConstraintMode};
    for mode in [ConstraintMode::FeasibilityFirst, ConstraintMode::Epsilon, ConstraintMode::StochasticRanking] {
        for execution in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
            let mut registry = SearchRegistry::with_multi_objective_builtins();
            registry.register_problem("vector", |_, _| Ok(Box::new(ConstrainedVector(VectorProblem { x: new_standalone_id() })))).unwrap();
            registry.register_scalarization("weighted", |values| Ok(values[0] * 0.25 + values[1] * 0.75)).unwrap();
            let mut config = config(&["pso", "nsga-ii", "nsga-iii"], 128);
            config.problem = ComponentSpec::new("vector");
            config.execution.mode = execution;
            config.execution.parallelism = 2;
            config.persistence.enabled = true;
            config.budget.slice_evaluations = 12;
            for algorithm in &mut config.algorithms {
                algorithm.constraints = Some(ConstraintHandlingConfig { mode, horizon: 5, ..Default::default() });
            }
            config.algorithms[0].scalarization = Some("weighted".into());
            let solver = HyperHeuristicSolver::new(config, registry).unwrap();
            let mut run = solver.open().unwrap();
            run.advance().unwrap();
            let saved = run.checkpoint().unwrap();
            assert!(saved.validate_kotlin_v1().is_err());
            let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
            run.run().unwrap(); resumed.run().unwrap();
            assert_eq!(run.transitions(), resumed.transitions(), "{mode:?}/{execution:?}");
            let expected = run.checkpoint().unwrap();
            let actual = resumed.checkpoint().unwrap();
            for id in expected.workers.keys() {
                let expected = without_constraint_timing(serde_json::to_value(&expected.workers[id]).unwrap());
                let actual = without_constraint_timing(serde_json::to_value(&actual.workers[id]).unwrap());
                assert_eq!(actual, expected, "{mode:?}/{execution:?}/{id}");
            }
            let result = run.result();
            assert!(!result.archive.is_empty());
            assert_eq!(result.evaluations, 128);
            assert!(result.archive.iter().all(|entry| entry.fitness.len() == 2 && entry.solution.values().all(|value| *value >= 1.0)));
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        }
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

struct CreditProblem {
    inner: Quadratic,
    coordinates: bool,
    malformed: bool,
}

impl SearchProblem<f64, f64> for CreditProblem {
    fn id(&self) -> &str { "credit" }
    fn fingerprint(&self) -> &str { "credit:1" }
    fn variable_order(&self) -> Vec<VariableId> { self.inner.variable_order() }
    fn lower_bound(&self, _: usize) -> Option<f64> { Some(0.0) }
    fn upper_bound(&self, _: usize) -> Option<f64> { Some(10.0) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.inner.initial_solutions(amount) }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { self.inner.objective(solution) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs > rhs }
    fn valid_objective(&self, objective: &f64) -> bool { objective.is_finite() }
    fn feasible(&self, solution: &Solution<f64>) -> bool { self.inner.feasible(solution) }
    fn admissible(&self, solution: &Solution<f64>) -> bool { (0.0..=10.0).contains(&solution[&self.inner.x]) }
    fn has_objective_coordinates(&self) -> bool { self.coordinates }
    fn objective_coordinates(&self, objective: &f64) -> crate::solver::heuristic::result::Result<Vec<f64>> {
        Ok(vec![if self.malformed { f64::NAN } else { -objective / 100.0 }])
    }
    fn has_constraint_violation(&self) -> bool { true }
    fn constraint_violation(&self, _: &Solution<f64>) -> crate::solver::heuristic::result::Result<f64> { Ok(0.0) }
    fn stateless(&self) -> bool { true }
}

struct CaptureCredit {
    contexts: Arc<std::sync::Mutex<Vec<RewardContext>>>,
    timed: bool,
}

impl RecoverableComponent for CaptureCredit { fn stateless(&self) -> bool { true } }
impl RewardPolicy for CaptureCredit {
    fn requires_coordinates(&self) -> bool { true }
    fn requires_elapsed_time(&self) -> bool { self.timed }
    fn reward(&mut self, context: &RewardContext) -> crate::solver::heuristic::result::Result<f64> {
        self.contexts.lock().unwrap().push(context.clone());
        Ok(context.contribution)
    }
}

fn credit_registry(
    calls: Arc<AtomicU64>,
    contexts: Arc<std::sync::Mutex<Vec<RewardContext>>>,
    coordinates: bool,
    malformed: bool,
    timed: bool,
) -> SearchRegistry<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    registry.register_problem("credit", move |_, _| Ok(Box::new(CreditProblem {
        inner: Quadratic { x: new_standalone_id(), calls: calls.clone(), offset: 0.0, fingerprint: "credit:1".into() },
        coordinates, malformed,
    }))).unwrap();
    registry.components.register_reward("capture-credit", move |_| Ok(Box::new(CaptureCredit { contexts: contexts.clone(), timed }))).unwrap();
    registry
}

#[test]
fn coordinate_rewards_receive_real_cost_and_canonical_coordinates_at_both_levels() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous, ExecutionMode::Asynchronous] {
        let calls = Arc::new(AtomicU64::new(0));
        let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut config = config(&["pso", "genetic"], 32);
        config.problem = ComponentSpec::new("credit"); config.learning.reward = ComponentSpec::new("capture-credit");
        config.execution.mode = mode; config.execution.parallelism = 2;
        config.budget.slice_evaluations = 8;
        config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant")]);
        let solver = HyperHeuristicSolver::new(config, credit_registry(calls.clone(), contexts.clone(), true, false, false)).unwrap();
        let result = solver.solve().unwrap();
        assert_eq!(result.evaluations, 32);
        assert_eq!(result.objective_calls, calls.load(Ordering::Relaxed));
        assert!(result.archive.iter().all(|value| value.fitness >= 0.0));
        let contexts = contexts.lock().unwrap();
        let outer: Vec<_> = contexts.iter().filter(|context| !context.action.contains('/')).collect();
        let inner: Vec<_> = contexts.iter().filter(|context| context.action.starts_with("inertia/constant:1:")).collect();
        assert_eq!(outer.len() as u64, result.decisions);
        assert!(!inner.is_empty(), "{mode:?}");
        assert!(outer[0].reference.is_empty());
        assert!(outer.iter().skip(1).any(|context| !context.reference.is_empty()));
        assert!(contexts.iter().all(|context| context.elapsed_nanos > 0 && context.evaluations > 0));
        assert!(contexts.iter().flat_map(|context| &context.candidates).all(|point| point.len() == 1 && point[0] <= 0.0));
        assert!(inner.iter().all(|context| !context.reference.is_empty()));
    }
}

#[test]
fn coordinate_capabilities_fail_preflight_without_implicit_scalar_projection() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let calls = Arc::new(AtomicU64::new(0));
        let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut config = config(&["pso"], 16); config.problem = ComponentSpec::new("credit");
        config.learning.reward = ComponentSpec::new("capture-credit"); config.execution.mode = mode; config.execution.parallelism = 2;
        let solver = HyperHeuristicSolver::new(config, credit_registry(calls.clone(), contexts.clone(), false, false, false)).unwrap();
        assert!(solver.open().err().unwrap().to_string().contains("coordinates"));
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert!(contexts.lock().unwrap().is_empty());
    }
}

#[test]
fn inconsistent_worker_coordinate_capabilities_fail_before_evaluation() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let factories = Arc::new(AtomicU64::new(0));
        let calls = Arc::new(AtomicU64::new(0));
        let constructed = factories.clone();
        let evaluated = calls.clone();
        let mut registry = SearchRegistry::with_builtins();
        registry.register_problem("credit", move |_, _| Ok(Box::new(CreditProblem {
            inner: Quadratic { x: new_standalone_id(), calls: evaluated.clone(), offset: 0.0, fingerprint: "credit:1".into() },
            coordinates: constructed.fetch_add(1, Ordering::Relaxed) == 0, malformed: false,
        }))).unwrap();
        let mut config = config(&["pso"], 16); config.problem = ComponentSpec::new("credit");
        config.execution.mode = mode; config.execution.parallelism = 2;
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        assert!(solver.open().err().unwrap().to_string().contains("incompatible definitions"));
        assert_eq!(factories.load(Ordering::Relaxed), 2);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn malformed_credit_coordinates_preserve_candidates_and_exactly_once_terminal_feedback() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let calls = Arc::new(AtomicU64::new(0));
        let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut config = config(&["pso"], 16); config.problem = ComponentSpec::new("credit");
        config.learning.reward = ComponentSpec::new("capture-credit"); config.execution.mode = mode; config.execution.parallelism = 2;
        config.budget.slice_evaluations = 4;
        let solver = HyperHeuristicSolver::new(config, credit_registry(calls.clone(), contexts.clone(), true, true, false)).unwrap();
        let mut run = solver.open().unwrap();
        assert!(run.advance().is_err());
        assert_eq!(run.result().evaluations, 4);
        assert_eq!(run.result().objective_calls, 4);
        assert_eq!(run.result().archive.len(), 4);
        assert_eq!(run.transitions().len(), 1);
        assert!(run.transitions()[0].failed && run.transitions()[0].terminal);
        assert_eq!(run.transitions()[0].reward, 0.0);
        assert!(contexts.lock().unwrap().is_empty());
        assert!(run.advance().is_ok());
        assert_eq!(run.transitions().len(), 1);
    }
}

#[test]
fn invalid_local_coordinates_fail_used_slot_feedback_without_losing_native_progress() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let constructed = Arc::new(AtomicU64::new(0));
        let mut registry = SearchRegistry::with_builtins();
        registry.register_problem("credit", move |_, _| Ok(Box::new(CreditProblem {
            inner: Quadratic { x: new_standalone_id(), calls: Arc::new(AtomicU64::new(0)), offset: 0.0, fingerprint: "credit:1".into() },
            coordinates: true, malformed: constructed.fetch_add(1, Ordering::Relaxed) > 0,
        }))).unwrap();
        let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
        registry.components.register_reward("capture-credit", move |_| Ok(Box::new(CaptureCredit { contexts: contexts.clone(), timed: false }))).unwrap();
        let mut config = config(&["pso"], 16); config.problem = ComponentSpec::new("credit");
        config.learning.reward = ComponentSpec::new("capture-credit"); config.persistence.enabled = true;
        config.execution.mode = mode; config.execution.parallelism = 2; config.budget.slice_evaluations = 4;
        config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant")]);
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap();
        assert!(run.advance().is_err());
        let saved = run.checkpoint().unwrap();
        assert_eq!(saved.evaluations, 8);
        assert_eq!(saved.steps, 2);
        assert_eq!(saved.workers["algorithm-0"].session.state["session"]["iteration"], 2);
        assert_eq!(saved.transitions.iter().filter(|event| event.scope == "algorithm").count(), 2);
        let inner: Vec<_> = saved.transitions.iter().filter(|event| event.scope == "algorithm-0/inertia").collect();
        assert_eq!(inner.len(), 1);
        assert!(inner[0].failed && inner[0].terminal);
        assert_eq!(inner[0].reward, 0.0);
        assert_eq!(run.result().objective_calls, 8);
        assert!(!run.result().archive.is_empty());
    }
}

#[test]
fn elapsed_time_rewards_reject_exact_schedule_control_but_allow_normal_search() {
    let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut config = config(&["pso", "genetic"], 16); config.problem = ComponentSpec::new("credit");
    config.learning.reward = ComponentSpec::new("capture-credit");
    config.execution.mode = ExecutionMode::Asynchronous; config.execution.parallelism = 2;
    config.budget.slice_evaluations = 4;
    let solver = HyperHeuristicSolver::new(config, credit_registry(Arc::new(AtomicU64::new(0)), contexts, true, false, true)).unwrap();
    let mut run = solver.open().unwrap();
    assert!(run.record_schedule().is_err());
    let source = ScheduleLog { version: 1, configuration: String::new(), problem: String::new(), events: Vec::new(), checksum: String::new() };
    assert!(run.replay_schedule(&source).unwrap_err().to_string().contains("elapsed-time rewards"));
    assert_eq!(run.run().unwrap().evaluations, 16);
    assert!(run.transitions().iter().filter(|event| event.scope == "algorithm").all(|event| event.elapsed_nanos > 0));
}

#[test]
fn coordinate_credit_preserves_composite_leaf_actions_and_checkpoint_recovery() {
    let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut config = config(&["island"], 64); config.problem = ComponentSpec::new("credit");
    config.learning.reward = ComponentSpec::new("capture-credit"); config.persistence.enabled = true;
    config.budget.slice_evaluations = 16;
    config.algorithms[0].import_mode = ImportMode::Native;
    config.algorithms[0].children = ["pso", "genetic"].into_iter().enumerate().map(|(index, kind)| {
        let mut child = AlgorithmConfig::new(format!("child-{index}"), kind); child.size = 4;
        if index == 0 { child.parameters.insert("inertia".into(), vec![ComponentSpec::new("constant")]); }
        child
    }).collect();
    let solver = HyperHeuristicSolver::new(config, credit_registry(Arc::new(AtomicU64::new(0)), contexts.clone(), true, false, false)).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap();
    let saved = run.checkpoint().unwrap();
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
    run.run().unwrap(); resumed.run().unwrap();
    assert_eq!(run.transitions(), resumed.transitions());
    assert_eq!(run.result().best().unwrap().fitness, resumed.result().best().unwrap().fitness);
    assert!(contexts.lock().unwrap().iter().any(|context| context.action.starts_with("inertia/constant:1:") && !context.reference.is_empty()));
}

struct CreditVector(VectorProblem);
impl SearchProblem<Vec<f64>, f64> for CreditVector {
    fn id(&self) -> &str { self.0.id() }
    fn fingerprint(&self) -> &str { self.0.fingerprint() }
    fn variable_order(&self) -> Vec<VariableId> { self.0.variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.0.lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.0.upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.0.initial_solutions(amount) }
    fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> { self.0.objective(solution) }
    fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { self.0.dominates(lhs, rhs) }
    fn equivalent(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { self.0.equivalent(lhs, rhs) }
    fn valid_objective(&self, objective: &Vec<f64>) -> bool { self.0.valid_objective(objective) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { self.0.feasible(solution) }
    fn multi_objective(&self) -> bool { true }
    fn has_objective_coordinates(&self) -> bool { true }
    fn objective_coordinates(&self, objective: &Vec<f64>) -> crate::solver::heuristic::result::Result<Vec<f64>> {
        Ok(objective.iter().map(|value| value / 10.0).collect())
    }
    fn has_constraint_violation(&self) -> bool { true }
    fn constraint_violation(&self, _: &Solution<f64>) -> crate::solver::heuristic::result::Result<f64> { Ok(0.0) }
    fn stateless(&self) -> bool { true }
}

#[test]
fn scalarized_and_nsga_credit_keep_normalized_canonical_vectors_after_recovery() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let contexts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut registry = SearchRegistry::with_multi_objective_builtins();
        registry.register_problem("vector", |_, _| Ok(Box::new(CreditVector(VectorProblem { x: new_standalone_id() })))).unwrap();
        registry.register_scalarization("weighted", |values| Ok(values[0] * 0.25 + values[1] * 0.75)).unwrap();
        let captured = contexts.clone();
        registry.components.register_reward("capture-credit", move |_| Ok(Box::new(CaptureCredit { contexts: captured.clone(), timed: false }))).unwrap();
        let mut config = config(&["pso", "nsga-ii", "nsga-iii"], 96); config.problem = ComponentSpec::new("vector");
        config.learning.reward = ComponentSpec::new("capture-credit");
        config.execution.mode = mode; config.execution.parallelism = 2;
        config.budget.slice_evaluations = 12; config.persistence.enabled = true;
        config.algorithms[0].scalarization = Some("weighted".into());
        config.algorithms[0].parameters.insert("inertia".into(), vec![ComponentSpec::new("constant")]);
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap();
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        run.run().unwrap(); resumed.run().unwrap();
        assert_eq!(run.transitions(), resumed.transitions());
        assert_eq!(run.result().evaluations, 96);
        assert!(run.result().archive.iter().all(|value| value.fitness.len() == 2));
        let contexts = contexts.lock().unwrap();
        assert!(contexts.iter().any(|context| context.action.starts_with("inertia/constant:1:")));
        let points: Vec<_> = contexts.iter().flat_map(|context| &context.candidates).collect();
        assert!(!points.is_empty());
        assert!(points.iter().all(|point| point.len() == 2 && point.iter().all(|value| (0.0..=0.4).contains(value))));
    }
}

fn adaptive_reward_specs() -> Vec<ComponentSpec> {
    vec![ComponentSpec::new("extreme-value").parameter("window", 3), ComponentSpec::new("dominance"),
        ComponentSpec::new("hypervolume-delta").parameter("calibrationObservations", 8),
        ComponentSpec::new("hypervolume-delta").parameter("reference", serde_json::json!([1.0]))
            .parameter("mode", "sampled").parameter("samples", 256)]
}

struct ScheduledCredit(ScheduledProblem);

impl SearchProblem<f64, f64> for ScheduledCredit {
    fn id(&self) -> &str { "scheduled-credit" }
    fn fingerprint(&self) -> &str { "scheduled-credit:1" }
    fn variable_order(&self) -> Vec<VariableId> { self.0.variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.0.lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.0.upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.0.initial_solutions(amount) }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { self.0.objective(solution) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn has_objective_coordinates(&self) -> bool { true }
    fn objective_coordinates(&self, value: &f64) -> crate::solver::heuristic::result::Result<Vec<f64>> { Ok(vec![value / 100.0]) }
    fn stateless(&self) -> bool { true }
}

fn adaptive_schedule_solver(reward: ComponentSpec, acceptance: &str, reverse: bool) -> HyperHeuristicSolver<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    let factories = Arc::new(AtomicU64::new(0));
    registry.register_problem("scheduled-credit", move |_, _| {
        let role = factories.fetch_add(1, Ordering::SeqCst) % 3;
        Ok(Box::new(ScheduledCredit(ScheduledProblem {
            inner: Quadratic { x: new_standalone_id(), calls: Arc::new(AtomicU64::new(0)), offset: 0.0, fingerprint: "scheduled-credit:1".into() },
            slow: role == if reverse { 2 } else { 1 }, fail: false, entered: None,
        })))
    }).unwrap();
    let mut config = non_stationary_config("choice-function", ExecutionMode::Asynchronous);
    config.problem = ComponentSpec::new("scheduled-credit"); config.budget.slice_evaluations = 8;
    config.learning.reward = reward; config.memory.acceptance = ComponentSpec::new(acceptance);
    HyperHeuristicSolver::new(config, registry).unwrap()
}

#[test]
fn adaptive_credit_and_acceptance_replay_opposite_async_order_and_complete_prefix_state() {
    for (reward, acceptance) in adaptive_reward_specs().into_iter().zip(["late", "auto", "record-to-record", "auto"]) {
        let solver = adaptive_schedule_solver(reward.clone(), acceptance, false);
        let mut run = solver.open().unwrap(); run.record_schedule().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let prefix = run.checkpoint().unwrap(); run.run().unwrap();
        let expected = run.checkpoint().unwrap(); let log = run.export_schedule().unwrap();
        let solver = adaptive_schedule_solver(reward.clone(), acceptance, true);
        let mut replay = solver.open().unwrap(); replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
        assert_deterministic_checkpoint_eq(&replay.checkpoint().unwrap(), &expected, &format!("adaptive/{}/{acceptance}", reward.r#type));
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&prefix), policy: None }).unwrap();
        resumed.replay_schedule(&log).unwrap(); resumed.run().unwrap();
        assert_deterministic_checkpoint_eq(&resumed.checkpoint().unwrap(), &expected, &format!("adaptive-prefix/{}/{acceptance}", reward.r#type));
        assert_eq!(resumed.export_schedule().unwrap(), log);
    }
}

#[test]
fn adaptive_rewards_and_acceptance_resume_full_both_level_state_without_extra_evaluations() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        for reward in adaptive_reward_specs() {
            for acceptance in ["late", "record-to-record", "auto"] {
                let calls = Arc::new(AtomicU64::new(0));
                let registry = credit_registry(calls.clone(), Arc::new(std::sync::Mutex::new(Vec::new())), true, false, false);
                let mut config = non_stationary_config("choice-function", mode);
                config.problem = ComponentSpec::new("credit");
                config.learning.reward = reward.clone(); config.memory.acceptance = ComponentSpec::new(acceptance);
                let solver = HyperHeuristicSolver::new(config, registry).unwrap();
                let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
                let saved = run.checkpoint().unwrap();
                assert!(saved.validate_kotlin_v1().is_err());
                let before_restore = calls.load(Ordering::Relaxed);
                let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
                assert_eq!(calls.load(Ordering::Relaxed), before_restore);
                run.run().unwrap(); resumed.run().unwrap();
                assert_eq!(run.result().evaluations, 128);
                assert_eq!(run.result().objective_calls, 128);
                assert_deterministic_checkpoint_eq(&resumed.checkpoint().unwrap(), &run.checkpoint().unwrap(),
                    &format!("{mode:?}/{}/{acceptance}", reward.r#type));
                let stateful_reward = reward.r#type != "dominance";
                assert_eq!(saved.components.contains_key("reward"), stateful_reward);
                assert!(saved.components.contains_key("acceptance"));
                for worker in saved.workers.values() {
                    assert_eq!(!worker.session.state["rewardState"].is_null(), stateful_reward);
                }
            }
        }
    }
}

fn local_update_spec(kind: &str) -> ComponentSpec {
    let mutation = ComponentSpec::new("mutation-stage").parameter("operator", serde_json::to_value(ComponentSpec::new("gaussian")).unwrap());
    match kind {
        "pipeline" | "vnd" | "elite-local" => ComponentSpec::new(kind).parameter("stages", serde_json::to_value(vec![mutation, ComponentSpec::new("opposition")]).unwrap()),
        "mutation-stage" => mutation,
        "neighborhood-stage" => ComponentSpec::new(kind).parameter("operator", serde_json::to_value(ComponentSpec::new("perturbation")).unwrap()),
        _ => ComponentSpec::new(kind),
    }
}

#[test]
fn local_updates_meter_every_intermediate_and_restore_complete_serial_and_synchronous_state() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        for kind in ["pipeline", "vnd", "elite-local", "mutation-stage", "neighborhood-stage", "pattern-search", "opposition"] {
            let calls = Arc::new(AtomicU64::new(0));
            let mut config = config(&["genetic", "genetic"], 256);
            config.execution.mode = mode; config.execution.parallelism = 2;
            config.persistence.enabled = true; config.budget.slice_evaluations = 48;
            config.selection = ComponentSpec::new("choice-function");
            for algorithm in &mut config.algorithms {
                algorithm.size = 2; algorithm.updates = vec![local_update_spec(kind)];
                algorithm.mutations = vec![ComponentSpec::new("none")];
                algorithm.selection = ComponentSpec::new("frr-mab");
            }
            let solver = HyperHeuristicSolver::new(config, registry(calls.clone())).unwrap();
            let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
            let saved = run.checkpoint().unwrap(); let before = calls.load(Ordering::Relaxed);
            let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
            assert_eq!(calls.load(Ordering::Relaxed), before);
            run.run().unwrap(); let after = calls.load(Ordering::Relaxed);
            assert_eq!(after, run.result().objective_calls);
            assert_eq!(run.result().evaluations, 256);
            assert_eq!(run.result().objective_calls, 256);
            resumed.run().unwrap();
            assert_eq!(calls.load(Ordering::Relaxed) - after, 256 - saved.objective_calls);
            assert_deterministic_checkpoint_eq(&run.checkpoint().unwrap(), &resumed.checkpoint().unwrap(), &format!("local/{kind}/{mode:?}"));
            assert!(run.transitions().iter().any(|event| event.scope.ends_with("/update")));
        }
    }
}

#[test]
fn local_vector_nsga_and_partial_steps_keep_original_objectives_and_actual_budget() {
    for kind in ["pipeline", "vnd", "elite-local", "pattern-search"] {
        let mut registry = SearchRegistry::with_multi_objective_builtins();
        registry.register_problem("vector", |_, _| Ok(Box::new(VectorProblem { x: new_standalone_id() }))).unwrap();
        let mut config = config(&["nsga-ii"], 97); config.problem = ComponentSpec::new("vector");
        config.persistence.enabled = true; config.budget.slice_evaluations = 20;
        config.algorithms[0].size = 2; config.algorithms[0].updates = vec![local_update_spec(kind)];
        config.algorithms[0].mutations = vec![ComponentSpec::new("none")];
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap();
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        run.run().unwrap(); resumed.run().unwrap();
        assert_eq!((run.result().evaluations, run.result().objective_calls), (97, 97), "{kind}");
        assert_deterministic_checkpoint_eq(&run.checkpoint().unwrap(), &resumed.checkpoint().unwrap(), kind);
        assert!(run.result().archive.iter().all(|entry| entry.fitness.len() == 2));
    }
}

#[test]
fn orthogonal_portfolios_preserve_small_niches_and_resume_native_state() {
    assert!(OrthogonalOperatorPortfolio::continuous(3).is_err());
    assert!(OrthogonalOperatorPortfolio::permutation(0).is_err());
    for permutation in [false, true] {
        let mut registry = registry(Arc::new(AtomicU64::new(0)));
        let mut config = config(&[], 240); config.persistence.enabled = true; config.budget.slice_evaluations = 24;
        config.algorithms = if permutation {
            registry.register_problem("permutation", |_, _| Ok(Box::new(PermutationProblem { order: (0..4).map(|_| new_standalone_id()).collect() }))).unwrap();
            config.problem = ComponentSpec::new("permutation");
            OrthogonalOperatorPortfolio::permutation(4).unwrap()
        } else { OrthogonalOperatorPortfolio::continuous(4).unwrap() };
        assert!(config.algorithms.iter().all(|algorithm| algorithm.updates.len() <= 2 && algorithm.mutations.len() <= 2));
        if !permutation { assert_eq!(config.algorithms.iter().map(|algorithm| algorithm.id.as_str()).collect::<Vec<_>>(), vec!["distribution", "adaptive-difference", "encircling", "escape", "intensification"]); }
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); for _ in 0..6 { run.advance().unwrap(); }
        let saved = run.checkpoint().unwrap();
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        run.run().unwrap(); resumed.run().unwrap();
        assert_eq!(run.result().evaluations, 240);
        assert_deterministic_checkpoint_eq(&run.checkpoint().unwrap(), &resumed.checkpoint().unwrap(), "orthogonal portfolio");
        if permutation {
            for entry in run.result().archive {
                let mut values: Vec<_> = entry.solution.values().copied().collect(); values.sort_by(f64::total_cmp);
                assert_eq!(values, vec![0.0, 1.0, 2.0, 3.0]);
            }
        }
    }
}

fn local_schedule_solver(kind: &str, reverse: bool) -> HyperHeuristicSolver<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    let factories = Arc::new(AtomicU64::new(0));
    registry.register_problem("scheduled", move |_, _| {
        let role = factories.fetch_add(1, Ordering::SeqCst) % 3;
        Ok(Box::new(ScheduledProblem { inner: Quadratic { x: new_standalone_id(), calls: Arc::new(AtomicU64::new(0)),
            offset: 0.0, fingerprint: "scheduled:1".into() }, slow: role == if reverse { 2 } else { 1 }, fail: false, entered: None }))
    }).unwrap();
    let mut config = config(&["genetic", "genetic"], 128);
    config.problem = ComponentSpec::new("scheduled"); config.execution.mode = ExecutionMode::Asynchronous;
    config.execution.parallelism = 2; config.persistence.enabled = true; config.budget.slice_evaluations = 16;
    config.selection = ComponentSpec::new("choice-function");
    for algorithm in &mut config.algorithms {
        algorithm.size = 2; algorithm.updates = vec![local_update_spec(kind)];
        algorithm.mutations = vec![ComponentSpec::new("none")]; algorithm.selection = ComponentSpec::new("adaptive-pursuit");
    }
    HyperHeuristicSolver::new(config, registry).unwrap()
}

#[test]
fn local_updates_replay_opposite_async_completion_order_and_quiescent_prefixes() {
    for kind in ["pipeline", "vnd", "elite-local", "pattern-search"] {
        let solver = local_schedule_solver(kind, false);
        let mut run = solver.open().unwrap(); run.record_schedule().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let prefix = run.checkpoint().unwrap(); run.run().unwrap();
        let expected = run.checkpoint().unwrap(); let log = run.export_schedule().unwrap();
        let solver = local_schedule_solver(kind, true);
        let mut replay = solver.open().unwrap(); replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
        assert_deterministic_checkpoint_eq(&replay.checkpoint().unwrap(), &expected, &format!("local-e4/{kind}"));
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&prefix), policy: None }).unwrap();
        resumed.replay_schedule(&log).unwrap(); resumed.run().unwrap();
        assert_deterministic_checkpoint_eq(&resumed.checkpoint().unwrap(), &expected, &format!("local-e4-prefix/{kind}"));
        assert_eq!(resumed.export_schedule().unwrap(), log);
    }
}

#[test]
fn local_composites_keep_native_constraint_ordering_and_restore_random_scan_state() {
    use crate::solver::heuristic::session::{ConstraintHandlingConfig, ConstraintMode};
    for mode in [ConstraintMode::FeasibilityFirst, ConstraintMode::Epsilon, ConstraintMode::StochasticRanking] {
        let registry = constraint_registry(Arc::new(std::sync::atomic::AtomicBool::new(false)));
        let mut config = config(&["genetic"], 160); config.persistence.enabled = true; config.budget.slice_evaluations = 32;
        config.algorithms[0].size = 2;
        config.algorithms[0].updates = ["pipeline", "vnd", "pattern-search"].map(local_update_spec).to_vec();
        config.algorithms[0].mutations = vec![ComponentSpec::new("none")];
        config.algorithms[0].constraints = Some(ConstraintHandlingConfig { mode, horizon: 5, ..Default::default() });
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap();
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        run.run().unwrap(); resumed.run().unwrap();
        assert_eq!(run.result().evaluations, 160);
        assert!(run.result().diagnostics.is_empty(), "{:?}", run.result().diagnostics);
        assert!(run.result().archive.iter().all(|entry| *entry.solution.values().next().unwrap() >= 0.0));
        assert_deterministic_checkpoint_eq(&run.checkpoint().unwrap(), &resumed.checkpoint().unwrap(), &format!("local-constraint/{mode:?}"));
    }
}

#[test]
fn built_in_time_rewards_reject_exact_replay_before_objective_calls_but_allow_normal_execution() {
    let calls = Arc::new(AtomicU64::new(0));
    let registry = credit_registry(calls.clone(), Arc::new(std::sync::Mutex::new(Vec::new())), true, false, false);
    let mut config = non_stationary_config("choice-function", ExecutionMode::Asynchronous);
    config.problem = ComponentSpec::new("credit"); config.learning.reward = ComponentSpec::new("time-normalized");
    let solver = HyperHeuristicSolver::new(config, registry).unwrap();
    let mut run = solver.open().unwrap(); assert!(run.record_schedule().is_err());
    let log = ScheduleLog { version: 1, configuration: String::new(), problem: String::new(), events: Vec::new(), checksum: String::new() };
    assert!(run.replay_schedule(&log).is_err()); assert_eq!(calls.load(Ordering::Relaxed), 0);
    run.run().unwrap(); assert_eq!(run.result().evaluations, 128);
    assert!(run.transitions().iter().all(|event| event.elapsed_nanos > 0 && event.reward.is_finite()));
}

#[test]
fn old_export_rejects_local_update_components_before_initialization_and_inside_children() {
    for kind in ["pattern-search", "opposition", "levy", "pipeline", "vnd", "elite-local", "mutation-stage", "neighborhood-stage"] {
        let mut config = config(&["genetic"], 16); config.persistence.enabled = true;
        let mut run = HyperHeuristicSolver::new(config, registry(Arc::new(AtomicU64::new(0)))).unwrap().open().unwrap();
        let saved = run.checkpoint().unwrap();
        for nested in [false, true] {
            let mut checkpoint = saved.clone();
            let mut algorithm = serde_json::json!({"id": "leaf", "algorithm": {"type": "genetic"}, "updates": [{"type": kind}]});
            if nested { algorithm = serde_json::json!({"id": "parent", "algorithm": {"type": "memetic"}, "children": [algorithm]}); }
            checkpoint.configuration_data["algorithms"] = serde_json::json!([algorithm]);
            let error = checkpoint.validate_kotlin_v1().unwrap_err().to_string();
            assert!(error.contains("local update components"), "{kind}/{nested}: {error}");
        }
    }
}

#[test]
fn old_export_rejects_adaptive_components_even_without_new_selector_or_inner_slots() {
    for kind in ["time-normalized", "extreme-value", "dominance", "hypervolume-delta", "late", "record-to-record", "auto"] {
        let registry = credit_registry(Arc::new(AtomicU64::new(0)), Arc::new(std::sync::Mutex::new(Vec::new())), true, false, false);
        let mut config = config(&["pso"], 16); config.problem = ComponentSpec::new("credit"); config.persistence.enabled = true;
        let acceptance = ["late", "record-to-record", "auto"].contains(&kind);
        if acceptance { config.memory.acceptance = ComponentSpec::new(kind); } else { config.learning.reward = ComponentSpec::new(kind); }
        let mut run = HyperHeuristicSolver::new(config, registry).unwrap().open().unwrap();
        let saved = run.checkpoint().unwrap();
        let error = saved.validate_kotlin_v1().unwrap_err().to_string();
        assert!(error.contains(if acceptance { "adaptive acceptance" } else { "adaptive rewards" }), "{kind}: {error}");
    }
}

#[test]
fn adaptive_capability_preflight_rejects_missing_coordinates_and_wrong_objective_modes() {
    for kind in ["time-normalized", "extreme-value", "hypervolume-delta"] {
        let calls = Arc::new(AtomicU64::new(0));
        let mut config = config(&["pso"], 16); config.learning.reward = ComponentSpec::new(kind);
        let solver = HyperHeuristicSolver::new(config, registry(calls.clone())).unwrap();
        assert!(solver.open().is_err()); assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
    for acceptance in ["late", "record-to-record"] {
        let mut registry = SearchRegistry::with_multi_objective_builtins();
        registry.register_problem("vector", |_, _| Ok(Box::new(CreditVector(VectorProblem { x: new_standalone_id() })))).unwrap();
        let mut config = config(&["nsga-ii"], 16); config.problem = ComponentSpec::new("vector");
        config.memory.acceptance = ComponentSpec::new(acceptance);
        assert!(HyperHeuristicSolver::new(config, registry).unwrap().open().is_err());
    }
    for reward in ["time-normalized", "extreme-value"] {
        let mut registry = SearchRegistry::with_multi_objective_builtins();
        registry.register_problem("vector", |_, _| Ok(Box::new(CreditVector(VectorProblem { x: new_standalone_id() })))).unwrap();
        let mut config = config(&["nsga-ii"], 16); config.problem = ComponentSpec::new("vector");
        config.learning.reward = ComponentSpec::new(reward);
        assert!(HyperHeuristicSolver::new(config, registry).unwrap().open().is_err());
    }
    let mut config = config(&["pso"], 16); config.memory.acceptance = ComponentSpec::new("record-to-record");
    let calls = Arc::new(AtomicU64::new(0));
    assert!(HyperHeuristicSolver::new(config, registry(calls.clone())).unwrap().open().is_err());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn vector_hv_and_auto_acceptance_keep_original_objectives_and_restore_frozen_geometry() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let mut registry = SearchRegistry::with_multi_objective_builtins();
        registry.register_problem("vector", |_, _| Ok(Box::new(CreditVector(VectorProblem { x: new_standalone_id() })))).unwrap();
        let mut config = config(&["nsga-ii", "nsga-iii"], 96); config.problem = ComponentSpec::new("vector");
        config.execution.mode = mode; config.execution.parallelism = 2; config.persistence.enabled = true;
        config.budget.slice_evaluations = 12; config.selection = ComponentSpec::new("adaptive-pursuit");
        config.learning.reward = ComponentSpec::new("hypervolume-delta").parameter("calibrationObservations", 4);
        config.memory.acceptance = ComponentSpec::new("auto");
        for algorithm in &mut config.algorithms { algorithm.updates = vec![ComponentSpec::new("single-point")]; }
        let solver = HyperHeuristicSolver::new(config, registry).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap();
        assert!(saved.components["reward"]["reference"].as_array().is_some_and(|value| value.len() == 2));
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        run.run().unwrap(); resumed.run().unwrap();
        assert_deterministic_checkpoint_eq(&resumed.checkpoint().unwrap(), &run.checkpoint().unwrap(), &format!("vector/{mode:?}"));
        assert!(run.result().archive.iter().all(|value| value.fitness.len() == 2));
    }
}
