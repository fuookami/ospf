use std::cell::Cell;
use std::rc::Rc;
use crate::solver::heuristic::operator::neighborhood::{RandomRuin, RegretKInsertRepair, RuinRecreateOperator};
use crate::solver::heuristic::runner::{EstimateState, RuinRecreateSearchSession, RuinRecreateSessionConfig};
use crate::solver::heuristic::session::codecs::IndividualCodec;
use crate::solver::heuristic::session::json::JsonValue;
use crate::solver::heuristic::result::Result;
use super::*;

struct Permutation {
    inner: PermutationProblem,
    calls: Arc<AtomicU64>,
    slow: bool,
}

impl SearchProblem<f64, f64> for Permutation {
    fn id(&self) -> &str { "ruin-permutation" }
    fn fingerprint(&self) -> &str { "ruin-permutation-v1" }
    fn variable_order(&self) -> Vec<VariableId> { self.inner.variable_order() }
    fn encoding(&self) -> crate::solver::heuristic::session::SearchEncoding { self.inner.encoding() }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.inner.initial_solutions(amount) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { self.inner.feasible(solution) }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.slow { std::thread::sleep(std::time::Duration::from_millis(2)); }
        self.inner.objective(solution)
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn stateless(&self) -> bool { true }
}

struct Estimates(Rc<Cell<u64>>);

impl EstimateState for Estimates {
    fn save(&self) -> Result<JsonValue> { Ok(self.0.get().into()) }
    fn load(&mut self, state: &JsonValue) -> Result<()> {
        self.0.set(state.as_u64().ok_or_else(|| invalid("bad estimate counter"))?); Ok(())
    }
}

fn domain_registry(calls: Arc<AtomicU64>, reverse: bool, recover_estimates: bool) -> SearchRegistry<f64, f64> {
    let mut registry = SearchRegistry::with_builtins();
    let factories = AtomicU64::new(0);
    registry.register_problem("ruin-permutation", move |_, _| {
        let role = factories.fetch_add(1, Ordering::Relaxed) % 3;
        Ok(Box::new(Permutation { inner: PermutationProblem { order: (0..4).map(|_| new_standalone_id()).collect() },
            calls: calls.clone(), slow: role == if reverse { 2 } else { 1 } }))
    }).unwrap();
    registry.register_algorithm("domain-ruin", move |context| {
        context.config.algorithm.validate_parameters(&["amount"])?;
        if !context.config.updates.is_empty() || !context.config.mutations.is_empty() || !context.config.neighborhoods.is_empty()
            || !context.config.parameters.is_empty() || context.config.scalarization.is_some() { return Err(invalid("unsupported domain ruin slots")); }
        let amount = usize::try_from(context.config.algorithm.integer("amount", 2)?).map_err(|_| invalid("ruin amount overflow"))?;
        let counter = Rc::new(Cell::new(0_u64));
        let observed = counter.clone();
        let operator = RuinRecreateOperator::new(RandomRuin, RegretKInsertRepair::new(move |_: &[f64], item: &f64, position| {
            observed.set(observed.get() + 1);
            Ok(Some(item * position as f64 + (observed.get() % 3) as f64))
        }, 2));
        let codec = Box::new(IndividualCodec::new(context.problem.variable_order()));
        let session = RuinRecreateSearchSession::new(context.problem, operator, codec,
            RuinRecreateSessionConfig { amount, random: SearchRandom::new(context.random.state() as u64), stateless_estimates: false })?;
        Ok(Box::new(if recover_estimates { session.with_estimate_state(Box::new(Estimates(counter))) } else { session }))
    }).unwrap();
    registry
}

fn domain_config(mode: ExecutionMode) -> HyperHeuristicConfig {
    let mut config = config(&["domain-ruin", "domain-ruin"], 64);
    config.problem = ComponentSpec::new("ruin-permutation"); config.execution.mode = mode;
    config.execution.parallelism = 2; config.persistence.enabled = true;
    config.selection = ComponentSpec::new("adaptive-pursuit");
    config.budget.slice_evaluations = 4; config.budget.slice_steps = 4;
    config
}

#[test]
fn domain_ruin_plugs_into_unchanged_coordinator_with_real_budgets_and_stateful_estimate_recovery() {
    for mode in [ExecutionMode::Serial, ExecutionMode::Synchronous] {
        let calls = Arc::new(AtomicU64::new(0));
        let solver = HyperHeuristicSolver::new(domain_config(mode), domain_registry(calls.clone(), false, true)).unwrap();
        let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
        let saved = run.checkpoint().unwrap(); let before = calls.load(Ordering::Relaxed);
        for worker in saved.workers.values() {
            assert!(worker.session.state.get("session").is_some());
            assert!(worker.session.state["slots"].as_object().unwrap().is_empty());
            if worker.initialized {
                assert!(!worker.session.state["localArchive"].as_array().unwrap().is_empty());
                assert!(worker.session.state["session"]["state"]["callbackState"].as_u64().unwrap() > 0);
            }
        }
        let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&saved), policy: None }).unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), before);
        run.run().unwrap(); assert_eq!(calls.load(Ordering::Relaxed), 64);
        resumed.run().unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 128 - saved.objective_calls);
        assert_eq!((run.result().evaluations, run.result().objective_calls), (64, 64));
        assert_deterministic_checkpoint_eq(&run.checkpoint().unwrap(), &resumed.checkpoint().unwrap(), &format!("domain-ruin/{mode:?}"));
        for entry in run.result().archive {
            let mut values: Vec<_> = entry.solution.values().copied().collect(); values.sort_by(f64::total_cmp);
            assert_eq!(values, vec![0.0, 1.0, 2.0, 3.0]);
        }
    }
}

