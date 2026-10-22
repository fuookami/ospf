//! 骨架会话适配 / Session adapters for skeletons
//!
//! 通用会话只认识 `SessionHooks`，本模块把骨架包装成钩子，从而让种群与多目标（NSGA）骨架都能被同一份会话驱动。
//! The generic session only knows `SessionHooks`; this module wraps skeletons into those hooks so population and
//! multi-objective (NSGA) skeletons both run under one session.

use std::cell::RefCell;
use crate::model::callback::Solution;
use crate::variable::VariableId;

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, LocalEvaluation, ObjectiveModel, VariableModel, to_f64};
use crate::solver::heuristic::operator::multi_objective::{
    CrowdingDistance, Dominance, FastNonDominatedSort, ReferencePoints,
};
use crate::solver::heuristic::result::HeuristicError;
use crate::solver::heuristic::runner::population_runner::{Population, PopulationRunner};
use crate::solver::heuristic::session::budget::SearchSlice;
use crate::solver::heuristic::session::protocol::{Result, SearchProblem};
use crate::solver::heuristic::session::runner_session::SessionHooks;

/// 单目标问题适配器：实现骨架所需的 `ObjectiveModel<V, V>` / Single-objective adapter implementing the skeleton's `ObjectiveModel<V, V>`
///
/// 骨架固定要求 `ObjectiveModel<V, V>`，因此单目标与多目标各有一个适配器；两者共享同一份变量能力代码（宏展开），
/// 以避免 Rust 中无法表达「`O != V`」的负约束问题。
/// Skeletons require `ObjectiveModel<V, V>`, so single- and multi-objective problems get separate adapters; both share the same
/// variable-capability code (via a macro) because Rust cannot express the negative bound "`O != V`".
pub struct ProblemModel<'p, P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    problem: &'p P,
    meter: Option<(&'p SearchSlice, &'p RefCell<Vec<Individual<f64, V>>>)>,
    errors: Option<&'p RefCell<Vec<String>>>,
    order: Vec<VariableId>,
    marker: std::marker::PhantomData<fn() -> V>,
}

impl<'p, P, V> ProblemModel<'p, P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    /// 以问题创建适配器 / Create an adapter from a problem
    pub fn new(problem: &'p P) -> Self {
        Self {
            order: problem.variable_order(),
            problem,
            meter: None,
            errors: None,
            marker: std::marker::PhantomData,
        }
    }
}

impl<'p, P, V> ProblemModel<'p, P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    /// 以预算和候选收集器创建 / Create with a budget and candidate collector
    pub fn metered(
        problem: &'p P,
        slice: &'p SearchSlice,
        candidates: &'p RefCell<Vec<Individual<f64, V>>>,
    ) -> Self {
        Self { meter: Some((slice, candidates)), ..Self::new(problem) }
    }

    /// 收集统一评估返回的错误，交由会话边界传播 / Collect metered evaluation errors for propagation at the session boundary
    pub fn with_errors(mut self, errors: &'p RefCell<Vec<String>>) -> Self {
        self.errors = Some(errors);
        self
    }
}
impl<'p, P, V> VariableModel<V> for ProblemModel<'p, P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    fn local_evaluation(&self) -> Option<&dyn LocalEvaluation<V>> { self.meter.is_some().then_some(self) }

    fn num_variables(&self) -> usize {
        self.order.len()
    }

    fn variable_order(&self) -> Vec<VariableId> {
        self.order.clone()
    }

    fn lower_bound(&self, index: usize) -> Option<f64> {
        self.problem.lower_bound(index)
    }

    fn upper_bound(&self, index: usize) -> Option<f64> {
        self.problem.upper_bound(index)
    }

    fn coerce_in(&self, _index: usize, value: V) -> V {
        value
    }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> {
        self.problem.initial_solutions(amount)
    }

    fn constraint_satisfied(&self, solution: &Solution<V>) -> Option<bool> {
        Some(self.problem.valid_solution(solution) && self.problem.admissible(solution))
    }
}

