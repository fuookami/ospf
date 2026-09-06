//! 启发式算法通用接口
//! Generic Heuristic Algorithm Interface

use crate::error::Result;
use crate::model::callback::{
    AbstractCallBackModel, AbstractCallBackModelInterface, CallBackModelInterface,
    MultiObjectiveCallBackModel, Solution, SolutionStatus,
};
use async_trait::async_trait;
use std::cmp::Ordering;

use super::{AbstractHeuristicPolicy, Iteration};

/// 启发式个体（解 + 目标值）/ Heuristic individual (solution + objective)
#[derive(Debug, Clone)]
pub struct HeuristicIndividual<V> {
    /// 解 / Solution
    pub solution: Solution<V>,
    /// 目标值（或适应度）/ Objective value (or fitness)
    pub fitness: V,
}

impl<V> HeuristicIndividual<V> {
    /// 创建启发式个体 / Create heuristic individual
    pub fn new(solution: Solution<V>, fitness: V) -> Self {
        Self { solution, fitness }
    }
}

/// 刷新优秀个体列表（对齐 Kotlin `refreshGoodIndividuals` 语义）/ Refresh good individuals (aligned with Kotlin `refreshGoodIndividuals`)
pub fn refresh_good_individuals<V, M>(
    good_individuals: &mut Vec<HeuristicIndividual<V>>,
    new_individuals: Vec<HeuristicIndividual<V>>,
    model: &M,
    solution_amount: usize,
) where
    V: Clone + PartialOrd,
    M: HeuristicCallBackModelInterface<V> + ?Sized,
{
    if solution_amount == 0 {
        good_individuals.clear();
        return;
    }

    good_individuals.extend(new_individuals);
    good_individuals.sort_by(|lhs, rhs| model.compare_objective(&lhs.fitness, &rhs.fitness));
    good_individuals.truncate(solution_amount);
}

/// 启发式求解结果 / Heuristic solve result
#[derive(Debug, Clone)]
pub struct HeuristicResult<V> {
    /// 最优解 / Best solution
    pub best_solution: Option<Solution<V>>,
    /// 最优目标值 / Best objective
    pub best_objective: Option<V>,
    /// 求解状态 / Solve status
    pub status: SolutionStatus,
    /// 迭代信息 / Iteration snapshot
    pub iteration: Iteration,
}

impl<V: Clone> HeuristicResult<V> {
    /// 从 callback 模型抓取结果快照 / Build result snapshot from callback model
    pub fn from_model<M>(model: &M, iteration: Iteration) -> Self
    where
        M: CallBackModelInterface<V> + ?Sized,
    {
        Self {
            best_solution: model.get_solution().cloned(),
            best_objective: model.get_objective_value(),
            status: model.get_status(),
            iteration,
        }
    }
}

/// 启发式算法回调模型扩展 / Callback model extension for heuristics
///
/// 该接口对齐 Kotlin 插件中算法对模型的最小需求：初始解、目标计算、目标比较、刷新。
/// This mirrors the minimal model contract expected by Kotlin heuristic plugins:
/// initial solutions, objective evaluation, objective comparison, and flush hook.
pub trait HeuristicCallBackModelInterface<V: Clone>: AbstractCallBackModelInterface<V> {
    /// 默认目标值 / Default objective
    fn default_objective(&self) -> Option<V> {
        None
    }

    /// 生成初始解 / Generate initial solutions
    fn initial_solutions(&self, _initial_solution_amount: usize) -> Vec<Solution<V>> {
        Vec::new()
    }

    /// 计算给定解目标值 / Evaluate objective for a solution
    fn objective(&self, _solution: &Solution<V>) -> Option<V> {
        None
    }

