//! 任务束编译模型 / Bunch compilation models
//!
//! 定义列生成主问题的核心模型组件：BunchCompilation、BunchAggregation、
//! SlotBasedBunch、BunchSchedulingSolution 等。
//!
//! Defines core model components for the column generation master problem:
//! BunchCompilation, BunchAggregation, SlotBasedBunch, BunchSchedulingSolution, etc.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::LinearMonomial;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::variable::Binary;

use crate::GanttError;
use crate::GanttResult;
use crate::domain::common::{BunchId, ExecutorId, ExecutorIdTrait};
use crate::domain::task_compilation::adapter::next_gantt_symbol_id;

// ============================================================================
// 任务束聚合 / Bunch Aggregation
// ============================================================================

/// 任务束聚合 / Bunch aggregation
///
/// 管理任务束集合的添加、去重和移除追踪。
/// Manages bunch collection with deduplication and removal tracking.
#[derive(Clone)]
pub struct BunchAggregation<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 按迭代分组的束列表 / Bunches grouped by iteration
    bunches_by_iteration: Vec<Vec<usize>>,
    /// 所有活跃束索引 / All active bunch indices
    bunches: Vec<BunchEntry<I>>,
    /// 已移除的束索引 / Removed bunch indices
    removed: HashSet<usize>,
    /// 迭代计数 / Iteration count
    iteration_count: usize,
}

/// 束条目 / Bunch entry
#[derive(Debug, Clone)]
pub struct BunchEntry<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 完整业务候选的稳定 ID，不从覆盖任务集合推测。 / Stable ID of the complete business candidate, never inferred from covered tasks.
    pub id: BunchId,
    /// 束索引 / Bunch index
    pub index: usize,
    /// 执行器 ID / Executor ID
    pub executor_id: I,
    /// 包含的任务索引 / Contained task indices
    pub task_indices: Vec<usize>,
    /// 束成本 / Bunch cost
    pub cost: f64,
    /// 所属迭代 / Iteration
    pub iteration: usize,
    /// 所属时隙；普通任务束为 None / Slot index; None for non-slot-based bunches
    pub slot_index: Option<usize>,
}

impl<I> std::fmt::Debug for BunchAggregation<I>
where
    I: ExecutorIdTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BunchAggregation")
            .field("bunch_count", &self.bunches.len())
            .field("removed_count", &self.removed.len())
            .field("iteration_count", &self.iteration_count)
            .finish()
    }
}

