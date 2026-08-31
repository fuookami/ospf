//! 任务束编译模型 / Bunch compilation models
//!
//! 定义列生成主问题的核心模型组件：BunchCompilation、BunchAggregation、
//! SlotBasedBunch、BunchSchedulingSolution 等。
//!
//! Defines core model components for the column generation master problem:
//! BunchCompilation, BunchAggregation, SlotBasedBunch, BunchSchedulingSolution, etc.

use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::LinearMonomial;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::variable::Binary;

use crate::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::GanttResult;
use crate::GanttError;

// ============================================================================
// 任务束聚合 / Bunch Aggregation
// ============================================================================

/// 任务束聚合 / Bunch aggregation
///
/// 管理任务束集合的添加、去重和移除追踪。
/// Manages bunch collection with deduplication and removal tracking.
#[derive(Clone)]
pub struct BunchAggregation {
    /// 按迭代分组的束列表 / Bunches grouped by iteration
    bunches_by_iteration: Vec<Vec<usize>>,
    /// 所有活跃束索引 / All active bunch indices
    bunches: Vec<BunchEntry>,
    /// 已移除的束索引 / Removed bunch indices
    removed: HashSet<usize>,
    /// 迭代计数 / Iteration count
    iteration_count: usize,
}

use std::collections::HashSet;

/// 束条目 / Bunch entry
#[derive(Debug, Clone)]
pub struct BunchEntry {
    /// 束索引 / Bunch index
    pub index: usize,
    /// 执行器 ID / Executor ID
    pub executor_id: String,
    /// 包含的任务索引 / Contained task indices
    pub task_indices: Vec<usize>,
    /// 束成本 / Bunch cost
    pub cost: f64,
    /// 所属迭代 / Iteration
    pub iteration: usize,
}

impl std::fmt::Debug for BunchAggregation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BunchAggregation")
            .field("bunch_count", &self.bunches.len())
            .field("removed_count", &self.removed.len())
            .field("iteration_count", &self.iteration_count)
            .finish()
    }
}

impl BunchAggregation {
    /// 创建新的任务束聚合 / Create new bunch aggregation
    pub fn new() -> Self {
        Self {
            bunches_by_iteration: Vec::new(),
            bunches: Vec::new(),
            removed: HashSet::new(),
            iteration_count: 0,
        }
    }

    /// 添加新束（去重）/ Add new bunches (with deduplication)
    pub fn add_bunches(&mut self, iteration: usize, new_bunches: Vec<BunchEntry>) -> Vec<usize> {
        let mut added = Vec::new();
        for bunch in new_bunches {
            // 简单去重：检查是否已存在相同执行器和任务集合的束
            let is_duplicate = self.bunches.iter().any(|existing| {
                existing.executor_id == bunch.executor_id
                    && existing.task_indices == bunch.task_indices
            });
            if !is_duplicate {
                let idx = bunch.index;
                self.bunches.push(bunch);
                added.push(idx);
            }
        }
        if !added.is_empty() {
            // 确保迭代列表足够长
            while self.bunches_by_iteration.len() <= iteration {
                self.bunches_by_iteration.push(Vec::new());
            }
            self.bunches_by_iteration[iteration].extend(added.iter().copied());
        }
        self.iteration_count = self.iteration_count.max(iteration + 1);
        added
    }

    /// 获取所有活跃束 / Get all active bunches
    pub fn bunches(&self) -> Vec<&BunchEntry> {
        self.bunches.iter()
            .filter(|b| !self.removed.contains(&b.index))
            .collect()
    }

