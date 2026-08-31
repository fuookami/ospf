//! 迭代任务编译 / Iterative task compilation
//!
//! 支持迭代列生成生命周期的编译模型，
//! 使用索引而非泛型任务类型，以简化接口。
//!
//! Compilation model supporting iterative column generation lifecycle,
//! using indices instead of generic task types to simplify the interface.

use std::collections::HashSet;
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::LinearMonomial;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;

use crate::domain::task::Cost;
use crate::domain::task_compilation::adapter::{
    extract_value, next_gantt_symbol_id,
    symbols_to_indexed_1d,
    IndexedLinearExpressionSymbols1,
};
use crate::GanttResult;
use crate::GanttError;

/// 添加的任务列 / Added task column
///
/// 代表一个任务-执行器对的列生成列。
/// Represents a task-executor pair column for column generation.
#[derive(Debug, Clone)]
pub struct AddedTaskColumn {
    /// 列索引 / Column index
    pub index: usize,
    /// 任务索引 / Task index
    pub task_index: usize,
    /// 执行器索引 / Executor index
    pub executor_index: usize,
    /// 所属迭代 / Iteration when added
    pub iteration: usize,
    /// x 变量模型索引 / x variable model index
    pub x_model_index: usize,
    /// 列成本 / Column cost
    pub cost: Cost<f64>,
}

/// 迭代任务编译 / Iterative task compilation
///
/// 使用索引的迭代任务编译模型，支持完整的列生成生命周期。
/// Iterative task compilation model using indices, supporting full column generation lifecycle.
pub struct IterativeTaskCompilation {
    /// 任务数量 / Task count
    pub n_tasks: usize,
    /// 执行器数量 / Executor count
    pub n_executors: usize,
    /// 是否允许取消任务 / Whether task cancellation is enabled
    pub task_cancel_enabled: bool,
    /// 是否包含执行器空闲变量 / Whether executor leisure variables are included
    pub with_executor_leisure: bool,

    /// 已添加的列 / Added columns
    pub columns: Vec<AddedTaskColumn>,
    /// 已移除的列索引 / Removed column indices
    pub removed_columns: HashSet<usize>,
    /// 列计数器 / Column counter
    pub column_counter: usize,

    /// y[task] 取消变量模型索引 / y[task] cancellation variable model indices
    pub y_indices: Vec<usize>,
    /// z[executor] 空闲变量模型索引 / z[executor] leisure variable model indices
    pub z_indices: Vec<usize>,

    // ---- 累积项源（Option C：分开存储，按需重建）----
    // Accumulated term sources (Option C: store separately, rebuild on demand)

    /// task_assignment[task] 项 / task_assignment terms per task
    pub task_assignment_terms: Vec<Vec<(usize, f64)>>,
    /// task_compilation[task] 项 / task_compilation terms per task
    pub task_compilation_terms: Vec<Vec<(usize, f64)>>,
    /// executor_compilation[executor] 项 / executor_compilation terms per executor
    pub executor_compilation_terms: Vec<Vec<(usize, f64)>>,

    /// 已固定的列索引 / Fixed column indices
    pub fixed_columns: HashSet<usize>,
}

impl std::fmt::Debug for IterativeTaskCompilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IterativeTaskCompilation")
            .field("n_tasks", &self.n_tasks)
            .field("n_executors", &self.n_executors)
            .field("column_count", &self.columns.len())
            .field("removed_count", &self.removed_columns.len())
            .field("fixed_count", &self.fixed_columns.len())
            .finish()
    }
}

impl IterativeTaskCompilation {
    /// 创建迭代任务编译 / Create iterative task compilation
    pub fn new(
        n_tasks: usize,
        n_executors: usize,
        task_cancel_enabled: bool,
        with_executor_leisure: bool,
    ) -> Self {
        Self {
            n_tasks,
            n_executors,
            task_cancel_enabled,
            with_executor_leisure,
            columns: Vec::new(),
            removed_columns: HashSet::new(),
            column_counter: 0,
            y_indices: Vec::new(),
            z_indices: Vec::new(),
            task_assignment_terms: vec![Vec::new(); n_tasks],
            task_compilation_terms: vec![Vec::new(); n_tasks],
            executor_compilation_terms: vec![Vec::new(); n_executors],
            fixed_columns: HashSet::new(),
        }
    }

