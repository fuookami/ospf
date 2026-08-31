//! 抽象回调模型基类
//! Abstract Callback Model Base Class

use super::callback_model_trait::{AbstractCallBackModelInterface, CallBackModelInterface};
use super::solution::{Solution, SolutionStatus};
use crate::error::CoreError;
use crate::solver::{SolverOutput, SolverStatus};
use crate::variable::VariableId;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// 回调模型求解执行器 / Callback model solve executor
pub type SolveExecutor<V> =
    Arc<dyn Fn(&mut AbstractCallBackModel<V>) -> crate::error::Result<SolverOutput> + Send + Sync>;
/// 启发式初始解提供器 / Heuristic initial solution provider
pub type InitialSolutionProvider<V> = Arc<dyn Fn(usize) -> Vec<Solution<V>> + Send + Sync>;
/// 启发式目标评估器 / Heuristic objective evaluator
pub type ObjectiveEvaluator<V> = Arc<dyn Fn(&Solution<V>) -> Option<V> + Send + Sync>;
/// 启发式值域收敛器 / Heuristic value coercer
pub type ValueCoercer<V> = Arc<dyn Fn(usize, V) -> V + Send + Sync>;

/// 抽象回调模型 / Abstract Callback Model
///
/// 提供 `CallBackModelInterface` 和 `AbstractCallBackModelInterface` 的默认实现。
/// Provides default implementations for `CallBackModelInterface` and `AbstractCallBackModelInterface`.
///
/// # 类型参数 / Type Parameters
/// - `V`: 值类型 / Value type
pub struct AbstractCallBackModel<V> {
    /// 模型名称 / Model name
    pub name: String,
    /// 当前解 / Current solution
    pub solution: Option<Solution<V>>,
    /// 解状态 / Solution status
    pub status: SolutionStatus,
    /// 目标值 / Objective value
    pub objective_value: Option<V>,
    /// 变量名称映射 / Variable name mapping
    pub variable_names: HashMap<VariableId, String>,
    /// 变量顺序（用于把求解器向量映射回 VariableId）/ Variable order for mapping solver vectors to VariableId
    pub variable_order: Vec<VariableId>,
    /// 变量数量 / Variable count
    pub num_vars: usize,
    /// 约束数量 / Constraint count
    pub num_constrs: usize,
    /// 求解执行器 / Solve executor
    pub solve_executor: Option<SolveExecutor<V>>,
    /// 启发式初始解提供器 / Heuristic initial solution provider
    pub initial_solution_provider: Option<InitialSolutionProvider<V>>,
    /// 启发式目标评估器 / Heuristic objective evaluator
    pub objective_evaluator: Option<ObjectiveEvaluator<V>>,
    /// 启发式值域收敛器 / Heuristic value coercer
    pub value_coercer: Option<ValueCoercer<V>>,
    /// 最近一次求解输出 / Last solver output
    pub last_solver_output: Option<SolverOutput>,
}

impl<V: Debug> std::fmt::Debug for AbstractCallBackModel<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbstractCallBackModel")
            .field("name", &self.name)
            .field("solution", &self.solution)
            .field("status", &self.status)
            .field("objective_value", &self.objective_value)
            .field("variable_names", &self.variable_names)
            .field("variable_order", &self.variable_order)
            .field("num_vars", &self.num_vars)
            .field("num_constrs", &self.num_constrs)
            .field("solve_executor_registered", &self.solve_executor.is_some())
            .field(
                "initial_solution_provider_registered",
                &self.initial_solution_provider.is_some(),
            )
            .field(
                "objective_evaluator_registered",
                &self.objective_evaluator.is_some(),
            )
            .field("value_coercer_registered", &self.value_coercer.is_some())
            .field("last_solver_output", &self.last_solver_output)
            .finish()
    }
}

