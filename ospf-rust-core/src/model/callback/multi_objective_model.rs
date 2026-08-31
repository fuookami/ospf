//! 多目标优化模型
//! Multi-Objective Optimization Model

use super::abstract_callback_model::AbstractCallBackModel;
use super::callback_model_trait::{
    AbstractCallBackModelInterface, CallBackModelInterface, MultiObjectiveModelInterface,
};
use super::solution::{SolutionStatus, SolutionWithStatus};
use crate::solver::SolverOutput;
use crate::variable::VariableId;
use async_trait::async_trait;
use std::collections::HashMap;

/// 多目标位置 / Multi-Objective Location
///
/// 标识多目标优化中目标的位置。
/// Identifies the location of an objective in multi-objective optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MultiObjectLocation {
    /// 目标索引 / Objective index
    pub index: usize,
    /// 是否启用 / Is enabled
    pub enabled: bool,
}

impl MultiObjectLocation {
    /// 创建新位置 / Create new location
    pub fn new(index: usize) -> Self {
        Self {
            index,
            enabled: true,
        }
    }

    /// 创建禁用的位置 / Create disabled location
    pub fn disabled(index: usize) -> Self {
        Self {
            index,
            enabled: false,
        }
    }

    /// 启用 / Enable
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用 / Disable
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 切换状态 / Toggle state
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

/// 多目标 / Multi-Objective
///
/// 存储多目标优化相关的信息。
/// Stores information related to multi-objective optimization.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
#[derive(Debug, Clone)]
pub struct MulObj<V> {
    /// 目标值列表 / Objective values
    pub objectives: Vec<Option<V>>,
    /// 目标权重 / Objective weights
    pub weights: Vec<V>,
    /// 目标位置 / Objective locations
    pub locations: Vec<MultiObjectLocation>,
    /// 帕累托前沿 / Pareto front
    pub pareto_front: Vec<SolutionWithStatus<V>>,
    /// 每个帕累托解对应的目标快照 / Objective snapshot for each Pareto solution
    pub pareto_objectives: Vec<Option<Vec<(usize, V)>>>,
}

impl<V> MulObj<V> {
    /// 创建空的多目标 / Create empty multi-objective
    pub fn new() -> Self {
        Self {
            objectives: Vec::new(),
            weights: Vec::new(),
            locations: Vec::new(),
            pareto_front: Vec::new(),
            pareto_objectives: Vec::new(),
        }
    }

    /// 创建带目标数量的多目标 / Create multi-objective with objective count
    pub fn with_count(count: usize, default_weight: V) -> Self
    where
        V: Clone,
    {
        Self {
            objectives: vec![None; count],
            weights: vec![default_weight; count],
            locations: (0..count).map(MultiObjectLocation::new).collect(),
            pareto_front: Vec::new(),
            pareto_objectives: Vec::new(),
        }
    }

    /// 获取目标数量 / Get objective count
    pub fn len(&self) -> usize {
        self.objectives.len()
    }

    /// 是否为空 / Is empty
    pub fn is_empty(&self) -> bool {
        self.objectives.is_empty()
    }

    /// 添加目标 / Add objective
    pub fn add_objective(&mut self, weight: V) {
        self.objectives.push(None);
        self.weights.push(weight);
        self.locations
            .push(MultiObjectLocation::new(self.objectives.len() - 1));
    }

    /// 设置目标值 / Set objective value
    pub fn set_objective(&mut self, index: usize, value: V) {
        if index < self.objectives.len() {
            self.objectives[index] = Some(value);
        }
    }

    /// 获取目标值 / Get objective value
    pub fn get_objective(&self, index: usize) -> Option<&V> {
        self.objectives.get(index).and_then(|v| v.as_ref())
    }

    /// 设置权重 / Set weight
    pub fn set_weight(&mut self, index: usize, weight: V) {
        if index < self.weights.len() {
            self.weights[index] = weight;
        }
    }

    /// 获取权重 / Get weight
    pub fn get_weight(&self, index: usize) -> Option<&V> {
        self.weights.get(index)
    }

    /// 清除所有目标值 / Clear all objective values
    pub fn clear_objectives(&mut self) {
        for obj in &mut self.objectives {
            *obj = None;
        }
    }

