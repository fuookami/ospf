//! 粒子群算法（PSO）求解器
//! Particle Swarm Optimization (PSO) Solver
//!
//! 说明：
//! - 该实现位于 `solver/solvers` 目录，作为 core 内置启发式求解器；
//! - 它复用 callback + heuristic 通用接口，不绑定具体 MILP/QP 后端。
//!
//! Notes:
//! - This implementation lives under `solver/solvers` as a built-in core heuristic solver;
//! - It reuses callback + heuristic generic interfaces and does not bind to a specific MILP/QP backend.

use crate::error::Result;
use crate::model::callback::{Solution, SolutionStatus};
use crate::solver::heuristic::{
    AbstractHeuristicPolicy, HeuristicAlgorithm, HeuristicCallBackModelInterface,
    HeuristicIndividual, HeuristicResult, HeuristicRuntimePolicy, Iteration,
    refresh_good_individuals,
};
use crate::variable::VariableId;
use async_trait::async_trait;
use num_traits::{Float, FromPrimitive};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// 随机数发生器 / Random number generator
pub type RandomGenerator = Arc<dyn Fn() -> f64 + Send + Sync>;
/// 初始速度发生器 / Initial velocity generator
pub type InitialVelocityGenerator<V> = Arc<dyn Fn(usize, VariableId) -> V + Send + Sync>;

/// 粒子 / Particle
#[derive(Debug, Clone)]
pub struct Particle<V> {
    /// 适应度（目标值）/ Fitness (objective value)
    pub fitness: V,
    /// 当前位置 / Current position
    pub position: Solution<V>,
    /// 当前速度 / Current velocity
    pub velocity: HashMap<VariableId, V>,
    /// 个体历史最好解 / Personal best
    pub current_best: Option<HeuristicIndividual<V>>,
}

impl<V> Particle<V> {
    /// 创建粒子 / Create particle
    pub fn new(
        fitness: V,
        position: Solution<V>,
        velocity: HashMap<VariableId, V>,
        current_best: Option<HeuristicIndividual<V>>,
    ) -> Self {
        Self {
            fitness,
            position,
            velocity,
            current_best,
        }
    }
}

/// 粒子群启发式求解器 / PSO heuristic solver
pub struct ParticleSwarmHeuristicSolver<V>
where
    V: Float + FromPrimitive + Send + Sync + 'static,
{
    /// 粒子数量 / Particle amount
    pub particle_amount: usize,
    /// 保留解数量 / Retained solution amount
    pub solution_amount: usize,
    /// 惯性系数 / Inertia coefficient
    pub w: V,
    /// 个体学习系数 / Personal learning coefficient
    pub c1: V,
    /// 群体学习系数 / Global learning coefficient
    pub c2: V,
    /// 最大速度 / Maximum velocity
    pub max_velocity: V,
    /// 当 objective hook 缺失时，是否回退调用 callback.solve() 评估适应度 / Whether to fallback to callback.solve() when objective hook is missing
    pub solve_on_objective_miss: bool,
    random_generator: RandomGenerator,
    initial_velocity_generator: InitialVelocityGenerator<V>,
}

impl<V> Debug for ParticleSwarmHeuristicSolver<V>
where
    V: Float + FromPrimitive + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParticleSwarmHeuristicSolver")
            .field("particle_amount", &self.particle_amount)
            .field("solution_amount", &self.solution_amount)
            .field("w", &self.w)
            .field("c1", &self.c1)
            .field("c2", &self.c2)
            .field("max_velocity", &self.max_velocity)
            .field("solve_on_objective_miss", &self.solve_on_objective_miss)
            .finish()
    }
}

impl<V> Clone for ParticleSwarmHeuristicSolver<V>
where
    V: Float + FromPrimitive + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            particle_amount: self.particle_amount,
            solution_amount: self.solution_amount,
            w: self.w,
            c1: self.c1,
            c2: self.c2,
            max_velocity: self.max_velocity,
            solve_on_objective_miss: self.solve_on_objective_miss,
            random_generator: self.random_generator.clone(),
            initial_velocity_generator: self.initial_velocity_generator.clone(),
        }
    }
}