impl<V> AbstractCallBackModel<V> {
    /// 创建新模型 / Create new model
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            solution: None,
            status: SolutionStatus::NotSolved,
            objective_value: None,
            variable_names: HashMap::new(),
            variable_order: Vec::new(),
            num_vars: 0,
            num_constrs: 0,
            solve_executor: None,
            initial_solution_provider: None,
            objective_evaluator: None,
            value_coercer: None,
            last_solver_output: None,
        }
    }

    /// 设置变量数量 / Set variable count
    pub fn set_num_variables(&mut self, count: usize) {
        self.num_vars = count;
    }

    /// 设置约束数量 / Set constraint count
    pub fn set_num_constraints(&mut self, count: usize) {
        self.num_constrs = count;
    }

    /// 注册变量名 / Register variable name
    pub fn register_variable_name(&mut self, id: VariableId, name: String) {
        self.variable_names.insert(id, name);
        if !self.variable_order.contains(&id) {
            self.variable_order.push(id);
        }
    }

    /// 设置变量顺序 / Set variable order
    pub fn set_variable_order(&mut self, order: Vec<VariableId>) {
        self.variable_order = order;
    }

    /// 清除变量顺序 / Clear variable order
    pub fn clear_variable_order(&mut self) {
        self.variable_order.clear();
    }

    /// 内部设置解 / Internal set solution
    pub fn set_solution_internal(&mut self, solution: Solution<V>) {
        self.solution = Some(solution);
    }

    /// 内部设置状态 / Internal set status
    pub fn set_status_internal(&mut self, status: SolutionStatus) {
        self.status = status;
    }

    /// 内部设置目标值 / Internal set objective value
    pub fn set_objective_internal(&mut self, value: Option<V>) {
        self.objective_value = value;
    }

    /// 设置求解执行器 / Set solve executor
    pub fn set_solve_executor(&mut self, executor: SolveExecutor<V>) {
        self.solve_executor = Some(executor);
    }

    /// 链式设置求解执行器 / Builder-style solve executor setter
    pub fn with_solve_executor(mut self, executor: SolveExecutor<V>) -> Self {
        self.solve_executor = Some(executor);
        self
    }

    /// 清除求解执行器 / Clear solve executor
    pub fn clear_solve_executor(&mut self) {
        self.solve_executor = None;
    }

    /// 是否已设置求解执行器 / Whether solve executor is set
    pub fn has_solve_executor(&self) -> bool {
        self.solve_executor.is_some()
    }

    /// 设置启发式初始解提供器 / Set heuristic initial solution provider
    pub fn set_initial_solution_provider(&mut self, provider: InitialSolutionProvider<V>) {
        self.initial_solution_provider = Some(provider);
    }

    /// 链式设置启发式初始解提供器 / Builder-style initial solution provider setter
    pub fn with_initial_solution_provider(mut self, provider: InitialSolutionProvider<V>) -> Self {
        self.initial_solution_provider = Some(provider);
        self
    }

    /// 清除启发式初始解提供器 / Clear heuristic initial solution provider
    pub fn clear_initial_solution_provider(&mut self) {
        self.initial_solution_provider = None;
    }

    /// 是否已设置启发式初始解提供器 / Whether heuristic initial solution provider is set
    pub fn has_initial_solution_provider(&self) -> bool {
        self.initial_solution_provider.is_some()
    }

    /// 生成启发式初始解 / Generate heuristic initial solutions
    pub fn generate_initial_solutions_for_heuristic(
        &self,
        initial_solution_amount: usize,
    ) -> Vec<Solution<V>>
    where
        V: Clone,
    {
        self.initial_solution_provider
            .as_ref()
            .map(|provider| provider(initial_solution_amount))
            .unwrap_or_default()
    }

    /// 设置启发式目标评估器 / Set heuristic objective evaluator
    pub fn set_objective_evaluator(&mut self, evaluator: ObjectiveEvaluator<V>) {
        self.objective_evaluator = Some(evaluator);
    }

    /// 链式设置启发式目标评估器 / Builder-style objective evaluator setter
    pub fn with_objective_evaluator(mut self, evaluator: ObjectiveEvaluator<V>) -> Self {
        self.objective_evaluator = Some(evaluator);
        self
    }

    /// 清除启发式目标评估器 / Clear heuristic objective evaluator
    pub fn clear_objective_evaluator(&mut self) {
        self.objective_evaluator = None;
    }

    /// 是否已设置启发式目标评估器 / Whether heuristic objective evaluator is set
    pub fn has_objective_evaluator(&self) -> bool {
        self.objective_evaluator.is_some()
    }

    /// 为启发式评估目标 / Evaluate objective for heuristic
    pub fn evaluate_objective_for_heuristic(&self, solution: &Solution<V>) -> Option<V>
    where
        V: Clone,
    {
        self.objective_evaluator
            .as_ref()
            .and_then(|evaluator| evaluator(solution))
    }

    /// 设置启发式值域收敛器 / Set heuristic value coercer
    pub fn set_value_coercer(&mut self, coercer: ValueCoercer<V>) {
        self.value_coercer = Some(coercer);
    }

    /// 链式设置启发式值域收敛器 / Builder-style value coercer setter
    pub fn with_value_coercer(mut self, coercer: ValueCoercer<V>) -> Self {
        self.value_coercer = Some(coercer);
        self
    }

    /// 清除启发式值域收敛器 / Clear heuristic value coercer
    pub fn clear_value_coercer(&mut self) {
        self.value_coercer = None;
    }

    /// 是否已设置启发式值域收敛器 / Whether heuristic value coercer is set
    pub fn has_value_coercer(&self) -> bool {
        self.value_coercer.is_some()
    }

    /// 对启发式值执行收敛 / Coerce heuristic value
    pub fn coerce_value_for_heuristic(&self, index: usize, value: V) -> V {
        if let Some(coercer) = self.value_coercer.as_ref() {
            coercer(index, value)
        } else {
            value
        }
    }

    /// 获取最近求解输出 / Get last solver output
    pub fn last_solver_output(&self) -> Option<&SolverOutput> {
        self.last_solver_output.as_ref()
    }

    fn map_solver_status(status: SolverStatus, has_solution: bool) -> SolutionStatus {
        match status {
            SolverStatus::Optimal => SolutionStatus::Optimal,
            SolverStatus::Infeasible => SolutionStatus::Infeasible,
            SolverStatus::Unbounded => SolutionStatus::Unbounded,
            SolverStatus::IterationLimit | SolverStatus::TimeLimit => {
                if has_solution {
                    SolutionStatus::Feasible
                } else {
                    SolutionStatus::Unknown
                }
            }
            _ => {
                if has_solution {
                    SolutionStatus::Feasible
                } else {
                    SolutionStatus::Unknown
                }
            }
        }
    }
}

