//! 动态模型生命周期 / Dynamic model lifecycle
//!
//! 为列生成、分支定价和类似迭代建模流程提供可复用的列状态、解回填、
//! warm start 和变量范围刷新能力。
//! Provides reusable column state, solution injection, warm-start, and variable-range
//! refresh capabilities for column generation, branch-and-price, and similar
//! iterative modeling flows.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::error::Result;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::VariableRange;

/// 列状态 / Column state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnState {
    /// 活跃列 / Active column
    Active,
    /// 已隐藏列 / Hidden column
    Hidden,
    /// 已固定列 / Fixed column
    Fixed,
    /// 已移除列 / Removed column
    Removed,
}

/// 动态模型状态 / Dynamic model state
#[derive(Debug, Clone, Default)]
pub struct DynamicModelState {
    hidden_columns: HashSet<usize>,
    fixed_columns: HashSet<usize>,
    removed_columns: HashSet<usize>,
    warm_start_columns: HashSet<usize>,
}

impl DynamicModelState {
    /// 创建空状态 / Create empty state
    pub fn new() -> Self {
        Self::default()
    }

    /// 隐藏列 / Hide columns
    pub fn hide_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        for column in columns {
            if !self.removed_columns.contains(&column) && !self.fixed_columns.contains(&column) {
                self.hidden_columns.insert(column);
            }
        }
    }

    /// 固定列 / Fix columns
    pub fn fix_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        for column in columns {
            if !self.removed_columns.contains(&column) {
                self.hidden_columns.remove(&column);
                self.fixed_columns.insert(column);
            }
        }
    }

    /// 固定二进制列取值 / Fix binary column value
    ///
    /// `1` 表示列必须被选择，`0` 表示列在当前节点不可选。
    /// `1` means the column must be selected; `0` means the column is not selectable
    /// in the current node.
    pub fn fix_binary_column(&mut self, column: usize, value: i8) {
        if value == 0 {
            self.hide_columns([column]);
        } else {
            self.fix_columns([column]);
        }
    }

    /// 移除列 / Remove columns
    pub fn remove_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        for column in columns {
            self.hidden_columns.remove(&column);
            self.fixed_columns.remove(&column);
            self.warm_start_columns.remove(&column);
            self.removed_columns.insert(column);
        }
    }

    /// 设置 warm start 列 / Set warm-start columns
    pub fn set_warm_start(&mut self, columns: impl IntoIterator<Item = usize>) {
        self.warm_start_columns.clear();
        for column in columns {
            if self.is_selectable(column) {
                self.warm_start_columns.insert(column);
            }
        }
    }

    /// 刷新临时状态 / Flush transient state
    pub fn flush_transient(&mut self) {
        self.hidden_columns.clear();
        self.fixed_columns.clear();
        self.warm_start_columns.clear();
    }

    /// 查询列状态 / Get column state
    pub fn column_state(&self, column: usize) -> ColumnState {
        if self.removed_columns.contains(&column) {
            ColumnState::Removed
        } else if self.fixed_columns.contains(&column) {
            ColumnState::Fixed
        } else if self.hidden_columns.contains(&column) {
            ColumnState::Hidden
        } else {
            ColumnState::Active
        }
    }

    /// 是否可被解提取选中 / Whether selectable by solution extraction
    pub fn is_selectable(&self, column: usize) -> bool {
        !matches!(
            self.column_state(column),
            ColumnState::Hidden | ColumnState::Removed
        )
    }

    /// 过滤可选列 / Filter selectable columns
    pub fn selectable_columns(&self, columns: impl IntoIterator<Item = usize>) -> Vec<usize> {
        columns
            .into_iter()
            .filter(|column| self.is_selectable(*column))
            .collect()
    }

    /// 是否已隐藏 / Whether hidden
    pub fn is_hidden(&self, column: usize) -> bool {
        self.hidden_columns.contains(&column)
    }

    /// 是否已固定 / Whether fixed
    pub fn is_fixed(&self, column: usize) -> bool {
        self.fixed_columns.contains(&column)
    }

    /// 是否已移除 / Whether removed
    pub fn is_removed(&self, column: usize) -> bool {
        self.removed_columns.contains(&column)
    }

    /// 已隐藏列 / Hidden columns
    pub fn hidden_columns(&self) -> Vec<usize> {
        sorted_columns(&self.hidden_columns)
    }

    /// 已固定列 / Fixed columns
    pub fn fixed_columns(&self) -> Vec<usize> {
        sorted_columns(&self.fixed_columns)
    }

    /// 已移除列 / Removed columns
    pub fn removed_columns(&self) -> Vec<usize> {
        sorted_columns(&self.removed_columns)
    }

    /// warm-start 列 / Warm-start columns
    pub fn warm_start_columns(&self) -> Vec<usize> {
        sorted_columns(&self.warm_start_columns)
    }
}