    /// 添加帕累托解 / Add Pareto solution
    fn current_objective_snapshot(&self) -> Option<Vec<(usize, V)>>
    where
        V: Clone,
    {
        let mut snapshot = Vec::new();
        for (index, objective) in self.objectives.iter().enumerate() {
            let enabled = self
                .locations
                .get(index)
                .map(|loc| loc.enabled)
                .unwrap_or(true);
            if !enabled {
                continue;
            }
            let value = objective.as_ref()?.clone();
            snapshot.push((index, value));
        }
        if snapshot.is_empty() {
            None
        } else {
            Some(snapshot)
        }
    }

    fn dominates_snapshot(&self, lhs: &[(usize, V)], rhs: &[V]) -> bool
    where
        V: PartialOrd,
    {
        let mut has_comparable_dimension = false;
        let mut strictly_better = false;

        for (index, lhs_value) in lhs {
            let Some(rhs_value) = rhs.get(*index) else {
                continue;
            };
            has_comparable_dimension = true;
            if lhs_value > rhs_value {
                return false;
            }
            if lhs_value < rhs_value {
                strictly_better = true;
            }
        }

        has_comparable_dimension && strictly_better
    }

    /// 添加帕累托解并保存当前目标快照 / Add a Pareto solution and save its objective snapshot.
    pub fn add_pareto_solution(&mut self, solution: SolutionWithStatus<V>)
    where
        V: Clone,
    {
        self.pareto_objectives
            .push(self.current_objective_snapshot());
        self.pareto_front.push(solution);
    }

    /// 清除帕累托前沿 / Clear Pareto front
    pub fn clear_pareto_front(&mut self) {
        self.pareto_front.clear();
        self.pareto_objectives.clear();
    }

    /// 获取帕累托前沿 / Get Pareto front
    pub fn get_pareto_front(&self) -> &[SolutionWithStatus<V>] {
        &self.pareto_front
    }

    /// 计算加权目标值 / Calculate weighted objective value
    pub fn calculate_weighted_objective(&self) -> Option<V>
    where
        V: Clone
            + Default
            + for<'a> std::ops::Add<&'a V, Output = V>
            + for<'a> std::ops::Mul<&'a V, Output = V>,
    {
        let mut total = V::default();
        let mut has_value = false;

        for (obj, weight) in self.objectives.iter().zip(self.weights.iter()) {
            if let Some(obj_val) = obj {
                total = total + &(obj_val.clone() * weight);
                has_value = true;
            }
        }

        if has_value { Some(total) } else { None }
    }

    /// 判断解是否被帕累托支配 / Check if solution is dominated by Pareto front
    pub fn is_dominated(&self, objectives: &[V]) -> bool
    where
        V: PartialOrd
            + Clone
            + Default
            + for<'a> std::ops::Add<&'a V, Output = V>
            + for<'a> std::ops::Mul<&'a V, Output = V>,
    {
        if objectives.is_empty() || self.pareto_front.is_empty() {
            return false;
        }

        let mut candidate_weighted = V::default();
        let mut has_candidate_component = false;

        for (index, objective_value) in objectives.iter().enumerate() {
            let enabled = self
                .locations
                .get(index)
                .map(|loc| loc.enabled)
                .unwrap_or(true);
            if !enabled {
                continue;
            }
            let Some(weight) = self.weights.get(index) else {
                continue;
            };
            candidate_weighted = candidate_weighted + &(objective_value.clone() * weight);
            has_candidate_component = true;
        }

        if !has_candidate_component {
            return false;
        }

        for snapshot in &self.pareto_objectives {
            let Some(pareto_objectives) = snapshot.as_ref() else {
                continue;
            };
            if self.dominates_snapshot(pareto_objectives, objectives) {
                return true;
            }
        }

        for pareto_solution in &self.pareto_front {
            if let Some(pareto_obj) = pareto_solution.objective_value.as_ref()
                && pareto_obj <= &candidate_weighted
            {
                return true;
            }
        }
        false
    }
}

impl<V> Default for MulObj<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// 多目标回调模型 / Multi-Objective Callback Model
///
/// 实现多目标优化回调模型。
/// Implements multi-objective optimization callback model.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
#[derive(Debug)]
pub struct MultiObjectiveCallBackModel<V> {
    /// 基础模型 / Base model
    pub base: AbstractCallBackModel<V>,
    /// 多目标数据 / Multi-objective data
    pub multi_obj: MulObj<V>,
}

impl<V> MultiObjectiveCallBackModel<V> {
    /// 创建新模型 / Create new model
    pub fn new(name: &str) -> Self {
        Self {
            base: AbstractCallBackModel::new(name),
            multi_obj: MulObj::new(),
        }
    }