#[async_trait]
impl<V> CallBackModelInterface<V> for AbstractCallBackModel<V>
where
    V: Clone + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve(&mut self) -> crate::error::Result<SolverOutput> {
        self.on_before_solve();
        let executor = self.solve_executor.clone().ok_or_else(|| {
            CoreError::NotImplemented("callback solve executor is not set".to_string())
        })?;
        let output = executor(self)?;
        self.last_solver_output = Some(output.clone());
        self.status = Self::map_solver_status(output.status, output.solution.is_some());
        if !output.status.is_feasible() {
            self.solution = None;
            self.objective_value = None;
        }
        self.on_after_solve(&output);
        Ok(output)
    }

    fn set_initial_solution(&mut self, solution: &Solution<V>) {
        self.solution = Some(solution.clone());
    }

    fn get_solution(&self) -> Option<&Solution<V>> {
        self.solution.as_ref()
    }

    fn get_status(&self) -> SolutionStatus {
        self.status
    }

    fn get_objective_value(&self) -> Option<V> {
        self.objective_value.clone()
    }

    fn clear_solution(&mut self) {
        self.solution = None;
        self.status = SolutionStatus::NotSolved;
        self.objective_value = None;
        self.last_solver_output = None;
    }
}

#[async_trait]
impl<V> AbstractCallBackModelInterface<V> for AbstractCallBackModel<V>
where
    V: Clone + Default + Send + Sync + 'static,
{
    fn num_variables(&self) -> usize {
        self.num_vars
    }

    fn num_constraints(&self) -> usize {
        self.num_constrs
    }

    fn get_variable_name(&self, id: VariableId) -> Option<&str> {
        self.variable_names.get(&id).map(|s| s.as_str())
    }

    fn get_variable_value(&self, id: VariableId) -> Option<V> {
        self.solution.as_ref()?.get(&id).cloned()
    }

    fn set_variable_value(&mut self, id: VariableId, value: V) {
        self.solution
            .get_or_insert_with(Solution::new)
            .insert(id, value);
    }

    fn get_all_values(&self) -> HashMap<VariableId, V> {
        self.solution.clone().unwrap_or_default()
    }

    fn calculate_objective(&self) -> Option<V> {
        self.objective_value.clone()
    }

    fn validate_solution(&self) -> bool {
        // 基类不提供验证功能
        // Base class does not provide validation
        self.solution.is_some()
    }

    fn get_violated_constraints(&self) -> Vec<usize> {
        // 基类不提供约束验证
        // Base class does not provide constraint validation
        Vec::new()
    }
}