impl<P: SearchProblem<f64, V>, V: HeuristicValue> LocalEvaluation<V> for ProblemModel<'_, P, V> {
    fn remaining(&self) -> u64 { self.meter.map_or(0, |(slice, _)| slice.remaining()) }

    fn evaluate(&self, solution: &Solution<V>) -> Option<usize> {
        if self.errors.is_some_and(|errors| !errors.borrow().is_empty()) { return None; }
        let (slice, candidates) = self.meter?;
        let evaluated = match self.problem.evaluate(solution.clone(), slice) {
            Ok(result) => result.candidate?,
            Err(error) => { if let Some(errors) = self.errors { errors.borrow_mut().push(error.to_string()); } return None; }
        };
        if !evaluated.fitness.is_finite() { return None; }
        let mut entries = candidates.borrow_mut();
        let key = entries.len(); entries.push(evaluated);
        Some(key)
    }

    fn compare(&self, lhs: usize, rhs: usize) -> Option<std::cmp::Ordering> {
        let entries = self.meter?.1.borrow();
        Some(self.problem.compare_individuals(entries.get(lhs)?, entries.get(rhs)?))
    }

    fn rank(&self, indices: &[usize]) -> Option<Vec<usize>> {
        let entries = self.meter?.1.borrow();
        let values = indices.iter().map(|index| entries.get(*index).cloned()).collect::<Option<Vec<_>>>()?;
        Some(self.problem.rank_indices(&values))
    }
}

impl<'p, P, V> ObjectiveModel<V, V> for ProblemModel<'p, P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    fn default_objective(&self) -> Option<V> {
        None
    }

    fn objective(&self, solution: &Solution<V>) -> Option<V> {
        if self.errors.is_some_and(|errors| !errors.borrow().is_empty()) { return None; }
        let objective = if let Some((_, candidates)) = self.meter {
            let key = LocalEvaluation::evaluate(self, solution)?;
            Some(candidates.borrow().get(key)?.fitness)
        } else {
            self.problem.objective(solution)
        };
        objective.map(crate::solver::heuristic::model::from_f64::<V>)
    }

    fn compare_objective(&self, lhs: &V, rhs: &V) -> Option<std::cmp::Ordering> {
        let left = to_f64(lhs);
        let right = to_f64(rhs);
        if self.problem.dominates(&left, &right) {
            Some(std::cmp::Ordering::Less)
        } else if self.problem.dominates(&right, &left) {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }

    fn compare_individuals(&self, lhs: &Individual<V, V>, rhs: &Individual<V, V>) -> std::cmp::Ordering {
        self.problem.compare_individuals(
            &Individual::new(lhs.solution.clone(), to_f64(&lhs.fitness)),
            &Individual::new(rhs.solution.clone(), to_f64(&rhs.fitness)),
        )
    }

    fn rank_indices(&self, entries: &[Individual<V, V>]) -> Vec<usize> {
        let entries: Vec<_> = entries.iter().map(|entry| Individual::new(entry.solution.clone(), to_f64(&entry.fitness))).collect();
        self.problem.rank_indices(&entries)
    }

    fn stochastic_constraints(&self) -> bool { self.problem.stochastic_constraints() }

    fn compare_constraint_tiers(&self, lhs: &Solution<V>, rhs: &Solution<V>) -> std::cmp::Ordering {
        match (self.problem.constraint_tier(lhs), self.problem.constraint_tier(rhs)) {
            (Some(a), Some(b)) => a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)),
            _ => std::cmp::Ordering::Equal,
        }
    }
}

/// 多目标问题适配器：实现 `ObjectiveModel<Vec<f64>, V>` / Multi-objective adapter implementing `ObjectiveModel<Vec<f64>, V>`
pub struct MultiObjectiveProblemModel<'p, P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    problem: &'p P,
    meter: Option<(&'p SearchSlice, &'p RefCell<Vec<Individual<Vec<f64>, V>>>)>,
    errors: Option<&'p RefCell<Vec<String>>>,
    order: Vec<VariableId>,
    marker: std::marker::PhantomData<fn() -> V>,
}

