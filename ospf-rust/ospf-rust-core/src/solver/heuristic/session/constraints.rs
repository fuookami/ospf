//! 原生约束排序及事务状态，不替换业务目标 / Native constraint ordering and transactional state without replacing objectives

use std::cell::RefCell;
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::session::budget::{SearchRandom, SearchSlice};
use crate::solver::heuristic::session::json::{JsonValue, invalid_state};
use crate::solver::heuristic::session::protocol::{Result, SearchEncoding, SearchEvaluationResult, SearchProblem};
use crate::variable::VariableId;

/// 约束排序模式 / Constraint ordering mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintMode {
    /// 严格可行优先 / Strict feasibility first
    #[default]
    FeasibilityFirst,
    /// 随完整提交步收紧容差 / Tighten tolerance with committed steps
    Epsilon,
    /// 独立随机流驱动有界批量扫描 / Bounded batch scans driven by an independent random stream
    StochasticRanking,
}

/// 原生搜索约束配置 / Native search constraint configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstraintHandlingConfig {
    /// 排序模式 / Ordering mode
    #[serde(skip_serializing_if = "default_mode")]
    pub mode: ConstraintMode,
    /// 初始容差；空值表示校准 / Initial tolerance; absent means calibration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_epsilon: Option<f64>,
    /// 容差归零的完整步数 / Committed steps until tolerance reaches zero
    #[serde(skip_serializing_if = "default_horizon")]
    pub horizon: u64,
    /// 收紧曲线指数 / Tightening exponent
    #[serde(skip_serializing_if = "default_exponent")]
    pub exponent: f64,
    /// 随机扫描按目标比较的概率 / Probability of objective comparison in random scans
    #[serde(skip_serializing_if = "default_probability")]
    pub objective_probability: f64,
}

fn default_mode(value: &ConstraintMode) -> bool { *value == ConstraintMode::FeasibilityFirst }
fn default_horizon(value: &u64) -> bool { *value == 1000 }
fn default_exponent(value: &f64) -> bool { *value == 2.0 }
fn default_probability(value: &f64) -> bool { *value == 0.45 }

impl Default for ConstraintHandlingConfig {
    fn default() -> Self {
        Self { mode: ConstraintMode::FeasibilityFirst, initial_epsilon: None,
            horizon: 1000, exponent: 2.0, objective_probability: 0.45 }
    }
}