    /// 比较目标值（更优返回 `Ordering::Less`）/ Compare objectives (`Ordering::Less` means better)
    fn compare_objective(&self, lhs: &V, rhs: &V) -> Ordering
    where
        V: PartialOrd,
    {
        lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal)
    }

    /// 值域收敛（按变量索引）/ Coerce value range by variable index
    fn coerce_in(&self, _index: usize, value: V) -> V {
        value
    }

    /// 刷新模型缓存 / Flush model cache
    fn flush(&mut self) {}
}

impl<V> HeuristicCallBackModelInterface<V> for AbstractCallBackModel<V>
where
    V: Clone + Default + Send + Sync + 'static,
{
    fn default_objective(&self) -> Option<V> {
        self.get_objective_value()
            .or_else(|| self.calculate_objective())
    }

    fn initial_solutions(&self, initial_solution_amount: usize) -> Vec<Solution<V>> {
        if initial_solution_amount == 0 {
            return Vec::new();
        }
        let generated = self.generate_initial_solutions_for_heuristic(initial_solution_amount);
        if generated.is_empty() {
            self.get_solution().cloned().into_iter().collect()
        } else {
            generated
        }
    }

    fn objective(&self, solution: &Solution<V>) -> Option<V> {
        self.evaluate_objective_for_heuristic(solution)
    }

    fn coerce_in(&self, index: usize, value: V) -> V {
        self.coerce_value_for_heuristic(index, value)
    }
}

impl<V> HeuristicCallBackModelInterface<V> for MultiObjectiveCallBackModel<V>
where
    V: Clone
        + Default
        + Send
        + Sync
        + 'static
        + for<'a> std::ops::Add<&'a V, Output = V>
        + for<'a> std::ops::Mul<&'a V, Output = V>,
{
    fn default_objective(&self) -> Option<V> {
        self.get_objective_value()
            .or_else(|| self.multi_obj.calculate_weighted_objective())
            .or_else(|| self.calculate_objective())
    }

    fn initial_solutions(&self, initial_solution_amount: usize) -> Vec<Solution<V>> {
        if initial_solution_amount == 0 {
            return Vec::new();
        }
        let generated = self
            .base
            .generate_initial_solutions_for_heuristic(initial_solution_amount);
        if generated.is_empty() {
            self.get_solution().cloned().into_iter().collect()
        } else {
            generated
        }
    }

    fn objective(&self, solution: &Solution<V>) -> Option<V> {
        self.base.evaluate_objective_for_heuristic(solution)
    }

    fn coerce_in(&self, index: usize, value: V) -> V {
        self.base.coerce_value_for_heuristic(index, value)
    }
}

/// 启发式策略运行时扩展 / Runtime extension for heuristic policy
///
/// Kotlin 策略里 `update(...)` 会拿到 best/good/populations/model，这里提供同等上下文。
/// Kotlin policy `update(...)` receives best/good/populations/model; this trait provides the same context.
pub trait HeuristicRuntimePolicy<V, M>: AbstractHeuristicPolicy
where
    V: Clone + PartialOrd,
    M: HeuristicCallBackModelInterface<V> + ?Sized,
{
    /// 值域收敛 / Coerce value
    fn coerce_in(&self, _iteration: &Iteration, index: usize, value: V, model: &M) -> V {
        model.coerce_in(index, value)
    }

    /// 带上下文的策略更新 / Context-aware policy update
    fn update_with_context(
        &mut self,
        _iteration: &Iteration,
        _better: bool,
        _best_individual: Option<&HeuristicIndividual<V>>,
        _good_individuals: &[HeuristicIndividual<V>],
        _populations: &[Vec<HeuristicIndividual<V>>],
        _model: &M,
    ) {
    }
}

impl<V, M, P> HeuristicRuntimePolicy<V, M> for P
where
    V: Clone + PartialOrd,
    M: HeuristicCallBackModelInterface<V> + ?Sized,
    P: AbstractHeuristicPolicy,
{
}