impl<I> BunchAggregation<I>
where
    I: ExecutorIdTrait,
{
    /// 为业务 ID 创建新的任务束聚合 / Create new bunch aggregation for domain ids
    pub fn new_for_ids() -> Self {
        Self {
            bunches_by_iteration: Vec::new(),
            bunches: Vec::new(),
            removed: HashSet::new(),
            iteration_count: 0,
        }
    }

    /// 按稳定 ID 原子增列；相同 ID 只在 payload 一致时幂等，退休后须用新索引重加。 / Atomically add by stable ID; identical payloads are idempotent and retired candidates require a fresh index.
    pub fn add_bunches(&mut self, iteration: usize, new_bunches: Vec<BunchEntry<I>>) -> GanttResult<Vec<usize>> {
        let iteration_count = iteration.checked_add(1).ok_or_else(|| GanttError::Calculation {
            message: "任务束迭代索引耗尽 / bunch iteration index exhausted".into(),
        })?;
        let mut staged = self.clone();
        let mut added = Vec::new();
        for mut bunch in new_bunches {
            if bunch.id.is_empty() || bunch.executor_id.is_empty() || !bunch.cost.is_finite()
                || bunch.task_indices.iter().collect::<HashSet<_>>().len() != bunch.task_indices.len()
            {
                return Err(GanttError::Calculation { message: "非法任务束身份、费用或重复任务 / invalid bunch identity, cost or duplicate tasks".into() });
            }
            let same_payload = |existing: &BunchEntry<I>| existing.id == bunch.id
                && existing.executor_id == bunch.executor_id && existing.task_indices == bunch.task_indices
                && existing.slot_index == bunch.slot_index && existing.cost == bunch.cost;
            if staged.bunches.iter().any(|existing| existing.id == bunch.id && !same_payload(existing)) {
                return Err(GanttError::Calculation { message: format!("任务束 ID 的 payload 冲突 / conflicting payload for bunch {}", bunch.id) });
            }
            if let Some(existing) = staged.get_bunch(bunch.index)
                && (!same_payload(existing) || staged.removed.contains(&existing.index))
            {
                return Err(GanttError::Calculation { message: format!("任务束索引已使用或退休 / bunch index already used or retired: {}", bunch.index) });
            }
            if staged.bunches.iter().any(|existing| existing.id == bunch.id && !staged.removed.contains(&existing.index)) {
                continue;
            }
            bunch.iteration = iteration;
            added.push(bunch.index);
            staged.bunches.push(bunch);
        }
        if !added.is_empty() {
            while staged.bunches_by_iteration.len() <= iteration {
                staged.bunches_by_iteration.push(Vec::new());
            }
            staged.bunches_by_iteration[iteration].extend(added.iter().copied());
        }
        staged.iteration_count = staged.iteration_count.max(iteration_count);
        *self = staged;
        Ok(added)
    }

    /// 获取所有活跃束 / Get all active bunches
    pub fn bunches(&self) -> Vec<&BunchEntry<I>> {
        self.bunches
            .iter()
            .filter(|b| !self.removed.contains(&b.index))
            .collect()
    }

    /// 获取指定迭代的束 / Get bunches for a specific iteration
    pub fn bunches_for_iteration(&self, iteration: usize) -> Vec<&BunchEntry<I>> {
        self.bunches_by_iteration
            .get(iteration)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.get_bunch(idx))
                    .filter(|b| !self.removed.contains(&b.index))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 标记束为已移除 / Mark bunch as removed
    pub fn remove(&mut self, index: usize) {
        if self.get_bunch(index).is_some() {
            self.removed.insert(index);
        }
    }

    /// 活跃束数量 / Active bunch count
    pub fn active_count(&self) -> usize {
        self.bunches.iter().filter(|bunch| !self.removed.contains(&bunch.index)).count()
    }

    /// 获取所有束条目（包含已移除的）/ Get all bunch entries (including removed)
    pub fn all_bunches(&self) -> &[BunchEntry<I>] {
        &self.bunches
    }

    /// 获取束条目 / Get bunch entry by index field
    pub fn get_bunch(&self, index: usize) -> Option<&BunchEntry<I>> {
        self.bunches.iter().find(|b| b.index == index)
    }
}

impl<I> Default for BunchAggregation<I>
where
    I: ExecutorIdTrait,
{
    fn default() -> Self {
        Self::new_for_ids()
    }
}

impl BunchAggregation<ExecutorId> {
    /// 创建新的任务束聚合 / Create new bunch aggregation
    pub fn new() -> Self {
        Self::new_for_ids()
    }
}

// ============================================================================
// 任务束编译 / Bunch Compilation
// ============================================================================

/// 任务束编译 / Bunch compilation
///
/// 列生成主问题的核心编译模型，管理：
/// - `x[iteration][bunch]` 二进制选择变量
/// - `y[task]` 取消变量
/// - `z[executor]` 空闲变量
/// - `bunchCost` 成本表达式
/// - `taskCompilation[task]` 任务编译表达式
/// - `executorCompilation[executor]` 执行器编译表达式
///
/// Core compilation model for column generation master problem, managing:
/// - `x[iteration][bunch]` binary selection variables
/// - `y[task]` cancellation variables
/// - `z[executor]` leisure variables
/// - `bunchCost` cost expression
/// - `taskCompilation[task]` task compilation expressions
/// - `executorCompilation[executor]` executor compilation expressions
#[derive(Clone)]
pub struct BunchCompilation<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 任务数量 / Task count
    pub n_tasks: usize,
    /// 执行器 ID 列表 / Executor ID list
    pub executor_ids: Vec<I>,
    /// 是否启用执行器空闲变量 / Whether executor leisure variables are enabled
    pub with_executor_leisure: bool,
    /// 束聚合 / Bunch aggregation
    pub aggregation: BunchAggregation<I>,
    /// y[task] 取消变量索引 / y[task] cancellation variable indices
    pub y_indices: Vec<usize>,
    /// z[executor] 空闲变量索引 / z[executor] leisure variable indices
    pub z_indices: Vec<usize>,
    /// x[iteration][bunch] 选择变量索引 / x[iteration][bunch] selection variable indices
    pub x_indices: Vec<Vec<usize>>,
    /// bunchCost 中间表达式 / bunchCost intermediate expression
    pub bunch_cost_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// taskCompilation[task] 中间表达式 / taskCompilation intermediate expressions
    pub task_compilation_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// executorCompilation[executor] 中间表达式 / executorCompilation intermediate expressions
    pub executor_compilation_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    registration: Option<(u64, usize, Vec<I>, bool)>,
    registered_auxiliary_indices: Option<(Vec<usize>, Vec<usize>)>,
    x_by_bunch: HashMap<usize, usize>,
}

