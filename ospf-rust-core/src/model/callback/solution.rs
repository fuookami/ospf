//! Solution 类型定义
//! Solution Type Definition

use std::collections::HashMap;
use crate::variable::VariableId;

/// 解向量 / Solution Vector
///
/// 表示变量的解值映射。
/// Represents the mapping of variable values in a solution.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
pub type Solution<V> = HashMap<VariableId, V>;

/// 部分解 / Partial Solution
///
/// 表示部分变量的解值映射。
/// Represents the mapping of partial variable values.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
pub type PartialSolution<V> = HashMap<VariableId, V>;

/// 解状态 / Solution Status
///
/// 表示解的状态。
/// Represents the status of a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionStatus {
    /// 最优 / Optimal
    Optimal,
    /// 可行 / Feasible
    Feasible,
    /// 不可行 / Infeasible
    Infeasible,
    /// 无界 / Unbounded
    Unbounded,
    /// 未求解 / NotSolved
    NotSolved,
    /// 未知 / Unknown
    Unknown,
}

impl Default for SolutionStatus {
    fn default() -> Self {
        Self::NotSolved
    }
}

/// 带状态的解 / Solution with Status
///
/// 包含解向量和状态的完整解表示。
/// Complete solution representation containing solution vector and status.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
#[derive(Debug, Clone)]
pub struct SolutionWithStatus<V> {
    /// 解向量 / Solution vector
    pub solution: Solution<V>,
    /// 解状态 / Solution status
    pub status: SolutionStatus,
    /// 目标值 / Objective value
    pub objective_value: Option<V>,
}

impl<V> Default for SolutionWithStatus<V> {
    fn default() -> Self {
        Self {
            solution: Solution::new(),
            status: SolutionStatus::default(),
            objective_value: None,
        }
    }
}

impl<V> SolutionWithStatus<V> {
    /// 创建新解 / Create new solution
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带状态的解 / Create solution with status
    pub fn with_status(solution: Solution<V>, status: SolutionStatus) -> Self {
        Self {
            solution,
            status,
            objective_value: None,
        }
    }

    /// 创建最优解 / Create optimal solution
    pub fn optimal(solution: Solution<V>, objective_value: V) -> Self {
        Self {
            solution,
            status: SolutionStatus::Optimal,
            objective_value: Some(objective_value),
        }
    }

    /// 创建可行解 / Create feasible solution
    pub fn feasible(solution: Solution<V>) -> Self {
        Self {
            solution,
            status: SolutionStatus::Feasible,
            objective_value: None,
        }
    }

    /// 创建不可行解 / Create infeasible solution
    pub fn infeasible() -> Self {
        Self {
            solution: Solution::new(),
            status: SolutionStatus::Infeasible,
            objective_value: None,
        }
    }

    /// 是否最优 / Is optimal
    pub fn is_optimal(&self) -> bool {
        self.status == SolutionStatus::Optimal
    }

    /// 是否可行 / Is feasible
    pub fn is_feasible(&self) -> bool {
        matches!(
            self.status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        )
    }

    /// 设置目标值 / Set objective value
    pub fn with_objective(mut self, value: V) -> Self {
        self.objective_value = Some(value);
        self
    }
}

// 类型别名 / Type Aliases
/// f64 精度的解 / Solution with f64 precision
pub type SolutionF64 = Solution<f64>;

/// f64 精度的带状态解 / Solution with status with f64 precision
pub type SolutionWithStatusF64 = SolutionWithStatus<f64>;