impl<'p, P, V> MultiObjectiveProblemModel<'p, P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    /// 以问题创建适配器 / Create an adapter from a problem
    pub fn new(problem: &'p P) -> Self {
        Self {
            order: problem.variable_order(),
            problem,
            meter: None,
            errors: None,
            marker: std::marker::PhantomData,
        }
    }

    /// 共享完整向量目标的计费账本 / Share a metered ledger retaining complete vector objectives
    pub fn metered(
        problem: &'p P,
        slice: &'p SearchSlice,
        candidates: &'p RefCell<Vec<Individual<Vec<f64>, V>>>,
    ) -> Self { Self { meter: Some((slice, candidates)), ..Self::new(problem) } }

    /// 共享原生会话错误通道 / Share the native session error channel
    pub fn with_errors(mut self, errors: &'p RefCell<Vec<String>>) -> Self { self.errors = Some(errors); self }
}

impl<'p, P, V> VariableModel<V> for MultiObjectiveProblemModel<'p, P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    fn local_evaluation(&self) -> Option<&dyn LocalEvaluation<V>> { self.meter.is_some().then_some(self) }

    fn num_variables(&self) -> usize {
        self.order.len()
    }

    fn variable_order(&self) -> Vec<VariableId> {
        self.order.clone()
    }

    fn lower_bound(&self, index: usize) -> Option<f64> {
        self.problem.lower_bound(index)
    }

    fn upper_bound(&self, index: usize) -> Option<f64> {
        self.problem.upper_bound(index)
    }

    fn coerce_in(&self, _index: usize, value: V) -> V {
        value
    }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> {
        self.problem.initial_solutions(amount)
    }

    fn constraint_satisfied(&self, solution: &Solution<V>) -> Option<bool> {
        Some(self.problem.valid_solution(solution) && self.problem.admissible(solution))
    }
}

impl<P: SearchProblem<Vec<f64>, V>, V: HeuristicValue> LocalEvaluation<V> for MultiObjectiveProblemModel<'_, P, V> {
    fn remaining(&self) -> u64 { self.meter.map_or(0, |(slice, _)| slice.remaining()) }

    fn evaluate(&self, solution: &Solution<V>) -> Option<usize> {
        if self.errors.is_some_and(|errors| !errors.borrow().is_empty()) { return None; }
        let (slice, candidates) = self.meter?;
        let evaluated = match self.problem.evaluate(solution.clone(), slice) {
            Ok(result) => result.candidate?,
            Err(error) => { if let Some(errors) = self.errors { errors.borrow_mut().push(error.to_string()); } return None; }
        };
        if evaluated.fitness.is_empty() || evaluated.fitness.iter().any(|value| !value.is_finite()) { return None; }
        let mut entries = candidates.borrow_mut();
        let key = entries.len(); entries.push(evaluated);
        Some(key)
    }

    fn compare(&self, lhs: usize, rhs: usize) -> Option<std::cmp::Ordering> {
        let entries = self.meter?.1.borrow();
        Some(self.problem.compare_individuals(entries.get(lhs)?, entries.get(rhs)?))
    }

    fn rank(&self, indices: &[usize]) -> Option<Vec<usize>> {
        let entries = self.meter?.1.borrow();
        let values = indices.iter().map(|index| entries.get(*index).cloned()).collect::<Option<Vec<_>>>()?;
        Some(self.problem.rank_indices(&values))
    }
}

impl<'p, P, V> ObjectiveModel<Vec<f64>, V> for MultiObjectiveProblemModel<'p, P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    fn default_objective(&self) -> Option<Vec<f64>> {
        None
    }

    fn objective(&self, solution: &Solution<V>) -> Option<Vec<f64>> {
        if let Some((_, candidates)) = self.meter {
            let key = LocalEvaluation::evaluate(self, solution)?;
            Some(candidates.borrow().get(key)?.fitness.clone())
        } else { self.problem.objective(solution) }
    }

    fn compare_objective(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> Option<std::cmp::Ordering> {
        if self.problem.dominates(lhs, rhs) {
            Some(std::cmp::Ordering::Less)
        } else if self.problem.dominates(rhs, lhs) {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }

    fn compare_individuals(&self, lhs: &Individual<Vec<f64>, V>, rhs: &Individual<Vec<f64>, V>) -> std::cmp::Ordering {
        self.problem.compare_individuals(lhs, rhs)
    }

    fn rank_indices(&self, entries: &[Individual<Vec<f64>, V>]) -> Vec<usize> { self.problem.rank_indices(entries) }

    fn stochastic_constraints(&self) -> bool { self.problem.stochastic_constraints() }

    fn compare_constraint_tiers(&self, lhs: &Solution<V>, rhs: &Solution<V>) -> std::cmp::Ordering {
        match (self.problem.constraint_tier(lhs), self.problem.constraint_tier(rhs)) {
            (Some(a), Some(b)) => a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)),
            _ => std::cmp::Ordering::Equal,
        }
    }
}