impl<I> std::fmt::Debug for BunchCompilation<I>
where
    I: ExecutorIdTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BunchCompilation")
            .field("n_tasks", &self.n_tasks)
            .field("executor_count", &self.executor_ids.len())
            .field("iteration_count", &self.x_indices.len())
            .field("with_executor_leisure", &self.with_executor_leisure)
            .finish()
    }
}

impl<I> BunchCompilation<I>
where
    I: ExecutorIdTrait,
{
    /// 使用业务 ID 创建新的任务束编译 / Create new bunch compilation with domain ids
    pub fn new_with_ids(
        n_tasks: usize,
        executor_ids: Vec<impl Into<I>>,
        with_executor_leisure: bool,
    ) -> Self {
        let executor_ids = executor_ids.into_iter().map(Into::into).collect();
        Self {
            n_tasks,
            executor_ids,
            with_executor_leisure,
            aggregation: BunchAggregation::new_for_ids(),
            y_indices: Vec::with_capacity(n_tasks),
            z_indices: Vec::new(),
            x_indices: Vec::new(),
            bunch_cost_symbol: None,
            task_compilation_symbols: Vec::with_capacity(n_tasks),
            executor_compilation_symbols: Vec::new(),
            registration: None,
            registered_auxiliary_indices: None,
            x_by_bunch: HashMap::new(),
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 注册 y[task] 取消变量、z[executor] 空闲变量和初始中间表达式。
    /// x 变量通过 `add_columns` 在列生成迭代中动态注册。
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        if self.is_registered() {
            return self.validate_model(model);
        }
        if self.executor_ids.iter().any(|executor| executor.is_empty())
            || self.executor_ids.iter().collect::<HashSet<_>>().len() != self.executor_ids.len()
        {
            return Err(GanttError::Calculation { message: "执行器 ID 为空或重复 / empty or duplicate executor ID".into() });
        }
        let mut next = self.clone();
        model.transaction(|model| next.register_initial(model))?;
        next.registration = Some((model.model_identity(), next.n_tasks, next.executor_ids.clone(), next.with_executor_leisure));
        next.registered_auxiliary_indices = Some((next.y_indices.clone(), next.z_indices.clone()));
        *self = next;
        Ok(())
    }

    pub(crate) fn is_registered(&self) -> bool {
        self.registration.is_some()
    }

    pub(crate) fn validate_model(&self, model: &MetaModel<f64>) -> GanttResult<()> {
        if self.registration.as_ref().is_none_or(|(identity, tasks, executors, leisure)| {
            *identity != model.model_identity() || *tasks != self.n_tasks
                || *executors != self.executor_ids || *leisure != self.with_executor_leisure
        }) || self.registered_auxiliary_indices.as_ref().is_none_or(|(y, z)| *y != self.y_indices || *z != self.z_indices) {
            return Err(GanttError::Calculation { message: "任务束编译未注册、跨模型使用或定义已改变 / unregistered, foreign-model or changed bunch compilation".into() });
        }
        Ok(())
    }

    /// 从历史列身份查找变量，删除不会压缩位置。 / Look up a variable by historical column identity without compacting positions after removal.
    pub fn x_index(&self, bunch_index: usize) -> Option<usize> {
        self.x_by_bunch.get(&bunch_index).copied()
    }

    fn register_initial(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        // 1. 注册 y[task] 取消变量
        self.y_indices.clear();
        for ti in 0..self.n_tasks {
            let var_name = format!("y_{}", ti);
            let var_item: ospf_rust_core::variable::VariableItem<Binary> =
                ospf_rust_core::variable::VariableItem::create(
                    ospf_rust_core::variable::VariableId::standalone(
                        next_gantt_symbol_id() as usize
                    ),
                    &var_name,
                );
            let model_idx =
                model
                    .register_variable(var_item)
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register variable {}: {:?}", var_name, e),
                    })?;
            self.y_indices.push(model_idx);
        }

        // 2. 注册 z[executor] 空闲变量
        if self.with_executor_leisure {
            self.z_indices.clear();
            for exec_id in &self.executor_ids {
                let var_name = format!("z_{}", exec_id);
                let var_item: ospf_rust_core::variable::VariableItem<Binary> =
                    ospf_rust_core::variable::VariableItem::create(
                        ospf_rust_core::variable::VariableId::standalone(
                            next_gantt_symbol_id() as usize
                        ),
                        &var_name,
                    );
                let model_idx =
                    model
                        .register_variable(var_item)
                        .map_err(|e| GanttError::Calculation {
                            message: format!("Failed to register variable {}: {:?}", var_name, e),
                        })?;
                self.z_indices.push(model_idx);
            }
        }

        // 3. 注册 bunchCost 空中间表达式（后续通过 add_columns 扩展）
        let cost_id = next_gantt_symbol_id();
        let cost_symbol = Arc::new(LinearExpressionSymbol::new(
            cost_id,
            "bunch_cost",
            vec![],
            0.0,
        ));
        model
            .add_symbol(cost_symbol.clone())
            .map_err(|e| GanttError::Calculation {
                message: format!("Failed to register bunch_cost: {:?}", e),
            })?;
        self.bunch_cost_symbol = Some(cost_symbol);

        // 4. 注册 taskCompilation[task] 中间表达式
        // taskCompilation[task] = sum(x[bunch] for bunch containing task) + y[task]
        // 初始只包含 y[task] 项
        self.task_compilation_symbols.clear();
        for ti in 0..self.n_tasks {
            let terms = vec![LinearMonomial::new(1.0, self.y_indices[ti])];
            let sym_id = next_gantt_symbol_id();
            let symbol = Arc::new(LinearExpressionSymbol::new(
                sym_id,
                &format!("task_compilation_{}", ti),
                terms,
                0.0,
            ));
            model
                .add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register task_compilation_{}: {:?}", ti, e),
                })?;
            self.task_compilation_symbols.push(symbol);
        }

        // 5. 注册 executorCompilation[executor] 中间表达式
        self.executor_compilation_symbols.clear();
        for (ei, exec_id) in self.executor_ids.iter().enumerate() {
            let terms = if self.with_executor_leisure {
                vec![LinearMonomial::new(1.0, self.z_indices[ei])]
            } else {
                vec![]
            };
            let sym_id = next_gantt_symbol_id();
            let symbol = Arc::new(LinearExpressionSymbol::new(
                sym_id,
                &format!("executor_compilation_{}", exec_id),
                terms,
                0.0,
            ));
            model
                .add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!(
                        "Failed to register executor_compilation_{}: {:?}",
                        exec_id, e
                    ),
                })?;
            self.executor_compilation_symbols.push(symbol);
        }

        Ok(())
    }

    /// 添加列 / Add columns
    ///
    /// 为新束注册 x 变量，更新 bunchCost、taskCompilation 和 executorCompilation 表达式。
    /// 基础层记录新增 x 变量；迭代层负责重建动态中间表达式并刷新约束入口。
    /// Registers x variables for new bunches; the iterative layer rebuilds dynamic
    /// intermediate expressions and refreshes constraint entry points.
    pub fn add_columns(
        &mut self,
        iteration: usize,
        new_bunches: Vec<BunchEntry<I>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        self.validate_model(model)?;
        if new_bunches.iter().any(|bunch| !self.executor_ids.contains(&bunch.executor_id)
            || bunch.task_indices.iter().any(|&task| task >= self.n_tasks))
        {
            return Err(GanttError::Calculation { message: "任务束引用未知执行器或任务 / bunch references an unknown executor or task".into() });
        }
        let mut next = self.clone();
        let deduped = next.aggregation.add_bunches(iteration, new_bunches)?;
        if deduped.is_empty() {
            return Ok(deduped);
        }
        model.transaction(|model| next.register_columns(iteration, &deduped, model))?;
        *self = next;
        Ok(deduped)
    }

    fn register_columns(
        &mut self,
        iteration: usize,
        deduped: &[usize],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {

        // 为每个新束注册 x 变量
        let mut iteration_x_indices = Vec::with_capacity(deduped.len());
        for &bunch_idx in deduped {
            let var_name = format!("x_{}_{}", iteration, bunch_idx);
            let var_item: ospf_rust_core::variable::VariableItem<Binary> =
                ospf_rust_core::variable::VariableItem::create(
                    ospf_rust_core::variable::VariableId::standalone(
                        next_gantt_symbol_id() as usize
                    ),
                    &var_name,
                );
            let model_idx =
                model
                    .register_variable(var_item)
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register variable {}: {:?}", var_name, e),
                    })?;
            iteration_x_indices.push(model_idx);
            self.x_by_bunch.insert(bunch_idx, model_idx);
        }

        // 确保迭代列表足够长
        while self.x_indices.len() <= iteration {
            self.x_indices.push(Vec::new());
        }
        self.x_indices[iteration].extend(iteration_x_indices);

        Ok(())
    }
}

