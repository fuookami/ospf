//! 迭代束编译 / Iterative bunch compilation
//!
//! 扩展 BunchCompilation 以支持完整的迭代列生成生命周期，
//! 包括 add_columns 时重建中间表达式、remove_columns 时软删除和标记、
//! 影子价格提取和结果提取。
//!
//! Extends BunchCompilation to support full iterative column generation lifecycle,
//! including intermediate expression rebuilding on add_columns, soft-delete on
//! remove_columns, shadow price extraction, and solution extraction.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::LinearMonomial;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::variable::VariableRange;

use crate::domain::bunch_compilation::model::{BunchCompilation, BunchEntry, BunchSolution};
use crate::domain::common::{ExecutorId, ExecutorIdTrait};
use crate::domain::task_compilation::adapter::{
    IndexedLinearExpressionSymbols1, extract_value, next_gantt_symbol_id, symbols_to_indexed_1d,
};
use crate::GanttError;
use crate::GanttResult;

/// 迭代束编译 / Iterative bunch compilation
///
/// 包装 BunchCompilation 并提供完整的迭代列生成生命周期支持。
/// 核心设计：由于 Rust 的 LinearExpressionSymbol 是不可变的，
/// 在 add_columns/remove_columns 时通过重建符号来更新中间表达式。
///
/// Wraps BunchCompilation with full iterative column generation lifecycle support.
/// Core design: Since Rust's LinearExpressionSymbol is immutable, intermediate
/// expressions are rebuilt when columns are added or removed.
#[derive(Clone)]
pub struct IterativeBunchCompilation<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 基础束编译 / Base bunch compilation
    pub base: BunchCompilation<I>,

    // ---- 累积项源（Option C：分开存储，按需重建）----
    // Accumulated term sources (Option C: store separately, rebuild on demand)

    /// bunchCost 项：(x_model_index, cost) / bunchCost terms
    pub cost_terms: Vec<(usize, f64)>,
    /// taskCompilation[task] 项 / taskCompilation terms per task
    pub task_compilation_terms: Vec<Vec<(usize, f64)>>,
    /// executorCompilation[executor] 项 / executorCompilation terms per executor
    pub executor_compilation_terms: Vec<Vec<(usize, f64)>>,

    /// 已固定的束索引 / Fixed bunch indices
    pub fixed_bunches: HashSet<usize>,
    /// 束索引到 x 变量模型索引的映射 / Bunch index to x variable model index mapping
    pub bunch_x_map: HashMap<usize, usize>,
}

impl<I> std::fmt::Debug for IterativeBunchCompilation<I>
where
    I: ExecutorIdTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IterativeBunchCompilation")
            .field("n_tasks", &self.base.n_tasks)
            .field("executor_count", &self.base.executor_ids.len())
            .field("active_bunch_count", &self.base.aggregation.active_count())
            .field("cost_terms_count", &self.cost_terms.len())
            .field("fixed_bunch_count", &self.fixed_bunches.len())
            .finish()
    }
}