/// 启发式算法接口 / Heuristic algorithm interface
#[async_trait]
pub trait HeuristicAlgorithm<V, M>: Send + Sync
where
    V: Clone + PartialOrd + Send + Sync + 'static,
    M: HeuristicCallBackModelInterface<V> + Send + Sync,
{
    /// 算法名称 / Algorithm name
    fn name(&self) -> &str;

    /// 运行算法 / Run algorithm
    async fn run<P>(&self, model: &mut M, policy: &mut P) -> Result<HeuristicResult<V>>
    where
        P: AbstractHeuristicPolicy + HeuristicRuntimePolicy<V, M> + Send + Sync;
}

/// callback 模型的启发式求解扩展 / Heuristic solve extension for callback models
#[async_trait]
pub trait HeuristicModelExt<V>: HeuristicCallBackModelInterface<V> + Send + Sync
where
    V: Clone + PartialOrd + Send + Sync + 'static,
{
    /// 使用启发式算法求解 / Solve via heuristic algorithm
    async fn solve_with_heuristic<A, P>(
        &mut self,
        algorithm: &A,
        policy: &mut P,
    ) -> Result<HeuristicResult<V>>
    where
        A: HeuristicAlgorithm<V, Self> + Send + Sync,
        P: AbstractHeuristicPolicy + HeuristicRuntimePolicy<V, Self> + Send + Sync,
        Self: Sized,
    {
        algorithm.run(self, policy).await
    }
}

#[async_trait]
impl<V, M> HeuristicModelExt<V> for M
where
    V: Clone + PartialOrd + Send + Sync + 'static,
    M: HeuristicCallBackModelInterface<V> + Send + Sync,
{
}

/// 单轮调用算法（最小实现）/ Single-shot algorithm (minimal implementation)
#[derive(Debug, Default, Clone, Copy)]
pub struct SingleShotHeuristic;

#[async_trait]
impl<V, M> HeuristicAlgorithm<V, M> for SingleShotHeuristic
where
    V: Clone + PartialOrd + Send + Sync + 'static,
    M: HeuristicCallBackModelInterface<V> + Send + Sync,
{
    fn name(&self) -> &str {
        "single_shot"
    }

    async fn run<P>(&self, model: &mut M, policy: &mut P) -> Result<HeuristicResult<V>>
    where
        P: AbstractHeuristicPolicy + HeuristicRuntimePolicy<V, M> + Send + Sync,
    {
        let mut iteration = Iteration::new();
        if !policy.finished(&iteration) {
            let output = model.solve().await?;
            let better = output.status.is_feasible();
            let population = if let (Some(solution), Some(fitness)) =
                (model.get_solution().cloned(), model.get_objective_value())
            {
                vec![HeuristicIndividual::new(solution, fitness)]
            } else {
                Vec::new()
            };
            policy.update_with_context(
                &iteration,
                better,
                population.first(),
                &population,
                std::slice::from_ref(&population),
                model,
            );
            policy.update(&iteration, better);
            iteration.next(better);
            model.flush();
        }
        Ok(HeuristicResult::from_model(model, iteration))
    }
}

/// 采样式启发式算法（通用基线）/ Sampling heuristic algorithm (generic baseline)
///
/// 该算法复用 callback 模型提供的 `initial_solutions + objective + compare_objective`：
/// - 初始化候选池；
/// - 按策略迭代采样；
/// - 维护 best/good 并回调 policy 上下文。
///
/// This algorithm reuses callback-model hooks `initial_solutions + objective + compare_objective`:
/// - build initial candidates;
/// - sample per iteration;
/// - maintain best/good and feed policy update context.
#[derive(Debug, Clone, Copy)]
pub struct SamplingHeuristic {
    /// 初始采样数量 / Initial sample size
    pub initial_solution_amount: usize,
    /// 每轮采样数量 / Per-iteration sample size
    pub generation_solution_amount: usize,
    /// 保留优秀解数量 / Number of good solutions to keep
    pub solution_amount: usize,
}

