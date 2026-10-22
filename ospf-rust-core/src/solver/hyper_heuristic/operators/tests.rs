//! 槽位信用生命周期回归 / Slot-credit lifecycle regressions

use std::sync::atomic::{AtomicU64, Ordering};
use crate::solver::heuristic::session::adapters::ProblemModel;
use crate::solver::hyper_heuristic::components::ComponentRegistry;
use crate::variable::{VariableId, new_standalone_id};
use super::*;

struct Problem { x: VariableId }

impl SearchProblem<f64, f64> for Problem {
    fn id(&self) -> &str { "slot-test" }
    fn fingerprint(&self) -> &str { "slot-test:1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
    fn initial_solutions(&self, _: usize) -> Vec<Solution<f64>> { vec![Solution::from([(self.x, 1.0)])] }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { Some(solution[&self.x]) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn stateless(&self) -> bool { true }
}

struct FailingChoice { inner: NonStationarySelector, fail: bool, discards: Arc<AtomicU64> }

impl SelectionPolicy for FailingChoice {
    fn revision(&self) -> u64 { self.inner.revision() }
    fn select(
        &mut self,
        context: &SelectionContext,
        available: &[String],
        random: &mut SearchRandom,
    ) -> Result<String> {
        let action = self.inner.select(context, available, random)?;
        if self.fail { panic!("selection failed after reserving credit"); }
        Ok(action)
    }
    fn observe(&mut self, event: &SelectionTransition) -> Result<()> { self.inner.observe(event) }
    fn snapshot(&self) -> Result<PolicySnapshot> { self.inner.snapshot() }
    fn restore(&mut self, snapshot: &PolicySnapshot) -> Result<()> { self.inner.restore(snapshot) }
    fn discard(&mut self, decision: u64) { self.discards.fetch_add(1, Ordering::Relaxed); self.inner.discard(decision); }
    fn predecessor(&self, decision: u64) -> Option<String> { self.inner.predecessor(decision) }
}

fn session(
    kind: &str,
    invoke_update: bool,
    selectors: &BTreeMap<String, SelectorFactory>,
) -> Box<dyn PolicySession<f64, f64>> {
    let problem = ProblemHandle::new(Box::new(Problem { x: new_standalone_id() }));
    let order = problem.variable_order();
    let mut config = AlgorithmConfig::new("local", "genetic");
    config.selection = ComponentSpec::new(kind);
    config.updates = vec![ComponentSpec::new("single-point")];
    config.mutations = vec![ComponentSpec::new("none")];
    let bindings = OperatorBindings::new(&config, &SearchRandom::new(42), true, &OperatorRegistry::new(), selectors, SearchEncoding::Continuous).unwrap();
    let update = bindings.update(Box::new(SinglePointCrossover));
    let hooks = HookedSkeleton::new(
        |problem: &ProblemHandle<f64, f64>, slice| {
            Ok(problem.initial_solutions(1).into_iter().filter_map(|solution| problem.evaluate(solution, slice).unwrap().candidate).collect())
        },
        move |problem: &mut ProblemHandle<f64, f64>, iteration, state: &Vec<Individual<f64, f64>>, slice| {
            let solutions = if invoke_update {
                update.update(&ProblemModel::new(problem), iteration, &[state[0].solution.clone()], &mut SearchRandom::new(42))
            } else { vec![state[0].solution.clone()] };
            Ok(solutions.into_iter().filter_map(|solution| problem.evaluate(solution, slice).unwrap().candidate).collect())
        },
        |state: &Vec<Individual<f64, f64>>| state.clone(),
    );
    let codecs = SessionCodecs::new(Some(Box::new(PopulationStateCodec::new(order.clone()))), Some(Box::new(IndividualListCodec::new(order.clone()))));
    let native = RunnerSession::new(Box::new(hooks), problem.clone(), codecs);
    let reward = ComponentRegistry::<f64, f64>::new().reward(&ComponentSpec::new("improvement")).unwrap();
    ControlledSession::wrap(Box::new(native), problem, bindings, Box::new(IndividualListCodec::new(order)), reward)
}

#[test]
fn uninvoked_slot_choices_are_discarded_without_observation() {
    for kind in ["frr-mab", "choice-function", "adaptive-pursuit"] {
        let mut session = session(kind, false, &BTreeMap::new());
        let result = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 10, 2)).unwrap();
        assert_eq!(result.steps, 2);
        assert!(session.drain_transitions().is_empty());
        for policy in session.policies().unwrap().values() {
            assert_eq!(policy.revision, 0);
            assert!(policy.extension["pending"].as_object().unwrap().is_empty());
        }
    }
}

#[test]
fn only_invoked_slots_receive_one_observation_per_actual_step() {
    let mut session = session("choice-function", true, &BTreeMap::new());
    let result = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 10, 2)).unwrap();
    assert_eq!(result.steps, 2);
    let policies = session.policies().unwrap();
    assert_eq!(policies["update"].revision, 1);
    assert_eq!(policies["mutation"].revision, 0);
    assert!(policies.values().all(|policy| policy.extension["pending"].as_object().unwrap().is_empty()));
    let events = session.drain_transitions();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].scope, "update");
    assert_eq!(events[0].decision, 1);
}

#[test]
fn exact_shared_budget_exhaustion_does_not_terminate_committed_slot_credit() {
    let mut session = session("choice-function", true, &BTreeMap::new());
    session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(10)), 10, 1)).unwrap();
    let budget = Arc::new(SearchBudget::new(2));
    let result = session.step(&mut SearchSlice::new(budget.clone(), 2, 1)).unwrap();
    assert_eq!(result.steps, 1);
    assert_eq!(budget.remaining(), 0);
    let event = session.drain_transitions().pop().unwrap();
    assert!(!event.terminal);
    assert!(session.policies().unwrap()["update"].extension["previous"].is_string());
}

#[test]
fn partially_selected_slots_are_discarded_after_selector_panics() {
    let counter = Arc::new(AtomicU64::new(0));
    let discards = Arc::new(AtomicU64::new(0));
    let observed = discards.clone();
    let mut selectors: BTreeMap<String, SelectorFactory> = BTreeMap::new();
    selectors.insert("fallible-choice".into(), Arc::new(move |_, actions, learning| {
        Ok(Box::new(FailingChoice { inner: NonStationarySelector::new(&ComponentSpec::new("choice-function"), actions, learning)?,
            fail: counter.fetch_add(1, Ordering::Relaxed) == 1, discards: observed.clone() }))
    }));
    let mut session = session("fallible-choice", false, &selectors);
    let result = session.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(100)), 10, 2)).unwrap();
    assert_eq!(result.steps, 1);
    assert_eq!(result.termination, SearchTermination::Failed);
    assert!(result.diagnostics.iter().any(|message| message.contains("panicked")));
    assert!(discards.load(Ordering::Relaxed) >= 2);
    assert!(session.drain_transitions().is_empty());
    assert!(session.policies().unwrap().values().all(|policy| policy.extension["pending"].as_object().unwrap().is_empty()));
}