/// 使用骨架自身熵源的种群钩子 / Population hooks using the skeleton's own entropy source
///
/// 会话不接管随机性：骨架保留自己的熵源，因此分片与否不会改变随机序列。
/// The session does not take over randomness: the skeleton keeps its own entropy source, so slicing never changes the random
/// stream.
pub struct PopulationHooks<P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    runner: PopulationRunner<V>,
    entropy: Box<dyn Entropy>,
    state: Option<Population<V>>,
    candidates: RefCell<Vec<Individual<f64, V>>>,
    errors: RefCell<Vec<String>>,
    marker: std::marker::PhantomData<fn() -> P>,
}

impl<P, V> PopulationHooks<P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    /// 创建种群钩子 / Create population hooks
    pub fn new(runner: PopulationRunner<V>, entropy: Box<dyn Entropy>) -> Self {
        Self {
            runner,
            entropy,
            state: None,
            candidates: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
            marker: std::marker::PhantomData,
        }
    }

    /// 骨架引用 / Skeleton reference
    pub fn runner(&self) -> &PopulationRunner<V> {
        &self.runner
    }

    /// 已建立的种群 / The established population
    pub fn population(&self) -> Option<&Population<V>> {
        self.state.as_ref()
    }
}

impl<P, V> SessionHooks<P, f64, V, Population<V>> for PopulationHooks<P, V>
where
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    fn initialize(&mut self, problem: &P, slice: &SearchSlice) -> Result<Population<V>> {
        let model = ProblemModel::<P, V>::metered(problem, slice, &self.candidates).with_errors(&self.errors);
        let population = self
            .runner
            .initialize_island(&model, self.entropy.as_mut());
        if population.is_empty() {
            return Err(HeuristicError::NoIndividual);
        }
        self.state = Some(population.clone());
        Ok(population)
    }

    fn advance(
        &mut self,
        problem: &mut P,
        iteration: &Iteration,
        state: &Population<V>,
        slice: &SearchSlice,
    ) -> Result<Population<V>> {
        let model = ProblemModel::<P, V>::metered(problem, slice, &self.candidates).with_errors(&self.errors);
        let advanced =
            self.runner
                .step_island(&model, iteration, state, self.entropy.as_mut());
        if advanced.is_empty() {
            return Err(HeuristicError::NoIndividual);
        }
        self.state = Some(advanced.clone());
        Ok(advanced)
    }

    fn drain_candidates(&mut self) -> Vec<Individual<f64, V>> {
        self.candidates.replace(Vec::new())
    }

    fn drain_errors(&mut self) -> Vec<String> { self.errors.replace(Vec::new()) }

    fn members(&self, state: &Population<V>) -> Vec<Individual<f64, V>> {
        state
            .iter()
            .map(|individual| {
                Individual::new(individual.solution.clone(), to_f64(&individual.fitness))
            })
            .collect()
    }
}

/// 供导入使用的合并策略：用候选替换最差个体并保持种群规模 / Import policy: replace the worst individuals while keeping the population size
pub fn replace_worst<V>(
    state: &Population<V>,
    candidates: &[Individual<V, V>],
    compare: impl Fn(&Individual<V, V>, &Individual<V, V>) -> std::cmp::Ordering,
) -> Population<V>
where
    V: HeuristicValue,
{
    let mut merged = state.clone();
    for candidate in candidates {
        if merged.len() < state.len() {
            merged.push(candidate.clone());
            continue;
        }
        let mut worst = 0;
        for index in 1..merged.len() {
            if compare(&merged[index], &merged[worst]) == std::cmp::Ordering::Greater {
                worst = index;
            }
        }
        if compare(candidate, &merged[worst]) == std::cmp::Ordering::Less {
            merged[worst] = candidate.clone();
        }
    }
    merged
}

