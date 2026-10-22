//! NSGA-II / NSGA-III 骨架 / NSGA-II and NSGA-III skeletons
//!
//! 与单目标骨架的区别是根本性的，因此这里是独立实现而不是通用骨架的实例：环境选择是「先分层、层内再按拥挤距离
//! （或参考点）截断」，收敛载体是一整条互不支配的前沿，目标向量由 `objectives_of` 直接给出。
//! The difference from the single-objective skeleton is fundamental, so this is a standalone implementation rather than an
//! instance of the generic skeleton: survivor selection layers first and truncates each layer by crowding distance (or by
//! reference points), the convergence carrier is a whole mutually non-dominated front, and objective vectors come straight
//! from `objectives_of`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::model::callback::Solution;

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};
use crate::solver::heuristic::operator::mutation::Mutation;
use crate::solver::heuristic::operator::multi_objective::{
    CrowdingDistance, Dominance, FastNonDominatedSort, ParetoArchive, ParetoDominance,
    ReferencePoints,
};
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::update::SolutionUpdate;
use crate::solver::heuristic::result::{
    HeuristicError, MultiObjectiveResult, MultiObjectiveSolver, Result,
};

/// 共享支配判据的适配器 / Adapter sharing a dominance relation
struct SharedDominance(Rc<dyn Dominance>);

impl Dominance for SharedDominance {
    fn dominates(&self, lhs: &[f64], rhs: &[f64]) -> bool {
        self.0.dominates(lhs, rhs)
    }
}

/// 末层截断策略 / Last-front truncation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionMode {
    /// NSGA-II：按拥挤距离 / NSGA-II: by crowding distance
    Crowding,
    /// NSGA-III：按 Das–Dennis 参考点 / NSGA-III: by Das–Dennis reference points
    Reference { divisions: usize },
}

/// 多目标骨架配置 / Multi-objective skeleton configuration
pub struct NsgaConfig {
    /// 种群规模 / Population size
    pub size: usize,
    /// 变异率 / Mutation rate
    pub mutation_rate: f64,
    /// 前沿档案容量 / Front archive capacity
    pub archive_amount: usize,
    /// 终止判据 / Stopping criterion
    pub stopping: Box<dyn StoppingCriterion>,
}

/// 多目标骨架（NSGA-II 默认，切换到参考点即 NSGA-III）/ Multi-objective skeleton (NSGA-II by default, NSGA-III with reference points)
pub struct Nsga2Runner<V>
where
    V: HeuristicValue,
{
    config: NsgaConfig,
    objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
    dominance: Rc<dyn Dominance>,
    update: Box<dyn SolutionUpdate<V>>,
    mutation: Box<dyn Mutation<V>>,
    entropy: RefCell<Box<dyn Entropy>>,
    mode: SelectionMode,
}