/// 动态列范围 / Dynamic column range
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRange {
    /// 下界 / Lower bound
    pub lower: f64,
    /// 上界 / Upper bound
    pub upper: f64,
}

impl ColumnRange {
    /// 创建列范围 / Create column range
    pub fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    /// 二进制默认范围 / Default binary range
    pub fn binary() -> Self {
        Self::new(0.0, 1.0)
    }

    /// 固定到 0 / Fixed to zero
    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    /// 固定到 1 / Fixed to one
    pub fn one() -> Self {
        Self::new(1.0, 1.0)
    }

    /// 转为 core 变量范围 / Convert to core variable range
    pub fn to_variable_range(self) -> VariableRange<f64> {
        VariableRange::bounded(self.lower, self.upper)
    }
}

impl Default for ColumnRange {
    fn default() -> Self {
        Self::binary()
    }
}

/// 动态模型快照 / Dynamic model snapshot
#[derive(Debug, Clone, Default)]
pub struct DynamicModelSnapshot {
    /// 列状态 / Column state
    pub state: DynamicModelState,
    /// 注入的求解器解 / Injected solver solution
    pub solution: Option<Vec<f64>>,
    /// 列范围覆盖 / Column range overrides
    pub column_ranges: HashMap<usize, ColumnRange>,
    /// 原始列范围 / Original column ranges
    pub original_column_ranges: HashMap<usize, VariableRange<f64>>,
    /// flush 次数 / Flush count
    pub flush_count: usize,
}

/// 动态模型生命周期 / Dynamic model lifecycle
///
/// 承载 Kotlin `model.setSolution()`、`model.flush()`、warm start 和列范围刷新语义，
/// 供各领域框架复用。
/// Carries Kotlin-style `model.setSolution()`, `model.flush()`, warm-start, and
/// variable-range refresh semantics for reuse by domain frameworks.
#[derive(Debug, Clone, Default)]
pub struct DynamicModelLifecycle {
    state: DynamicModelState,
    solution: Option<Vec<f64>>,
    column_ranges: HashMap<usize, ColumnRange>,
    original_column_ranges: HashMap<usize, VariableRange<f64>>,
    flush_count: usize,
}

impl DynamicModelLifecycle {
    /// 创建空生命周期 / Create empty lifecycle
    pub fn new() -> Self {
        Self::default()
    }

    /// 从列状态创建生命周期 / Create lifecycle from column state
    pub fn from_state(state: DynamicModelState) -> Self {
        Self {
            state,
            solution: None,
            column_ranges: HashMap::new(),
            original_column_ranges: HashMap::new(),
            flush_count: 0,
        }
    }

    /// 读取列状态 / Read column state
    pub fn state(&self) -> &DynamicModelState {
        &self.state
    }

    /// 可变读取列状态 / Mutably read column state
    pub fn state_mut(&mut self) -> &mut DynamicModelState {
        &mut self.state
    }

    /// 克隆列状态 / Clone column state
    pub fn column_state_facade(&self) -> DynamicModelState {
        self.state.clone()
    }

    /// 注入求解器解 / Inject solver solution
    pub fn set_solution(&mut self, solution: impl Into<Vec<f64>>) {
        self.solution = Some(solution.into());
    }

    /// 将解写入模型 / Write solution into model
    pub fn set_solution_to_model(&mut self, model: &mut MetaModel<f64>, solution: Vec<f64>) {
        model.set_solution(&solution);
        self.solution = Some(solution);
    }