impl BunchCompilation<ExecutorId> {
    /// 创建新的任务束编译 / Create new bunch compilation
    pub fn new(
        n_tasks: usize,
        executor_ids: Vec<impl Into<ExecutorId>>,
        with_executor_leisure: bool,
    ) -> Self {
        Self::new_with_ids(n_tasks, executor_ids, with_executor_leisure)
    }
}

// ============================================================================
// 任务束调度解 / Bunch Scheduling Solution
// ============================================================================

/// 束解摘要 / Bunch solution summary
#[derive(Debug, Clone)]
pub struct BunchSolutionSummary {
    /// 选中束数量 / Number of selected bunches
    pub bunch_count: usize,
    /// 已分配任务数量 / Number of assigned tasks
    pub assigned_task_count: usize,
    /// 取消任务数量 / Number of canceled tasks
    pub canceled_task_count: usize,
    /// 总任务数量 / Total task count
    pub total_task_count: usize,
}

/// 束调度解 / Bunch scheduling solution
#[derive(Debug, Clone)]
pub struct BunchSolution {
    /// 选中的束索引列表 / Selected bunch indices
    pub selected_bunches: Vec<usize>,
    /// 取消的任务索引列表 / Canceled task indices
    pub canceled_tasks: Vec<usize>,
}

