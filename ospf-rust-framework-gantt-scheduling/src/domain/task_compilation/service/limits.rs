//! 任务编译限制与目标族 / Task compilation limits and objective families
//!
//! 每个限制实现 `Pipeline<MetaModel<f64>>` trait。
//! 由于 `Pipeline::invoke` 接收 `&M`（不可变引用），约束和目标的注册在 `register(&mut M)` 中完成。
//!
//! Each limit implements `Pipeline<MetaModel<f64>>` trait.
//! Since `Pipeline::invoke` receives `&M` (immutable reference),
//! constraint and objective registration is done in `register(&mut M)`.

use std::sync::Arc;
use ospf_rust_core::error::Result;
use ospf_rust_core::model::flatten::Linear;
use ospf_rust_core::model::flatten::LinearMonomial;
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::LinearIntermediateSymbol;
use ospf_rust_core::variable::UContinuous;
use ospf_rust_framework::model::pipeline::Pipeline;

use super::super::model::{Compilation, Switch};
use crate::domain::task::{AssignmentPolicyTrait, ExecutorTrait, TaskTrait};

// ============================================================================
// 辅助函数 / Helper Functions
// ============================================================================

/// 从 LinearExpressionSymbol 列表展开为多项式系数列表 / Expand LinearExpressionSymbol list to polynomial coefficient lists
///
/// 每个符号的 `to_linear_polynomial()` 返回 `Linear<f64>`，
/// 展开为 `Vec<(usize, f64)>` 即 `(var_index, coefficient)` 对列表。
///
/// Each symbol's `to_linear_polynomial()` returns `Linear<f64>`,
/// expanded to `Vec<(usize, f64)>` i.e. `(var_index, coefficient)` pair list.
fn expand_symbols_to_polynomials(symbols: &[Arc<LinearExpressionSymbol<f64>>]) -> Vec<Vec<(usize, f64)>> {
    symbols.iter()
        .map(|s| {
            let poly = s.as_ref().to_linear_polynomial();
            poly.monomials().iter()
                .map(|m: &ospf_rust_core::model::flatten::LinearMonomial<f64>| (m.var_index(), *m.coefficient()))
                .collect()
        })
        .collect()
}

// ============================================================================
// 约束型 Pipeline / Constraint Pipelines
// ============================================================================

/// 任务编译约束 / Task compilation constraint
///
/// 确保每个任务被恰好分配到一个执行器或被取消：`task_compilation[task] == 1`。
/// `task_compilation[task]` 是中间表达式 `sum(x[task, executor]) + y[task]`，
/// 约束展开后为 `sum(x[task, executor]) + y[task] == 1`。
///
/// Ensures each task is either assigned to exactly one executor or canceled.
/// `task_compilation[task]` is the intermediate expression `sum(x[task, executor]) + y[task]`,
/// expanded constraint is `sum(x[task, executor]) + y[task] == 1`.
#[derive(Debug)]
pub struct TaskCompilationConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 每个任务的编译多项式项列表 / Polynomial terms for each task's compilation
    /// task_compilation_polynomials[task] = [(var_idx, coefficient), ...]
    ///
    /// 注意：这是从已注册 `LinearExpressionSymbol` 展开的系数，不是独立的裸系数。
    /// 注册到模型的符号是源，这里只是 `add_linear_constraint` API 的适配层。
    /// Note: These are coefficients expanded from registered `LinearExpressionSymbol`s,
    /// not independent raw coefficients. The registered symbols are the source of truth;
    /// this is the adapter layer for `add_linear_constraint` API.
    pub task_compilation_polynomials: Vec<Vec<(usize, f64)>>,
}

impl TaskCompilationConstraint {
    /// 从 LinearExpressionSymbol 列表创建约束 / Create constraint from LinearExpressionSymbol list
    ///
    /// 从 Compilation 的 `task_compilation_symbols` 展开多项式系数。
    /// Expands polynomial coefficients from Compilation's `task_compilation_symbols`.
    pub fn from_symbols(symbols: &[Arc<LinearExpressionSymbol<f64>>]) -> Self {
        Self {
            name: "task_compilation_constraint".to_string(),
            group: None,
            task_compilation_polynomials: expand_symbols_to_polynomials(symbols),
        }
    }

    /// 从 Compilation 组件创建约束 / Create constraint from Compilation component
    ///
    /// 便捷构造器，直接从 Compilation 的中间符号展开多项式。
    /// Convenience constructor, expands polynomials directly from Compilation's intermediate symbols.
    pub fn from_compilation<T, E, A>(compilation: &Compilation<T, E, A>) -> Self
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        Self::from_symbols(&compilation.task_compilation_symbols)
    }
}