    /// 将缓存解应用到模型 / Apply cached solution to model
    ///
    /// 若生命周期中存在缓存解，则按求解器顺序写回模型；没有缓存解时保持
    /// 模型当前状态不变，清除解应通过 `clear_solution_in_model` 显式完成。
    /// If a cached solution exists, writes it back to the model in solver order;
    /// without a cached solution, leaves the model unchanged. Clearing should be
    /// done explicitly through `clear_solution_in_model`.
    pub fn apply_solution_to_model(&self, model: &mut MetaModel<f64>) {
        if let Some(solution) = &self.solution {
            model.set_solution(solution);
        }
    }

    /// 清除注入解 / Clear injected solution
    pub fn clear_solution(&mut self) {
        self.solution = None;
    }

    /// 清除注入解并同步模型 / Clear injected solution and sync model
    pub fn clear_solution_in_model(&mut self, model: &mut MetaModel<f64>) {
        self.clear_solution();
        model.clear_solution();
    }

    /// 当前注入解 / Current injected solution
    pub fn solution(&self) -> Option<&[f64]> {
        self.solution.as_deref()
    }

    /// 从当前解写入 warm start / Write warm start from current solution
    pub fn set_warm_start_from_solution(&mut self, threshold: f64) {
        let Some(solution) = &self.solution else {
            self.state.set_warm_start([]);
            return;
        };
        let columns = solution
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                if *value >= threshold && self.state.is_selectable(index) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        self.state.set_warm_start(columns);
    }

    /// 写入 warm start 列 / Write warm-start columns
    pub fn set_warm_start(&mut self, columns: impl IntoIterator<Item = usize>) {
        self.state.set_warm_start(columns);
    }

    /// warm-start 列 / Warm-start columns
    pub fn warm_start_columns(&self) -> Vec<usize> {
        self.state.warm_start_columns()
    }