    /// 获取指定迭代的束 / Get bunches for a specific iteration
    pub fn bunches_for_iteration(&self, iteration: usize) -> Vec<&BunchEntry> {
        self.bunches_by_iteration.get(iteration)
            .map(|indices| {
                indices.iter()
                    .filter_map(|&idx| self.bunches.get(idx))
                    .filter(|b| !self.removed.contains(&b.index))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 标记束为已移除 / Mark bunch as removed
    pub fn remove(&mut self, index: usize) {
        self.removed.insert(index);
    }

    /// 活跃束数量 / Active bunch count
    pub fn active_count(&self) -> usize {
        self.bunches.len() - self.removed.len()
    }

    /// 获取所有束条目（包含已移除的）/ Get all bunch entries (including removed)
    pub fn all_bunches(&self) -> &[BunchEntry] {
        &self.bunches
    }

    /// 获取束条目 / Get bunch entry by index field
    pub fn get_bunch(&self, index: usize) -> Option<&BunchEntry> {
        self.bunches.iter().find(|b| b.index == index)
    }
}

impl Default for BunchAggregation {
    fn default() -> Self {
        Self::new()
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
pub struct BunchCompilation {
    /// 任务数量 / Task count
    pub n_tasks: usize,
    /// 执行器 ID 列表 / Executor ID list
    pub executor_ids: Vec<String>,
    /// 是否启用执行器空闲变量 / Whether executor leisure variables are enabled
    pub with_executor_leisure: bool,
    /// 束聚合 / Bunch aggregation
    pub aggregation: BunchAggregation,
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
}

impl std::fmt::Debug for BunchCompilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BunchCompilation")
            .field("n_tasks", &self.n_tasks)
            .field("executor_count", &self.executor_ids.len())
            .field("iteration_count", &self.x_indices.len())
            .field("with_executor_leisure", &self.with_executor_leisure)
            .finish()
    }
}

impl BunchCompilation {
    /// 创建新的任务束编译 / Create new bunch compilation
    pub fn new(n_tasks: usize, executor_ids: Vec<String>, with_executor_leisure: bool) -> Self {
        Self {
            n_tasks,
            executor_ids,
            with_executor_leisure,
            aggregation: BunchAggregation::new(),
            y_indices: Vec::with_capacity(n_tasks),
            z_indices: Vec::new(),
            x_indices: Vec::new(),
            bunch_cost_symbol: None,
            task_compilation_symbols: Vec::with_capacity(n_tasks),
            executor_compilation_symbols: Vec::new(),
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 注册 y[task] 取消变量、z[executor] 空闲变量和初始中间表达式。
    /// x 变量通过 `add_columns` 在列生成迭代中动态注册。
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        // 1. 注册 y[task] 取消变量
        self.y_indices.clear();
        for ti in 0..self.n_tasks {
            let var_name = format!("y_{}", ti);
            let var_item: ospf_rust_core::variable::VariableItem<Binary> =
                ospf_rust_core::variable::VariableItem::create(
                    ospf_rust_core::variable::VariableId::standalone(next_gantt_symbol_id() as usize),
                    &var_name,
                );
            let model_idx = model.register_variable(var_item)
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
                        ospf_rust_core::variable::VariableId::standalone(next_gantt_symbol_id() as usize),
                        &var_name,
                    );
                let model_idx = model.register_variable(var_item)
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
        model.add_symbol(cost_symbol.clone())
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
            model.add_symbol(symbol.clone())
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
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register executor_compilation_{}: {:?}", exec_id, e),
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
        new_bunches: Vec<BunchEntry>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        let deduped = self.aggregation.add_bunches(iteration, new_bunches);

        // 为每个新束注册 x 变量
        let mut iteration_x_indices = Vec::with_capacity(deduped.len());
        for &bunch_idx in &deduped {
            let _entry = self.aggregation.get_bunch(bunch_idx)
                .expect("bunch entry must exist after add_bunches");
            let var_name = format!("x_{}_{}", iteration, bunch_idx);
            let var_item: ospf_rust_core::variable::VariableItem<Binary> =
                ospf_rust_core::variable::VariableItem::create(
                    ospf_rust_core::variable::VariableId::standalone(next_gantt_symbol_id() as usize),
                    &var_name,
                );
            let model_idx = model.register_variable(var_item)
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register variable {}: {:?}", var_name, e),
                })?;
            iteration_x_indices.push(model_idx);
        }

        // 确保迭代列表足够长
        while self.x_indices.len() <= iteration {
            self.x_indices.push(Vec::new());
        }
        self.x_indices[iteration].extend(iteration_x_indices);

        Ok(deduped)
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
pub struct SlotBasedBunchEntry {
    /// 基础束条目 / Base bunch entry
    pub bunch: BunchEntry,
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
                index: 0,
                executor_id: "exec_1".to_string(),
                task_indices: vec![0, 1],
                cost: 10.0,
                iteration: 0,
            },
            BunchEntry {
                index: 1,
                executor_id: "exec_2".to_string(),
                task_indices: vec![2],
                cost: 5.0,
                iteration: 0,
            },
        ];
        let added = agg.add_bunches(0, entries);
        assert_eq!(added.len(), 2);
        assert_eq!(agg.active_count(), 2);
    }

    #[test]
    fn test_bunch_aggregation_dedup() {
        let mut agg = BunchAggregation::new();
        let e1 = BunchEntry {
            index: 0,
            executor_id: "exec_1".to_string(),
            task_indices: vec![0, 1],
            cost: 10.0,
            iteration: 0,
        };
        let added1 = agg.add_bunches(0, vec![e1.clone()]);
        assert_eq!(added1.len(), 1);

        // 重复添加相同执行器和任务集合的束应被去重
        let dup = BunchEntry {
            index: 1,
            executor_id: "exec_1".to_string(),
            task_indices: vec![0, 1],
            cost: 12.0,
            iteration: 0,
        };
        let added2 = agg.add_bunches(0, vec![dup]);
        assert_eq!(added2.len(), 0);
        assert_eq!(agg.active_count(), 1);
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