/// 多目标前沿维护：非支配分层 + 末层截断 / Multi-objective front maintenance: non-dominated layering plus last-front truncation
///
/// 与 `Nsga2Runner` 的选择逻辑同构，但独立出来以便会话复用，而不必把骨架生命周期塞进会话。
/// Structurally identical to `Nsga2Runner`'s selection but extracted so the session can reuse it without pulling the skeleton
/// lifecycle into the session.
pub fn select_front<V>(
    candidates: Vec<Individual<Vec<f64>, V>>,
    amount: usize,
    dominance: &dyn Dominance,
    divisions: Option<usize>,
) -> Vec<Individual<Vec<f64>, V>>
where
    V: HeuristicValue,
{
    if candidates.len() <= amount {
        return candidates;
    }

    let fronts = FastNonDominatedSort::sort(
        &candidates,
        |individual| individual.fitness.clone(),
        dominance,
    );

    let mut selected: Vec<Individual<Vec<f64>, V>> = Vec::with_capacity(amount);
    let mut last_front: Vec<Individual<Vec<f64>, V>> = Vec::new();
    for front in fronts {
        if selected.len() + front.len() <= amount {
            selected.extend(front);
        } else {
            last_front = front;
            break;
        }
    }

    if last_front.is_empty() {
        return selected;
    }

    match divisions {
        None => {
            let distances = CrowdingDistance::assign(
                &last_front
                    .iter()
                    .map(|individual| individual.fitness.clone())
                    .collect::<Vec<_>>(),
            );
            let remaining = amount - selected.len();
            let mut order: Vec<usize> = (0..last_front.len()).collect();
            for _ in 0..remaining {
                if order.is_empty() {
                    break;
                }
                let mut best_position = 0;
                let mut best_distance = f64::NEG_INFINITY;
                for position in 0..order.len() {
                    let distance = distances[order[position]];
                    if distance > best_distance {
                        best_distance = distance;
                        best_position = position;
                    }
                }
                let index = order.remove(best_position);
                selected.push(last_front[index].clone());
            }
        }
        Some(divisions) => {
            fill_by_reference(&mut selected, last_front, amount, divisions);
        }
    }

    selected
}

/// 先按约束层级分组，再保留各组的原始 Pareto 几何 / Group by constraint tiers before preserving each group's original Pareto geometry
pub fn select_constrained_front<P, V>(
    problem: &P,
    candidates: Vec<Individual<Vec<f64>, V>>,
    amount: usize,
    dominance: &dyn Dominance,
    divisions: Option<usize>,
) -> Vec<Individual<Vec<f64>, V>>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    if !problem.has_constraints() { return select_front(candidates, amount, dominance, divisions); }
    if problem.stochastic_constraints() {
        return problem.rank_indices(&candidates).into_iter().take(amount).map(|index| candidates[index].clone()).collect();
    }
    let mut tiers: Vec<_> = candidates.into_iter().map(|individual| {
        (problem.constraint_tier(&individual.solution).unwrap_or((1, f64::INFINITY)), individual)
    }).collect();
    tiers.sort_by(|a, b| a.0.0.cmp(&b.0.0).then_with(|| a.0.1.total_cmp(&b.0.1)));
    let mut selected = Vec::new();
    let mut offset = 0;
    while offset < tiers.len() && selected.len() < amount {
        let mut end = offset + 1;
        while end < tiers.len() && tiers[end].0 == tiers[offset].0 { end += 1; }
        let group = tiers[offset..end].iter().map(|(_, individual)| individual.clone()).collect();
        selected.extend(select_front(group, amount - selected.len(), dominance, divisions));
        offset = end;
    }
    selected
}