impl ConstraintHandlingConfig {
    /// 校验数值和跨端整数范围 / Validate numbers and cross-runtime integer ranges
    pub fn validate(&self) -> Result<()> {
        if self.initial_epsilon.is_some_and(|value| !value.is_finite() || value < 0.0)
            || self.horizon == 0 || self.horizon > i64::MAX as u64
            || !self.exponent.is_finite() || self.exponent <= 0.0
            || !self.objective_probability.is_finite() || !(0.0..=1.0).contains(&self.objective_probability) {
            return Err(invalid_state("constraint configuration / 约束配置"));
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConstraintState {
    #[serde(default = "version", skip_serializing_if = "default_version")]
    version: u32,
    config: ConstraintHandlingConfig,
    steps: u64,
    scale: Option<f64>,
    random: i64,
}

fn version() -> u32 { 1 }
fn default_version(value: &u32) -> bool { *value == 1 }

#[derive(Clone, Copy)]
struct Reading { violation: f64, feasible: bool }

struct Runtime<V> {
    state: ConstraintState,
    readings: Vec<(Solution<V>, Reading)>,
    pending_maximum: f64,
    failure: Option<String>,
}

impl<V: HeuristicValue> Runtime<V> {
    fn begin(&mut self) {
        self.readings.clear();
        self.pending_maximum = 0.0;
        self.failure = None;
    }

    fn epsilon(&self) -> f64 {
        self.state.scale.unwrap_or(0.0) * libm::pow(
            (1.0 - self.state.steps as f64 / self.state.config.horizon as f64).clamp(0.0, 1.0),
            self.state.config.exponent,
        )
    }

    fn tier(&self, reading: Reading) -> (u8, f64) {
        let relaxed = self.state.config.mode == ConstraintMode::Epsilon;
        if reading.feasible || relaxed && self.epsilon() > 0.0 && reading.violation <= self.epsilon() {
            (0, 0.0)
        } else { (1, reading.violation) }
    }
}

/// 独占叶会话的约束视图；默认保留源问题的搜索禁区 / Constraint view owned by a leaf session, preserving source admission by default
///
/// 不会猜测 admissible 是否被覆盖。需放行默认不可行域时，显式覆盖 constraint_admissible。
/// Never guesses whether admissible was overridden; explicitly override constraint_admissible to relax the default domain.
pub struct ConstraintProblem<P, O, V: HeuristicValue> {
    inner: P,
    runtime: RefCell<Runtime<V>>,
    marker: std::marker::PhantomData<fn() -> O>,
}

impl<P, O: Clone, V: HeuristicValue> ConstraintProblem<P, O, V>
where
    P: SearchProblem<O, V>,
{
    /// 装配独立约束随机流，拒绝缺失能力和重复装配 / Attach an independent constraint stream, rejecting missing capability and duplicate attachment
    pub fn new(
        inner: P,
        config: ConstraintHandlingConfig,
        random: SearchRandom,
    ) -> Result<Self> {
        config.validate()?;
        if !inner.has_constraint_violation() || inner.has_constraints() {
            return Err(invalid_state("missing constraint callback or duplicate attachment / 缺少约束回调或重复装配"));
        }
        let scale = config.initial_epsilon;
        Ok(Self { inner, runtime: RefCell::new(Runtime {
            state: ConstraintState { version: 1, config, steps: 0, scale, random: random.state() as i64 },
            readings: Vec::new(), pending_maximum: 0.0, failure: None,
        }), marker: std::marker::PhantomData })
    }

    /// 当前有效容差 / Current effective tolerance
    pub fn epsilon(&self) -> f64 { self.runtime.borrow().epsilon() }

    fn reading(&self, solution: &Solution<V>) -> Result<Reading> {
        {
            let runtime = self.runtime.borrow();
            if let Some(failure) = &runtime.failure { return Err(invalid_state(failure)); }
            if let Some((_, reading)) = runtime.readings.iter().find(|(cached, _)| cached == solution) {
                return Ok(*reading);
            }
        }
        // 回调不持有 RefCell 借用；错误锁定本事务，不能被排序器静默忽略。 / Callbacks hold no RefCell borrow; errors latch for the transaction.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let violation = self.inner.checked_constraint_violation(solution)?;
            Ok(Reading { violation: if violation == 0.0 { 0.0 } else { violation }, feasible: self.inner.feasible(solution) })
        })).unwrap_or_else(|_| Err(invalid_state("constraint callback panicked / 约束回调异常")));
        let mut runtime = self.runtime.borrow_mut();
        match result {
            Ok(reading) => {
                runtime.pending_maximum = runtime.pending_maximum.max(reading.violation);
                runtime.readings.push((solution.clone(), reading));
                Ok(reading)
            }
            Err(error) => { runtime.failure = Some(error.to_string()); Err(error) }
        }
    }
}