    /// 创建带目标数量的模型 / Create model with objective count
    pub fn with_objectives(name: &str, count: usize, default_weight: V) -> Self
    where
        V: Clone,
    {
        Self {
            base: AbstractCallBackModel::new(name),
            multi_obj: MulObj::with_count(count, default_weight),
        }
    }

    /// 获取基础模型引用 / Get base model reference
    pub fn as_base(&self) -> &AbstractCallBackModel<V> {
        &self.base
    }

    /// 获取基础模型可变引用 / Get base model mutable reference
    pub fn as_base_mut(&mut self) -> &mut AbstractCallBackModel<V> {
        &mut self.base
    }
}

// 委托实现 CallBackModelInterface / Delegate implementation of CallBackModelInterface
#[async_trait]
impl<V: Clone + Send + Sync + 'static> CallBackModelInterface<V>
    for MultiObjectiveCallBackModel<V>
{
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn solve(&mut self) -> crate::error::Result<SolverOutput> {
        self.base.solve().await
    }

    fn set_initial_solution(&mut self, solution: &super::solution::Solution<V>) {
        self.base.set_initial_solution(solution);
    }

    fn get_solution(&self) -> Option<&super::solution::Solution<V>> {
        self.base.get_solution()
    }

    fn get_status(&self) -> SolutionStatus {
        self.base.get_status()
    }

    fn get_objective_value(&self) -> Option<V> {
        self.base.get_objective_value()
    }

    fn clear_solution(&mut self) {
        self.base.clear_solution();
    }
}

// 委托实现 AbstractCallBackModelInterface / Delegate implementation of AbstractCallBackModelInterface
#[async_trait]
impl<V: Clone + Default + Send + Sync + 'static> AbstractCallBackModelInterface<V>
    for MultiObjectiveCallBackModel<V>
{
    fn num_variables(&self) -> usize {
        self.base.num_variables()
    }

    fn num_constraints(&self) -> usize {
        self.base.num_constraints()
    }

    fn get_variable_name(&self, id: VariableId) -> Option<&str> {
        self.base.get_variable_name(id)
    }

    fn get_variable_value(&self, id: VariableId) -> Option<V> {
        self.base.get_variable_value(id)
    }

    fn set_variable_value(&mut self, id: VariableId, value: V) {
        self.base.set_variable_value(id, value);
    }

    fn get_all_values(&self) -> HashMap<VariableId, V> {
        self.base.get_all_values()
    }

    fn calculate_objective(&self) -> Option<V> {
        self.base.calculate_objective()
    }

    fn validate_solution(&self) -> bool {
        self.base.validate_solution()
    }

    fn get_violated_constraints(&self) -> Vec<usize> {
        self.base.get_violated_constraints()
    }
}

#[async_trait]
impl<V: Clone + Default + Send + Sync + 'static> MultiObjectiveModelInterface<V>
    for MultiObjectiveCallBackModel<V>
where
    V: for<'a> std::ops::Add<&'a V, Output = V> + for<'a> std::ops::Mul<&'a V, Output = V>,
{
    fn num_objectives(&self) -> usize {
        self.multi_obj.len()
    }

    fn get_all_objectives(&self) -> Vec<Option<V>> {
        self.multi_obj.objectives.clone()
    }

    fn get_objective(&self, index: usize) -> Option<V> {
        self.multi_obj.get_objective(index).cloned()
    }

    fn set_objective_weights(&mut self, weights: Vec<V>) {
        self.multi_obj.weights = weights;
    }

    fn get_objective_weights(&self) -> &[V] {
        &self.multi_obj.weights
    }

    fn get_pareto_front(&self) -> Vec<SolutionWithStatus<V>> {
        self.multi_obj.pareto_front.clone()
    }

    fn add_pareto_solution(&mut self, solution: SolutionWithStatus<V>) {
        self.multi_obj.add_pareto_solution(solution);
    }

    fn clear_pareto_front(&mut self) {
        self.multi_obj.clear_pareto_front();
    }
}

// 类型别名 / Type Aliases
/// f64 精度的多目标位置 / Multi-objective location with f64 precision
pub type MultiObjectLocationF64 = MultiObjectLocation;

/// f64 精度的多目标 / Multi-objective with f64 precision
pub type MulObjF64 = MulObj<f64>;

/// f64 精度的多目标回调模型 / Multi-objective callback model with f64 precision
pub type MultiObjectiveCallBackModelF64 = MultiObjectiveCallBackModel<f64>;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::{MulObj, MultiObjectiveCallBackModel};
    use crate::model::callback::{
        AbstractCallBackModelInterface, CallBackModelInterface, MultiObjectiveModelInterface,
        Solution, SolutionStatus, SolutionWithStatus,
    };
    use crate::solver::SolverOutput;
    use crate::variable::new_standalone_id;