    /// 隐藏列 / Hide columns
    pub fn hide_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.state.hide_columns(columns.iter().copied());
        for column in columns {
            if !self.state.is_removed(column) && !self.state.is_fixed(column) {
                self.column_ranges.insert(column, ColumnRange::zero());
            }
        }
    }

    /// 隐藏列并同步模型 / Hide columns and sync model
    pub fn hide_columns_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<()> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.remember_original_ranges(model, columns.iter().copied());
        self.hide_columns(columns.iter().copied());
        self.apply_columns_to_model(model, columns)
    }

    /// 固定列 / Fix columns
    pub fn fix_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.state.fix_columns(columns.iter().copied());
        for column in columns {
            if !self.state.is_removed(column) {
                self.column_ranges.insert(column, ColumnRange::one());
            }
        }
    }

    /// 固定列并同步模型 / Fix columns and sync model
    pub fn fix_columns_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<()> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.remember_original_ranges(model, columns.iter().copied());
        self.fix_columns(columns.iter().copied());
        self.apply_columns_to_model(model, columns)
    }

    /// 固定二进制列 / Fix binary column
    pub fn fix_binary_column(&mut self, column: usize, value: i8) {
        if value == 0 {
            self.hide_columns([column]);
        } else {
            self.fix_columns([column]);
        }
    }

    /// 固定二进制列并同步模型 / Fix binary column and sync model
    pub fn fix_binary_column_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
        column: usize,
        value: i8,
    ) -> Result<()> {
        if value == 0 {
            self.hide_columns_in_model(model, [column])
        } else {
            self.fix_columns_in_model(model, [column])
        }
    }

    /// 移除列 / Remove columns
    pub fn remove_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.state.remove_columns(columns.iter().copied());
        for column in columns {
            self.column_ranges.insert(column, ColumnRange::zero());
        }
    }

    /// 移除列并同步模型 / Remove columns and sync model
    pub fn remove_columns_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<()> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.remember_original_ranges(model, columns.iter().copied());
        self.remove_columns(columns.iter().copied());
        self.apply_columns_to_model(model, columns)
    }

    /// 刷新列范围 / Refresh column ranges
    ///
    /// 清除临时隐藏、固定和 warm-start 状态，保留已移除列。
    /// Clears transient hidden, fixed, and warm-start states while keeping removed columns.
    pub fn flush(&mut self) {
        self.flush_count += 1;
        self.state.flush_transient();
        self.column_ranges
            .retain(|column, _| self.state.is_removed(*column));
    }

    /// 刷新生命周期并同步模型 / Flush lifecycle and sync model
    pub fn flush_model(&mut self, model: &mut MetaModel<f64>, force: bool) -> Result<()> {
        let restoring = self.restorable_columns();
        self.flush();
        model.flush(force);
        self.restore_columns_in_model(model, restoring)?;
        self.apply_columns_to_model(model, self.column_ranges.keys().copied())
    }

    /// 恢复指定列为默认范围 / Restore columns to default range
    pub fn restore_column_ranges(&mut self, columns: impl IntoIterator<Item = usize>) {
        let restoring = columns.into_iter().collect::<HashSet<_>>();
        self.column_ranges
            .retain(|column, _| !restoring.contains(column));
    }

    /// 恢复指定列并同步模型 / Restore columns and sync model
    pub fn restore_column_ranges_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<()> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        self.restore_column_ranges(columns.iter().copied());
        self.restore_columns_in_model(model, columns)
    }

    /// 查询列范围 / Get column range
    pub fn column_range(&self, column: usize) -> Option<ColumnRange> {
        if self.state.is_removed(column) {
            None
        } else {
            Some(
                self.column_ranges
                    .get(&column)
                    .copied()
                    .unwrap_or_else(ColumnRange::binary),
            )
        }
    }

    /// 已 flush 次数 / Flush count
    pub fn flush_count(&self) -> usize {
        self.flush_count
    }

    /// 将当前范围覆盖同步到模型 / Sync current range overrides to model
    pub fn apply_ranges_to_model(&self, model: &mut MetaModel<f64>) -> Result<()> {
        self.apply_columns_to_model(model, self.column_ranges.keys().copied())
    }

    /// 创建快照 / Create snapshot
    pub fn snapshot(&self) -> DynamicModelSnapshot {
        DynamicModelSnapshot {
            state: self.state.clone(),
            solution: self.solution.clone(),
            column_ranges: self.column_ranges.clone(),
            original_column_ranges: self.original_column_ranges.clone(),
            flush_count: self.flush_count,
        }
    }

    /// 恢复快照 / Restore snapshot
    pub fn restore(&mut self, snapshot: DynamicModelSnapshot) {
        self.state = snapshot.state;
        self.solution = snapshot.solution;
        self.column_ranges = snapshot.column_ranges;
        self.original_column_ranges = snapshot.original_column_ranges;
        self.flush_count = snapshot.flush_count;
    }

    fn restorable_columns(&self) -> Vec<usize> {
        self.state
            .hidden_columns()
            .into_iter()
            .chain(self.state.fixed_columns())
            .collect()
    }

    fn remember_original_ranges(
        &mut self,
        model: &MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) {
        for column in columns {
            if !self.original_column_ranges.contains_key(&column) {
                if let Some(range) = model.variable_range_by_index(column) {
                    self.original_column_ranges.insert(column, range);
                }
            }
        }
    }

    fn restore_columns_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<()> {
        for column in columns {
            if self.state.is_removed(column) {
                continue;
            }
            if let Some(range) = self.original_column_ranges.remove(&column) {
                model.set_variable_range_by_index(column, range)?;
            }
        }
        Ok(())
    }

    fn apply_columns_to_model(
        &self,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<()> {
        for column in columns {
            if let Some(range) = self.column_ranges.get(&column) {
                model.set_variable_range_by_index(column, range.to_variable_range())?;
            }
        }
        Ok(())
    }
}

/// 动态列上下文 / Dynamic column context
///
/// 为领域列生成模型提供统一的列索引映射和 lifecycle 同步入口。
/// 领域框架只需要维护“领域列索引 -> `MetaModel` 变量索引”的映射，
/// 隐藏、固定、移除、flush 和解提取语义由 framework lifecycle 统一承接。
/// Provides a unified column-index mapping and lifecycle synchronization entry
/// for domain column-generation models. Domain frameworks only need to maintain
/// the mapping from domain column indices to `MetaModel` variable indices, while
/// hide, fix, remove, flush, and solution-extraction semantics are handled by the
/// shared framework lifecycle.
pub trait DynamicColumnContext {
    /// 返回领域列到模型变量索引的映射 / Return domain-column to model-variable mappings
    fn column_model_indices(&self, columns: &[usize]) -> Vec<(usize, usize)>;