    /// 注册初始模型 / Register initial model
    ///
    /// 注册 y[task] 取消变量和 z[executor] 空闲变量。
    /// Registers y[task] cancellation variables and z[executor] leisure variables.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        // 1. 注册 y[task] 取消变量
        if self.task_cancel_enabled {
            self.y_indices.clear();
            for ti in 0..self.n_tasks {
                let var_name = format!("y_{}", ti);
                let var_item = ospf_rust_core::variable::VariableItem::<ospf_rust_core::variable::Binary>::create(
                    ospf_rust_core::variable::VariableId::standalone(next_gantt_symbol_id() as usize),
                    &var_name,
                );
                let model_idx = model.register_variable(var_item)
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register variable {}: {:?}", var_name, e),
                    })?;
                self.y_indices.push(model_idx);
            }

            // 初始化 task_compilation_terms 的 y 项
            for ti in 0..self.n_tasks {
                self.task_compilation_terms[ti] = vec![(self.y_indices[ti], 1.0)];
            }
        }

        // 2. 注册 z[executor] 空闲变量
        if self.with_executor_leisure {
            self.z_indices.clear();
            for ei in 0..self.n_executors {
                let var_name = format!("z_{}", ei);
                let var_item = ospf_rust_core::variable::VariableItem::<ospf_rust_core::variable::Binary>::create(
                    ospf_rust_core::variable::VariableId::standalone(next_gantt_symbol_id() as usize),
                    &var_name,
                );
                let model_idx = model.register_variable(var_item)
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register variable {}: {:?}", var_name, e),
                    })?;
                self.z_indices.push(model_idx);
            }

            // 初始化 executor_compilation_terms 的 z 项
            for ei in 0..self.n_executors {
                self.executor_compilation_terms[ei] = vec![(self.z_indices[ei], 1.0)];
            }
        }

        Ok(())
    }

    /// 添加列 / Add columns
    ///
    /// 为新的任务-执行器对注册变量并更新中间表达式。
    /// Registers variables for new task-executor pairs and updates intermediate expressions.
    pub fn add_columns(
        &mut self,
        iteration: usize,
        new_pairs: Vec<(usize, usize, Cost<f64>)>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<AddedTaskColumn>> {
        let mut added_columns = Vec::new();

        for (task_idx, executor_idx, cost) in new_pairs {
            // 去重检查
            let is_duplicate = self.columns.iter().any(|c| {
                !self.removed_columns.contains(&c.index)
                    && c.task_index == task_idx
                    && c.executor_index == executor_idx
            });
            if is_duplicate {
                continue;
            }

            // 注册 x 变量
            let var_name = format!("x_add_{}_{}_{}", iteration, task_idx, executor_idx);
            let var_item = ospf_rust_core::variable::VariableItem::<ospf_rust_core::variable::Binary>::create(
                ospf_rust_core::variable::VariableId::standalone(next_gantt_symbol_id() as usize),
                &var_name,
            );
            let x_model_idx = model.register_variable(var_item)
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register variable {}: {:?}", var_name, e),
                })?;

            let col_idx = self.column_counter;
            self.column_counter += 1;

            let column = AddedTaskColumn {
                index: col_idx,
                task_index: task_idx,
                executor_index: executor_idx,
                iteration,
                x_model_index: x_model_idx,
                cost,
            };
            self.columns.push(column.clone());

            // 更新项累积器
            if task_idx < self.task_assignment_terms.len() {
                self.task_assignment_terms[task_idx].push((x_model_idx, 1.0));
            }
            if task_idx < self.task_compilation_terms.len() {
                self.task_compilation_terms[task_idx].push((x_model_idx, 1.0));
            }
            if executor_idx < self.executor_compilation_terms.len() {
                self.executor_compilation_terms[executor_idx].push((x_model_idx, 1.0));
            }

            added_columns.push(column);
        }

        // 重建中间表达式
        self.rebuild_intermediate_symbols(model)?;

        Ok(added_columns)
    }

    /// 移除列 / Remove columns
    ///
    /// 软删除列，从项累积器中移除对应项。
    /// Soft-deletes columns, removes corresponding terms from accumulators.
    pub fn remove_columns(&mut self, column_indices: &[usize]) {
        for &col_idx in column_indices {
            self.removed_columns.insert(col_idx);
            self.fixed_columns.remove(&col_idx);

            if let Some(col) = self.columns.iter().find(|c| c.index == col_idx) {
                let x_idx = col.x_model_index;

                if col.task_index < self.task_assignment_terms.len() {
                    self.task_assignment_terms[col.task_index].retain(|(idx, _)| *idx != x_idx);
                }
                if col.task_index < self.task_compilation_terms.len() {
                    self.task_compilation_terms[col.task_index].retain(|(idx, _)| *idx != x_idx);
                }
                if col.executor_index < self.executor_compilation_terms.len() {
                    self.executor_compilation_terms[col.executor_index].retain(|(idx, _)| *idx != x_idx);
                }
            }
        }
    }

    /// 重建中间表达式 / Rebuild intermediate expressions
    ///
    /// 基于当前项累积器重建所有中间表达式符号。
    /// Rebuilds all intermediate expression symbols from current term accumulators.
    pub fn rebuild_intermediate_symbols(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        // 重建 task_assignment_symbols
        let mut task_assignment_symbols = Vec::with_capacity(self.n_tasks);
        for ti in 0..self.n_tasks {
            let terms: Vec<LinearMonomial<f64>> = self.task_assignment_terms[ti].iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();
            let sym_id = next_gantt_symbol_id();
            let symbol = Arc::new(LinearExpressionSymbol::new(
                sym_id,
                &format!("task_assignment_{}", ti),
                terms,
                0.0,
            ));
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to rebuild task_assignment_{}: {:?}", ti, e),
                })?;
            task_assignment_symbols.push(symbol);
        }

        // 重建 task_compilation_symbols
        let mut task_compilation_symbols = Vec::with_capacity(self.n_tasks);
        for ti in 0..self.n_tasks {
            let terms: Vec<LinearMonomial<f64>> = self.task_compilation_terms[ti].iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();
            let sym_id = next_gantt_symbol_id();
            let symbol = Arc::new(LinearExpressionSymbol::new(
                sym_id,
                &format!("task_compilation_{}", ti),
                terms,
                0.0,
            ));
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to rebuild task_compilation_{}: {:?}", ti, e),
                })?;
            task_compilation_symbols.push(symbol);
        }

        // 重建 executor_compilation_symbols
        let mut executor_compilation_symbols = Vec::with_capacity(self.n_executors);
        for ei in 0..self.n_executors {
            let terms: Vec<LinearMonomial<f64>> = self.executor_compilation_terms[ei].iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();
            let sym_id = next_gantt_symbol_id();
            let symbol = Arc::new(LinearExpressionSymbol::new(
                sym_id,
                &format!("executor_compilation_{}", ei),
                terms,
                0.0,
            ));
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to rebuild executor_compilation_{}: {:?}", ei, e),
                })?;
            executor_compilation_symbols.push(symbol);
        }

        Ok(())
    }

    /// 刷新符号池 / Refresh symbol pool
    ///
    /// 重建中间表达式并返回最新的索引符号组合。
    /// Rebuilds intermediate expressions and returns the latest indexed symbol combinations.
    ///
    /// 用于约束层在列操作后获取最新的符号引用。
    /// Used by constraint layer to get latest symbol references after column operations.
    pub fn refresh_symbols(
        &mut self,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<(
        Option<IndexedLinearExpressionSymbols1<usize>>,
        Option<IndexedLinearExpressionSymbols1<usize>>,
        Option<IndexedLinearExpressionSymbols1<usize>>,
    )> {
        self.rebuild_intermediate_symbols(model)?;
        Ok(self.active_symbols())
    }

    /// 替换符号池 / Replace symbol pool
    ///
    /// 强制重建所有中间表达式符号，替换现有符号池。
    /// Forces rebuild of all intermediate expression symbols, replacing the existing symbol pool.
    ///
    /// 用于完全重建场景（如 warm start 后）。
    /// Used for full rebuild scenarios (e.g., after warm start).
    pub fn replace_symbol_pool(
        &mut self,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.rebuild_intermediate_symbols(model)
    }

    /// 获取活跃符号 / Get active symbols
    ///
    /// 返回当前最新的索引符号组合（不触发重建）。
    /// Returns the latest indexed symbol combinations without triggering rebuild.
    ///
    /// 返回值为 (task_assignment, task_compilation, executor_compilation)。
    /// Return value is (task_assignment, task_compilation, executor_compilation).
    pub fn active_symbols(
        &self,
    ) -> (
        Option<IndexedLinearExpressionSymbols1<usize>>,
        Option<IndexedLinearExpressionSymbols1<usize>>,
        Option<IndexedLinearExpressionSymbols1<usize>>,
    ) {
        let task_keys: Vec<usize> = (0..self.n_tasks).collect();
        let executor_keys: Vec<usize> = (0..self.n_executors).collect();

        // 重建临时 Vec 用于构建索引（rebuild_intermediate_symbols 不保存符号到 self）
        // 从 term 累积器重建符号以构建索引
        let task_assignment_symbols: Vec<Arc<LinearExpressionSymbol<f64>>> =
            self.task_assignment_terms.iter().enumerate().map(|(ti, terms)| {
                let monomials: Vec<ospf_rust_core::model::flatten::LinearMonomial<f64>> = terms.iter()
                    .map(|&(idx, coeff)| ospf_rust_core::model::flatten::LinearMonomial::new(coeff, idx))
                    .collect();
                Arc::new(LinearExpressionSymbol::new(
                    0, // placeholder ID
                    &format!("task_assignment_{}", ti),
                    monomials,
                    0.0,
                ))
            }).collect();

        let task_compilation_symbols: Vec<Arc<LinearExpressionSymbol<f64>>> =
            self.task_compilation_terms.iter().enumerate().map(|(ti, terms)| {
                let monomials: Vec<ospf_rust_core::model::flatten::LinearMonomial<f64>> = terms.iter()
                    .map(|&(idx, coeff)| ospf_rust_core::model::flatten::LinearMonomial::new(coeff, idx))
                    .collect();
                Arc::new(LinearExpressionSymbol::new(
                    0,
                    &format!("task_compilation_{}", ti),
                    monomials,
                    0.0,
                ))
            }).collect();

        let executor_compilation_symbols: Vec<Arc<LinearExpressionSymbol<f64>>> =
            self.executor_compilation_terms.iter().enumerate().map(|(ei, terms)| {
                let monomials: Vec<ospf_rust_core::model::flatten::LinearMonomial<f64>> = terms.iter()
                    .map(|&(idx, coeff)| ospf_rust_core::model::flatten::LinearMonomial::new(coeff, idx))
                    .collect();
                Arc::new(LinearExpressionSymbol::new(
                    0,
                    &format!("executor_compilation_{}", ei),
                    monomials,
                    0.0,
                ))
            }).collect();

        let task_assignment_indexed = if !task_assignment_symbols.is_empty() {
            Some(symbols_to_indexed_1d("task_assignment", &task_keys, &task_assignment_symbols))
        } else {
            None
        };
        let task_compilation_indexed = if !task_compilation_symbols.is_empty() {
            Some(symbols_to_indexed_1d("task_compilation", &task_keys, &task_compilation_symbols))
        } else {
            None
        };
        let executor_compilation_indexed = if !executor_compilation_symbols.is_empty() {
            Some(symbols_to_indexed_1d("executor_compilation", &executor_keys, &executor_compilation_symbols))
        } else {
            None
        };

        (task_assignment_indexed, task_compilation_indexed, executor_compilation_indexed)
    }

    /// 全局固定 / Globally fix
    ///
    /// 将指定列的选择变量固定为已选择（值为 1）。
    /// Fixes specified column selection variables to selected (value = 1).
    pub fn globally_fix(&mut self, column_indices: &HashSet<usize>) {
        for &idx in column_indices {
            self.fixed_columns.insert(idx);
        }
    }

    /// 局部固定 / Locally fix
    ///
    /// 将解值超过阈值的列标记为固定。
    /// Marks columns with solution value above threshold as fixed.
    pub fn locally_fix(
        &mut self,
        threshold: f64,
        solution: &[f64],
    ) -> HashSet<usize> {
        let mut newly_fixed = HashSet::new();

        for col in &self.columns {
            if self.removed_columns.contains(&col.index)
                || self.fixed_columns.contains(&col.index)
            {
                continue;
            }

            if let Some(value) = extract_value(solution, col.x_model_index) {
                if value >= threshold {
                    self.fixed_columns.insert(col.index);
                    newly_fixed.insert(col.index);
                }
            }
        }

        newly_fixed
    }

    /// 刷新变量范围 / Flush variable ranges
    ///
    /// 清除固定集合，为下一轮迭代准备。
    /// Clears fixed set, preparing for next iteration.
    pub fn flush(&mut self) {
        self.fixed_columns.clear();
    }

    /// 提取已固定的列 / Extract fixed columns
    pub fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize> {
        let mut fixed = HashSet::new();
        for col in &self.columns {
            if self.removed_columns.contains(&col.index) {
                continue;
            }
            if let Some(value) = extract_value(solution, col.x_model_index) {
                if value > 0.5 {
                    fixed.insert(col.index);
                }
            }
        }
        fixed
    }

    /// 提取保留的列 / Extract kept columns
    pub fn extract_kept(&self, solution: &[f64]) -> HashSet<usize> {
        let mut kept = HashSet::new();
        for col in &self.columns {
            if self.removed_columns.contains(&col.index) {
                continue;
            }
            if let Some(value) = extract_value(solution, col.x_model_index) {
                if value > 1e-6 {
                    kept.insert(col.index);
                }
            }
        }
        kept
    }

    /// 活跃列数量 / Active column count
    pub fn active_column_count(&self) -> usize {
        self.columns.len() - self.removed_columns.len()
    }

    /// 获取每个任务的列 / Get columns per task
    pub fn columns_for_task(&self, task_index: usize) -> Vec<&AddedTaskColumn> {
        self.columns.iter()
            .filter(|c| {
                c.task_index == task_index
                    && !self.removed_columns.contains(&c.index)
            })
            .collect()
    }

    /// 获取每个执行器的列 / Get columns per executor
    pub fn columns_for_executor(&self, executor_index: usize) -> Vec<&AddedTaskColumn> {
        self.columns.iter()
            .filter(|c| {
                c.executor_index == executor_index
                    && !self.removed_columns.contains(&c.index)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterative_task_compilation_register() {
        let mut model = MetaModel::<f64>::new("test_iter_task_register");

        let mut compilation = IterativeTaskCompilation::new(
            3, // 3 tasks
            2, // 2 executors
            true,
            true,
        );
        compilation.register(&mut model).unwrap();

        assert_eq!(compilation.y_indices.len(), 3);
        assert_eq!(compilation.z_indices.len(), 2);
        assert_eq!(compilation.task_assignment_terms.len(), 3);
        assert_eq!(compilation.executor_compilation_terms.len(), 2);

        // y 项应在 task_compilation_terms 中
        for ti in 0..3 {
            assert_eq!(compilation.task_compilation_terms[ti].len(), 1);
            assert_eq!(compilation.task_compilation_terms[ti][0].0, compilation.y_indices[ti]);
        }

        // z 项应在 executor_compilation_terms 中
        for ei in 0..2 {
            assert_eq!(compilation.executor_compilation_terms[ei].len(), 1);
            assert_eq!(compilation.executor_compilation_terms[ei][0].0, compilation.z_indices[ei]);
        }
    }

    #[test]
    fn test_iterative_task_compilation_add_and_remove_columns() {
        let mut model = MetaModel::<f64>::new("test_iter_task_add_remove");

        let mut compilation = IterativeTaskCompilation::new(
            2, // 2 tasks
            1, // 1 executor
            false,
            false,
        );
        compilation.register(&mut model).unwrap();

        // 添加 2 个列
        let new_pairs = vec![
            (0, 0, Cost::empty()), // task 0 -> executor 0
            (1, 0, Cost::empty()), // task 1 -> executor 0
        ];
        let added = compilation.add_columns(0, new_pairs, &mut model).unwrap();
        assert_eq!(added.len(), 2);
        assert_eq!(compilation.active_column_count(), 2);

        // task 0 应有 y[0] + x[0,0] 两个项
        assert_eq!(compilation.task_assignment_terms[0].len(), 1); // 只有 x
        // executor 0 应有 z[0] + x[0,0] + x[1,0] 三个项 -> 没有 z
        assert_eq!(compilation.executor_compilation_terms[0].len(), 2); // 两个 x

        // 移除第 1 个列
        compilation.remove_columns(&[added[0].index]);
        assert_eq!(compilation.active_column_count(), 1);

        // task 0 应只剩 0 项（列被移除）
        assert_eq!(compilation.task_assignment_terms[0].len(), 0);
    }

    #[test]
    fn test_iterative_task_compilation_globally_fix() {
        let mut compilation = IterativeTaskCompilation::new(2, 1, false, false);

        let fixed: HashSet<usize> = [0, 2].into_iter().collect();
        compilation.globally_fix(&fixed);
        assert!(compilation.fixed_columns.contains(&0));
        assert!(compilation.fixed_columns.contains(&2));
        assert!(!compilation.fixed_columns.contains(&1));
    }

    #[test]
    fn test_iterative_task_compilation_dedup() {
        let mut model = MetaModel::<f64>::new("test_iter_task_dedup");

        let mut compilation = IterativeTaskCompilation::new(
            2, // 2 tasks
            1, // 1 executor
            false,
            false,
        );
        compilation.register(&mut model).unwrap();

        // 第一批列
        let pairs_1 = vec![
            (0, 0, Cost::empty()), // task 0 -> executor 0
        ];
        compilation.add_columns(0, pairs_1, &mut model).unwrap();

        // 重复列（应被去重）
        let pairs_2 = vec![
            (0, 0, Cost::empty()), // 同任务-执行器 → 去重
            (1, 0, Cost::empty()), // 不同任务 → 不去重
        ];
        let added = compilation.add_columns(1, pairs_2, &mut model).unwrap();

        // 只有束 1 的列被添加（束 0 的重复列被去重）
        assert_eq!(added.len(), 1);
        assert_eq!(compilation.active_column_count(), 2);
    }
}