impl BunchSolution {
    /// 创建空的束解 / Create empty bunch solution
    pub fn empty() -> Self {
        Self {
            selected_bunches: Vec::new(),
            canceled_tasks: Vec::new(),
        }
    }

    /// 获取解摘要 / Get solution summary
    pub fn summary(&self) -> BunchSolutionSummary {
        let assigned = self.selected_bunches.len(); // 简化：束数量代替任务数量
        let canceled = self.canceled_tasks.len();
        BunchSolutionSummary {
            bunch_count: self.selected_bunches.len(),
            assigned_task_count: assigned,
            canceled_task_count: canceled,
            total_task_count: assigned + canceled,
        }
    }
}

// ============================================================================
// SlotBasedBunch
// ============================================================================

/// 基于时隙的束标记 / Slot-based bunch marker
///
/// 关联束与其所属的时隙。
/// Associates a bunch with its time slot.
#[derive(Debug, Clone)]
pub struct SlotBasedBunchEntry<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 基础束条目 / Base bunch entry
    pub bunch: BunchEntry<I>,
    /// 时隙索引 / Slot index
    pub slot_index: usize,
}

// ============================================================================
// SlotBasedCapacityResult
// ============================================================================

/// 基于时隙的产能结果 / Slot-based capacity result
///
/// 存储每个时隙的产能排程结果，用于束生成约束。
/// Stores capacity scheduling results per time slot for bunch generation constraints.
#[derive(Debug, Clone)]
pub struct SlotBasedCapacityResult {
    /// 时隙索引 / Slot index
    pub slot_index: usize,
    /// 产出量映射：product_id -> quantity / Produce quantities by product
    pub produce_quantities: HashMap<String, f64>,
    /// 消耗量映射：material_id -> quantity / Consumption quantities by material
    pub consumption_quantities: HashMap<String, f64>,
    /// 资源使用量映射：resource_id -> quantity / Resource usage quantities by resource
    pub resource_usage_quantities: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bunch_aggregation_add() {
        let mut agg = BunchAggregation::new();
        let entries = vec![
            BunchEntry {
                id: "bunch-0".into(),
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0, 1],
                cost: 10.0,
                iteration: 0,
                slot_index: None,
            },
            BunchEntry {
                id: "bunch-1".into(),
                index: 1,
                executor_id: "exec_2".into(),
                task_indices: vec![2],
                cost: 5.0,
                iteration: 0,
                slot_index: None,
            },
        ];
        let added = agg.add_bunches(0, entries).unwrap();
        assert_eq!(added.len(), 2);
        assert_eq!(agg.active_count(), 2);
    }

    #[test]
    fn test_bunch_aggregation_preserves_distinct_candidates_with_same_tasks() {
        let mut agg = BunchAggregation::new();
        let e1 = BunchEntry {
            id: "bunch-0".into(),
            index: 0,
            executor_id: "exec_1".into(),
            task_indices: vec![0, 1],
            cost: 10.0,
            iteration: 0,
            slot_index: None,
        };
        let added1 = agg.add_bunches(0, vec![e1.clone()]).unwrap();
        assert_eq!(added1.len(), 1);

        // 覆盖集合相同不代表恢复记录相同。 / Equal coverage does not imply equal recovery records.
        let dup = BunchEntry {
            id: "bunch-1".into(),
            index: 1,
            executor_id: "exec_1".into(),
            task_indices: vec![0, 1],
            cost: 12.0,
            iteration: 0,
            slot_index: None,
        };
        let added2 = agg.add_bunches(0, vec![dup]).unwrap();
        assert_eq!(added2, vec![1]);
        assert_eq!(agg.active_count(), 2);
    }

    fn entry(index: usize) -> BunchEntry {
        BunchEntry { id: format!("candidate-{index}").into(), index, executor_id: "exec_1".into(), task_indices: vec![0], cost: 10.0, iteration: 0, slot_index: None }
    }

    #[test]
    fn stable_id_is_idempotent_but_rejects_payload_conflicts_atomically() {
        let mut aggregation = BunchAggregation::new();
        let first = entry(40);
        assert_eq!(aggregation.add_bunches(2, vec![first.clone()]).unwrap(), vec![40]);
        let mut duplicate = first.clone();
        duplicate.index = 90;
        assert!(aggregation.add_bunches(3, vec![duplicate]).unwrap().is_empty());
        for conflicting in [
            BunchEntry { cost: 9.0, ..first.clone() },
            BunchEntry { task_indices: vec![1], ..first.clone() },
            BunchEntry { executor_id: "exec_2".into(), ..first.clone() },
            BunchEntry { slot_index: Some(0), ..first.clone() },
        ] {
            assert!(aggregation.add_bunches(3, vec![entry(41), conflicting]).is_err());
            assert_eq!(aggregation.all_bunches().len(), 1);
            assert!(aggregation.bunches_for_iteration(3).is_empty());
        }
        assert_eq!(aggregation.bunches_for_iteration(2)[0].index, 40);
        assert_eq!(aggregation.bunches_for_iteration(2)[0].iteration, 2);
    }

    #[test]
    fn retired_candidate_requires_fresh_index_and_cannot_change_payload() {
        let mut aggregation = BunchAggregation::new();
        let first = entry(40);
        aggregation.add_bunches(0, vec![first.clone()]).unwrap();
        aggregation.remove(40);
        aggregation.remove(999);
        assert_eq!(aggregation.active_count(), 0);
        assert!(aggregation.add_bunches(1, vec![first.clone()]).is_err());
        assert!(aggregation.add_bunches(1, vec![BunchEntry { index: 41, cost: 20.0, ..first.clone() }]).is_err());
        assert_eq!(aggregation.add_bunches(1, vec![BunchEntry { index: 41, ..first }]).unwrap(), vec![41]);
        assert!(aggregation.bunches_for_iteration(0).is_empty());
        assert_eq!(aggregation.bunches_for_iteration(1)[0].index, 41);
        assert_eq!(aggregation.active_count(), 1);
    }

    #[test]
    fn compilation_rejects_unregistered_foreign_and_changed_models() {
        let mut compilation = BunchCompilation::new(1, vec!["exec_1"], false);
        let mut model = MetaModel::<f64>::new("owner");
        let mut foreign = MetaModel::<f64>::new("foreign");
        assert!(compilation.add_columns(0, vec![entry(40)], &mut model).is_err());
        compilation.register(&mut model).unwrap();
        let tokens = model.as_basic().num_tokens();
        let symbols = model.as_basic().symbols().len();
        compilation.register(&mut model).unwrap();
        assert_eq!(model.as_basic().num_tokens(), tokens);
        assert_eq!(model.as_basic().symbols().len(), symbols);
        assert!(compilation.register(&mut foreign).is_err());
        assert!(compilation.add_columns(0, vec![entry(40)], &mut foreign).is_err());
        assert_eq!(foreign.as_basic().num_tokens(), 0);
        compilation.n_tasks = 2;
        assert!(compilation.register(&mut model).is_err());
        assert_eq!(model.as_basic().num_tokens(), tokens);
    }

    #[test]
    fn compilation_prevalidates_complete_batch_and_duplicate_executors() {
        for executors in [vec!["exec_1", "exec_1"], vec![""]] {
            let mut compilation = BunchCompilation::new(1, executors, false);
            let mut model = MetaModel::<f64>::new("invalid_executors");
            assert!(compilation.register(&mut model).is_err());
            assert_eq!(model.as_basic().num_tokens(), 0);
        }
        let mut compilation = BunchCompilation::new(1, vec!["exec_1"], false);
        let mut model = MetaModel::<f64>::new("batch");
        compilation.register(&mut model).unwrap();
        let invalid = [
            BunchEntry { executor_id: "unknown".into(), ..entry(41) },
            BunchEntry { task_indices: vec![1], ..entry(41) },
            BunchEntry { task_indices: vec![0, 0], ..entry(41) },
            BunchEntry { cost: f64::NAN, ..entry(41) },
            BunchEntry { id: "".into(), ..entry(41) },
            BunchEntry { index: 40, ..entry(41) },
        ];
        for invalid in invalid {
            assert!(compilation.add_columns(0, vec![entry(40), invalid], &mut model).is_err());
            assert_eq!(model.as_basic().num_tokens(), 1);
            assert!(compilation.aggregation.all_bunches().is_empty());
            assert!(compilation.x_indices.is_empty());
        }
    }

    #[test]
    fn test_bunch_aggregation_keeps_same_tasks_in_different_slots() {
        let mut aggregation = BunchAggregation::new();
        let entries = vec![
            BunchEntry {
                id: "bunch-0".into(),
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0],
                cost: 10.0,
                iteration: 0,
                slot_index: Some(0),
            },
            BunchEntry {
                id: "bunch-1".into(),
                index: 1,
                executor_id: "exec_1".into(),
                task_indices: vec![0],
                cost: 10.0,
                iteration: 0,
                slot_index: Some(1),
            },
        ];

        let added = aggregation.add_bunches(0, entries).unwrap();

        assert_eq!(added, vec![0, 1]);
        assert_eq!(aggregation.active_count(), 2);
    }

    #[test]
    fn test_bunch_solution_summary() {
        let solution = BunchSolution {
            selected_bunches: vec![0, 1, 2],
            canceled_tasks: vec![3],
        };
        let summary = solution.summary();
        assert_eq!(summary.bunch_count, 3);
        assert_eq!(summary.canceled_task_count, 1);
        assert_eq!(summary.total_task_count, 4);
    }

    #[test]
    fn test_bunch_compilation_register() {
        let mut model = MetaModel::<f64>::new("test_bunch_compilation");

        let mut compilation = BunchCompilation::new(
            3, // 3 tasks
            vec!["exec_1".to_string(), "exec_2".to_string()],
            true, // with executor leisure
        );
        compilation.register(&mut model).unwrap();

        assert_eq!(compilation.y_indices.len(), 3);
        assert_eq!(compilation.z_indices.len(), 2);
        assert!(compilation.bunch_cost_symbol.is_some());
        assert_eq!(compilation.task_compilation_symbols.len(), 3);
        assert_eq!(compilation.executor_compilation_symbols.len(), 2);
    }
}