impl<V> Default for AbstractCallBackModel<V> {
    fn default() -> Self {
        Self::new("default")
    }
}

// 类型别名 / Type Aliases
/// f64 精度的抽象回调模型 / Abstract callback model with f64 precision
pub type AbstractCallBackModelF64 = AbstractCallBackModel<f64>;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use crate::model::callback::CallBackModelInterface;
    use crate::solver::SolverOutput;
    use crate::variable::new_standalone_id;

    use super::AbstractCallBackModel;

    #[tokio::test]
    async fn abstract_callback_model_solve_uses_executor_and_syncs_status() {
        let mut model = AbstractCallBackModel::<f64>::new("cb");
        let variable_id = new_standalone_id();
        model.set_variable_order(vec![variable_id]);
        model.set_solve_executor(Arc::new(move |callback_model| {
            callback_model.set_solution_internal(HashMap::from([(variable_id, 2.0)]));
            callback_model.set_objective_internal(Some(5.5));
            Ok(SolverOutput::optimal(5.5, vec![2.0]))
        }));

        let output = model.solve().await.expect("callback solve should succeed");
        assert!(output.status.is_optimal());
        assert_eq!(
            model.get_status(),
            crate::model::callback::SolutionStatus::Optimal
        );
        assert_eq!(model.get_objective_value(), Some(5.5));
        let solution = model.get_solution().expect("solution should be synced");
        assert_eq!(solution.get(&variable_id).copied(), Some(2.0));
    }

    #[tokio::test]
    async fn abstract_callback_model_solve_without_executor_returns_not_implemented() {
        let mut model = AbstractCallBackModel::<f64>::new("cb");
        let err = model
            .solve()
            .await
            .expect_err("solve should fail without executor");
        assert!(format!("{err}").contains("callback solve executor is not set"));
    }

    #[test]
    fn abstract_callback_model_heuristic_hooks_are_invoked() {
        let provider_called = Arc::new(AtomicUsize::new(0));
        let evaluator_called = Arc::new(AtomicUsize::new(0));
        let coercer_called = Arc::new(AtomicUsize::new(0));
        let x = new_standalone_id();

        let provider_counter = provider_called.clone();
        let evaluator_counter = evaluator_called.clone();
        let coercer_counter = coercer_called.clone();
        let mut model = AbstractCallBackModel::<f64>::new("heuristic_hooks");
        model.set_initial_solution_provider(Arc::new(move |amount| {
            provider_counter.fetch_add(1, Ordering::Relaxed);
            if amount == 0 {
                Vec::new()
            } else {
                vec![HashMap::from([(x, 1.0)])]
            }
        }));
        model.set_objective_evaluator(Arc::new(move |solution| {
            evaluator_counter.fetch_add(1, Ordering::Relaxed);
            Some(solution.values().copied().sum())
        }));
        model.set_value_coercer(Arc::new(move |_index, value| {
            coercer_counter.fetch_add(1, Ordering::Relaxed);
            value.clamp(0.0, 1.0)
        }));

        let solutions = model.generate_initial_solutions_for_heuristic(1);
        assert_eq!(solutions.len(), 1);
        assert_eq!(
            model.evaluate_objective_for_heuristic(&solutions[0]),
            Some(1.0)
        );
        assert_eq!(model.coerce_value_for_heuristic(0, 2.0), 1.0);
        assert_eq!(provider_called.load(Ordering::Relaxed), 1);
        assert_eq!(evaluator_called.load(Ordering::Relaxed), 1);
        assert_eq!(coercer_called.load(Ordering::Relaxed), 1);
    }
}