impl<P, O: Clone, V: HeuristicValue> SearchProblem<O, V> for ConstraintProblem<P, O, V>
where
    P: SearchProblem<O, V>,
{
    fn id(&self) -> &str { self.inner.id() }
    fn fingerprint(&self) -> &str { self.inner.fingerprint() }
    fn variable_order(&self) -> Vec<VariableId> { self.inner.variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.inner.lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.inner.upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> { self.inner.initial_solutions(amount) }
    fn objective(&self, solution: &Solution<V>) -> Option<O> { self.inner.objective(solution) }
    fn dominates(&self, lhs: &O, rhs: &O) -> bool { self.inner.dominates(lhs, rhs) }
    fn equivalent(&self, lhs: &O, rhs: &O) -> bool { self.inner.equivalent(lhs, rhs) }
    fn valid_objective(&self, objective: &O) -> bool { self.inner.valid_objective(objective) }
    fn feasible(&self, solution: &Solution<V>) -> bool { self.inner.feasible(solution) }
    fn admissible(&self, solution: &Solution<V>) -> bool { self.inner.constraint_admissible(solution) }
    fn constraint_admissible(&self, solution: &Solution<V>) -> bool { self.admissible(solution) }
    fn has_objective_coordinates(&self) -> bool { self.inner.has_objective_coordinates() }
    fn objective_coordinates(&self, objective: &O) -> Result<Vec<f64>> { self.inner.objective_coordinates(objective) }
    fn checked_objective_coordinates(&self, objective: &O) -> Result<Vec<f64>> { self.inner.checked_objective_coordinates(objective) }
    fn has_constraint_violation(&self) -> bool { self.inner.has_constraint_violation() }
    fn constraint_violation(&self, solution: &Solution<V>) -> Result<f64> { self.inner.constraint_violation(solution) }
    fn checked_constraint_violation(&self, solution: &Solution<V>) -> Result<f64> { self.inner.checked_constraint_violation(solution) }
    fn multi_objective(&self) -> bool { self.inner.multi_objective() }
    fn valid_solution(&self, solution: &Solution<V>) -> bool { self.inner.valid_solution(solution) }
    fn encoding(&self) -> SearchEncoding { self.inner.encoding() }
    fn flush(&mut self) { self.inner.flush(); }
    fn stateless(&self) -> bool { self.inner.stateless() }
    fn save_state(&self) -> Result<JsonValue> { self.inner.save_state() }
    fn load_state(&mut self, state: &JsonValue) -> Result<()> { self.inner.load_state(state) }
    fn evaluate(&self, solution: Solution<V>, slice: &SearchSlice) -> Result<SearchEvaluationResult<O, V>> {
        if !slice.acquire_shared() { return Ok(SearchEvaluationResult { admitted: false, candidate: None }); }
        if !self.valid_solution(&solution) || !self.admissible(&solution) {
            return Ok(SearchEvaluationResult { admitted: true, candidate: None });
        }
        self.observe_constraints(&solution)?;
        let reserved = slice.with_prepaid_request();
        self.inner.evaluate_constraint_admissible(solution, &reserved)
    }
    fn has_constraints(&self) -> bool { true }
    fn stochastic_constraints(&self) -> bool { self.runtime.borrow().state.config.mode == ConstraintMode::StochasticRanking }
    fn observe_constraints(&self, solution: &Solution<V>) -> Result<()> { self.reading(solution).map(|_| ()) }
    fn constraint_failure(&self) -> Option<String> { self.runtime.borrow().failure.clone() }
    fn constraint_tier(&self, solution: &Solution<V>) -> Option<(u8, f64)> {
        self.reading(solution).ok().map(|reading| self.runtime.borrow().tier(reading))
    }
    fn begin_constraints(&self) { self.runtime.borrow_mut().begin(); }
    fn commit_constraints(&self) -> Result<()> {
        let mut runtime = self.runtime.borrow_mut();
        if let Some(failure) = &runtime.failure { return Err(invalid_state(failure)); }
        if runtime.state.steps == i64::MAX as u64 { return Err(invalid_state("constraint step overflow / 约束步数溢出")); }
        if runtime.state.scale.is_none() { runtime.state.scale = Some(runtime.pending_maximum); }
        runtime.state.steps += 1;
        runtime.readings.clear();
        Ok(())
    }
    fn save_constraints(&self) -> Result<JsonValue> {
        serde_json::to_value(&self.runtime.borrow().state).map_err(invalid_state)
    }
    fn load_constraints(&self, value: &JsonValue) -> Result<()> {
        let state: ConstraintState = serde_json::from_value(value.clone()).map_err(invalid_state)?;
        let mut runtime = self.runtime.borrow_mut();
        if state.version != 1 || state.config != runtime.state.config || state.steps > i64::MAX as u64
            || state.scale.is_some_and(|value| !value.is_finite() || value < 0.0)
            || state.config.initial_epsilon.is_some() && state.scale != state.config.initial_epsilon
            || state.steps > 0 && state.scale.is_none() {
            return Err(invalid_state("incompatible constraint state / 不兼容的约束状态"));
        }
        runtime.state = state;
        runtime.begin();
        Ok(())
    }
    fn rank_constraints(
        &self,
        solutions: &[Solution<V>],
        objective: &dyn Fn(usize, usize) -> Ordering,
    ) -> Option<Vec<usize>> {
        if !self.stochastic_constraints() { return None; }
        let mut ranked: Vec<usize> = (0..solutions.len()).collect();
        let mut random = SearchRandom::new(self.runtime.borrow().state.random as u64);
        let probability = self.runtime.borrow().state.config.objective_probability;
        for _ in 0..ranked.len() {
            let mut swapped = false;
            for index in 0..ranked.len().saturating_sub(1) {
                let a = ranked[index];
                let b = ranked[index + 1];
                let (Ok(left), Ok(right)) = (self.reading(&solutions[a]), self.reading(&solutions[b])) else {
                    return Some((0..solutions.len()).collect());
                };
                let by_objective = left.feasible && right.feasible || random.next_unit() < probability;
                let compared = if by_objective { objective(a, b) } else {
                    let runtime = self.runtime.borrow();
                    let lhs = runtime.tier(left);
                    let rhs = runtime.tier(right);
                    lhs.0.cmp(&rhs.0).then_with(|| lhs.1.total_cmp(&rhs.1)).then_with(|| objective(a, b))
                };
                if compared == Ordering::Greater { ranked.swap(index, index + 1); swapped = true; }
            }
            if !swapped { break; }
        }
        self.runtime.borrow_mut().state.random = random.state() as i64;
        Some(ranked)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::solver::heuristic::individual::Individual;
    use crate::solver::heuristic::session::{SearchBudget, SearchSlice};
    use crate::variable::new_standalone_id;
    use super::*;

    struct Problem { x: VariableId, violation: f64, relax: bool }

    impl SearchProblem<f64, f64> for Problem {
        fn id(&self) -> &str { "constraint-test" }
        fn fingerprint(&self) -> &str { "constraint-test:1" }
        fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
        fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
            (0..amount).map(|index| Solution::from([(self.x, index as f64 - 2.0)])).collect()
        }
        fn objective(&self, solution: &Solution<f64>) -> Option<f64> { Some(solution[&self.x]) }
        fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
        fn feasible(&self, solution: &Solution<f64>) -> bool { solution[&self.x] >= 0.0 }
        fn constraint_admissible(&self, solution: &Solution<f64>) -> bool {
            if self.relax { solution[&self.x] >= -5.0 } else { self.admissible(solution) }
        }
        fn has_constraint_violation(&self) -> bool { true }
        fn constraint_violation(&self, solution: &Solution<f64>) -> Result<f64> {
            Ok(if self.violation.is_nan() { f64::NAN } else { (-solution[&self.x]).max(0.0) * self.violation })
        }
    }

    fn problem(config: ConstraintHandlingConfig) -> ConstraintProblem<Problem, f64, f64> {
        ConstraintProblem::new(Problem { x: new_standalone_id(), violation: 1.0, relax: true }, config, SearchRandom::new(42)).unwrap()
    }

    fn individual(problem: &impl SearchProblem<f64, f64>, value: f64) -> Individual<f64, f64> {
        Individual::new(Solution::from([(problem.variable_order()[0], value)]), value)
    }

    #[test]
    fn epsilon_matches_shared_strictmath_binary64_vectors_and_keeps_required_state() {
        let problem = problem(ConstraintHandlingConfig {
            mode: ConstraintMode::Epsilon, initial_epsilon: Some(0.75), horizon: 10001,
            exponent: 1.37, ..ConstraintHandlingConfig::default()
        });
        for (steps, expected) in [(1283, 0x3fe3e28720023000), (1284, 0x3fe3e1ba572774e1), (2028, 0x3fe1982836c8e2ec)] {
            let mut state = problem.save_constraints().unwrap(); state["steps"] = steps.into();
            problem.load_constraints(&state).unwrap();
            assert_eq!(problem.epsilon().to_bits(), expected);
        }
        let problem = self::problem(ConstraintHandlingConfig::default());
        assert_eq!(problem.save_constraints().unwrap(), serde_json::json!({"config": {}, "steps": 0, "scale": null, "random": 42}));
    }

    #[test]
    fn historical_archive_does_not_consume_stochastic_population_ranking_stream() {
        let problem = problem(ConstraintHandlingConfig {
            mode: ConstraintMode::StochasticRanking, objective_probability: 1.0, ..ConstraintHandlingConfig::default()
        });
        let before = problem.save_constraints().unwrap();
        let mut archive = crate::solver::heuristic::session::archive::SessionArchive::new(3);
        archive.update(&problem, vec![individual(&problem, -2.0), individual(&problem, 1.0), individual(&problem, -1.0)]);
        assert_eq!(archive.items().iter().map(|entry| entry.fitness).collect::<Vec<_>>(), vec![1.0, -1.0, -2.0]);
        assert_eq!(problem.save_constraints().unwrap(), before);
        assert_eq!(problem.rank_indices(archive.items()), vec![2, 1, 0]);
        assert_ne!(problem.save_constraints().unwrap()["random"], before["random"]);
    }

    #[test]
    fn zero_violation_does_not_prove_strict_feasibility_or_replace_objectives() {
        let source = Problem { x: new_standalone_id(), violation: 0.0, relax: true };
        let problem = ConstraintProblem::new(source, ConstraintHandlingConfig::default(), SearchRandom::new(1)).unwrap();
        let infeasible = individual(&problem, -1.0);
        let feasible = individual(&problem, 1.0);
        assert_eq!(problem.compare_individuals(&infeasible, &feasible), Ordering::Greater);
        assert_eq!(problem.objective(&infeasible.solution), Some(-1.0));
        assert!(!problem.feasible(&infeasible.solution));
    }

    #[test]
    fn default_admission_is_conservative_and_explicit_relaxation_preserves_exclusions() {
        let source = Problem { x: new_standalone_id(), violation: 1.0, relax: false };
        let strict = ConstraintProblem::new(source, ConstraintHandlingConfig::default(), SearchRandom::new(1)).unwrap();
        assert!(!strict.admissible(&individual(&strict, -1.0).solution));
        let relaxed = problem(ConstraintHandlingConfig::default());
        assert!(relaxed.admissible(&individual(&relaxed, -1.0).solution));
        assert!(!relaxed.admissible(&individual(&relaxed, -6.0).solution));
    }

    #[test]
    fn epsilon_calibrates_only_on_commit_and_is_zero_at_horizon() {
        let problem = problem(ConstraintHandlingConfig { mode: ConstraintMode::Epsilon, horizon: 4, ..Default::default() });
        let before = problem.save_constraints().unwrap();
        problem.begin_constraints();
        problem.observe_constraints(&individual(&problem, -4.0).solution).unwrap();
        assert_eq!(problem.epsilon(), 0.0);
        problem.load_constraints(&before).unwrap();
        assert_eq!(problem.save_constraints().unwrap(), before);
        problem.observe_constraints(&individual(&problem, -2.0).solution).unwrap();
        problem.commit_constraints().unwrap();
        assert_eq!(problem.epsilon(), 2.0 * 0.75_f64.powi(2));
        for _ in 0..3 {
            problem.begin_constraints();
            problem.observe_constraints(&individual(&problem, -5.0).solution).unwrap();
            problem.commit_constraints().unwrap();
        }
        assert_eq!(problem.epsilon(), 0.0);
        assert_eq!(problem.save_constraints().unwrap()["scale"], 2.0);
    }

    #[test]
    fn explicit_epsilon_relaxes_only_with_positive_tolerance() {
        for epsilon in [0.0, 2.0] {
            let problem = problem(ConstraintHandlingConfig { mode: ConstraintMode::Epsilon, initial_epsilon: Some(epsilon), ..Default::default() });
            let compared = problem.compare_individuals(&individual(&problem, -1.0), &individual(&problem, 1.0));
            assert_eq!(compared, if epsilon > 0.0 { Ordering::Less } else { Ordering::Greater });
        }
    }

    #[test]
    fn stochastic_ranking_is_bounded_restorable_and_pair_comparisons_do_not_draw() {
        let problem = problem(ConstraintHandlingConfig { mode: ConstraintMode::StochasticRanking, ..Default::default() });
        let entries: Vec<_> = [-4.0, -3.0, 2.0, 1.0, -1.0].into_iter().map(|value| individual(&problem, value)).collect();
        let before = problem.save_constraints().unwrap();
        problem.compare_individuals(&entries[0], &entries[1]);
        assert_eq!(problem.save_constraints().unwrap(), before);
        let ranked = problem.rank_indices(&entries);
        let after = problem.save_constraints().unwrap();
        assert_ne!(before["random"], after["random"]);
        problem.load_constraints(&before).unwrap();
        assert_eq!(problem.rank_indices(&entries), ranked);
        assert_eq!(problem.save_constraints().unwrap(), after);
        assert_eq!(problem.rank_indices(&[]), Vec::<usize>::new());
    }

    #[test]
    fn ranking_probability_extremes_respect_objective_and_constraint_modes() {
        for probability in [0.0, 1.0] {
            let problem = problem(ConstraintHandlingConfig { mode: ConstraintMode::StochasticRanking,
                objective_probability: probability, ..Default::default() });
            let entries: Vec<_> = [-2.0, 1.0, -1.0].into_iter().map(|value| individual(&problem, value)).collect();
            assert_eq!(problem.rank_indices(&entries), if probability == 0.0 { vec![1, 2, 0] } else { vec![0, 2, 1] });
        }
    }

    #[test]
    fn invalid_reading_charges_request_but_not_objective_and_latches_failure() {
        for violation in [f64::NAN, -1.0] {
            let source = Problem { x: new_standalone_id(), violation, relax: true };
            let problem = ConstraintProblem::new(source, ConstraintHandlingConfig::default(), SearchRandom::new(0)).unwrap();
            let budget = Arc::new(SearchBudget::new(10));
            let slice = SearchSlice::new(budget.clone(), 10, 1);
            assert!(problem.evaluate(individual(&problem, -1.0).solution, &slice).is_err());
            assert_eq!(budget.used(), 1);
            assert_eq!(budget.objective_calls(), 0);
            assert!(problem.constraint_failure().is_some());
            assert!(problem.commit_constraints().is_err());
        }
    }

    #[test]
    fn constraint_restore_rejects_bad_versions_counts_scales_and_definition() {
        let problem = problem(ConstraintHandlingConfig::default());
        let before = problem.save_constraints().unwrap();
        for (key, value) in [("version", serde_json::json!(2)), ("steps", serde_json::json!(-1)),
            ("scale", serde_json::json!(-1)), ("config", serde_json::json!({"horizon":2}))] {
            let mut bad = before.clone(); bad[key] = value;
            assert!(problem.load_constraints(&bad).is_err());
            assert_eq!(problem.save_constraints().unwrap(), before);
        }
        let mut bad = before.clone(); bad["steps"] = 1.into();
        assert!(problem.load_constraints(&bad).is_err());
        assert!(ConstraintProblem::new(problem, ConstraintHandlingConfig::default(), SearchRandom::new(0)).is_err());
    }

    #[test]
    fn invalid_constraint_configuration_is_rejected() {
        for config in [
            ConstraintHandlingConfig { horizon: 0, ..Default::default() },
            ConstraintHandlingConfig { exponent: 0.0, ..Default::default() },
            ConstraintHandlingConfig { initial_epsilon: Some(f64::INFINITY), ..Default::default() },
            ConstraintHandlingConfig { objective_probability: 1.1, ..Default::default() },
        ] { assert!(config.validate().is_err()); }
    }
}