/// 按参考方向补足末层 / Fill the last front by reference directions
fn fill_by_reference<V>(
    selected: &mut Vec<Individual<Vec<f64>, V>>,
    front: Vec<Individual<Vec<f64>, V>>,
    amount: usize,
    divisions: usize,
) where
    V: HeuristicValue,
{
    let remaining = amount.saturating_sub(selected.len());
    let dimension = front
        .first()
        .map(|individual| individual.fitness.len())
        .unwrap_or(0);
    let reference_points = ReferencePoints::das_dennis(dimension, divisions.max(1));
    if reference_points.is_empty() {
        selected.extend(front.into_iter().take(remaining));
        return;
    }

    let mut minimum = vec![f64::INFINITY; dimension];
    let mut maximum = vec![f64::NEG_INFINITY; dimension];
    for individual in selected.iter().chain(front.iter()) {
        for index in 0..dimension {
            let value = individual.fitness[index];
            minimum[index] = minimum[index].min(value);
            maximum[index] = maximum[index].max(value);
        }
    }

    let normalise = |individual: &Individual<Vec<f64>, V>| -> Vec<f64> {
        (0..dimension)
            .map(|index| {
                let span = maximum[index] - minimum[index];
                if span <= 1.0e-12 {
                    0.0
                } else {
                    (individual.fitness[index] - minimum[index]) / span
                }
            })
            .collect()
    };

    let nearest_reference = |objectives: &[f64]| -> usize {
        let mut nearest = 0;
        let mut best_distance = f64::INFINITY;
        for (index, reference) in reference_points.iter().enumerate() {
            let distance = ReferencePoints::perpendicular_distance(objectives, reference);
            if distance < best_distance {
                best_distance = distance;
                nearest = index;
            }
        }
        nearest
    };

    let mut niche_count = vec![0_usize; reference_points.len()];
    for individual in selected.iter() {
        let objectives = normalise(individual);
        niche_count[nearest_reference(&objectives)] += 1;
    }

    let mut available = front;
    for _ in 0..remaining {
        if available.is_empty() {
            break;
        }
        let mut target_reference = 0;
        let mut fewest = usize::MAX;
        for index in 0..niche_count.len() {
            if niche_count[index] < fewest
                && available
                    .iter()
                    .any(|candidate| nearest_reference(&normalise(candidate)) == index)
            {
                fewest = niche_count[index];
                target_reference = index;
            }
        }

        let mut chosen_index = None;
        let mut best_distance = f64::INFINITY;
        for (index, candidate) in available.iter().enumerate() {
            let objectives = normalise(candidate);
            if nearest_reference(&objectives) != target_reference {
                continue;
            }
            let distance = ReferencePoints::perpendicular_distance(
                &objectives,
                &reference_points[target_reference],
            );
            if distance < best_distance {
                best_distance = distance;
                chosen_index = Some(index);
            }
        }

        let chosen_index = chosen_index.unwrap_or(0);
        selected.push(available.remove(chosen_index));
        niche_count[target_reference] += 1;
    }
}

/// 多目标会话钩子 / Multi-objective session hooks
///
/// 每步执行「繁殖 → 非支配排序 → 截断」，`O = Vec<f64>`；档案由会话层维护。
/// Each step runs "breed → non-dominated sort → truncate" with `O = Vec<f64>`; the archive is maintained by the session.
pub struct NsgaSessionHooks<P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    size: usize,
    mutation_rate: f64,
    dominance: Box<dyn Dominance>,
    update: Box<dyn crate::solver::heuristic::operator::update::SolutionUpdate<V>>,
    mutation: Box<dyn crate::solver::heuristic::operator::mutation::Mutation<V>>,
    divisions: Option<usize>,
    entropy: Box<dyn Entropy>,
    state: Option<Vec<Individual<Vec<f64>, V>>>,
    candidates: RefCell<Vec<Individual<Vec<f64>, V>>>,
    errors: RefCell<Vec<String>>,
    marker: std::marker::PhantomData<fn() -> P>,
}