impl Default for SamplingHeuristic {
    fn default() -> Self {
        Self {
            initial_solution_amount: 32,
            generation_solution_amount: 16,
            solution_amount: 4,
        }
    }
}

impl SamplingHeuristic {
    /// 创建采样式启发式算法 / Create sampling heuristic
    pub fn new(
        initial_solution_amount: usize,
        generation_solution_amount: usize,
        solution_amount: usize,
    ) -> Self {
        Self {
            initial_solution_amount,
            generation_solution_amount,
            solution_amount,
        }
    }

    fn evaluate_population<V, M, P>(
        &self,
        iteration: &Iteration,
        model: &M,
        policy: &P,
        solutions: Vec<Solution<V>>,
    ) -> Vec<HeuristicIndividual<V>>
    where
        V: Clone + PartialOrd,
        M: HeuristicCallBackModelInterface<V> + ?Sized,
        P: HeuristicRuntimePolicy<V, M> + ?Sized,
    {
        let mut population = Vec::with_capacity(solutions.len());
        for mut solution in solutions {
            let mut variable_ids: Vec<_> = solution.keys().copied().collect();
            variable_ids.sort_unstable_by(|lhs, rhs| {
                lhs.group_id
                    .cmp(&rhs.group_id)
                    .then(lhs.index_in_group.cmp(&rhs.index_in_group))
            });
            for (index, variable_id) in variable_ids.into_iter().enumerate() {
                if let Some(value) = solution.get(&variable_id).cloned() {
                    let coerced = policy.coerce_in(iteration, index, value, model);
                    solution.insert(variable_id, coerced);
                }
            }
            if let Some(fitness) = model
                .objective(&solution)
                .or_else(|| model.default_objective())
            {
                population.push(HeuristicIndividual::new(solution, fitness));
            }
        }
        population.sort_by(|lhs, rhs| model.compare_objective(&lhs.fitness, &rhs.fitness));
        population
    }
}

