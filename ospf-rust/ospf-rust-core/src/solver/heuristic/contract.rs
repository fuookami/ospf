//! 求解器契约 / Solver contracts

use crate::model::callback::Solution;

use super::individual::Individual;
use super::iteration::Iteration;
use super::model::{HeuristicValue, ObjectiveModel};
use super::result::{HeuristicError, HeuristicResult, HeuristicSolutionStatus, Result};

/// 搜索骨架契约 / Search skeleton contract
///
/// 骨架只负责生命周期：初始化、推进一步、把状态摊平为个体。循环、档案、终止与观测者由 [`crate::solver::heuristic::runner::drive`]
/// 统一实现，骨架自身不重复这些逻辑。
/// A skeleton owns only the lifecycle: initialise, advance one step, and flatten the state into individuals. The loop,
/// archive, stopping, and observers are implemented once by [`crate::solver::heuristic::runner::drive`]; skeletons never
/// duplicate them.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub trait Runner<O, V>
where
    V: HeuristicValue,
{
    /// 运行搜索 / Run the search
    ///
    /// @param model 目标模型 / Objective model
    /// @param iteration 迭代计数器，随搜索推进 / Iteration counter, advanced by the search
    /// @return 档案中的个体，按优度排列 / Individuals in the archive, ordered by quality
    fn run<M: ObjectiveModel<O, V>>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<Vec<Individual<O, V>>>;
}

/// 启发式求解器契约 / Heuristic solver contract
///
/// 所有算法对外的统一入口：算法名、结果结构与状态语义由本接口固定，具体算法只提供骨架。
/// The unified external entry point of every algorithm: algorithm name, result shape, and status semantics are fixed
/// here; a concrete algorithm only supplies a skeleton.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub trait HeuristicSolver<O, V>
where
    V: HeuristicValue,
{
    /// 算法名 / Algorithm name
    fn name(&self) -> &str;

    /// 求解模型 / Solve the model
    fn solve<M: ObjectiveModel<O, V>>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<HeuristicResult<O, V>>;
}

/// 骨架驱动的求解器 / Skeleton-backed solver
///
/// 把任意 [`Runner`] 适配为 [`HeuristicSolver`]，并统一按 `constraint_satisfied` 判定结果状态。
/// Adapts any [`Runner`] into a [`HeuristicSolver`] and derives the result status from `constraint_satisfied`.
///
/// # 类型参数 / Type parameters
/// - `R`: 骨架类型 / Skeleton type
pub struct RunnerSolver<R> {
    name: String,
    runner: R,
}

impl<R> RunnerSolver<R> {
    /// 创建求解器 / Create a solver
    pub fn new(name: impl Into<String>, runner: R) -> Self {
        Self {
            name: name.into(),
            runner,
        }
    }

    /// 骨架引用 / Skeleton reference
    pub fn runner(&self) -> &R {
        &self.runner
    }

    /// 骨架可变引用 / Mutable skeleton reference
    pub fn runner_mut(&mut self) -> &mut R {
        &mut self.runner
    }

    /// 拆出骨架 / Take the skeleton out
    pub fn into_inner(self) -> R {
        self.runner
    }
}

impl<O, V, R> HeuristicSolver<O, V> for RunnerSolver<R>
where
    V: HeuristicValue,
    O: Clone,
    R: Runner<O, V>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn solve<M: ObjectiveModel<O, V>>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<HeuristicResult<O, V>> {
        let individuals = self.runner.run(model, iteration)?;
        let best = individuals.first().ok_or(HeuristicError::NoIndividual)?;
        let status = match model.constraint_satisfied(&best.solution) {
            Some(false) => HeuristicSolutionStatus::Infeasible,
            _ => HeuristicSolutionStatus::Feasible,
        };
        Ok(HeuristicResult::new(
            Some(best.solution.clone()),
            Some(best.fitness.clone()),
            status,
            iteration.clone(),
        ))
    }
}

/// 解值维度辅助 / Solution dimensionality helper
pub fn solution_dimension<V>(solution: &Solution<V>) -> usize {
    solution.len()
}