impl<I> IterativeBunchCompilation<I>
where
    I: ExecutorIdTrait,
{
    /// 使用业务 ID 创建迭代束编译 / Create iterative bunch compilation with domain ids
    pub fn new_with_ids(
        n_tasks: usize,
        executor_ids: Vec<impl Into<I>>,
        with_executor_leisure: bool,
    ) -> Self {
        let executor_ids = executor_ids.into_iter().map(Into::into).collect::<Vec<_>>();
        let base = BunchCompilation::new_with_ids(
            n_tasks,
            executor_ids.clone(),
            with_executor_leisure,
        );
        let n_executors = executor_ids.len();

        Self {
            base,
            cost_terms: Vec::new(),
            task_compilation_terms: vec![Vec::new(); n_tasks],
            executor_compilation_terms: vec![Vec::new(); n_executors],
            fixed_bunches: HashSet::new(),
            bunch_x_map: HashMap::new(),
        }
    }

    /// 注册初始模型 / Register initial model
    ///
    /// 委托给基础编译，并初始化项累积器。
    /// Delegates to base compilation and initializes term accumulators.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.base.register(model)?;

        // 初始化 taskCompilation 的 y 项
        for ti in 0..self.base.n_tasks {
            self.task_compilation_terms[ti] =
                vec![(self.base.y_indices[ti], 1.0)];
        }

        // 初始化 executorCompilation 的 z 项
        for (ei, _) in self.base.executor_ids.iter().enumerate() {
            if self.base.with_executor_leisure {
                self.executor_compilation_terms[ei] =
                    vec![(self.base.z_indices[ei], 1.0)];
            } else {
                self.executor_compilation_terms[ei] = Vec::new();
            }
        }

        Ok(())
    }

    /// 添加列 / Add columns
    ///
    /// 注册 x 变量、更新项累积器、重建中间表达式。
    /// Registers x variables, updates term accumulators, rebuilds intermediate expressions.
    pub fn add_columns(
        &mut self,
        iteration: usize,
        new_bunches: Vec<BunchEntry<I>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        // 委托基础编译注册 x 变量
        let added_indices = self.base.add_columns(iteration, new_bunches, model)?;

        // 更新项累积器
        // 需要找到每个添加束对应的 x 变量模型索引
        // 由于 base.add_columns 返回的是 bunch index 列表，
        // 而 x 变量是按添加顺序注册的，我们需要从 base.x_indices 中获取
        // 当前迭代的 x 变量索引
        let iter_x_indices = self.base.x_indices.get(iteration)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // 计算当前迭代中已添加的 x 变量数量（在本次 add_columns 调用之前）
        let prev_x_count = iter_x_indices.len().saturating_sub(added_indices.len());

        for (pos, &bunch_idx) in added_indices.iter().enumerate() {
            let entry = self.base.aggregation.get_bunch(bunch_idx)
                .expect("bunch entry must exist after add_bunches");

            // 获取 x 变量模型索引
            let x_model_idx = iter_x_indices[prev_x_count + pos];

            // 更新 bunch_x_map
            self.bunch_x_map.insert(bunch_idx, x_model_idx);

            // 更新 costTerms
            self.cost_terms.push((x_model_idx, entry.cost));

            // 更新 taskCompilation[task] 项
            for &task_idx in &entry.task_indices {
                if task_idx < self.task_compilation_terms.len() {
                    self.task_compilation_terms[task_idx].push((x_model_idx, 1.0));
                }
            }

            // 更新 executorCompilation[executor] 项
            if let Some(exec_idx) = self.find_executor_index(&entry.executor_id) {
                if exec_idx < self.executor_compilation_terms.len() {
                    self.executor_compilation_terms[exec_idx].push((x_model_idx, 1.0));
                }
            }
        }

        // 重建中间表达式
        self.rebuild_intermediate_symbols(model)?;

        Ok(added_indices)
    }

    /// 移除列 / Remove columns
    ///
    /// 软删除列，从项累积器中移除对应项，标记为已移除。
    /// Soft-deletes columns, removes corresponding terms from accumulators.
    pub fn remove_columns(&mut self, bunch_indices: &[usize]) {
        for &bunch_idx in bunch_indices {
            // 在聚合中标记为已移除
            self.base.aggregation.remove(bunch_idx);
            self.fixed_bunches.remove(&bunch_idx);

            // 从 bunch_x_map 获取 x 变量索引
            if let Some(&x_idx) = self.bunch_x_map.get(&bunch_idx) {
                // 从 costTerms 中移除
                self.cost_terms.retain(|(idx, _)| *idx != x_idx);

                // 从 taskCompilation 和 executorCompilation 中移除
                for terms in &mut self.task_compilation_terms {
                    terms.retain(|(idx, _)| *idx != x_idx);
                }
                for terms in &mut self.executor_compilation_terms {
                    terms.retain(|(idx, _)| *idx != x_idx);
                }

                self.bunch_x_map.remove(&bunch_idx);
            }
        }

        // 注意：不在此处重建符号，调用方应在所有列操作完成后调用 rebuild_intermediate_symbols
    }

    /// 重建中间表达式 / Rebuild intermediate expressions
    ///
    /// 基于当前项累积器重建所有中间表达式符号。
    /// Rebuilds all intermediate expression symbols from current term accumulators.
    pub fn rebuild_intermediate_symbols(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        // 重建 bunchCost
        let cost_terms: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        let cost_id = next_gantt_symbol_id();
        let cost_symbol = Arc::new(LinearExpressionSymbol::new(
            cost_id,
            "bunch_cost",
            cost_terms,
            0.0,
        ));
        model.add_symbol(cost_symbol.clone())
            .map_err(|e| GanttError::Calculation {
                message: format!("Failed to rebuild bunch_cost: {:?}", e),
            })?;
        self.base.bunch_cost_symbol = Some(cost_symbol);

        // 重建 taskCompilation[task]
        self.base.task_compilation_symbols.clear();
        for ti in 0..self.base.n_tasks {
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
            self.base.task_compilation_symbols.push(symbol);
        }

        // 重建 executorCompilation[executor]
        self.base.executor_compilation_symbols.clear();
        for (ei, exec_id) in self.base.executor_ids.iter().enumerate() {
            let terms: Vec<LinearMonomial<f64>> = self.executor_compilation_terms[ei].iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();
            let sym_id = next_gantt_symbol_id();
            let symbol = Arc::new(LinearExpressionSymbol::new(
                sym_id,
                &format!("executor_compilation_{}", exec_id),
                terms,
                0.0,
            ));
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to rebuild executor_compilation_{}: {:?}", exec_id, e),
                })?;
            self.base.executor_compilation_symbols.push(symbol);
        }

        Ok(())
    }

    /// 刷新符号池 / Refresh symbol pool
    ///
    /// 重建中间表达式并返回最新的索引符号组合。
    /// Rebuilds intermediate expressions and returns the latest indexed symbol combinations.
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
    pub fn active_symbols(
        &self,
    ) -> (
        Option<IndexedLinearExpressionSymbols1<usize>>,
        Option<IndexedLinearExpressionSymbols1<usize>>,
        Option<IndexedLinearExpressionSymbols1<usize>>,
    ) {
        let task_keys: Vec<usize> = (0..self.base.n_tasks).collect();
        let executor_keys: Vec<usize> = (0..self.base.executor_ids.len()).collect();

        // 从 term 累积器构建临时符号以创建索引
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

        // bunch cost 作为一维索引
        let cost_symbols: Vec<Arc<LinearExpressionSymbol<f64>>> = if !self.cost_terms.is_empty() {
            let monomials: Vec<ospf_rust_core::model::flatten::LinearMonomial<f64>> = self.cost_terms.iter()
                .map(|&(idx, coeff)| ospf_rust_core::model::flatten::LinearMonomial::new(coeff, idx))
                .collect();
            vec![Arc::new(LinearExpressionSymbol::new(0, "bunch_cost", monomials, 0.0))]
        } else {
            vec![]
        };

        let cost_indexed = if !cost_symbols.is_empty() {
            Some(symbols_to_indexed_1d("bunch_cost", &[0usize], &cost_symbols))
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

        (cost_indexed, task_compilation_indexed, executor_compilation_indexed)
    }

    /// 全局固定 / Globally fix
    ///
    /// 将指定束的选择变量固定为已选择（值为 1）。
    /// Fixes specified bunch selection variables to selected (value = 1).
    pub fn globally_fix(
        &mut self,
        bunch_indices: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        for &idx in bunch_indices {
            self.fixed_bunches.insert(idx);
            if let Some(&x_idx) = self.bunch_x_map.get(&idx) {
                model
                    .fix_variable_by_index(x_idx, 1.0)
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to globally fix bunch {}: {:?}", idx, e),
                    })?;
            }
        }
        Ok(())
    }

    /// 局部固定 / Locally fix
    ///
    /// 将解值超过阈值的束标记为固定。
    /// Marks bunches with solution value above threshold as fixed.
    pub fn locally_fix(
        &mut self,
        _iteration: usize,
        threshold: f64,
        solution: &[f64],
        current_fixed: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        let mut newly_fixed = HashSet::new();
        let mut best = None::<(usize, f64)>;

        for &bunch_idx in &self.base.aggregation.bunches()
            .into_iter()
            .map(|b| b.index)
            .collect::<Vec<_>>()
        {
            if self.fixed_bunches.contains(&bunch_idx) || current_fixed.contains(&bunch_idx) {
                continue;
            }

            if let Some(&x_idx) = self.bunch_x_map.get(&bunch_idx) {
                if let Some(value) = extract_value(solution, x_idx) {
                    if best.map_or(true, |(_, best_value)| value >= best_value) {
                        best = Some((bunch_idx, value));
                    }
                    if value >= threshold {
                        self.fixed_bunches.insert(bunch_idx);
                        newly_fixed.insert(bunch_idx);
                        model
                            .fix_variable_by_index(x_idx, 1.0)
                            .map_err(|e| GanttError::Calculation {
                                message: format!("Failed to locally fix bunch {}: {:?}", bunch_idx, e),
                            })?;
                    }
                }
            }
        }

        if newly_fixed.is_empty() {
            if let Some((bunch_idx, value)) = best {
                if value >= 1.0 - threshold {
                    self.fixed_bunches.insert(bunch_idx);
                    newly_fixed.insert(bunch_idx);
                    if let Some(&x_idx) = self.bunch_x_map.get(&bunch_idx) {
                        model
                            .fix_variable_by_index(x_idx, 1.0)
                            .map_err(|e| GanttError::Calculation {
                                message: format!("Failed to locally fix best bunch {}: {:?}", bunch_idx, e),
                            })?;
                    }
                }
            }
        }

        Ok(newly_fixed)
    }

    /// 隐藏束 / Hide bunches
    ///
    /// 将指定束的选择变量固定为 0。
    /// Fixes selected bunch variables to 0.
    pub fn hide_bunches(
        &self,
        bunch_indices: impl IntoIterator<Item = usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        for bunch_idx in bunch_indices {
            if let Some(&x_idx) = self.bunch_x_map.get(&bunch_idx) {
                model
                    .fix_variable_by_index(x_idx, 0.0)
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to hide bunch {}: {:?}", bunch_idx, e),
                    })?;
            }
        }
        Ok(())
    }

    /// 恢复非移除束的范围 / Restore non-removed bunch ranges
    pub fn restore_non_removed_ranges(&self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        for (&bunch_idx, &x_idx) in &self.bunch_x_map {
            if self.base.aggregation.get_bunch(bunch_idx).is_some() {
                model
                    .set_variable_range_by_index(x_idx, VariableRange::bounded(0.0, 1.0))
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to restore bunch {} range: {:?}", bunch_idx, e),
                    })?;
            }
        }
        Ok(())
    }

    /// 提取隐藏执行器 / Extract hidden executors
    pub fn extract_hidden_executors(&self, solution: &[f64]) -> HashSet<I> {
        let mut hidden = HashSet::new();
        for (executor_index, &z_idx) in self.base.z_indices.iter().enumerate() {
            if let Some(value) = extract_value(solution, z_idx) {
                if value > 1e-6 {
                    if let Some(executor_id) = self.base.executor_ids.get(executor_index) {
                        hidden.insert(executor_id.clone());
                    }
                }
            }
        }
        hidden
    }

    /// 刷新变量范围 / Flush variable ranges
    ///
    /// 重置非固定列的变量范围，为下一轮迭代准备。
    /// Resets variable ranges for non-fixed columns, preparing for next iteration.
    pub fn flush(&mut self) {
        // 在 Kotlin 中，flush 将所有非固定列的 x 变量范围重置为 [0, 1]
        // 这里我们清除固定集合，因为迭代间的固定状态由调用方管理
        self.fixed_bunches.clear();
    }

    /// 提取已固定的束 / Extract fixed bunches
    ///
    /// 提取解值为 1 的束索引。
    /// Extracts bunch indices with solution value = 1.
    pub fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize> {
        let mut fixed = HashSet::new();
        for (&bunch_idx, &x_idx) in &self.bunch_x_map {
            if let Some(value) = extract_value(solution, x_idx) {
                if value > 0.5 {
                    fixed.insert(bunch_idx);
                }
            }
        }
        fixed
    }

    /// 提取保留的束 / Extract kept bunches
    ///
    /// 提取解值 > 0 的束索引（LP 松弛解中）。
    /// Extracts bunch indices with solution value > 0 (in LP relaxation).
    pub fn extract_kept(&self, solution: &[f64]) -> HashSet<usize> {
        let mut kept = HashSet::new();
        for (&bunch_idx, &x_idx) in &self.bunch_x_map {
            if let Some(value) = extract_value(solution, x_idx) {
                if value > 1e-6 {
                    kept.insert(bunch_idx);
                }
            }
        }
        kept
    }

    /// 提取束解 / Extract bunch solution
    ///
    /// 从解向量中提取选中的束和取消的任务。
    /// Extracts selected bunches and canceled tasks from solution vector.
    pub fn extract_solution(&self, solution: &[f64]) -> BunchSolution {
        let mut selected_bunches = Vec::new();
        let mut assigned_tasks = HashSet::new();

        for (&bunch_idx, &x_idx) in &self.bunch_x_map {
            if let Some(value) = extract_value(solution, x_idx) {
                if value > 0.5 {
                    selected_bunches.push(bunch_idx);
                    // 记录该束包含的任务
                    if let Some(entry) = self.base.aggregation.get_bunch(bunch_idx) {
                        for &task_idx in &entry.task_indices {
                            assigned_tasks.insert(task_idx);
                        }
                    }
                }
            }
        }

        // 未分配的任务视为取消
        let canceled_tasks: Vec<usize> = (0..self.base.n_tasks)
            .filter(|ti| !assigned_tasks.contains(ti))
            .collect();

        BunchSolution {
            selected_bunches,
            canceled_tasks,
        }
    }

    /// 活跃束数量 / Active bunch count
    pub fn active_bunch_count(&self) -> usize {
        self.base.aggregation.active_count()
    }

    /// 获取束条目 / Get bunch entry
    pub fn get_bunch_entry(&self, bunch_index: usize) -> Option<BunchEntry<I>> {
        self.base.aggregation.get_bunch(bunch_index).cloned()
    }

    /// 获取指定迭代的所有 x 变量模型索引 / Get x variable model indices for a given iteration
    pub fn x_indices_for_iteration(&self, iteration: usize) -> &[usize] {
        self.base.x_indices.get(iteration).map_or(&[], |v| v)
    }

    // ---- 内部辅助方法 / Internal helper methods ----

    /// 找到执行器 ID 对应的索引 / Find executor index by ID
    fn find_executor_index(&self, executor_id: &I) -> Option<usize> {
        self.base
            .executor_ids
            .iter()
            .position(|id| id == executor_id)
    }
}