    /// 标记领域列已移除 / Mark domain columns as removed
    fn mark_columns_removed(&mut self, columns: &[usize]);

    /// 活跃列数量 / Active column count
    fn active_column_count(&self) -> usize;

    /// 已移除列数量 / Removed column count
    fn removed_column_count(&self) -> usize;

    /// 隐藏领域列并同步模型 / Hide domain columns and sync model
    fn hide_dynamic_columns_in_model(
        &self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        columns: &[usize],
    ) -> Result<()> {
        let model_indices = self
            .column_model_indices(columns)
            .into_iter()
            .map(|(_, model_index)| model_index)
            .collect::<Vec<_>>();
        lifecycle.hide_columns_in_model(model, model_indices)
    }

    /// 固定领域列并同步模型 / Fix domain columns and sync model
    fn fix_dynamic_columns_in_model(
        &self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        columns: &[usize],
    ) -> Result<()> {
        let model_indices = self
            .column_model_indices(columns)
            .into_iter()
            .map(|(_, model_index)| model_index)
            .collect::<Vec<_>>();
        lifecycle.fix_columns_in_model(model, model_indices)
    }

    /// 移除领域列并同步模型 / Remove domain columns and sync model
    fn remove_dynamic_columns_in_model(
        &mut self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        columns: &[usize],
    ) -> Result<()> {
        let model_indices = self
            .column_model_indices(columns)
            .into_iter()
            .map(|(_, model_index)| model_index)
            .collect::<Vec<_>>();
        self.mark_columns_removed(columns);
        lifecycle.remove_columns_in_model(model, model_indices)
    }

    /// 刷新 lifecycle 并同步模型 / Flush lifecycle and sync model
    fn flush_dynamic_model(
        &mut self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        force: bool,
    ) -> Result<()> {
        lifecycle.flush_model(model, force)
    }

    /// 提取 lifecycle 可选列解 / Extract lifecycle-selectable column values
    fn extract_selectable_column_values(
        &self,
        columns: &[usize],
        solution: &[f64],
        lifecycle: &DynamicModelLifecycle,
    ) -> HashMap<usize, f64> {
        self.column_model_indices(columns)
            .into_iter()
            .filter_map(|(column, model_index)| {
                if !lifecycle.state().is_selectable(model_index) {
                    return None;
                }
                solution.get(model_index).copied().map(|value| (column, value))
            })
            .collect()
    }
}

fn sorted_columns(columns: &HashSet<usize>) -> Vec<usize> {
    let mut columns = columns.iter().copied().collect::<Vec<_>>();
    columns.sort_unstable();
    columns
}

#[cfg(test)]
mod tests {
    use super::*;

    use ospf_rust_core::variable::{Binary, Continuous};

    #[test]
    fn state_tracks_column_lifecycle() {
        let mut state = DynamicModelState::new();

        state.hide_columns([1, 2]);
        state.fix_columns([2, 3]);
        state.remove_columns([1]);

        assert_eq!(state.column_state(0), ColumnState::Active);
        assert_eq!(state.column_state(1), ColumnState::Removed);
        assert_eq!(state.column_state(2), ColumnState::Fixed);
        assert_eq!(state.column_state(3), ColumnState::Fixed);
        assert_eq!(state.selectable_columns([0, 1, 2, 3]), vec![0, 2, 3]);
        assert_eq!(state.fixed_columns(), vec![2, 3]);
        assert_eq!(state.removed_columns(), vec![1]);
    }

    #[test]
    fn lifecycle_writes_warm_start_from_injected_solution() {
        let mut lifecycle = DynamicModelLifecycle::new();
        lifecycle.hide_columns([1]);
        lifecycle.remove_columns([3]);
        lifecycle.set_solution(vec![0.2, 1.0, 0.91, 1.0]);
        lifecycle.set_warm_start_from_solution(0.9);

        assert_eq!(lifecycle.solution().unwrap()[2], 0.91);
        assert_eq!(lifecycle.warm_start_columns(), vec![2]);
    }