impl<P, V> NsgaSessionHooks<P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    /// 创建多目标钩子 / Create multi-objective hooks
    pub fn new(
        size: usize,
        dominance: Box<dyn Dominance>,
        update: Box<dyn crate::solver::heuristic::operator::update::SolutionUpdate<V>>,
        mutation: Box<dyn crate::solver::heuristic::operator::mutation::Mutation<V>>,
        entropy: Box<dyn Entropy>,
        divisions: Option<usize>,
    ) -> Self {
        Self {
            size,
            mutation_rate: 0.1,
            dominance,
            update,
            mutation,
            divisions,
            entropy,
            state: None,
            candidates: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
            marker: std::marker::PhantomData,
        }
    }

    /// 设置变异率 / Set the mutation rate
    pub fn with_mutation_rate(mut self, mutation_rate: f64) -> Self {
        self.mutation_rate = mutation_rate;
        self
    }

    /// 当前前沿 / Current front
    pub fn front(&self) -> Option<&[Individual<Vec<f64>, V>]> {
        self.state.as_deref()
    }

    fn evaluate(&self, problem: &P, solution: Solution<V>, slice: &SearchSlice) -> Option<Individual<Vec<f64>, V>> {
        if !self.errors.borrow().is_empty() { return None; }
        let evaluated = match problem.evaluate(solution, slice) {
            Ok(result) => result.candidate?,
            Err(error) => { self.errors.borrow_mut().push(error.to_string()); return None; }
        };
        if evaluated.fitness.is_empty() || evaluated.fitness.iter().any(|value| !value.is_finite()) {
            return None;
        }
        self.candidates.borrow_mut().push(evaluated.clone());
        Some(evaluated)
    }
    fn breed(
        &mut self,
        problem: &mut P,
        iteration: &Iteration,
        population: &[Individual<Vec<f64>, V>],
        slice: &SearchSlice,
    ) -> Vec<Individual<Vec<f64>, V>> {
        if population.is_empty() {
            return Vec::new();
        }

        let mut offspring = Vec::new();
        for index in 0..population.len() {
            let parents = vec![
                population[index].solution.clone(),
                population[(index + 1) % population.len()].solution.clone(),
            ];
            let model = MultiObjectiveProblemModel::<P, V>::metered(problem, slice, &self.candidates).with_errors(&self.errors);
            for solution in self
                .update
                .update(&model, iteration, &parents, self.entropy.as_mut())
            {
                if let Some(individual) = self.evaluate(problem, solution, slice) {
                    offspring.push(individual);
                }
            }
        }

        for individual in population {
            let model = MultiObjectiveProblemModel::<P, V>::metered(problem, slice, &self.candidates).with_errors(&self.errors);
            let solution = self.mutation.mutate(
                &model,
                iteration,
                &individual.solution,
                self.mutation_rate,
                self.entropy.as_mut(),
            );
            if let Some(mutant) = self.evaluate(problem, solution, slice) {
                offspring.push(mutant);
            }
        }

        offspring
    }
}

impl<P, V> SessionHooks<P, Vec<f64>, V, Vec<Individual<Vec<f64>, V>>>
    for NsgaSessionHooks<P, V>
where
    P: SearchProblem<Vec<f64>, V>,
    V: HeuristicValue,
{
    fn initialize(
        &mut self,
        problem: &P,
        slice: &SearchSlice,
    ) -> Result<Vec<Individual<Vec<f64>, V>>> {
        let population: Vec<Individual<Vec<f64>, V>> = problem
            .initial_solutions(self.size)
            .into_iter()
            .filter_map(|solution| self.evaluate(problem, solution, slice))
            .take(self.size)
            .collect();
        if population.is_empty() {
            return Err(HeuristicError::NoIndividual);
        }
        let selected = select_constrained_front(
            problem,
            population,
            self.size,
            self.dominance.as_ref(),
            self.divisions,
        );
        self.state = Some(selected.clone());
        Ok(selected)
    }

    fn advance(
        &mut self,
        problem: &mut P,
        iteration: &Iteration,
        state: &Vec<Individual<Vec<f64>, V>>,
        slice: &SearchSlice,
    ) -> Result<Vec<Individual<Vec<f64>, V>>> {
        let offspring = self.breed(problem, iteration, state, slice);
        let mut candidates = state.clone();
        candidates.extend(offspring);
        let selected = select_constrained_front(
            problem,
            candidates,
            self.size,
            self.dominance.as_ref(),
            self.divisions,
        );
        if selected.is_empty() {
            return Err(HeuristicError::NoIndividual);
        }
        self.state = Some(selected.clone());
        Ok(selected)
    }

    fn drain_candidates(&mut self) -> Vec<Individual<Vec<f64>, V>> {
        self.candidates.replace(Vec::new())
    }

    fn drain_errors(&mut self) -> Vec<String> { self.errors.replace(Vec::new()) }

    fn members(&self, state: &Vec<Individual<Vec<f64>, V>>) -> Vec<Individual<Vec<f64>, V>> {
        state.clone()
    }
}