    #[test]
    fn is_dominated_uses_pareto_objective_snapshots() {
        let mut mul_obj = MulObj::with_count(2, 1.0_f64);
        mul_obj.set_objective(0, 1.0);
        mul_obj.set_objective(1, 2.0);
        mul_obj.add_pareto_solution(SolutionWithStatus::feasible(Solution::new()));

        assert!(mul_obj.is_dominated(&[1.0, 2.5]));
        assert!(!mul_obj.is_dominated(&[0.5, 2.5]));
        assert!(!mul_obj.is_dominated(&[1.0, 2.0]));
    }

    #[test]
    fn is_dominated_respects_enabled_objective_locations() {
        let mut mul_obj = MulObj::with_count(2, 1.0_f64);
        mul_obj.locations[1].disable();
        mul_obj.set_objective(0, 2.0);
        mul_obj.set_objective(1, 100.0);
        mul_obj.add_pareto_solution(SolutionWithStatus::feasible(Solution::new()));

        assert!(mul_obj.is_dominated(&[3.0, 0.0]));
        assert!(!mul_obj.is_dominated(&[1.0, 1000.0]));
    }

    #[test]
    fn is_dominated_falls_back_to_weighted_objective_when_snapshot_missing() {
        let mut mul_obj = MulObj::with_count(2, 1.0_f64);
        mul_obj.clear_objectives();
        mul_obj.add_pareto_solution(SolutionWithStatus::optimal(Solution::new(), 5.0));

        assert!(mul_obj.is_dominated(&[3.0, 3.0]));
        assert!(!mul_obj.is_dominated(&[2.0, 2.0]));
    }

    #[tokio::test]
    async fn multi_objective_callback_solve_syncs_base_status_solution_and_objective() {
        let x = new_standalone_id();
        let mut model = MultiObjectiveCallBackModel::with_objectives("mo_cb", 2, 1.0_f64);
        model.multi_obj.set_objective(0, 1.0);
        model.multi_obj.set_objective(1, 2.0);
        model.base.set_variable_order(vec![x]);
        model
            .base
            .set_solve_executor(Arc::new(move |callback_model| {
                callback_model.set_solution_internal(HashMap::from([(x, 3.0)]));
                callback_model.set_objective_internal(Some(7.0));
                Ok(SolverOutput::optimal(7.0, vec![3.0]))
            }));

        let output = model
            .solve()
            .await
            .expect("multi-objective callback solve should succeed");
        assert!(output.status.is_optimal());
        assert_eq!(model.get_status(), SolutionStatus::Optimal);
        assert_eq!(model.get_objective_value(), Some(7.0));
        assert_eq!(
            model
                .get_solution()
                .and_then(|solution| solution.get(&x))
                .copied(),
            Some(3.0)
        );
        assert_eq!(
            model
                .base
                .last_solver_output()
                .and_then(|last| last.objective_value),
            Some(7.0)
        );
    }

    #[tokio::test]
    async fn multi_objective_callback_pareto_chain_can_consume_exported_solution() {
        let x = new_standalone_id();
        let mut model = MultiObjectiveCallBackModel::with_objectives("mo_pareto", 2, 1.0_f64);
        model.multi_obj.set_objective(0, 1.0);
        model.multi_obj.set_objective(1, 2.0);
        model.base.set_variable_order(vec![x]);
        model
            .base
            .set_solve_executor(Arc::new(move |callback_model| {
                callback_model.set_solution_internal(HashMap::from([(x, 1.5)]));
                callback_model.set_objective_internal(Some(3.0));
                Ok(SolverOutput::optimal(3.0, vec![1.5]))
            }));

        model
            .solve()
            .await
            .expect("multi-objective callback solve should succeed");
        let exported = model.export_solution();
        model.add_pareto_solution(exported);

        assert_eq!(model.get_pareto_front().len(), 1);
        assert!(model.multi_obj.is_dominated(&[1.0, 3.0]));
        assert!(!model.multi_obj.is_dominated(&[0.1, 0.2]));
    }
}