#[test]
fn legacy_unwrapped_custom_checkpoints_keep_passthrough_state_without_fabricating_history() {
    let solver = HyperHeuristicSolver::new(domain_config(ExecutionMode::Serial),
        domain_registry(Arc::new(AtomicU64::new(0)), false, true)).unwrap();
    let mut run = solver.open().unwrap(); run.advance().unwrap(); run.advance().unwrap();
    let mut legacy = run.checkpoint().unwrap();
    for worker in legacy.workers.values_mut() {
        worker.session = serde_json::from_value(worker.session.state["session"].clone()).unwrap();
    }
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&legacy), policy: None }).unwrap();
    for worker in resumed.checkpoint().unwrap().workers.values() {
        assert!(worker.session.state.get("localArchive").is_none());
        assert!(worker.session.state.get("current").is_some());
    }
    run.run().unwrap(); resumed.run().unwrap();
    let expected = run.checkpoint().unwrap(); let actual = resumed.checkpoint().unwrap();
    assert_eq!(actual.archive, expected.archive);
    assert_eq!(actual.working, expected.working);
    assert_eq!(actual.policy, expected.policy);
    assert_eq!((actual.evaluations, actual.objective_calls, actual.steps), (expected.evaluations, expected.objective_calls, expected.steps));
    for (id, worker) in actual.workers {
        assert_eq!(worker.session.state, expected.workers[&id].session.state["session"]["state"]);
    }
}

#[test]
fn domain_ruin_e4_reexecutes_reversed_completion_order_and_restores_estimates_from_prefix() {
    let config = domain_config(ExecutionMode::Asynchronous);
    let solver = HyperHeuristicSolver::new(config.clone(), domain_registry(Arc::new(AtomicU64::new(0)), false, true)).unwrap();
    let mut run = solver.open().unwrap(); run.record_schedule().unwrap(); run.advance().unwrap(); run.advance().unwrap();
    let prefix = run.checkpoint().unwrap(); run.run().unwrap();
    let log = run.export_schedule().unwrap(); let expected = run.checkpoint().unwrap();
    let solver = HyperHeuristicSolver::new(config, domain_registry(Arc::new(AtomicU64::new(0)), true, true)).unwrap();
    let mut replay = solver.open().unwrap(); replay.replay_schedule(&log).unwrap(); replay.run().unwrap();
    assert_deterministic_checkpoint_eq(&replay.checkpoint().unwrap(), &expected, "domain-ruin/e4");
    let mut resumed = solver.open_with(RunOptions { checkpoint: Some(&prefix), policy: None }).unwrap();
    resumed.replay_schedule(&log).unwrap(); resumed.run().unwrap();
    assert_eq!(resumed.export_schedule().unwrap(), log);
    assert_deterministic_checkpoint_eq(&resumed.checkpoint().unwrap(), &expected, "domain-ruin/e4-prefix");
}

#[test]
fn domain_ruin_preflight_refuses_missing_estimate_recovery_and_invalid_amount_without_objectives() {
    let calls = Arc::new(AtomicU64::new(0));
    let solver = HyperHeuristicSolver::new(domain_config(ExecutionMode::Serial), domain_registry(calls.clone(), false, false)).unwrap();
    assert!(solver.open().is_err());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let mut config = domain_config(ExecutionMode::Serial); config.algorithms[0].algorithm = config.algorithms[0].algorithm.clone().parameter("amount", 5);
    let solver = HyperHeuristicSolver::new(config, domain_registry(calls.clone(), false, true)).unwrap();
    assert!(solver.open().is_err()); assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn domain_ruin_failed_estimate_keeps_prefix_in_global_memory_and_credits_failure_once() {
    let calls = Arc::new(AtomicU64::new(0));
    let mut registry = domain_registry(calls.clone(), false, true);
    registry.register_algorithm("failed-ruin", |context| {
        let codec = Box::new(IndividualCodec::new(context.problem.variable_order()));
        let operator = RuinRecreateOperator::new(RandomRuin, RegretKInsertRepair::new(|_: &[f64], _: &f64, _| Err(invalid("estimate failed")), 2));
        Ok(Box::new(RuinRecreateSearchSession::new(context.problem, operator, codec,
            RuinRecreateSessionConfig { amount: 2, random: SearchRandom::new(42), stateless_estimates: true })?))
    }).unwrap();
    let mut config = domain_config(ExecutionMode::Serial);
    config.algorithms = vec![AlgorithmConfig::new("ruin", "failed-ruin")];
    let mut run = HyperHeuristicSolver::new(config, registry).unwrap().open().unwrap();
    assert!(run.advance().is_err());
    let saved = run.checkpoint().unwrap();
    assert_eq!((saved.evaluations, saved.objective_calls, saved.steps), (1, 1, 1));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(run.result().archive.len(), 1);
    assert_eq!((saved.dispatched, saved.completed, saved.policy.root.revision), (1, 1, 1));
    assert_eq!(saved.transitions.len(), 1); assert!(saved.transitions[0].terminal);
    assert!(!saved.workers["ruin"].session.state["session"]["state"]["current"].is_null());
}