impl IterativeBunchCompilation<ExecutorId> {
    /// 创建迭代束编译 / Create iterative bunch compilation
    pub fn new(
        n_tasks: usize,
        executor_ids: Vec<impl Into<ExecutorId>>,
        with_executor_leisure: bool,
    ) -> Self {
        Self::new_with_ids(n_tasks, executor_ids, with_executor_leisure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterative_bunch_compilation_register() {
        let mut model = MetaModel::<f64>::new("test_iterative_bunch_register");

        let mut compilation = IterativeBunchCompilation::new(
            3,
            vec!["exec_1".to_string(), "exec_2".to_string()],
            true,
        );
        compilation.register(&mut model).unwrap();

        assert_eq!(compilation.base.y_indices.len(), 3);
        assert_eq!(compilation.base.z_indices.len(), 2);
        assert!(compilation.base.bunch_cost_symbol.is_some());
        assert_eq!(compilation.task_compilation_terms.len(), 3);
        assert_eq!(compilation.executor_compilation_terms.len(), 2);

        // y 项应在 taskCompilation_terms 中
        for ti in 0..3 {
            assert_eq!(compilation.task_compilation_terms[ti].len(), 1);
            assert_eq!(compilation.task_compilation_terms[ti][0].0, compilation.base.y_indices[ti]);
        }

        // z 项应在 executorCompilation_terms 中
        for ei in 0..2 {
            assert_eq!(compilation.executor_compilation_terms[ei].len(), 1);
            assert_eq!(compilation.executor_compilation_terms[ei][0].0, compilation.base.z_indices[ei]);
        }
    }

    #[test]
    fn test_iterative_bunch_compilation_add_columns() {
        let mut model = MetaModel::<f64>::new("test_iterative_bunch_add_columns");

        let mut compilation = IterativeBunchCompilation::new(
            3,
            vec!["exec_1".to_string(), "exec_2".to_string()],
            true,
        );
        compilation.register(&mut model).unwrap();

        // 添加初始列
        let bunches = vec![
            BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0, 1],
                cost: 5.0,
                iteration: 0,
                slot_index: None,
            },
            BunchEntry {
                index: 1,
                executor_id: "exec_2".into(),
                task_indices: vec![2],
                cost: 3.0,
                iteration: 0,
                slot_index: None,
            },
        ];

        let added = compilation.add_columns(0, bunches, &mut model).unwrap();
        assert_eq!(added.len(), 2);

        // 验证项累积器更新
        assert_eq!(compilation.cost_terms.len(), 2);
        assert!(compilation.cost_terms.iter().any(|(_, c)| (*c - 5.0).abs() < f64::EPSILON));
        assert!(compilation.cost_terms.iter().any(|(_, c)| (*c - 3.0).abs() < f64::EPSILON));

        // task 0 应有 y[0] + x[0] 两个项
        assert_eq!(compilation.task_compilation_terms[0].len(), 2);
        // task 2 应有 y[2] + x[1] 两个项
        assert_eq!(compilation.task_compilation_terms[2].len(), 2);

        // executor 0 应有 z[0] + x[0] 两个项
        assert_eq!(compilation.executor_compilation_terms[0].len(), 2);
        // executor 1 应有 z[1] + x[1] 两个项
        assert_eq!(compilation.executor_compilation_terms[1].len(), 2);

        // 中间表达式应已重建
        assert!(compilation.base.bunch_cost_symbol.is_some());
        assert_eq!(compilation.base.task_compilation_symbols.len(), 3);
        assert_eq!(compilation.base.executor_compilation_symbols.len(), 2);
    }