impl<V> Nsga2Runner<V>
where
    V: HeuristicValue,
{
    /// 创建 NSGA-II 骨架 / Create an NSGA-II skeleton
    pub fn new(
        size: usize,
        objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
        dominance: Box<dyn Dominance>,
        update: Box<dyn SolutionUpdate<V>>,
        mutation: Box<dyn Mutation<V>>,
        entropy: Box<dyn Entropy>,
        stopping: Box<dyn StoppingCriterion>,
    ) -> Self {
        Self {
            config: NsgaConfig {
                size,
                mutation_rate: 0.1,
                archive_amount: 100,
                stopping,
            },
            objectives_of,
            dominance: Rc::from(dominance),
            update,
            mutation,
            entropy: RefCell::new(entropy),
            mode: SelectionMode::Crowding,
        }
    }

    /// 以最小化支配创建 / Create with minimisation dominance
    pub fn minimization(
        size: usize,
        objective_amount: usize,
        objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
        update: Box<dyn SolutionUpdate<V>>,
        mutation: Box<dyn Mutation<V>>,
        entropy: Box<dyn Entropy>,
        stopping: Box<dyn StoppingCriterion>,
    ) -> Self {
        Self::new(
            size,
            objectives_of,
            Box::new(ParetoDominance::minimization(objective_amount)),
            update,
            mutation,
            entropy,
            stopping,
        )
    }

    /// 切换到参考点截断（NSGA-III）/ Switch to reference-point truncation (NSGA-III)
    pub fn with_reference_divisions(mut self, divisions: usize) -> Self {
        self.mode = SelectionMode::Reference {
            divisions: divisions.max(1),
        };
        self
    }

    /// 设置变异率 / Set the mutation rate
    pub fn with_mutation_rate(mut self, mutation_rate: f64) -> Self {
        self.config.mutation_rate = mutation_rate;
        self
    }

    /// 设置档案容量 / Set the archive capacity
    pub fn with_archive_amount(mut self, archive_amount: usize) -> Self {
        self.config.archive_amount = archive_amount.max(1);
        self
    }

    fn evaluate(&self, solution: Solution<V>) -> Option<Individual<Vec<f64>, V>> {
        let objectives = (self.objectives_of)(&solution);
        if objectives.is_empty() || !objectives.iter().all(|value| value.is_finite()) {
            return None;
        }
        Some(Individual::new(solution, objectives))
    }

    fn breed<M>(
        &self,
        model: &M,
        iteration: &Iteration,
        population: &[Individual<Vec<f64>, V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Individual<Vec<f64>, V>>
    where
        M: VariableModel<V>,
    {
        if population.is_empty() {
            return Vec::new();
        }

        let mut offspring = Vec::new();
        for index in 0..population.len() {
            let parents = vec![
                population[index].solution.clone(),
                population[(index + 1) % population.len()].solution.clone(),
            ];
            for solution in self.update.update(model, iteration, &parents, entropy) {
                if let Some(individual) = self.evaluate(solution) {
                    offspring.push(individual);
                }
            }
        }

        for individual in population {
            let solution = self.mutation.mutate(
                model,
                iteration,
                &individual.solution,
                self.config.mutation_rate,
                entropy,
            );
            if let Some(mutant) = self.evaluate(solution) {
                offspring.push(mutant);
            }
        }

        offspring
    }

    /// 环境选择 / Survivor selection
    fn select(
        &self,
        candidates: Vec<Individual<Vec<f64>, V>>,
    ) -> Vec<Individual<Vec<f64>, V>> {
        if candidates.len() <= self.config.size {
            return candidates;
        }

        let fronts = FastNonDominatedSort::sort(
            &candidates,
            |individual| individual.fitness.clone(),
            &*self.dominance,
        );

        let mut selected = Vec::with_capacity(self.config.size);
        let mut last_front: Vec<Individual<Vec<f64>, V>> = Vec::new();
        for front in fronts {
            if selected.len() + front.len() <= self.config.size {
                selected.extend(front);
            } else {
                last_front = front;
                break;
            }
        }

        if last_front.is_empty() {
            return selected;
        }

        match self.mode {
            SelectionMode::Crowding => {
                self.fill_by_crowding(&mut selected, last_front);
            }
            SelectionMode::Reference { divisions } => {
                self.fill_by_reference(&mut selected, last_front, divisions);
            }
        }
        selected
    }

    /// 按拥挤距离补足末层 / Fill the last front by crowding distance
    fn fill_by_crowding(
        &self,
        selected: &mut Vec<Individual<Vec<f64>, V>>,
        front: Vec<Individual<Vec<f64>, V>>,
    ) {
        let distances = CrowdingDistance::assign(
            &front
                .iter()
                .map(|individual| individual.fitness.clone())
                .collect::<Vec<_>>(),
        );
        let remaining = self.config.size - selected.len();
        let mut order: Vec<usize> = (0..front.len()).collect();
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
            selected.push(front[index].clone());
        }
    }

    /// 按参考方向补足末层 / Fill the last front by reference directions
    fn fill_by_reference(
        &self,
        selected: &mut Vec<Individual<Vec<f64>, V>>,
        front: Vec<Individual<Vec<f64>, V>>,
        divisions: usize,
    ) {
        let remaining = self.config.size - selected.len();
        let dimension = front
            .first()
            .map(|individual| individual.fitness.len())
            .unwrap_or(0);
        let reference_points = ReferencePoints::das_dennis(dimension, divisions);
        if reference_points.is_empty() {
            // 参考点生成失败时退化为「按目标顺序截断」，保证仍然返回合法结果而不 panic。
            // When the reference points cannot be generated, fall back to truncation in objective order.
            selected.extend(front.into_iter().take(remaining));
            return;
        }

        // 归一化：用「已选个体 + 末层候选」的逐目标极值做 min-max，使距离在量纲上可比。
        // Normalisation: min-max per objective over "already selected + last front".
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

        // 统计每个参考方向当前已有的成员数。
        // Count the current members of every reference direction.
        let mut niche_count = vec![0_usize; reference_points.len()];
        for individual in selected.iter() {
            let objectives = normalise(individual);
            niche_count[nearest_reference(&objectives)] += 1;
        }

        let mut available: Vec<Individual<Vec<f64>, V>> = front;
        for _ in 0..remaining {
            if available.is_empty() {
                break;
            }

            // 逐次补位：每次找成员最少的参考方向，把该方向上距离最近的末层个体纳入。
            // Fill one slot at a time: pick the reference direction with the fewest members and take the closest candidate.
            let mut target_reference = 0;
            let mut fewest = usize::MAX;
            for index in 0..niche_count.len() {
                if niche_count[index] < fewest
                    && available.iter().any(|candidate| {
                        nearest_reference(&normalise(candidate)) == index
                    })
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
}

impl<V> MultiObjectiveSolver<V> for Nsga2Runner<V>
where
    V: HeuristicValue,
{
    fn name(&self) -> &str {
        match self.mode {
            SelectionMode::Crowding => "nsga-ii",
            SelectionMode::Reference { .. } => "nsga-iii",
        }
    }

    fn solve<M>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<MultiObjectiveResult<V>>
    where
        M: VariableModel<V>,
    {
        let mut state: Vec<Individual<Vec<f64>, V>> = model
            .initial_solutions(self.config.size)
            .into_iter()
            .filter_map(|solution| self.evaluate(solution))
            .take(self.config.size)
            .collect();
        if state.is_empty() {
            return Err(HeuristicError::NoIndividual);
        }

        let mut archive = ParetoArchive::new(
            self.config.archive_amount,
            Box::new(|individual: &Individual<Vec<f64>, V>| individual.fitness.clone()),
            Box::new(SharedDominance(self.dominance.clone())),
        );
        archive.offer(state.clone());
        iteration.next(true);

        while !self.config.stopping.should_stop(iteration) {
            let offspring = self.breed(model, iteration, &state, &mut **self.entropy.borrow_mut());
            let mut candidates = state.clone();
            candidates.extend(offspring);
            state = self.select(candidates);

            let before = archive.size();
            archive.offer(state.clone());
            iteration.next(archive.size() > before);
        }

        let ordered = FastNonDominatedSort::sort(
            archive.items(),
            |individual| individual.fitness.clone(),
            &*self.dominance,
        )
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let front = ordered
            .iter()
            .map(|individual| individual.solution.clone())
            .collect();
        let objectives = ordered
            .iter()
            .map(|individual| individual.fitness.clone())
            .collect();
        Ok(MultiObjectiveResult::new(front, objectives, iteration.clone()))
    }
}

/// NSGA-III 骨架 / NSGA-III skeleton
pub struct Nsga3Runner<V>
where
    V: HeuristicValue,
{
    inner: Nsga2Runner<V>,
}

impl<V> Nsga3Runner<V>
where
    V: HeuristicValue,
{
    /// 创建 NSGA-III 骨架 / Create an NSGA-III skeleton
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        size: usize,
        objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
        dominance: Box<dyn Dominance>,
        update: Box<dyn SolutionUpdate<V>>,
        mutation: Box<dyn Mutation<V>>,
        entropy: Box<dyn Entropy>,
        stopping: Box<dyn StoppingCriterion>,
        divisions: usize,
    ) -> Self {
        Self {
            inner: Nsga2Runner::new(
                size,
                objectives_of,
                dominance,
                update,
                mutation,
                entropy,
                stopping,
            )
            .with_reference_divisions(divisions),
        }
    }

    /// 设置变异率 / Set the mutation rate
    pub fn with_mutation_rate(mut self, mutation_rate: f64) -> Self {
        self.inner = self.inner.with_mutation_rate(mutation_rate);
        self
    }

    /// 设置档案容量 / Set the archive capacity
    pub fn with_archive_amount(mut self, archive_amount: usize) -> Self {
        self.inner = self.inner.with_archive_amount(archive_amount);
        self
    }
}

impl<V> MultiObjectiveSolver<V> for Nsga3Runner<V>
where
    V: HeuristicValue,
{
    fn name(&self) -> &str {
        "nsga-iii"
    }

    fn solve<M>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<MultiObjectiveResult<V>>
    where
        M: VariableModel<V>,
    {
        self.inner.solve(model, iteration)
    }
}
