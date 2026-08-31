//! 求解结果定义
//! Solver Output Definitions

use std::sync::Arc;
use std::time::Duration;

use crate::error::Result;

/// 求解状态 / Solver Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverStatus {
    /// 最优 / Optimal
    Optimal,
    /// 可行（非最优） / Feasible (non-optimal)
    Feasible,
    /// 不可行 / Infeasible
    Infeasible,
    /// 不可行或无界 / Infeasible or unbounded
    InfeasibleOrUnbounded,
    /// 无界 / Unbounded
    Unbounded,
    /// 达到迭代上限 / Iteration limit
    IterationLimit,
    /// 达到时间上限 / Time limit
    TimeLimit,
    /// 数值错误 / Numeric error
    NumericError,
    /// 未开始 / Not started
    NotStarted,
    /// 求解中 / Solving
    Solving,
    /// 用户中断 / User interrupt
    UserInterrupt,
    /// 未知 / Unknown
    Unknown,
}

impl SolverStatus {
    /// 检查是否找到最优解 / Check if optimal solution found
    pub fn is_optimal(&self) -> bool {
        matches!(self, SolverStatus::Optimal)
    }

    /// 检查是否可行 / Check if feasible
    pub fn is_feasible(&self) -> bool {
        matches!(
            self,
            SolverStatus::Optimal
                | SolverStatus::Feasible
                | SolverStatus::IterationLimit
                | SolverStatus::TimeLimit
        )
    }

    /// 检查是否不可行 / Check if infeasible
    pub fn is_infeasible(&self) -> bool {
        matches!(
            self,
            SolverStatus::Infeasible | SolverStatus::InfeasibleOrUnbounded
        )
    }

    /// 检查是否无界 / Check if unbounded
    pub fn is_unbounded(&self) -> bool {
        matches!(self, SolverStatus::Unbounded)
    }
}

/// 求解结果 / Solver Output
#[derive(Debug, Clone)]
pub struct SolverOutput {
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 目标值 / Objective value
    pub objective_value: Option<f64>,
    /// 解向量 / Solution vector
    pub solution: Option<Vec<f64>>,
    /// 对偶解 / Dual solution
    pub dual_solution: Option<Vec<f64>>,
    /// 二次约束对偶解 / Quadratic-constraint dual solution
    pub quadratic_dual_solution: Option<Vec<f64>>,
    /// 求解时间 / Solve time
    pub solve_time: Duration,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数（MIP）/ Node count (MIP)
    pub node_count: Option<usize>,
    /// MIP Gap / MIP Gap
    pub mip_gap: Option<f64>,
    /// 最优下界（MIP）/ Best bound (MIP)
    pub best_bound: Option<f64>,
}

impl SolverOutput {
    /// 创建新的求解结果 / Create new solver output
    pub fn new(status: SolverStatus) -> Self {
        Self {
            status,
            objective_value: None,
            solution: None,
            dual_solution: None,
            quadratic_dual_solution: None,
            solve_time: Duration::ZERO,
            iterations: None,
            node_count: None,
            mip_gap: None,
            best_bound: None,
        }
    }

    /// 创建最优解结果 / Create optimal solution output
    pub fn optimal(objective_value: f64, solution: Vec<f64>) -> Self {
        Self {
            status: SolverStatus::Optimal,
            objective_value: Some(objective_value),
            solution: Some(solution),
            dual_solution: None,
            quadratic_dual_solution: None,
            solve_time: Duration::ZERO,
            iterations: None,
            node_count: None,
            mip_gap: None,
            best_bound: None,
        }
    }

    /// 创建不可行结果 / Create infeasible output
    pub fn infeasible() -> Self {
        Self::new(SolverStatus::Infeasible)
    }

    /// 创建无界结果 / Create unbounded output
    pub fn unbounded() -> Self {
        Self::new(SolverStatus::Unbounded)
    }

    /// 设置目标值 / Set objective value
    pub fn with_objective(mut self, value: f64) -> Self {
        self.objective_value = Some(value);
        self
    }

    /// 设置解向量 / Set solution
    pub fn with_solution(mut self, solution: Vec<f64>) -> Self {
        self.solution = Some(solution);
        self
    }

    /// 设置对偶解 / Set dual solution
    pub fn with_dual(mut self, dual: Vec<f64>) -> Self {
        self.dual_solution = Some(dual);
        self
    }

    /// 设置二次约束对偶解 / Set quadratic-constraint dual solution
    pub fn with_quadratic_dual(mut self, dual: Vec<f64>) -> Self {
        self.quadratic_dual_solution = Some(dual);
        self
    }

    /// 设置求解时间 / Set solve time
    pub fn with_time(mut self, time: Duration) -> Self {
        self.solve_time = time;
        self
    }

    /// 设置迭代次数 / Set iteration count
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = Some(iterations);
        self
    }

    /// 检查是否有解 / Check if has solution
    pub fn has_solution(&self) -> bool {
        self.solution.is_some()
    }

    /// 获取解向量引用 / Get solution reference
    pub fn get_solution(&self) -> Option<&[f64]> {
        self.solution.as_deref()
    }
}

/// 求解状态快照 / Solving status snapshot
#[derive(Debug, Clone)]
pub struct SolvingStatus {
    /// 求解器名称 / Solver name
    pub solver: String,
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 当前目标值 / Current objective value
    pub objective_value: Option<f64>,
    /// 最优下界 / Best bound
    pub best_bound: Option<f64>,
    /// MIP Gap / MIP gap
    pub mip_gap: Option<f64>,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数 / Node count
    pub node_count: Option<usize>,
    /// 累计耗时 / Elapsed time
    pub solve_time: Duration,
}

impl SolvingStatus {
    /// 创建“求解中”状态 / Build "solving" status
    pub fn solving(solver: impl Into<String>) -> Self {
        Self {
            solver: solver.into(),
            status: SolverStatus::Solving,
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
            solve_time: Duration::ZERO,
        }
    }

    /// 从输出构造状态 / Build status from solver output
    pub fn from_output(solver: impl Into<String>, output: &SolverOutput) -> Self {
        Self {
            solver: solver.into(),
            status: output.status,
            objective_value: output.objective_value,
            best_bound: output.best_bound,
            mip_gap: output.mip_gap,
            iterations: output.iterations,
            node_count: output.node_count,
            solve_time: output.solve_time,
        }
    }
}

/// 求解状态回调 / Solving status callback
pub type SolvingStatusCallback = Arc<dyn Fn(&SolvingStatus) -> Result<()> + Send + Sync>;