impl<V> ParticleSwarmHeuristicSolver<V>
where
    V: Float + FromPrimitive + Send + Sync + 'static,
{
    /// 创建 PSO 求解器 / Create PSO solver
    pub fn new(
        particle_amount: usize,
        solution_amount: usize,
        w: V,
        c1: V,
        c2: V,
        max_velocity: V,
    ) -> Self {
        Self {
            particle_amount,
            solution_amount,
            w,
            c1,
            c2,
            max_velocity,
            solve_on_objective_miss: true,
            // 默认固定 0.5，保证无 `rand` feature 时依旧可运行
            // Default fixed 0.5 so it works without `rand` feature.
            random_generator: Arc::new(|| 0.5),
            initial_velocity_generator: Arc::new(|_, _| V::zero()),
        }
    }

    /// 设置随机数生成器 / Set random generator
    pub fn with_random_generator(mut self, random_generator: RandomGenerator) -> Self {
        self.random_generator = random_generator;
        self
    }

    /// 设置初始速度生成器 / Set initial velocity generator
    pub fn with_initial_velocity_generator(
        mut self,
        initial_velocity_generator: InitialVelocityGenerator<V>,
    ) -> Self {
        self.initial_velocity_generator = initial_velocity_generator;
        self
    }

    /// 设置在 objective hook 缺失时是否调用 callback.solve() / Configure whether callback.solve() is used when objective hook is missing
    pub fn with_solve_on_objective_miss(mut self, enabled: bool) -> Self {
        self.solve_on_objective_miss = enabled;
        self
    }

    fn sample_random(&self) -> V {
        V::from_f64((self.random_generator)()).unwrap_or_else(V::zero)
    }

    fn sorted_variable_ids(solution: &Solution<V>) -> Vec<VariableId> {
        let mut variable_ids: Vec<_> = solution.keys().copied().collect();
        variable_ids.sort_unstable_by(|lhs, rhs| {
            lhs.group_id
                .cmp(&rhs.group_id)
                .then(lhs.index_in_group.cmp(&rhs.index_in_group))
        });
        variable_ids
    }

    fn to_individual(particle: &Particle<V>) -> HeuristicIndividual<V> {
        HeuristicIndividual::new(particle.position.clone(), particle.fitness)
    }

    async fn evaluate_fitness<M>(&self, model: &mut M, solution: &Solution<V>) -> Option<V>
    where
        M: HeuristicCallBackModelInterface<V> + Send + Sync + ?Sized,
    {
        if let Some(fitness) = model.objective(solution) {
            return Some(fitness);
        }
        if self.solve_on_objective_miss {
            model.set_initial_solution(solution);
            if let Ok(output) = model.solve().await
                && output.status.is_feasible()
                && let Some(value) = model
                    .get_objective_value()
                    .or_else(|| output.objective_value.and_then(V::from_f64))
            {
                return Some(value);
            }
        }
        model.default_objective()
    }

    async fn initialize_particles<M>(&self, model: &mut M) -> Vec<Particle<V>>
    where
        M: HeuristicCallBackModelInterface<V> + Send + Sync + ?Sized,
    {
        let mut particles = Vec::new();
        for solution in model.initial_solutions(self.particle_amount.max(1)) {
            let Some(fitness) = self.evaluate_fitness(model, &solution).await else {
                continue;
            };
            let velocity = Self::sorted_variable_ids(&solution)
                .into_iter()
                .enumerate()
                .map(|(index, variable_id)| {
                    (
                        variable_id,
                        (self.initial_velocity_generator)(index, variable_id),
                    )
                })
                .collect();
            particles.push(Particle::new(fitness, solution, velocity, None));
        }
        particles
    }

    fn build_particle_from_solution(&self, solution: Solution<V>, fitness: V) -> Particle<V> {
        let velocity = Self::sorted_variable_ids(&solution)
            .into_iter()
            .enumerate()
            .map(|(index, variable_id)| {
                (
                    variable_id,
                    (self.initial_velocity_generator)(index, variable_id),
                )
            })
            .collect();
        Particle::new(fitness, solution, velocity, None)
    }

    async fn accelerate<M, P>(
        &self,
        iteration: &Iteration,
        particle: &Particle<V>,
        best_particle: &Particle<V>,
        model: &mut M,
        policy: &P,
    ) -> Particle<V>
    where
        M: HeuristicCallBackModelInterface<V> + Send + Sync + ?Sized,
        P: HeuristicRuntimePolicy<V, M> + ?Sized,
    {
        let local_best_position = particle
            .current_best
            .as_ref()
            .map(|best| &best.solution)
            .unwrap_or(&particle.position);

        let mut new_position = particle.position.clone();
        let mut new_velocity = particle.velocity.clone();
        let variable_ids = Self::sorted_variable_ids(&particle.position);

        for (index, variable_id) in variable_ids.into_iter().enumerate() {
            let position = particle
                .position
                .get(&variable_id)
                .copied()
                .unwrap_or_else(V::zero);
            let velocity = particle
                .velocity
                .get(&variable_id)
                .copied()
                .unwrap_or_else(V::zero);
            let local_best = local_best_position
                .get(&variable_id)
                .copied()
                .unwrap_or(position);
            let global_best = best_particle
                .position
                .get(&variable_id)
                .copied()
                .unwrap_or(position);

            let r1 = self.sample_random();
            let r2 = self.sample_random();
            let mut velocity_next = self.w * velocity
                + self.c1 * r1 * (local_best - position)
                + self.c2 * r2 * (global_best - position);
            if velocity_next > self.max_velocity {
                velocity_next = self.max_velocity;
            } else if velocity_next < -self.max_velocity {
                velocity_next = -self.max_velocity;
            }

            let position_next = policy.coerce_in(iteration, index, position + velocity_next, model);
            new_position.insert(variable_id, position_next);
            new_velocity.insert(variable_id, velocity_next);
        }

        let new_fitness = self
            .evaluate_fitness(model, &new_position)
            .await
            .unwrap_or(particle.fitness);
        let candidate_best = HeuristicIndividual::new(new_position.clone(), new_fitness);

        let baseline_best = particle.current_best.clone().unwrap_or_else(|| {
            HeuristicIndividual::new(particle.position.clone(), particle.fitness)
        });
        let retained_best = if model
            .compare_objective(&candidate_best.fitness, &baseline_best.fitness)
            == Ordering::Less
        {
            candidate_best.clone()
        } else {
            baseline_best
        };

        Particle::new(new_fitness, new_position, new_velocity, Some(retained_best))
    }
}