#[async_trait]
impl<V, M> HeuristicAlgorithm<V, M> for SamplingHeuristic
where
    V: Clone + PartialOrd + Send + Sync + 'static,
    M: HeuristicCallBackModelInterface<V> + Send + Sync,
{
    fn name(&self) -> &str {
        "sampling"
    }

    async fn run<P>(&self, model: &mut M, policy: &mut P) -> Result<HeuristicResult<V>>
    where
        P: AbstractHeuristicPolicy + HeuristicRuntimePolicy<V, M> + Send + Sync,
    {
        let mut iteration = Iteration::new();
        let keep_amount = self.solution_amount.max(1);
        let mut best_individual: Option<HeuristicIndividual<V>> = None;
        let mut good_individuals: Vec<HeuristicIndividual<V>> = Vec::new();

        let initial_population = self.evaluate_population(
            &iteration,
            model,
            policy,
            model.initial_solutions(self.initial_solution_amount.max(keep_amount)),
        );
        if let Some(initial_best) = initial_population.first().cloned() {
            best_individual = Some(initial_best.clone());
            model.set_initial_solution(&initial_best.solution);
        }
        refresh_good_individuals(
            &mut good_individuals,
            initial_population.clone(),
            model,
            keep_amount,
        );
        policy.update_with_context(
            &iteration,
            best_individual.is_some(),
            best_individual.as_ref(),
            &good_individuals,
            std::slice::from_ref(&initial_population),
            model,
        );

        if best_individual.is_none()
            && let (Some(solution), Some(objective)) =
                (model.get_solution().cloned(), model.get_objective_value())
        {
            let fallback = HeuristicIndividual::new(solution, objective);
            best_individual = Some(fallback.clone());
            good_individuals.push(fallback);
        }

        while !policy.finished(&iteration) {
            let population = self.evaluate_population(
                &iteration,
                model,
                policy,
                model.initial_solutions(self.generation_solution_amount.max(1)),
            );

            let mut better = false;
            if let Some(population_best) = population.first().cloned() {
                match best_individual.as_ref() {
                    Some(current_best) => {
                        if model.compare_objective(&population_best.fitness, &current_best.fitness)
                            == Ordering::Less
                        {
                            best_individual = Some(population_best.clone());
                            model.set_initial_solution(&population_best.solution);
                            better = true;
                        }
                    }
                    None => {
                        best_individual = Some(population_best.clone());
                        model.set_initial_solution(&population_best.solution);
                        better = true;
                    }
                }
            }
            refresh_good_individuals(
                &mut good_individuals,
                population.clone(),
                model,
                keep_amount,
            );

            policy.update_with_context(
                &iteration,
                better,
                best_individual.as_ref(),
                &good_individuals,
                &[population],
                model,
            );
            policy.update(&iteration, better);
            iteration.next(better);
            model.flush();
        }

        let best_solution = best_individual
            .as_ref()
            .map(|individual| individual.solution.clone())
            .or_else(|| model.get_solution().cloned());
        let best_objective = best_individual
            .as_ref()
            .map(|individual| individual.fitness.clone())
            .or_else(|| model.get_objective_value());
        let status = if best_solution.is_some() {
            SolutionStatus::Feasible
        } else {
            model.get_status()
        };

        Ok(HeuristicResult {
            best_solution,
            best_objective,
            status,
            iteration,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::model::callback::{
        AbstractCallBackModel, AbstractCallBackModelInterface, CallBackModelInterface, Solution,
        SolutionStatus,
    };
    use crate::solver::SolverOutput;
    use crate::variable::{VariableId, new_standalone_id};

    use crate::solver::heuristic::HeuristicPolicy;

    use super::{
        HeuristicCallBackModelInterface, HeuristicIndividual, HeuristicModelExt, SamplingHeuristic,
        SingleShotHeuristic, refresh_good_individuals,
    };

    #[tokio::test]
    async fn callback_model_can_be_consumed_by_heuristic_algorithm() {
        let mut model = AbstractCallBackModel::<f64>::new("heuristic_model");
        let variable_id = new_standalone_id();
        model.set_solve_executor(Arc::new(move |callback_model| {
            callback_model.set_solution_internal(HashMap::from([(variable_id, 3.0)]));
            callback_model.set_objective_internal(Some(7.0));
            Ok(SolverOutput::optimal(7.0, vec![3.0]))
        }));

        let algorithm = SingleShotHeuristic;
        let mut policy = HeuristicPolicy::new().with_iteration_limit(10);
        let result = model
            .solve_with_heuristic(&algorithm, &mut policy)
            .await
            .expect("heuristic solve should succeed");

        assert_eq!(result.status, SolutionStatus::Optimal);
        assert_eq!(result.best_objective, Some(7.0));
        assert_eq!(
            result
                .best_solution
                .and_then(|solution| solution.get(&variable_id).copied()),
            Some(3.0)
        );
        assert!(model.is_optimal());
    }

    struct MockHeuristicModel {
        name: String,
        sample_pool: Vec<Solution<f64>>,
        solution: Option<Solution<f64>>,
        objective: Option<f64>,
        status: SolutionStatus,
    }

    impl MockHeuristicModel {
        fn new(sample_pool: Vec<Solution<f64>>) -> Self {
            Self {
                name: "mock_heuristic_model".to_string(),
                sample_pool,
                solution: None,
                objective: None,
                status: SolutionStatus::NotSolved,
            }
        }
    }

    #[async_trait]
    impl CallBackModelInterface<f64> for MockHeuristicModel {
        fn name(&self) -> &str {
            &self.name
        }

        async fn solve(&mut self) -> crate::error::Result<SolverOutput> {
            self.status = if self.solution.is_some() {
                SolutionStatus::Feasible
            } else {
                SolutionStatus::Infeasible
            };
            Ok(SolverOutput::new(crate::solver::SolverStatus::Optimal))
        }

        fn set_initial_solution(&mut self, solution: &Solution<f64>) {
            self.solution = Some(solution.clone());
            self.objective = self.calculate_objective();
        }

        fn get_solution(&self) -> Option<&Solution<f64>> {
            self.solution.as_ref()
        }

        fn get_status(&self) -> SolutionStatus {
            self.status
        }

        fn get_objective_value(&self) -> Option<f64> {
            self.objective
        }

        fn clear_solution(&mut self) {
            self.solution = None;
            self.objective = None;
            self.status = SolutionStatus::NotSolved;
        }
    }

    #[async_trait]
    impl AbstractCallBackModelInterface<f64> for MockHeuristicModel {
        fn num_variables(&self) -> usize {
            2
        }

        fn num_constraints(&self) -> usize {
            0
        }

        fn get_variable_name(&self, _id: VariableId) -> Option<&str> {
            None
        }

        fn get_variable_value(&self, id: VariableId) -> Option<f64> {
            self.solution.as_ref()?.get(&id).copied()
        }

        fn set_variable_value(&mut self, id: VariableId, value: f64) {
            self.solution
                .get_or_insert_with(Solution::new)
                .insert(id, value);
            self.objective = self.calculate_objective();
        }

        fn get_all_values(&self) -> HashMap<VariableId, f64> {
            self.solution.clone().unwrap_or_default()
        }

        fn calculate_objective(&self) -> Option<f64> {
            Some(self.solution.as_ref()?.values().copied().sum())
        }

        fn validate_solution(&self) -> bool {
            self.solution.is_some()
        }

        fn get_violated_constraints(&self) -> Vec<usize> {
            Vec::new()
        }
    }

    impl HeuristicCallBackModelInterface<f64> for MockHeuristicModel {
        fn initial_solutions(&self, initial_solution_amount: usize) -> Vec<Solution<f64>> {
            self.sample_pool
                .iter()
                .take(initial_solution_amount.min(self.sample_pool.len()))
                .cloned()
                .collect()
        }

        fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
            Some(solution.values().copied().sum())
        }
    }

    #[test]
    fn refresh_good_individuals_keeps_best_prefix() {
        let x = new_standalone_id();
        let model = MockHeuristicModel::new(Vec::new());
        let mut good = vec![
            HeuristicIndividual::new(HashMap::from([(x, 3.0)]), 3.0),
            HeuristicIndividual::new(HashMap::from([(x, 5.0)]), 5.0),
        ];
        let incoming = vec![
            HeuristicIndividual::new(HashMap::from([(x, 2.0)]), 2.0),
            HeuristicIndividual::new(HashMap::from([(x, 4.0)]), 4.0),
        ];

        refresh_good_individuals(&mut good, incoming, &model, 3);
        let fitness: Vec<f64> = good.iter().map(|individual| individual.fitness).collect();
        assert_eq!(fitness, vec![2.0, 3.0, 4.0]);
    }

    #[tokio::test]
    async fn sampling_heuristic_consumes_callback_model_initial_solutions() {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let pool = vec![
            HashMap::from([(x, 5.0), (y, 5.0)]),
            HashMap::from([(x, 1.0), (y, 1.0)]),
            HashMap::from([(x, 3.0), (y, 3.0)]),
        ];
        let mut model = MockHeuristicModel::new(pool);

        let algorithm = SamplingHeuristic::new(3, 2, 2);
        let mut policy = HeuristicPolicy::new().with_iteration_limit(2);
        let result = model
            .solve_with_heuristic(&algorithm, &mut policy)
            .await
            .expect("sampling heuristic should complete");

        assert_eq!(result.status, SolutionStatus::Feasible);
        assert_eq!(result.best_objective, Some(2.0));
        let best_x = result
            .best_solution
            .as_ref()
            .and_then(|solution| solution.get(&x))
            .copied();
        assert_eq!(best_x, Some(1.0));
    }
}
