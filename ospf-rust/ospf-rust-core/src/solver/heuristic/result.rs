//! 启发式求解结果与错误 / Heuristic solve result and errors

use crate::model::callback::Solution;

use super::iteration::Iteration;

/// 启发式求解状态 / Heuristic solution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeuristicSolutionStatus {
    /// 可行 / Feasible
    Feasible,
    /// 不可行 / Infeasible
    Infeasible,
}

/// 启发式求解结果 / Heuristic solve result
///
/// 所有启发式求解器共享的统一返回结构：最优解、最优目标、状态与迭代快照。
/// The uniform return of every heuristic solver: best solution, best objective, status, and iteration snapshot.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 解值类型 / Solution value type
#[derive(Debug, Clone)]
pub struct HeuristicResult<O, V> {
    /// 最优解 / Best solution
    pub best_solution: Option<Solution<V>>,
    /// 最优目标值 / Best objective
    pub best_objective: Option<O>,
    /// 求解状态 / Solve status
    pub status: HeuristicSolutionStatus,
    /// 迭代信息 / Iteration snapshot
    pub iteration: Iteration,
}

impl<O, V> HeuristicResult<O, V> {
    /// 创建求解结果 / Create a solve result
    pub fn new(
        best_solution: Option<Solution<V>>,
        best_objective: Option<O>,
        status: HeuristicSolutionStatus,
        iteration: Iteration,
    ) -> Self {
        Self {
            best_solution,
            best_objective,
            status,
            iteration,
        }
    }
}

/// 启发式求解错误 / Heuristic solve error
#[derive(Debug, thiserror::Error)]
pub enum HeuristicError {
    /// 参数非法 / Invalid argument
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// 搜索未产生任何个体 / The search produced no individual
    #[error("the search produced no individual")]
    NoIndividual,
    /// 模型错误 / Model error
    #[error("model error: {0}")]
    Model(String),
    /// 预算耗尽 / Budget exhausted
    #[error("search budget exhausted")]
    BudgetExhausted,
}

/// 启发式模块结果别名 / Result alias of the heuristic module
pub type Result<T> = std::result::Result<T, HeuristicError>;

/// 多目标求解结果 / Multi-objective solve result
///
/// 多目标没有「唯一最优」，因此返回的是一整条前沿，而不是单个解。
/// Multi-objective problems have no single optimum, so the result is an entire front rather than one solution.
#[derive(Debug, Clone)]
pub struct MultiObjectiveResult<V>
where
    V: crate::solver::heuristic::model::HeuristicValue,
{
    front: Vec<crate::model::callback::Solution<V>>,
    objectives: Vec<Vec<f64>>,
    iteration: Iteration,
}

impl<V> MultiObjectiveResult<V>
where
    V: crate::solver::heuristic::model::HeuristicValue,
{
    /// 创建多目标结果 / Create a multi-objective result
    pub fn new(
        front: Vec<crate::model::callback::Solution<V>>,
        objectives: Vec<Vec<f64>>,
        iteration: Iteration,
    ) -> Self {
        Self {
            front,
            objectives,
            iteration,
        }
    }

    /// 前沿上的解 / Solutions on the front
    pub fn front(&self) -> &[crate::model::callback::Solution<V>] {
        &self.front
    }

    /// 与前沿一一对应的目标向量 / Objective vectors aligned with the front
    pub fn objectives(&self) -> &[Vec<f64>] {
        &self.objectives
    }

    /// 迭代信息 / Iteration info
    pub fn iteration(&self) -> &Iteration {
        &self.iteration
    }
}

/// 多目标求解器契约 / Multi-objective solver contract
pub trait MultiObjectiveSolver<V>
where
    V: crate::solver::heuristic::model::HeuristicValue,
{
    /// 算法名 / Algorithm name
    fn name(&self) -> &str;

    /// 求解模型 / Solve the model
    fn solve<M>(
        &mut self,
        model: &mut M,
        iteration: &mut Iteration,
    ) -> Result<MultiObjectiveResult<V>>
    where
        M: crate::solver::heuristic::model::VariableModel<V>;
}