impl Pipeline<MetaModel<f64>> for TaskCompilationConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (ti, terms) in self.task_compilation_polynomials.iter().enumerate() {
            if terms.is_empty() {
                log::warn!("Task {} has empty compilation polynomial, skipping constraint", ti);
                continue;
            }
            if let Err(e) = model.add_eq_constraint(
                terms,
                1.0,
                &format!("{}_{}", self.name, ti),
            ) {
                log::warn!("Failed to register constraint {}: {:?}", format!("{}_{}", self.name, ti), e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 执行器编译约束 / Executor compilation constraint
///
/// 确保每个执行器被使用或空闲：`executor_compilation[executor] == 1`。
/// `executor_compilation[executor]` 是中间表达式 `sum(x[task, executor]) + z[executor]`，
/// 约束展开后为 `sum(x[task, executor]) + z[executor] == 1`。
///
/// Ensures each executor is used or idle.
/// `executor_compilation[executor]` is the intermediate expression `sum(x[task, executor]) + z[executor]`,
/// expanded constraint is `sum(x[task, executor]) + z[executor] == 1`.
#[derive(Debug)]
pub struct ExecutorCompilationConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 每个执行器的编译多项式项列表 / Polynomial terms for each executor's compilation
    /// executor_compilation_polynomials[executor] = [(var_idx, coefficient), ...]
    ///
    /// 从已注册 `LinearExpressionSymbol` 展开，是 `add_linear_constraint` API 适配层。
    /// Expanded from registered `LinearExpressionSymbol`s; adapter layer for `add_linear_constraint`.
    pub executor_compilation_polynomials: Vec<Vec<(usize, f64)>>,
}

impl ExecutorCompilationConstraint {
    /// 从 LinearExpressionSymbol 列表创建约束 / Create constraint from LinearExpressionSymbol list
    pub fn from_symbols(symbols: &[Arc<LinearExpressionSymbol<f64>>]) -> Self {
        Self {
            name: "executor_compilation_constraint".to_string(),
            group: None,
            executor_compilation_polynomials: expand_symbols_to_polynomials(symbols),
        }
    }

    /// 从 Compilation 组件创建约束 / Create constraint from Compilation component
    pub fn from_compilation<T, E, A>(compilation: &Compilation<T, E, A>) -> Self
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        Self::from_symbols(&compilation.executor_compilation_symbols)
    }
}

impl Pipeline<MetaModel<f64>> for ExecutorCompilationConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (ei, terms) in self.executor_compilation_polynomials.iter().enumerate() {
            if terms.is_empty() {
                log::warn!("Executor {} has empty compilation polynomial, skipping constraint", ei);
                continue;
            }
            if let Err(e) = model.add_eq_constraint(
                terms,
                1.0,
                &format!("{}_{}", self.name, ei),
            ) {
                log::warn!("Failed to register constraint {}: {:?}", format!("{}_{}", self.name, ei), e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务冲突约束 / Task conflict constraint
///
/// 防止两个冲突任务分配到同一执行器：`x[i,e] + x[j,e] <= 1`。
/// Prevents two conflicting tasks from being assigned to the same executor.
pub struct TaskConflictConstraint {
    name: String,
    /// 冲突对：(task_i, task_j, executor) / Conflict pairs: (task_i, task_j, executor)
    pub conflict_pairs: Vec<(usize, usize, usize)>,
    /// x[task, executor] 的模型索引获取器 / x[task, executor] model index accessor
    pub x_model_index: Arc<dyn Fn(usize, usize) -> Option<usize> + Send + Sync>,
}

impl std::fmt::Debug for TaskConflictConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskConflictConstraint")
            .field("name", &self.name)
            .field("conflict_pairs", &self.conflict_pairs)
            .finish()
    }
}

impl TaskConflictConstraint {
    pub fn new(
        conflict_pairs: Vec<(usize, usize, usize)>,
        x_model_index: Arc<dyn Fn(usize, usize) -> Option<usize> + Send + Sync>,
    ) -> Self {
        Self {
            name: "task_conflict_constraint".to_string(),
            conflict_pairs,
            x_model_index,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskConflictConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        for &(ti, tj, ei) in &self.conflict_pairs {
            let idx_i = (self.x_model_index)(ti, ei);
            let idx_j = (self.x_model_index)(tj, ei);
            if let (Some(i), Some(j)) = (idx_i, idx_j) {
                if let Err(e) = model.add_le_constraint(
                    &[(i, 1.0), (j, 1.0)], 1.0,
                    &format!("{}_{}_{}_{}", self.name, ti, tj, ei),
                ) {
                    log::warn!("Failed to register constraint: {:?}", e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务时间冲突约束 / Task time conflict constraint
///
/// 防止时间重叠的任务分配到同一执行器。
/// Prevents time-overlapping tasks from being assigned to the same executor.
pub struct TaskTimeConflictConstraint {
    name: String,
    /// 时间重叠对：(task_i, task_j, executor) / Time overlap pairs: (task_i, task_j, executor)
    pub overlap_pairs: Vec<(usize, usize, usize)>,
    /// x[task, executor] 的模型索引获取器 / x[task, executor] model index accessor
    pub x_model_index: Arc<dyn Fn(usize, usize) -> Option<usize> + Send + Sync>,
}

impl std::fmt::Debug for TaskTimeConflictConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskTimeConflictConstraint")
            .field("name", &self.name)
            .field("overlap_pairs", &self.overlap_pairs)
            .finish()
    }
}

impl TaskTimeConflictConstraint {
    pub fn new(
        overlap_pairs: Vec<(usize, usize, usize)>,
        x_model_index: Arc<dyn Fn(usize, usize) -> Option<usize> + Send + Sync>,
    ) -> Self {
        Self {
            name: "task_time_conflict_constraint".to_string(),
            overlap_pairs,
            x_model_index,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskTimeConflictConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        for &(ti, tj, ei) in &self.overlap_pairs {
            let idx_i = (self.x_model_index)(ti, ei);
            let idx_j = (self.x_model_index)(tj, ei);
            if let (Some(i), Some(j)) = (idx_i, idx_j) {
                if let Err(e) = model.add_le_constraint(
                    &[(i, 1.0), (j, 1.0)], 1.0,
                    &format!("{}_{}_{}_{}", self.name, ti, tj, ei),
                ) {
                    log::warn!("Failed to register constraint: {:?}", e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务延迟时间约束 / Task delay time constraint
///
/// 约束任务不能晚于排程时间开始：`est[task] <= scheduled_start`。
/// Constrains tasks to not start later than scheduled time.
#[derive(Debug)]
pub struct TaskDelayTimeConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (est_model_index, scheduled_start_value) 列表 / List
    pub constraints: Vec<(usize, f64)>,
}

impl TaskDelayTimeConstraint {
    pub fn new(constraints: Vec<(usize, f64)>) -> Self {
        Self {
            name: "task_delay_time_constraint".to_string(),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskDelayTimeConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (i, &(est_idx, scheduled_start)) in self.constraints.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(
                &[(est_idx, 1.0)], scheduled_start,
                &format!("{}_{}", self.name, i),
            ) {
                log::warn!("Failed to register constraint: {:?}", e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务提前时间约束 / Task advance time constraint
///
/// 约束任务不能早于排程时间开始：`est[task] >= scheduled_start`。
/// Constrains tasks to not start earlier than scheduled time.
#[derive(Debug)]
pub struct TaskAdvanceTimeConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (est_model_index, scheduled_start_value) 列表 / List
    pub constraints: Vec<(usize, f64)>,
}

impl TaskAdvanceTimeConstraint {
    pub fn new(constraints: Vec<(usize, f64)>) -> Self {
        Self {
            name: "task_advance_time_constraint".to_string(),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskAdvanceTimeConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (i, &(est_idx, scheduled_start)) in self.constraints.iter().enumerate() {
            if let Err(e) = model.add_ge_constraint(
                &[(est_idx, 1.0)], scheduled_start,
                &format!("{}_{}", self.name, i),
            ) {
                log::warn!("Failed to register constraint: {:?}", e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 目标型 Pipeline / Objective Pipelines
// ============================================================================

/// 任务执行器成本最小化 / Task executor cost minimization
///
/// 最小化 `sum(cost[task, executor] * x[task, executor])`。
/// Minimizes total task-executor assignment cost.
#[derive(Debug)]
pub struct TaskExecutorCostMinimization {
    name: String,
    /// (x_model_index, cost) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl TaskExecutorCostMinimization {
    pub fn new(cost_terms: Vec<(usize, f64)>) -> Self {
        Self {
            name: "task_executor_cost_minimization".to_string(),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskExecutorCostMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, cost)| LinearMonomial::new(cost, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务取消成本最小化 / Task cancellation cost minimization
///
/// 最小化 `sum(cost[task] * y[task])`。
/// Minimizes total task cancellation cost.
#[derive(Debug)]
pub struct TaskCostMinimization {
    name: String,
    /// (y_model_index, cost) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl TaskCostMinimization {
    pub fn new(cost_terms: Vec<(usize, f64)>) -> Self {
        Self {
            name: "task_cost_minimization".to_string(),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskCostMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, cost)| LinearMonomial::new(cost, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 完工时间最小化 / Makespan minimization
///
/// 最小化 `coefficient * makespan`。
/// Minimizes `coefficient * makespan`.
#[derive(Debug)]
pub struct MakespanMinimization {
    name: String,
    /// makespan 结果变量的模型索引 / Makespan result variable model index
    pub makespan_model_index: usize,
    /// 目标系数 / Objective coefficient
    pub coefficient: f64,
}

impl MakespanMinimization {
    pub fn new(makespan_model_index: usize, coefficient: f64) -> Self {
        Self {
            name: "makespan_minimization".to_string(),
            makespan_model_index,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for MakespanMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        let polynomial = Linear::new(
            vec![LinearMonomial::new(self.coefficient, self.makespan_model_index)],
            0.0,
        );
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 切换成本最小化 / Switch cost minimization
///
/// 最小化 `sum(coefficient[e,i,j] * switch[e,i,j])`。
/// Minimizes `sum(coefficient[e,i,j] * switch[e,i,j])`.
#[derive(Debug)]
pub struct SwitchCostMinimization {
    name: String,
    /// (switch_model_index, coefficient) 列表 / (switch_model_index, coefficient) list
    pub cost_terms: Vec<(usize, f64)>,
}

impl SwitchCostMinimization {
    /// 创建新的切换成本目标 / Create new switch-cost objective
    pub fn new(cost_terms: Vec<(usize, f64)>) -> Self {
        Self {
            name: "switch_cost_minimization".to_string(),
            cost_terms,
        }
    }

    /// 从 Switch 组件创建目标 / Create objective from Switch component
    pub fn from_switch(switch: &Switch, coefficient: f64) -> Self {
        let cost_terms = switch.switch_model_indices.iter()
            .filter_map(|index| index.map(|idx| (idx, coefficient)))
            .collect();
        Self::new(cost_terms)
    }
}

impl Pipeline<MetaModel<f64>> for SwitchCostMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 切换时间最小化 / Switch time minimization
///
/// 最小化 `sum(coefficient[i,j] * switch_time[i,j])`。
/// Minimizes `sum(coefficient[i,j] * switch_time[i,j])`.
#[derive(Debug)]
pub struct SwitchTimeMinimization {
    name: String,
    /// 切换时间目标多项式 / Switch-time objective polynomial
    pub polynomial: Linear<f64>,
    /// 阈值 / Threshold
    pub threshold: f64,
    /// 阈值松弛目标系数 / Threshold slack objective coefficient
    pub threshold_coefficient: f64,
}

impl SwitchTimeMinimization {
    /// 创建新的切换时间目标 / Create new switch-time objective
    pub fn new(polynomial: Linear<f64>) -> Self {
        Self {
            name: "switch_time_minimization".to_string(),
            polynomial,
            threshold: 0.0,
            threshold_coefficient: 1.0,
        }
    }

    /// 创建带阈值的切换时间目标 / Create switch-time objective with threshold
    ///
    /// 当阈值大于零时，目标最小化 `max(0, switch_time - threshold)`。
    /// When threshold is positive, minimizes `max(0, switch_time - threshold)`.
    pub fn with_threshold(
        polynomial: Linear<f64>,
        threshold: f64,
        coefficient: f64,
    ) -> Self {
        Self {
            name: "switch_time_minimization".to_string(),
            polynomial,
            threshold,
            threshold_coefficient: coefficient,
        }
    }

    /// 从 Switch 组件创建目标 / Create objective from Switch component
    pub fn from_switch(switch: &Switch, coefficient: f64) -> Self {
        let mut monomials = Vec::new();
        let mut constant = 0.0;
        for symbol in switch.switch_time_symbols.iter().flatten() {
            let polynomial = symbol.to_linear_polynomial();
            for monomial in polynomial.monomials() {
                monomials.push(LinearMonomial::new(
                    *monomial.coefficient() * coefficient,
                    monomial.var_index(),
                ));
            }
            constant += *polynomial.constant_term() * coefficient;
        }
        Self::new(Linear::new(monomials, constant))
    }

    /// 从 Switch 组件创建带阈值目标 / Create threshold objective from Switch component
    pub fn from_switch_with_threshold(
        switch: &Switch,
        threshold: f64,
        coefficient: f64,
    ) -> Self {
        let mut objective = Self::from_switch(switch, 1.0);
        objective.threshold = threshold;
        objective.threshold_coefficient = coefficient;
        objective
    }
}

impl Pipeline<MetaModel<f64>> for SwitchTimeMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.polynomial.monomials().is_empty()
            && self.polynomial.constant_term().abs() <= f64::EPSILON
        {
            return;
        }
        if self.threshold <= f64::EPSILON {
            let sub_obj = SubObjective::minimize(self.scaled_polynomial(), &self.name);
            model.add_sub_objective(sub_obj);
            return;
        }

        let slack_idx = match model
            .as_basic_mut()
            .register_auto_variable::<UContinuous>(&format!("{}_threshold_slack", self.name))
        {
            Ok(idx) => idx,
            Err(e) => {
                log::warn!("Failed to register {} threshold slack: {:?}", self.name, e);
                return;
            }
        };
        let mut terms: Vec<(usize, f64)> = self.polynomial.monomials()
            .iter()
            .map(|monomial| (monomial.var_index(), -*monomial.coefficient()))
            .collect();
        terms.push((slack_idx, 1.0));
        if let Err(e) = model.add_ge_constraint(
            &terms,
            *self.polynomial.constant_term() - self.threshold,
            &format!("{}_threshold", self.name),
        ) {
            log::warn!("Failed to register {} threshold constraint: {:?}", self.name, e);
            return;
        }

        let polynomial = Linear::new(
            vec![LinearMonomial::new(self.threshold_coefficient, slack_idx)],
            0.0,
        );
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

impl SwitchTimeMinimization {
    fn scaled_polynomial(&self) -> Linear<f64> {
        let monomials = self.polynomial.monomials()
            .iter()
            .map(|monomial| {
                LinearMonomial::new(
                    *monomial.coefficient() * self.threshold_coefficient,
                    monomial.var_index(),
                )
            })
            .collect();
        Linear::new(
            monomials,
            *self.polynomial.constant_term() * self.threshold_coefficient,
        )
    }
}

/// 任务延迟时间最小化 / Task delay time minimization
///
/// 最小化 `sum(coefficient * delay_time[task])`。
/// Minimizes total delay time cost.
#[derive(Debug)]
pub struct TaskDelayTimeMinimization {
    name: String,
    /// (delay_time_model_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl TaskDelayTimeMinimization {
    pub fn new(cost_terms: Vec<(usize, f64)>) -> Self {
        Self {
            name: "task_delay_time_minimization".to_string(),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskDelayTimeMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务提前时间最小化 / Task advance time minimization
///
/// 最小化 `sum(coefficient * advance_time[task])`。
/// Minimizes total advance time cost.
#[derive(Debug)]
pub struct TaskAdvanceTimeMinimization {
    name: String,
    /// (advance_time_model_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl TaskAdvanceTimeMinimization {
    pub fn new(cost_terms: Vec<(usize, f64)>) -> Self {
        Self {
            name: "task_advance_time_minimization".to_string(),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskAdvanceTimeMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务超最大延迟时间约束 / Task over-max delay time constraint
///
/// 约束延迟时间不超过最大延迟限制：`delay_time[task] <= max_delay`。
/// Constrains delay time to not exceed maximum delay limit.
#[derive(Debug)]
pub struct TaskOverMaxDelayTimeConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (delay_time_model_index, max_delay_value) 列表 / List
    /// 只有 `max_delay` 非空的任务才出现在此列表中。
    /// Only tasks with non-null `max_delay` appear in this list.
    pub constraints: Vec<(usize, f64)>,
}

impl TaskOverMaxDelayTimeConstraint {
    /// 创建新的超最大延迟时间约束 / Create new over-max delay time constraint
    ///
    /// 从 TaskTime 的 `delay_time_model_indices` 和任务 `max_delay` 属性构建约束列表。
    /// 只有 `delay_time_model_indices[task]` 非空且 `max_delay_values[task]` 非空的任务才被包含。
    /// Builds constraint list from TaskTime's delay_time_model_indices and task max_delay values.
    /// Only tasks with both non-null delay_time_model_index and max_delay are included.
    pub fn new(
        delay_time_model_indices: &[Option<usize>],
        max_delay_values: &[Option<f64>],
    ) -> Self {
        let constraints: Vec<(usize, f64)> = delay_time_model_indices.iter()
            .zip(max_delay_values.iter())
            .filter_map(|(idx, max_delay)| {
                if let (Some(i), Some(m)) = (idx, max_delay) {
                    Some((*i, *m))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: "task_over_max_delay_time".to_string(),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskOverMaxDelayTimeConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (i, &(delay_idx, max_delay)) in self.constraints.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(
                &[(delay_idx, 1.0)], max_delay,
                &format!("{}_{}", self.name, i),
            ) {
                log::warn!("Failed to register constraint: {:?}", e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务超最大提前时间约束 / Task over-max advance time constraint
///
/// 约束提前时间不超过最大提前限制：`advance_time[task] <= max_advance`。
/// Constrains advance time to not exceed maximum advance limit.
#[derive(Debug)]
pub struct TaskOverMaxAdvanceTimeConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (advance_time_model_index, max_advance_value) 列表 / List
    pub constraints: Vec<(usize, f64)>,
}

impl TaskOverMaxAdvanceTimeConstraint {
    /// 创建新的超最大提前时间约束 / Create new over-max advance time constraint
    pub fn new(
        advance_time_model_indices: &[Option<usize>],
        max_advance_values: &[Option<f64>],
    ) -> Self {
        let constraints: Vec<(usize, f64)> = advance_time_model_indices.iter()
            .zip(max_advance_values.iter())
            .filter_map(|(idx, max_advance)| {
                if let (Some(i), Some(m)) = (idx, max_advance) {
                    Some((*i, *m))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: "task_over_max_advance_time".to_string(),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskOverMaxAdvanceTimeConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (i, &(advance_idx, max_advance)) in self.constraints.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(
                &[(advance_idx, 1.0)], max_advance,
                &format!("{}_{}", self.name, i),
            ) {
                log::warn!("Failed to register constraint: {:?}", e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务最晚结束时间约束 / Task delay last end time constraint
///
/// 约束任务在截止时间前完成：`est[task] + duration <= last_end_time`，
/// 等价于 `est[task] <= last_end_time - duration`。
///
/// Constrains tasks to finish before deadline: `est[task] + duration <= last_end_time`,
/// equivalent to `est[task] <= last_end_time - duration`.
#[derive(Debug)]
pub struct TaskDelayLastEndTimeConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (est_model_index, adjusted_bound) 列表 / List
    /// adjusted_bound = last_end_time - duration（solver 值域）/ adjusted_bound = last_end_time - duration (solver domain)
    pub constraints: Vec<(usize, f64)>,
}

impl TaskDelayLastEndTimeConstraint {
    /// 创建新的最晚结束时间约束 / Create new delay last end time constraint
    ///
    /// 从 `est` 的模型索引和调整后的边界值构建约束。
    /// 调整方式：`adjusted_bound = last_end_time - duration`。
    ///
    /// Builds constraints from est model indices and adjusted bound values.
    /// Adjustment: `adjusted_bound = last_end_time - duration`.
    pub fn new(
        est_model_indices: &[Option<usize>],
        last_end_time_values: &[Option<f64>],
        durations: &[f64],
    ) -> Self {
        let constraints: Vec<(usize, f64)> = est_model_indices.iter()
            .zip(last_end_time_values.iter())
            .zip(durations.iter())
            .filter_map(|((idx, last_end), duration)| {
                if let (Some(i), Some(l)) = (idx, last_end) {
                    Some((*i, l - duration))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: "task_delay_last_end_time".to_string(),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskDelayLastEndTimeConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (i, &(end_idx, last_end)) in self.constraints.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(
                &[(end_idx, 1.0)], last_end,
                &format!("{}_{}", self.name, i),
            ) {
                log::warn!("Failed to register constraint: {:?}", e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 任务最早结束时间约束 / Task advance earliest end time constraint
///
/// 约束任务不能早于最早结束时间完成：`est[task] + duration >= earliest_end_time`，
/// 等价于 `est[task] >= earliest_end_time - duration`。
///
/// Constrains tasks to not finish before earliest end time: `est[task] + duration >= earliest_end_time`,
/// equivalent to `est[task] >= earliest_end_time - duration`.
#[derive(Debug)]
pub struct TaskAdvanceEarliestEndTimeConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (est_model_index, adjusted_bound) 列表 / List
    /// adjusted_bound = earliest_end_time - duration（solver 值域）/ adjusted_bound = earliest_end_time - duration (solver domain)
    pub constraints: Vec<(usize, f64)>,
}

impl TaskAdvanceEarliestEndTimeConstraint {
    /// 创建新的最早结束时间约束 / Create new advance earliest end time constraint
    ///
    /// 从 `est` 的模型索引和调整后的边界值构建约束。
    /// 调整方式：`adjusted_bound = earliest_end_time - duration`。
    pub fn new(
        est_model_indices: &[Option<usize>],
        earliest_end_time_values: &[Option<f64>],
        durations: &[f64],
    ) -> Self {
        let constraints: Vec<(usize, f64)> = est_model_indices.iter()
            .zip(earliest_end_time_values.iter())
            .zip(durations.iter())
            .filter_map(|((idx, earliest_end), duration)| {
                if let (Some(i), Some(e)) = (idx, earliest_end) {
                    Some((*i, e - duration))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: "task_advance_earliest_end_time".to_string(),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TaskAdvanceEarliestEndTimeConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (i, &(end_idx, earliest_end)) in self.constraints.iter().enumerate() {
            if let Err(e) = model.add_ge_constraint(
                &[(end_idx, 1.0)], earliest_end,
                &format!("{}_{}", self.name, i),
            ) {
                log::warn!("Failed to register constraint: {:?}", e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 执行器成本最小化 / Executor cost minimization
///
/// 最小化 `sum(executor_cost[executor] * executor_compilation[executor])`。
/// Minimizes total executor usage cost.
#[derive(Debug)]
pub struct ExecutorCostMinimization {
    name: String,
    /// (executor_compilation_model_index, executor_cost) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ExecutorCostMinimization {
    /// 创建新的执行器成本最小化 / Create new executor cost minimization
    pub fn new(cost_terms: Vec<(usize, f64)>) -> Self {
        Self {
            name: "executor_cost_minimization".to_string(),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ExecutorCostMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, cost)| LinearMonomial::new(cost, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 执行器空闲最小化 / Executor leisure minimization
///
/// 最小化空闲执行器数量：`sum(z[executor])`。
/// Minimizes the number of idle executors.
#[derive(Debug)]
pub struct ExecutorLeisureMinimization {
    name: String,
    /// z[executor] 的模型索引列表 / z[executor] model index list
    pub leisure_indices: Vec<usize>,
}

impl ExecutorLeisureMinimization {
    /// 创建新的执行器空闲最小化 / Create new executor leisure minimization
    pub fn new(leisure_indices: Vec<usize>) -> Self {
        Self {
            name: "executor_leisure_minimization".to_string(),
            leisure_indices,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ExecutorLeisureMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.leisure_indices.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.leisure_indices.iter()
            .map(|&idx| LinearMonomial::new(1.0, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::variable::{Binary, UContinuous};
    use crate::domain::task_compilation::adapter::IndexedVariableArray1;

    #[test]
    fn test_task_compilation_constraint_from_symbols() {
        let mut model = MetaModel::<f64>::new("test_constraint");
        let task_indices: Vec<usize> = vec![0, 1];
        let executor_indices: Vec<usize> = vec![0, 1];

        // 注册 x[task, executor] 二维变量
        let x: crate::domain::task_compilation::adapter::IndexedVariableArray2<usize, usize, Binary> =
            crate::domain::task_compilation::adapter::IndexedVariableArray2::new(
                "x", &task_indices, &executor_indices, &mut model,
            ).unwrap();

        // 注册 y[task] 一维变量
        let y: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("y", &task_indices, &mut model).unwrap();

        // 构建 task_compilation 符号
        let mut symbols = Vec::new();
        for &ti in &task_indices {
            let mut terms = Vec::new();
            for &ei in &executor_indices {
                if let Some(idx) = x.model_index(&ti, &ei) {
                    terms.push((idx, 1.0));
                }
            }
            if let Some(idx) = y.model_index(&ti) {
                terms.push((idx, 1.0));
            }
            let symbol = crate::domain::task_compilation::adapter::build_linear_expression_symbol(
                &format!("task_compilation_{}", ti), &terms, 0.0,
            );
            symbols.push(symbol);
        }

        // 从符号创建约束
        let constraint = TaskCompilationConstraint::from_symbols(&symbols);

        // 验证多项式展开正确
        assert_eq!(constraint.task_compilation_polynomials.len(), 2);
        // 每个任务：2 个 executor x 变量 + 1 个 y 变量 = 3 项
        for poly in &constraint.task_compilation_polynomials {
            assert_eq!(poly.len(), 3);
            // 所有系数应为 1.0
            for &(_, coeff) in poly {
                assert!((coeff - 1.0).abs() < 1e-10);
            }
        }

        // 注册约束
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_executor_compilation_constraint_from_symbols() {
        let mut model = MetaModel::<f64>::new("test_executor_constraint");
        let task_indices: Vec<usize> = vec![0, 1];
        let executor_indices: Vec<usize> = vec![0, 1];

        let x: crate::domain::task_compilation::adapter::IndexedVariableArray2<usize, usize, Binary> =
            crate::domain::task_compilation::adapter::IndexedVariableArray2::new(
                "x", &task_indices, &executor_indices, &mut model,
            ).unwrap();

        let z: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("z", &executor_indices, &mut model).unwrap();

        // 构建 executor_compilation 符号
        let mut symbols = Vec::new();
        for &ei in &executor_indices {
            let mut terms = Vec::new();
            for &ti in &task_indices {
                if let Some(idx) = x.model_index(&ti, &ei) {
                    terms.push((idx, 1.0));
                }
            }
            if let Some(idx) = z.model_index(&ei) {
                terms.push((idx, 1.0));
            }
            let symbol = crate::domain::task_compilation::adapter::build_linear_expression_symbol(
                &format!("executor_compilation_{}", ei), &terms, 0.0,
            );
            symbols.push(symbol);
        }

        let constraint = ExecutorCompilationConstraint::from_symbols(&symbols);

        assert_eq!(constraint.executor_compilation_polynomials.len(), 2);
        // 每个执行器：2 个 task x 变量 + 1 个 z 变量 = 3 项
        for poly in &constraint.executor_compilation_polynomials {
            assert_eq!(poly.len(), 3);
        }

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_makespan_minimization() {
        let mut model = MetaModel::<f64>::new("test_obj");
        let makespan_idx = model.register_auto_variable::<UContinuous>("makespan").unwrap();

        let pipeline = MakespanMinimization::new(makespan_idx, 1.0);
        pipeline.register(&mut model);
        pipeline.invoke(&model).unwrap();
    }

    #[test]
    fn test_switch_objective_pipelines_register_objectives() {
        let mut model = MetaModel::<f64>::new("test_switch_objectives");
        let switch_idx = model.register_auto_variable::<Binary>("switch").unwrap();
        let time_idx = model.register_auto_variable::<UContinuous>("switch_time").unwrap();

        let cost_pipeline = SwitchCostMinimization::new(vec![(switch_idx, 2.0)]);
        cost_pipeline.register(&mut model);
        cost_pipeline.invoke(&model).unwrap();

        let time_pipeline = SwitchTimeMinimization::new(Linear::new(
            vec![LinearMonomial::new(3.0, time_idx)],
            0.0,
        ));
        time_pipeline.register(&mut model);
        time_pipeline.invoke(&model).unwrap();

        assert_eq!(model.objective().sub_objectives.len(), 2);
        assert_eq!(model.objective().sub_objectives[0].name, "switch_cost_minimization");
        assert_eq!(model.objective().sub_objectives[1].name, "switch_time_minimization");
    }

    #[test]
    fn test_switch_time_minimization_with_threshold_registers_slack() {
        let mut model = MetaModel::<f64>::new("test_switch_time_threshold");
        let time_idx = model.register_auto_variable::<UContinuous>("switch_time").unwrap();
        let pipeline = SwitchTimeMinimization::with_threshold(
            Linear::new(vec![LinearMonomial::new(1.0, time_idx)], 0.0),
            3.0,
            2.0,
        );

        pipeline.register(&mut model);
        pipeline.invoke(&model).unwrap();

        let mechanism = model.try_to_mechanism_model().unwrap();
        assert!(mechanism.as_basic().num_constraints() >= 1);
        assert_eq!(model.objective().sub_objectives.len(), 1);
        assert_eq!(model.objective().sub_objectives[0].name, "switch_time_minimization");
    }

    #[test]
    fn test_task_over_max_delay_constraint() {
        let delay_indices: Vec<Option<usize>> = vec![Some(0), None, Some(5)];
        let max_delays: Vec<Option<f64>> = vec![Some(10.0), Some(20.0), None];

        let constraint = TaskOverMaxDelayTimeConstraint::new(&delay_indices, &max_delays);

        // 只有 task 0 同时有 delay_index 和 max_delay
        assert_eq!(constraint.constraints.len(), 1);
        assert_eq!(constraint.constraints[0], (0, 10.0));
    }

    #[test]
    fn test_task_delay_last_end_time_constraint() {
        let est_indices: Vec<Option<usize>> = vec![Some(0), Some(3)];
        let last_end_times: Vec<Option<f64>> = vec![Some(100.0), None];
        let durations: Vec<f64> = vec![10.0, 20.0];

        let constraint = TaskDelayLastEndTimeConstraint::new(&est_indices, &last_end_times, &durations);

        // 只有 task 0 有 last_end_time
        assert_eq!(constraint.constraints.len(), 1);
        // adjusted_bound = 100.0 - 10.0 = 90.0
        assert_eq!(constraint.constraints[0], (0, 90.0));
    }

    // ========================================================================
    // 集成测试 / Integration Tests
    // ========================================================================

    /// 测试用的简单任务 / Simple task for integration testing
    #[derive(Debug, Clone)]
    struct IntegrationTestTask {
        id: String,
        name: String,
    }

    impl IntegrationTestTask {
        fn new(id: &str, name: &str) -> Self {
            Self { id: id.into(), name: name.to_string() }
        }
    }

    use crate::domain::task::{BasicAssignmentPolicy, BasicExecutor};

    impl<E: crate::domain::task::ExecutorTrait, A: crate::domain::task::AssignmentPolicyTrait<E>> TaskTrait<E, A> for IntegrationTestTask {
        type Id = String;

        fn id(&self) -> &Self::Id { &self.id }
        fn name(&self) -> &str { &self.name }
    }

    /// 端到端测试：from_compilation 创建约束并注册到模型
    /// End-to-end test: from_compilation creates constraints and registers to model
    #[test]
    fn test_from_compilation_constraint_pipeline() {
        use crate::domain::task_compilation::model::Compilation;
        use crate::domain::task_compilation::SolutionAnalyzer;

        let tasks = vec![
            IntegrationTestTask::new("t0", "Task 0"),
            IntegrationTestTask::new("t1", "Task 1"),
        ];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
            BasicExecutor::new("e1", "Executor 1"),
        ];

        let mut model = MetaModel::<f64>::new("test_from_compilation");

        // 注册 Compilation 组件
        let mut compilation: Compilation<IntegrationTestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            Compilation::new(tasks, executors, true, false);
        compilation.register(&mut model).unwrap();

        // 使用 from_compilation 创建约束
        let task_constraint = TaskCompilationConstraint::from_compilation(&compilation);
        let executor_constraint = ExecutorCompilationConstraint::from_compilation(&compilation);

        // 验证多项式正确展开
        // task_compilation: 2 tasks, 每个有 2 个 x 项 + 1 个 y 项
        assert_eq!(task_constraint.task_compilation_polynomials.len(), 2);
        for poly in &task_constraint.task_compilation_polynomials {
            assert_eq!(poly.len(), 3); // 2 executors + 1 cancel var
        }

        // executor_compilation: 2 executors, 每个有 2 个 x 项（无 z，因为 leisure=false）
        assert_eq!(executor_constraint.executor_compilation_polynomials.len(), 2);
        for poly in &executor_constraint.executor_compilation_polynomials {
            assert_eq!(poly.len(), 2); // 2 tasks, no leisure var
        }

        // 注册约束
        task_constraint.register(&mut model);
        task_constraint.invoke(&model).unwrap();
        executor_constraint.register(&mut model);
        executor_constraint.invoke(&model).unwrap();

        // 验证 SolutionAnalyzer 也能工作
        let n_vars = model.register_auto_variable::<Binary>("padding").unwrap() + 1;
        let mut solution = vec![0.0; n_vars];
        // t0 -> e1, t1 canceled
        if let Some(ref x) = compilation.x {
            if let Some(idx) = x.model_index(&0, &1) { solution[idx] = 1.0; }
        }
        if let Some(ref y) = compilation.y {
            if let Some(idx) = y.model_index(&1) { solution[idx] = 1.0; }
        }

        let result = SolutionAnalyzer::analyze(&compilation, &solution);
        assert_eq!(result.assigned.len(), 1);
        assert_eq!(result.assigned[0].task_index, 0);
        assert_eq!(result.assigned[0].executor_index, 1);
        assert!(result.canceled.contains(&1));
    }

    /// 端到端测试：from_compilation 带 executor leisure
    /// End-to-end test: from_compilation with executor leisure
    #[test]
    fn test_from_compilation_with_leisure() {
        use crate::domain::task_compilation::model::Compilation;

        let tasks = vec![
            IntegrationTestTask::new("t0", "Task 0"),
        ];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
            BasicExecutor::new("e1", "Executor 1"),
        ];

        let mut model = MetaModel::<f64>::new("test_from_compilation_leisure");

        let mut compilation: Compilation<IntegrationTestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            Compilation::new(tasks, executors, false, true);
        compilation.register(&mut model).unwrap();

        let executor_constraint = ExecutorCompilationConstraint::from_compilation(&compilation);

        // executor_compilation: 2 executors, 每个有 1 个 x 项 + 1 个 z 项
        assert_eq!(executor_constraint.executor_compilation_polynomials.len(), 2);
        for poly in &executor_constraint.executor_compilation_polynomials {
            assert_eq!(poly.len(), 2); // 1 task x var + 1 leisure z var
        }

        executor_constraint.register(&mut model);
        executor_constraint.invoke(&model).unwrap();
    }
}