    #[test]
    fn lifecycle_flush_keeps_removed_columns_and_restores_ranges() {
        let mut lifecycle = DynamicModelLifecycle::new();
        lifecycle.hide_columns([1]);
        lifecycle.fix_columns([2]);
        lifecycle.remove_columns([3]);
        lifecycle.set_warm_start([2]);

        assert_eq!(lifecycle.column_range(1), Some(ColumnRange::zero()));
        assert_eq!(lifecycle.column_range(2), Some(ColumnRange::one()));
        assert_eq!(lifecycle.column_range(3), None);

        lifecycle.flush();

        assert_eq!(lifecycle.flush_count(), 1);
        assert_eq!(lifecycle.column_range(1), Some(ColumnRange::binary()));
        assert_eq!(lifecycle.column_range(2), Some(ColumnRange::binary()));
        assert_eq!(lifecycle.column_range(3), None);
        assert!(lifecycle.warm_start_columns().is_empty());
    }

    #[test]
    fn lifecycle_syncs_ranges_with_meta_model() {
        let mut model = MetaModel::<f64>::new("dynamic");
        let x = model
            .as_basic_mut()
            .register_auto_variable::<Binary>("x")
            .unwrap();
        let y = model
            .as_basic_mut()
            .register_auto_variable_with_range::<Continuous>(
                "y",
                VariableRange::bounded(-2.0, 3.0),
            )
            .unwrap();
        let mut lifecycle = DynamicModelLifecycle::new();

        lifecycle.hide_columns_in_model(&mut model, [x]).unwrap();
        lifecycle.fix_columns_in_model(&mut model, [y]).unwrap();

        assert_eq!(
            model.variable_range_by_index(x),
            Some(VariableRange::bounded(0.0, 0.0))
        );
        assert_eq!(
            model.variable_range_by_index(y),
            Some(VariableRange::bounded(1.0, 1.0))
        );

        lifecycle.flush_model(&mut model, false).unwrap();

        assert_eq!(
            model.variable_range_by_index(x),
            Some(VariableRange::bounded(0.0, 1.0))
        );
        assert_eq!(
            model.variable_range_by_index(y),
            Some(VariableRange::bounded(-2.0, 3.0))
        );
        assert!(!model.has_solution());
    }

    #[test]
    fn lifecycle_snapshot_restores_solution_and_column_state() {
        let mut lifecycle = DynamicModelLifecycle::new();
        lifecycle.fix_columns([1]);
        lifecycle.set_solution(vec![0.0, 1.0]);
        let snapshot = lifecycle.snapshot();

        lifecycle.hide_columns([2]);
        lifecycle.clear_solution();
        lifecycle.flush();

        lifecycle.restore(snapshot);

        assert!(lifecycle.state().is_fixed(1));
        assert!(!lifecycle.state().is_hidden(2));
        assert_eq!(lifecycle.solution(), Some([0.0, 1.0].as_slice()));
        assert_eq!(lifecycle.column_range(1), Some(ColumnRange::one()));
    }

    #[test]
    fn lifecycle_applies_cached_solution_to_meta_model() {
        let mut model = MetaModel::<f64>::new("solution_apply");
        model
            .as_basic_mut()
            .register_auto_variable::<Binary>("x")
            .unwrap();
        model
            .as_basic_mut()
            .register_auto_variable::<Binary>("y")
            .unwrap();

        let mut lifecycle = DynamicModelLifecycle::new();
        lifecycle.set_solution(vec![0.0, 1.0]);
        lifecycle.apply_solution_to_model(&mut model);

        assert_eq!(
            model.solution_by_solver_order(),
            vec![Some(0.0), Some(1.0)]
        );

        lifecycle.clear_solution();
        lifecycle.apply_solution_to_model(&mut model);
        assert_eq!(
            model.solution_by_solver_order(),
            vec![Some(0.0), Some(1.0)]
        );

        lifecycle.clear_solution_in_model(&mut model);
        assert_eq!(model.solution_by_solver_order(), vec![None, None]);
    }
}