impl<V> Default for ParticleSwarmHeuristicSolver<V>
where
    V: Float + FromPrimitive + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(
            100,
            1,
            V::from_f64(0.4).unwrap_or_else(V::one),
            V::from_f64(2.0).unwrap_or_else(V::one),
            V::from_f64(2.0).unwrap_or_else(V::one),
            V::from_f64(10_000.0).unwrap_or_else(V::one),
        )
    }
}

#[async_trait]
impl<V, M> HeuristicAlgorithm<V, M> for ParticleSwarmHeuristicSolver<V>
where
    V: Float + FromPrimitive + Send + Sync + 'static,
    M: HeuristicCallBackModelInterface<V> + Send + Sync,
{
    fn name(&self) -> &str {
        "pso"
    }

    async fn run<P>(&self, model: &mut M, policy: &mut P) -> Result<HeuristicResult<V>>
    where
        P: AbstractHeuristicPolicy + HeuristicRuntimePolicy<V, M> + Send + Sync,
    {
        let mut iteration = Iteration::new();
        let keep_amount = self.solution_amount.max(1);
        let mut particles = self.initialize_particles(model).await;

        if particles.is_empty() {
            let output = model.solve().await?;
            if output.status.is_feasible()
                && let Some(fitness) = model
                    .get_objective_value()
                    .or_else(|| output.objective_value.and_then(V::from_f64))
                    .or_else(|| model.default_objective())
                && let Some(solution) = model.get_solution().cloned()
            {
                particles.push(self.build_particle_from_solution(solution, fitness));
            }
        }

        if particles.is_empty() {
            return Ok(HeuristicResult::from_model(model, iteration));
        }

        particles.sort_by(|lhs, rhs| model.compare_objective(&lhs.fitness, &rhs.fitness));
        let mut best_particle = particles
            .first()
            .cloned()
            .expect("particles is non-empty after initialization");

        let mut good_individuals: Vec<HeuristicIndividual<V>> =
            particles.iter().map(Self::to_individual).collect();
        good_individuals.sort_by(|lhs, rhs| model.compare_objective(&lhs.fitness, &rhs.fitness));
        good_individuals.truncate(keep_amount);

        while !policy.finished(&iteration) {
            let mut new_particles = Vec::with_capacity(particles.len());
            for particle in &particles {
                new_particles.push(
                    self.accelerate(&iteration, particle, &best_particle, model, policy)
                        .await,
                );
            }
            new_particles.sort_by(|lhs, rhs| model.compare_objective(&lhs.fitness, &rhs.fitness));

            let mut global_better = false;
            if let Some(new_best_particle) = new_particles.first()
                && model.compare_objective(&new_best_particle.fitness, &best_particle.fitness)
                    == Ordering::Less
            {
                best_particle = new_best_particle.clone();
                global_better = true;
            }

            let new_individuals: Vec<HeuristicIndividual<V>> =
                new_particles.iter().map(Self::to_individual).collect();
            refresh_good_individuals(
                &mut good_individuals,
                new_individuals.clone(),
                model,
                keep_amount,
            );

            let populations = vec![new_individuals];
            let best_individual = Self::to_individual(&best_particle);
            policy.update_with_context(
                &iteration,
                global_better,
                Some(&best_individual),
                &good_individuals,
                &populations,
                model,
            );
            policy.update(&iteration, global_better);
            iteration.next(global_better);
            model.flush();

            particles = new_particles;
        }

        model.set_initial_solution(&best_particle.position);
        Ok(HeuristicResult {
            best_solution: Some(best_particle.position),
            best_objective: Some(best_particle.fitness),
            status: SolutionStatus::Feasible,
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
    use crate::solver::heuristic::{
        HeuristicCallBackModelInterface, HeuristicModelExt, HeuristicPolicy,
    };
    use crate::solver::{SolverOutput, SolverStatus};
    use crate::variable::{VariableId, new_standalone_id};

    use super::ParticleSwarmHeuristicSolver;

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
                name: "mock_pso_model".to_string(),
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
                SolutionStatus::Unknown
            };
            Ok(SolverOutput::new(SolverStatus::Optimal))
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

    #[tokio::test]
    async fn pso_solver_consumes_callback_model_initial_solutions() {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let pool = vec![
            HashMap::from([(x, 5.0), (y, 5.0)]),
            HashMap::from([(x, 1.0), (y, 1.0)]),
            HashMap::from([(x, 3.0), (y, 3.0)]),
        ];
        let mut model = MockHeuristicModel::new(pool);

        let algorithm = ParticleSwarmHeuristicSolver::new(3, 2, 0.0, 0.0, 0.0, 10.0);
        let mut policy = HeuristicPolicy::new().with_iteration_limit(0);
        let result = model
            .solve_with_heuristic(&algorithm, &mut policy)
            .await
            .expect("pso solve should complete");

        assert_eq!(result.status, SolutionStatus::Feasible);
        assert_eq!(result.best_objective, Some(2.0));
        let best_x = result
            .best_solution
            .as_ref()
            .and_then(|solution| solution.get(&x))
            .copied();
        assert_eq!(best_x, Some(1.0));
    }

    #[tokio::test]
    async fn pso_solver_falls_back_to_callback_solve_chain_when_initial_solutions_missing() {
        let mut model = AbstractCallBackModel::<f64>::new("pso_callback_chain");
        let x = new_standalone_id();
        model.set_solve_executor(Arc::new(move |callback_model| {
            callback_model.set_solution_internal(HashMap::from([(x, 2.0)]));
            callback_model.set_objective_internal(Some(2.0));
            Ok(SolverOutput::optimal(2.0, vec![2.0]))
        }));

        let algorithm = ParticleSwarmHeuristicSolver::new(8, 1, 0.0, 0.0, 0.0, 10.0);
        let mut policy = HeuristicPolicy::new().with_iteration_limit(0);
        let result = model
            .solve_with_heuristic(&algorithm, &mut policy)
            .await
            .expect("pso solve should fallback to callback solve chain");

        assert_eq!(result.status, SolutionStatus::Feasible);
        assert_eq!(result.best_objective, Some(2.0));
        assert_eq!(
            result
                .best_solution
                .and_then(|solution| solution.get(&x).copied()),
            Some(2.0)
        );
    }

    struct SolveOnlyHeuristicModel {
        name: String,
        sample_pool: Vec<Solution<f64>>,
        solution: Option<Solution<f64>>,
        objective: Option<f64>,
        status: SolutionStatus,
    }

    impl SolveOnlyHeuristicModel {
        fn new(sample_pool: Vec<Solution<f64>>) -> Self {
            Self {
                name: "solve_only_heuristic_model".to_string(),
                sample_pool,
                solution: None,
                objective: None,
                status: SolutionStatus::NotSolved,
            }
        }
    }

    #[async_trait]
    impl CallBackModelInterface<f64> for SolveOnlyHeuristicModel {
        fn name(&self) -> &str {
            &self.name
        }

        async fn solve(&mut self) -> crate::error::Result<SolverOutput> {
            if let Some(solution) = self.solution.as_ref() {
                let objective = solution.values().copied().sum::<f64>();
                self.objective = Some(objective);
                self.status = SolutionStatus::Feasible;
                Ok(SolverOutput::optimal(objective, Vec::new()))
            } else {
                self.objective = None;
                self.status = SolutionStatus::Infeasible;
                Ok(SolverOutput::infeasible())
            }
        }

        fn set_initial_solution(&mut self, solution: &Solution<f64>) {
            self.solution = Some(solution.clone());
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
    impl AbstractCallBackModelInterface<f64> for SolveOnlyHeuristicModel {
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
        }

        fn get_all_values(&self) -> HashMap<VariableId, f64> {
            self.solution.clone().unwrap_or_default()
        }

        fn calculate_objective(&self) -> Option<f64> {
            self.objective
        }

        fn validate_solution(&self) -> bool {
            self.solution.is_some()
        }

        fn get_violated_constraints(&self) -> Vec<usize> {
            Vec::new()
        }
    }

    impl HeuristicCallBackModelInterface<f64> for SolveOnlyHeuristicModel {
        fn initial_solutions(&self, initial_solution_amount: usize) -> Vec<Solution<f64>> {
            self.sample_pool
                .iter()
                .take(initial_solution_amount.min(self.sample_pool.len()))
                .cloned()
                .collect()
        }
    }

    #[tokio::test]
    async fn pso_solver_uses_callback_solve_when_objective_hook_missing() {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let pool = vec![
            HashMap::from([(x, 5.0), (y, 5.0)]),
            HashMap::from([(x, 1.0), (y, 1.0)]),
            HashMap::from([(x, 3.0), (y, 3.0)]),
        ];
        let mut model = SolveOnlyHeuristicModel::new(pool);

        let algorithm = ParticleSwarmHeuristicSolver::new(3, 2, 0.0, 0.0, 0.0, 10.0)
            .with_solve_on_objective_miss(true);
        let mut policy = HeuristicPolicy::new().with_iteration_limit(0);
        let result = model
            .solve_with_heuristic(&algorithm, &mut policy)
            .await
            .expect("pso solve should evaluate fitness through callback solve");

        assert_eq!(result.status, SolutionStatus::Feasible);
        assert_eq!(result.best_objective, Some(2.0));
        let best_x = result
            .best_solution
            .as_ref()
            .and_then(|solution| solution.get(&x))
            .copied();
        assert_eq!(best_x, Some(1.0));
    }

    #[tokio::test]
    async fn pso_solver_runs_without_solver_executor_when_heuristic_hooks_are_registered() {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let mut model = AbstractCallBackModel::<f64>::new("pso_no_solver_executor");
        model.set_initial_solution_provider(Arc::new(move |amount| {
            let mut pool = vec![
                HashMap::from([(x, 5.0), (y, 5.0)]),
                HashMap::from([(x, 1.0), (y, 1.0)]),
                HashMap::from([(x, 3.0), (y, 3.0)]),
            ];
            pool.truncate(amount.min(pool.len()));
            pool
        }));
        model.set_objective_evaluator(Arc::new(|solution| Some(solution.values().copied().sum())));
        model.set_value_coercer(Arc::new(|_index, value| value.clamp(0.0, 10.0)));

        let algorithm = ParticleSwarmHeuristicSolver::new(3, 2, 0.0, 0.0, 0.0, 10.0)
            .with_solve_on_objective_miss(false);
        let mut policy = HeuristicPolicy::new().with_iteration_limit(0);
        let result = model
            .solve_with_heuristic(&algorithm, &mut policy)
            .await
            .expect("pso should run in pure heuristic mode without solver executor");

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