    #[test]
    fn test_iterative_bunch_compilation_remove_columns() {
        let mut model = MetaModel::<f64>::new("test_iterative_bunch_remove");

        let mut compilation = IterativeBunchCompilation::new(
            2,
            vec!["exec_1".to_string()],
            false,
        );
        compilation.register(&mut model).unwrap();

        let bunches = vec![
            BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0],
                cost: 2.0,
                iteration: 0,
                slot_index: None,
            },
            BunchEntry {
                index: 1,
                executor_id: "exec_1".into(),
                task_indices: vec![1],
                cost: 3.0,
                iteration: 0,
                slot_index: None,
            },
        ];

        compilation.add_columns(0, bunches, &mut model).unwrap();
        assert_eq!(compilation.cost_terms.len(), 2);

        // 移除第 1 个束
        compilation.remove_columns(&[1]);
        assert_eq!(compilation.cost_terms.len(), 1);
        assert_eq!(compilation.task_compilation_terms[1].len(), 1); // 只剩 y[1]
        assert_eq!(compilation.active_bunch_count(), 1);
    }

    #[test]
    fn test_iterative_bunch_compilation_globally_fix() {
        let mut model = MetaModel::<f64>::new("test_iterative_bunch_global_fix");
        let mut compilation = IterativeBunchCompilation::new(
            2,
            vec!["exec_1".to_string()],
            false,
        );
        compilation.register(&mut model).unwrap();
        compilation
            .add_columns(
                0,
                vec![BunchEntry {
                    index: 0,
                    executor_id: "exec_1".into(),
                    task_indices: vec![0],
                    cost: 2.0,
                    iteration: 0,
                    slot_index: None,
                }],
                &mut model,
            )
            .unwrap();

        let fixed: HashSet<usize> = [0, 2].into_iter().collect();
        compilation.globally_fix(&fixed, &mut model).unwrap();
        assert!(compilation.fixed_bunches.contains(&0));
        assert!(compilation.fixed_bunches.contains(&2));
        assert!(!compilation.fixed_bunches.contains(&1));
        let x_idx = *compilation.bunch_x_map.get(&0).unwrap();
        assert_eq!(
            model.variable_range_by_index(x_idx),
            Some(VariableRange::bounded(1.0, 1.0))
        );
    }

    #[test]
    fn test_iterative_bunch_compilation_extract_solution() {
        let mut model = MetaModel::<f64>::new("test_iterative_bunch_extract");

        let mut compilation = IterativeBunchCompilation::new(
            3,
            vec!["exec_1".to_string(), "exec_2".to_string()],
            true,
        );
        compilation.register(&mut model).unwrap();

        let bunches = vec![
            BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0, 1],
                cost: 5.0,
                iteration: 0,
                slot_index: None,
            },
            BunchEntry {
                index: 1,
                executor_id: "exec_2".into(),
                task_indices: vec![2],
                cost: 3.0,
                iteration: 0,
                slot_index: None,
            },
        ];

        compilation.add_columns(0, bunches, &mut model).unwrap();

        // 构造解向量：使用足够大的向量
        // 确定 x 变量的最大索引来分配解向量
        let max_idx = compilation.bunch_x_map.values().copied()
            .chain(compilation.base.y_indices.iter().copied())
            .chain(compilation.base.z_indices.iter().copied())
            .max()
            .unwrap_or(0);
        let mut solution = vec![0.0; max_idx + 1];

        // 设置 y 变量为 0（不取消）
        for &y_idx in &compilation.base.y_indices {
            solution[y_idx] = 0.0;
        }

        // 设置 x[0] = 1（选中束 0），x[1] = 0（不选束 1）
        if let Some(&x0_idx) = compilation.bunch_x_map.get(&0) {
            solution[x0_idx] = 1.0;
        }
        if let Some(&x1_idx) = compilation.bunch_x_map.get(&1) {
            solution[x1_idx] = 0.0;
        }

        let result = compilation.extract_solution(&solution);
        assert!(result.selected_bunches.contains(&0));
        assert!(!result.selected_bunches.contains(&1));
        // 任务 0, 1 被束 0 分配，任务 2 未被分配 → 取消
        assert!(result.canceled_tasks.contains(&2));
    }

    #[test]
    fn test_iterative_bunch_compilation_dedup() {
        let mut model = MetaModel::<f64>::new("test_iterative_bunch_dedup");

        let mut compilation = IterativeBunchCompilation::new(
            2,
            vec!["exec_1".to_string()],
            false,
        );
        compilation.register(&mut model).unwrap();

        // 第一批列
        let bunches_1 = vec![
            BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0, 1],
                cost: 5.0,
                iteration: 0,
                slot_index: None,
            },
        ];
        compilation.add_columns(0, bunches_1, &mut model).unwrap();

        // 重复列（应被去重）
        let bunches_2 = vec![
            BunchEntry {
                index: 1,
                executor_id: "exec_1".into(),
                task_indices: vec![0, 1], // 同执行器、同任务集合 → 去重
                cost: 4.0,
                iteration: 1,
                slot_index: None,
            },
            BunchEntry {
                index: 2,
                executor_id: "exec_1".into(),
                task_indices: vec![0], // 不同任务集合 → 不去重
                cost: 2.0,
                iteration: 1,
                slot_index: None,
            },
        ];
        let added = compilation.add_columns(1, bunches_2, &mut model).unwrap();

        // 只有束 2 被添加（束 1 被去重）
        assert_eq!(added.len(), 1);
    }
}
