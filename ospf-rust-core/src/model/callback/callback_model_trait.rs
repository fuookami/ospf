//! 回调模型接口 Trait
//! Callback Model Interface Trait

use super::solution::{Solution, SolutionStatus, SolutionWithStatus};
use crate::solver::SolverOutput;
use crate::variable::VariableId;
use async_trait::async_trait;
use std::collections::HashMap;

/// 回调模型接口 / Callback Model Interface
///
/// 提供求解器回调功能的基础接口。
/// Provides base interface for solver callback functionality.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
#[async_trait]
pub trait CallBackModelInterface<V>: Send + Sync {
    /// 获取模型名称 / Get model name
    fn name(&self) -> &str;

    /// 求解模型 / Solve model
    async fn solve(&mut self) -> crate::error::Result<SolverOutput>;

    /// 设置初始解 / Set initial solution
    fn set_initial_solution(&mut self, solution: &Solution<V>);

    /// 获取当前解 / Get current solution
    fn get_solution(&self) -> Option<&Solution<V>>;

    /// 获取解状态 / Get solution status
    fn get_status(&self) -> SolutionStatus;

    /// 获取目标值 / Get objective value
    fn get_objective_value(&self) -> Option<V>;

    /// 清除解 / Clear solution
    fn clear_solution(&mut self);

    /// 是否已求解 / Is solved
    fn is_solved(&self) -> bool {
        !matches!(self.get_status(), SolutionStatus::NotSolved)
    }

    /// 是否可行 / Is feasible
    fn is_feasible(&self) -> bool {
        matches!(
            self.get_status(),
            SolutionStatus::Optimal | SolutionStatus::Feasible
        )
    }

    /// 是否最优 / Is optimal
    fn is_optimal(&self) -> bool {
        self.get_status() == SolutionStatus::Optimal
    }

    /// 回调：求解开始前 / Callback: before solve starts
    fn on_before_solve(&mut self) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }

    /// 回调：求解结束后 / Callback: after solve finishes
    fn on_after_solve(&mut self, _result: &SolverOutput) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }

    /// 回调：找到新解时 / Callback: when new solution is found
    fn on_new_solution(&mut self, _solution: &Solution<V>, _objective: V) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }

    /// 回调：找到更优解时 / Callback: when better solution is found
    fn on_better_solution(&mut self, _solution: &Solution<V>, _objective: V) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }
}

/// 抽象回调模型接口 / Abstract Callback Model Interface
///
/// 扩展 `CallBackModelInterface`，提供更多高级功能。
/// Extends `CallBackModelInterface`, providing more advanced features.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
#[async_trait]
pub trait AbstractCallBackModelInterface<V: Clone>: CallBackModelInterface<V> {
    /// 获取变量数量 / Get variable count
    fn num_variables(&self) -> usize;

    /// 获取约束数量 / Get constraint count
    fn num_constraints(&self) -> usize;

    /// 获取变量名 / Get variable name
    fn get_variable_name(&self, id: VariableId) -> Option<&str>;

    /// 获取变量值 / Get variable value
    fn get_variable_value(&self, id: VariableId) -> Option<V>;

    /// 设置变量值 / Set variable value
    fn set_variable_value(&mut self, id: VariableId, value: V);

    /// 获取所有变量值 / Get all variable values
    fn get_all_values(&self) -> HashMap<VariableId, V>;

    /// 批量设置变量值 / Set multiple variable values
    fn set_values(&mut self, values: &HashMap<VariableId, V>) {
        for (id, value) in values {
            self.set_variable_value(*id, value.clone());
        }
    }

    /// 计算目标值 / Calculate objective value
    fn calculate_objective(&self) -> Option<V>;

    /// 验证解是否可行 / Validate if solution is feasible
    fn validate_solution(&self) -> bool;

    /// 获取违反的约束 / Get violated constraints
    fn get_violated_constraints(&self) -> Vec<usize>;

    /// 导出解 / Export solution
    fn export_solution(&self) -> SolutionWithStatus<V> {
        let solution = self.get_solution().cloned().unwrap_or_default();
        let status = self.get_status();
        let objective_value = self.get_objective_value();

        SolutionWithStatus {
            solution,
            status,
            objective_value,
        }
    }

    /// 回调：变量值改变时 / Callback: when variable value changes
    fn on_variable_changed(&mut self, _id: VariableId, _old_value: Option<V>, _new_value: V) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }

    /// 回调：约束违反时 / Callback: when constraint is violated
    fn on_constraint_violated(&mut self, _constraint_index: usize, _violation: V) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }
}

/// 多目标优化模型接口 / Multi-Objective Optimization Model Interface
///
/// 支持多目标优化的回调模型接口。
/// Callback model interface supporting multi-objective optimization.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
#[async_trait]
pub trait MultiObjectiveModelInterface<V: Clone>: AbstractCallBackModelInterface<V> {
    /// 获取目标数量 / Get objective count
    fn num_objectives(&self) -> usize;

    /// 获取各目标值 / Get all objective values
    fn get_all_objectives(&self) -> Vec<Option<V>>;

    /// 获取指定目标值 / Get specific objective value
    fn get_objective(&self, index: usize) -> Option<V>;

    /// 设置目标权重 / Set objective weights
    fn set_objective_weights(&mut self, weights: Vec<V>);

    /// 获取目标权重 / Get objective weights
    fn get_objective_weights(&self) -> &[V];

    /// 计算加权目标值 / Calculate weighted objective value
    fn calculate_weighted_objective(&self) -> Option<V>
    where
        V: Clone
            + Default
            + for<'a> std::ops::Add<&'a V, Output = V>
            + for<'a> std::ops::Mul<&'a V, Output = V>,
    {
        let objectives = self.get_all_objectives();
        let weights = self.get_objective_weights();

        if objectives.len() != weights.len() {
            return None;
        }

        let mut total = V::default();
        for (obj, weight) in objectives.iter().zip(weights.iter()) {
            if let Some(obj_val) = obj {
                total = total + &(obj_val.clone() * weight);
            }
        }
        Some(total)
    }

    /// 获取帕累托前沿解 / Get Pareto front solutions
    fn get_pareto_front(&self) -> Vec<SolutionWithStatus<V>>;

    /// 添加帕累托解 / Add Pareto solution
    fn add_pareto_solution(&mut self, solution: SolutionWithStatus<V>);

    /// 清除帕累托前沿 / Clear Pareto front
    fn clear_pareto_front(&mut self);

    /// 回调：找到帕累托解时 / Callback: when Pareto solution is found
    fn on_pareto_solution_found(&mut self, _solution: &SolutionWithStatus<V>) {
        // 默认实现：什么都不做 / Default implementation: do nothing
    }
}
