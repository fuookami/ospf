//! 迭代束编译上下文 / Iterative bunch compilation context
//!
//! 定义迭代列生成中束编译上下文的接口和影子价格提取。
//! Defines interface and shadow price extraction for bunch compilation context
//! in iterative column generation.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::error::Result;
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::model::pipeline::{CGPipeline, Pipeline};
use ospf_rust_framework::model::shadow_price::{
    BasicShadowPriceMap, ShadowPrice, ShadowPriceKey, ShadowPriceMap,
};
use ospf_rust_framework::solver::column_generation_solver::LinearDualSolution;

use crate::GanttResult;
use crate::domain::bunch_compilation::iterative::IterativeBunchCompilation;
use crate::domain::bunch_compilation::model::{BunchEntry, BunchSolution};
use crate::domain::common::{
    ConstraintIndexMap, GanttDynamicModelLifecycle, GanttModelStateFacade,
};

// ============================================================================
// 束编译上下文 trait / Bunch Compilation Context Trait
// ============================================================================

/// 迭代束编译上下文 trait / Iterative bunch compilation context trait
///
/// 定义迭代列生成中束编译上下文的接口。
/// Defines interface for bunch compilation context in iterative column generation.
pub trait IterativeBunchCompilationContext: Send + Sync {
    /// 注册到模型 / Register to model
    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()>;

    /// 添加列 / Add columns
    fn add_columns(
        &mut self,
        iteration: usize,
        new_bunches: Vec<BunchEntry>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>>;

    /// 移除列 / Remove columns
    fn remove_columns(&mut self, bunch_indices: &[usize]);

    /// 隐藏束并同步模型 / Hide bunches and sync model
    fn hide_bunches_in_model(
        &mut self,
        _bunch_indices: &[usize],
        _model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        Ok(())
    }

    /// 全局固定束并同步模型 / Globally fix bunches and sync model
    fn globally_fix_in_model(
        &mut self,
        _bunch_indices: &HashSet<usize>,
        _model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        Ok(())
    }

    /// 局部固定束并同步模型 / Locally fix bunches and sync model
    fn locally_fix_in_model(
        &mut self,
        _iteration: usize,
        _threshold: f64,
        solution: &[f64],
        current_fixed: &HashSet<usize>,
        _model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        Ok(self
            .extract_kept(solution)
            .difference(current_fixed)
            .copied()
            .collect())
    }

    /// 恢复非移除束范围 / Restore non-removed bunch ranges
    fn restore_non_removed_ranges_in_model(&mut self, _model: &mut MetaModel<f64>) -> GanttResult<()> {
        Ok(())
    }

    /// 刷新动态范围 / Flush dynamic ranges
    fn flush(&mut self) {}

    /// 应用动态生命周期 / Apply dynamic lifecycle
    fn apply_lifecycle(&mut self, lifecycle: &mut GanttDynamicModelLifecycle) {
        let removed = lifecycle.state().removed_columns();
        if !removed.is_empty() {
            self.remove_columns(&removed);
        }
    }

    /// 提取影子价格 / Extract shadow price
    ///
    /// 从 LP 对偶解提取每个任务的影子价格。
    /// Extracts shadow price per task from LP dual solution.
    fn extract_shadow_price(
        &self,
        dual_solution: &LinearDualSolution,
        constraint_name_to_index: &HashMap<String, usize>,
    ) -> HashMap<usize, f64>;

    /// 构建核心约束索引映射 / Build core constraint index map
    fn build_constraint_index_map(
        &self,
        constraint_name_to_index: &HashMap<String, usize>,
    ) -> ConstraintIndexMap {
        let mut map = ConstraintIndexMap::new();
        map.register_task_compilation_constraints(
            self.task_count_for_shadow_price(),
            constraint_name_to_index,
        );
        map
    }

    /// 提取影子价格（稳定约束映射优先）/ Extract shadow price with stable constraint map first
    fn extract_shadow_price_with_index_map(
        &self,
        dual_solution: &LinearDualSolution,
        constraint_index_map: &ConstraintIndexMap,
        constraint_name_to_index: &HashMap<String, usize>,
    ) -> HashMap<usize, f64> {
        let prices = constraint_index_map.extract_task_shadow_prices(
            self.task_count_for_shadow_price(),
            &dual_solution.constraints,
        );
        if prices.is_empty() {
            self.extract_shadow_price(dual_solution, constraint_name_to_index)
        } else {
            prices
        }
    }

    /// 影子价格任务数量 / Task count for shadow price extraction
    fn task_count_for_shadow_price(&self) -> usize;

    /// 提取已固定的束 / Extract fixed bunches
    fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize>;

    /// 提取保留的束 / Extract kept bunches
    fn extract_kept(&self, solution: &[f64]) -> HashSet<usize>;

    /// 提取隐藏执行器 / Extract hidden executors
    fn extract_hidden_executors(&self, _solution: &[f64]) -> HashSet<String> {
        HashSet::new()
    }

    /// 分析解 / Analyze solution
    fn analyze_solution(&self, solution: &[f64]) -> BunchSolution;

    /// 基于模型状态分析解 / Analyze solution with model state
    fn analyze_solution_with_state(
        &self,
        solution: &[f64],
        model_state: &GanttModelStateFacade,
    ) -> BunchSolution {
        let mut solution = self.analyze_solution(solution);
        solution
            .selected_bunches
            .retain(|bunch_index| model_state.is_selectable(*bunch_index));
        for fixed_column in model_state.fixed_columns() {
            if model_state.is_selectable(fixed_column)
                && !solution.selected_bunches.contains(&fixed_column)
            {
                solution.selected_bunches.push(fixed_column);
            }
        }
        solution.selected_bunches.sort_unstable();
        solution.selected_bunches.dedup();
        solution
    }

    /// 基于动态生命周期分析解 / Analyze solution with dynamic lifecycle
    fn analyze_solution_with_lifecycle(
        &self,
        solution: &[f64],
        lifecycle: &GanttDynamicModelLifecycle,
    ) -> BunchSolution {
        self.analyze_solution_with_state(solution, lifecycle.state())
    }

    /// 活跃列数量 / Active column count
    fn column_count(&self) -> usize;

    /// 获取束条目 / Get bunch entry
    fn get_bunch_entry(&self, bunch_index: usize) -> Option<BunchEntry>;
}

/// 基础束编译上下文实现 / Basic bunch compilation context implementation
///
/// 使用 IterativeBunchCompilation 实现的默认上下文。
/// Default context implementation using IterativeBunchCompilation.
#[derive(Clone, Debug)]
pub struct BasicBunchCompilationContext {
    /// 迭代束编译 / Iterative bunch compilation
    pub compilation: IterativeBunchCompilation,
}

impl BasicBunchCompilationContext {
    /// 创建基础束编译上下文 / Create basic bunch compilation context
    pub fn new(n_tasks: usize, executor_ids: Vec<String>, with_executor_leisure: bool) -> Self {
        Self {
            compilation: IterativeBunchCompilation::new(
                n_tasks,
                executor_ids,
                with_executor_leisure,
            ),
        }
    }
}

impl IterativeBunchCompilationContext for BasicBunchCompilationContext {
    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.compilation.register(model)
    }

    fn add_columns(
        &mut self,
        iteration: usize,
        new_bunches: Vec<BunchEntry>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        self.compilation.add_columns(iteration, new_bunches, model)
    }

    fn remove_columns(&mut self, bunch_indices: &[usize]) {
        self.compilation.remove_columns(bunch_indices);
    }

    fn hide_bunches_in_model(
        &mut self,
        bunch_indices: &[usize],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.compilation.hide_bunches(bunch_indices.iter().copied(), model)
    }

    fn globally_fix_in_model(
        &mut self,
        bunch_indices: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.compilation.globally_fix(bunch_indices, model)
    }

    fn locally_fix_in_model(
        &mut self,
        iteration: usize,
        threshold: f64,
        solution: &[f64],
        current_fixed: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        self.compilation
            .locally_fix(iteration, threshold, solution, current_fixed, model)
    }

    fn restore_non_removed_ranges_in_model(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.compilation.restore_non_removed_ranges(model)
    }

    fn flush(&mut self) {
        self.compilation.flush();
    }

    fn apply_lifecycle(&mut self, lifecycle: &mut GanttDynamicModelLifecycle) {
        let removed = lifecycle.state().removed_columns();
        if !removed.is_empty() {
            self.compilation.remove_columns(&removed);
        }
    }

    fn extract_shadow_price(
        &self,
        dual_solution: &LinearDualSolution,
        constraint_name_to_index: &HashMap<String, usize>,
    ) -> HashMap<usize, f64> {
        let mut shadow_prices = HashMap::new();

        // 从 taskCompilation 约束的对偶值提取影子价格
        // Extract shadow prices from dual values of taskCompilation constraints
        for ti in 0..self.compilation.base.n_tasks {
            let constraint_name = format!("task_compilation_{}", ti);
            if let Some(&idx) = constraint_name_to_index.get(&constraint_name) {
                if idx < dual_solution.constraints.len() {
                    let price = dual_solution.constraints[idx];
                    if price.abs() > f64::EPSILON {
                        shadow_prices.insert(ti, price);
                    }
                }
            }
        }

        shadow_prices
    }

    fn task_count_for_shadow_price(&self) -> usize {
        self.compilation.base.n_tasks
    }

    fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize> {
        self.compilation.extract_fixed(solution)
    }

    fn extract_kept(&self, solution: &[f64]) -> HashSet<usize> {
        self.compilation.extract_kept(solution)
    }

    fn extract_hidden_executors(&self, solution: &[f64]) -> HashSet<String> {
        self.compilation.extract_hidden_executors(solution)
    }

    fn analyze_solution(&self, solution: &[f64]) -> BunchSolution {
        self.compilation.extract_solution(solution)
    }

    fn column_count(&self) -> usize {
        self.compilation.active_bunch_count()
    }

    fn get_bunch_entry(&self, bunch_index: usize) -> Option<BunchEntry> {
        self.compilation.get_bunch_entry(bunch_index)
    }
}

// ============================================================================
// 束影子价格 Pipeline / Bunch Shadow Price Pipeline
// ============================================================================

/// 任务影子价格键 / Task shadow price key
///
/// 用于标识每个任务的影子价格。
/// Used to identify shadow price per task.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TaskShadowPriceKey {
    /// 任务索引 / Task index
    pub task_index: usize,
}

/// 束影子价格提取 Pipeline / Bunch shadow price extraction pipeline
///
/// 实现 CGPipeline 以支持影子价格提取。
/// Implements CGPipeline for shadow price extraction.
pub struct BunchShadowPricePipeline {
    /// 任务数量 / Task count
    pub n_tasks: usize,
}

impl BunchShadowPricePipeline {
    /// 创建束影子价格 Pipeline / Create bunch shadow price pipeline
    pub fn new(n_tasks: usize) -> Self {
        Self { n_tasks }
    }
}

impl Pipeline<MetaModel<f64>> for BunchShadowPricePipeline {
    fn name(&self) -> &str {
        "bunch_shadow_price"
    }

    fn constraint_group(
        &self,
    ) -> Option<&ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup> {
        None
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

impl CGPipeline<TaskShadowPriceKey, MetaModel<f64>, BasicShadowPriceMap<TaskShadowPriceKey>>
    for BunchShadowPricePipeline
{
    type Extractor = fn(&BasicShadowPriceMap<TaskShadowPriceKey>, &TaskShadowPriceKey) -> f64;

    fn extractor(&self) -> Option<Self::Extractor> {
        Some(
            |map: &BasicShadowPriceMap<TaskShadowPriceKey>, key: &TaskShadowPriceKey| {
                let sp_key = ShadowPriceKey::named::<Self>(format!("task_{}", key.task_index));
                map.get(&sp_key).map(|sp| sp.price).unwrap_or(0.0)
            },
        )
    }

    fn refresh(
        &self,
        shadow_price_map: &mut BasicShadowPriceMap<TaskShadowPriceKey>,
        _model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> Result<()> {
        for (ti, &price) in shadow_prices.iter().enumerate() {
            if ti < self.n_tasks && price.abs() > f64::EPSILON {
                let key = ShadowPriceKey::named::<Self>(format!("task_{}", ti));
                shadow_price_map.put(ShadowPrice::new(key, price));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task_compilation::context::IterativeTaskCompilationContext;

    #[test]
    fn test_basic_bunch_compilation_context_register() {
        let mut model = MetaModel::<f64>::new("test_bunch_context_register");
        let mut ctx = BasicBunchCompilationContext::new(2, vec!["exec_1".to_string()], false);
        ctx.register(&mut model).unwrap();
        assert_eq!(ctx.column_count(), 0); // 没有束
    }

    #[test]
    fn test_basic_bunch_compilation_context_add_columns() {
        let mut model = MetaModel::<f64>::new("test_bunch_context_add");
        let mut ctx = BasicBunchCompilationContext::new(2, vec!["exec_1".to_string()], false);
        ctx.register(&mut model).unwrap();

        let bunches = vec![BunchEntry {
            index: 0,
            executor_id: "exec_1".to_string(),
            task_indices: vec![0, 1],
            cost: 5.0,
            iteration: 0,
        }];

        let added = ctx.add_columns(0, bunches, &mut model).unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(ctx.column_count(), 1);
    }

    #[test]
    fn test_bunch_shadow_price_pipeline() {
        let pipeline = BunchShadowPricePipeline::new(3);
        let mut map = BasicShadowPriceMap::<TaskShadowPriceKey>::new();

        // 模拟影子价格
        let shadow_prices = vec![1.0, 2.0, 0.0];
        let model = MetaModel::<f64>::new("test_shadow_price");

        pipeline.refresh(&mut map, &model, &shadow_prices).unwrap();

        // 验证提取器
        let extractor = pipeline.extractor().unwrap();
        let key = TaskShadowPriceKey { task_index: 0 };
        let price = extractor(&map, &key);
        assert!((price - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_basic_bunch_compilation_context_builds_constraint_index_map() {
        let ctx = BasicBunchCompilationContext::new(3, vec!["exec_1".to_string()], false);
        let mut name_to_idx = HashMap::new();
        name_to_idx.insert("task_compilation_0".to_string(), 1);
        name_to_idx.insert("task_compilation_1".to_string(), 3);

        let map = ctx.build_constraint_index_map(&name_to_idx);
        let dual = LinearDualSolution::new(vec![0.0, 2.0, 0.0, 4.0], vec![]);
        let prices = ctx.extract_shadow_price_with_index_map(&dual, &map, &HashMap::new());

        assert_eq!(map.len(), 2);
        assert_eq!(prices.get(&0), Some(&2.0));
        assert_eq!(prices.get(&1), Some(&4.0));
        assert!(!prices.contains_key(&2));
    }

    #[test]
    fn test_task_and_bunch_contexts_build_consistent_shadow_price_maps() {
        let task_ctx = crate::domain::task_compilation::context::BasicTaskCompilationContext::new(
            2, 1, true, false,
        );
        let bunch_ctx = BasicBunchCompilationContext::new(2, vec!["exec_1".to_string()], false);
        let mut name_to_idx = HashMap::new();
        name_to_idx.insert("task_compilation_0".to_string(), 2);
        name_to_idx.insert("task_compilation_1".to_string(), 5);

        let task_map = task_ctx.build_constraint_index_map(&name_to_idx);
        let bunch_map = bunch_ctx.build_constraint_index_map(&name_to_idx);
        let dual = LinearDualSolution::new(vec![0.0, 0.0, 1.0, 0.0, 0.0, 2.0], vec![]);

        assert_eq!(
            task_ctx.extract_shadow_price_with_index_map(&dual, &task_map, &HashMap::new(),),
            bunch_ctx.extract_shadow_price_with_index_map(&dual, &bunch_map, &HashMap::new(),),
        );
    }

    #[test]
    fn test_basic_bunch_compilation_context_filters_solution_by_model_state() {
        let mut model = MetaModel::<f64>::new("test_bunch_context_state_solution");
        let mut ctx = BasicBunchCompilationContext::new(2, vec!["exec_1".to_string()], false);
        ctx.register(&mut model).unwrap();
        ctx.add_columns(
            0,
            vec![
                BunchEntry {
                    index: 0,
                    executor_id: "exec_1".to_string(),
                    task_indices: vec![0],
                    cost: 1.0,
                    iteration: 0,
                },
                BunchEntry {
                    index: 1,
                    executor_id: "exec_1".to_string(),
                    task_indices: vec![1],
                    cost: 1.0,
                    iteration: 0,
                },
            ],
            &mut model,
        )
        .unwrap();

        let max_index = ctx.compilation.bunch_x_map.values().copied().max().unwrap();
        let mut solver_solution = vec![0.0; max_index + 1];
        for x_index in ctx.compilation.bunch_x_map.values() {
            solver_solution[*x_index] = 1.0;
        }

        let mut model_state = GanttModelStateFacade::new();
        model_state.hide_columns([1]);
        let solution = ctx.analyze_solution_with_state(&solver_solution, &model_state);

        assert_eq!(solution.selected_bunches, vec![0]);
    }

    #[test]
    fn test_basic_bunch_compilation_context_forces_fixed_columns_in_solution() {
        let mut model = MetaModel::<f64>::new("test_bunch_context_fixed_solution");
        let mut ctx = BasicBunchCompilationContext::new(2, vec!["exec_1".to_string()], false);
        ctx.register(&mut model).unwrap();
        ctx.add_columns(
            0,
            vec![
                BunchEntry {
                    index: 0,
                    executor_id: "exec_1".to_string(),
                    task_indices: vec![0],
                    cost: 1.0,
                    iteration: 0,
                },
                BunchEntry {
                    index: 1,
                    executor_id: "exec_1".to_string(),
                    task_indices: vec![1],
                    cost: 1.0,
                    iteration: 0,
                },
            ],
            &mut model,
        )
        .unwrap();

        let max_index = ctx.compilation.bunch_x_map.values().copied().max().unwrap();
        let solver_solution = vec![0.0; max_index + 1];

        let mut model_state = GanttModelStateFacade::new();
        model_state.fix_columns([1]);
        model_state.hide_columns([0]);
        let solution = ctx.analyze_solution_with_state(&solver_solution, &model_state);

        assert_eq!(solution.selected_bunches, vec![1]);
    }

    #[test]
    fn test_basic_bunch_compilation_context_removed_overrides_fixed_columns() {
        let mut model = MetaModel::<f64>::new("test_bunch_context_removed_solution");
        let mut ctx = BasicBunchCompilationContext::new(1, vec!["exec_1".to_string()], false);
        ctx.register(&mut model).unwrap();
        ctx.add_columns(
            0,
            vec![BunchEntry {
                index: 0,
                executor_id: "exec_1".to_string(),
                task_indices: vec![0],
                cost: 1.0,
                iteration: 0,
            }],
            &mut model,
        )
        .unwrap();

        let x_index = *ctx.compilation.bunch_x_map.get(&0).unwrap();
        let mut solver_solution = vec![0.0; x_index + 1];
        solver_solution[x_index] = 1.0;

        let mut model_state = GanttModelStateFacade::new();
        model_state.fix_columns([0]);
        model_state.remove_columns([0]);
        let solution = ctx.analyze_solution_with_state(&solver_solution, &model_state);

        assert!(solution.selected_bunches.is_empty());
    }

    #[test]
    fn test_basic_bunch_compilation_context_syncs_bunch_ranges_to_model() {
        let mut model = MetaModel::<f64>::new("test_bunch_context_range_sync");
        let mut ctx = BasicBunchCompilationContext::new(2, vec!["exec_1".to_string()], false);
        ctx.register(&mut model).unwrap();
        ctx.add_columns(
            0,
            vec![
                BunchEntry {
                    index: 0,
                    executor_id: "exec_1".to_string(),
                    task_indices: vec![0],
                    cost: 1.0,
                    iteration: 0,
                },
                BunchEntry {
                    index: 1,
                    executor_id: "exec_1".to_string(),
                    task_indices: vec![1],
                    cost: 1.0,
                    iteration: 0,
                },
            ],
            &mut model,
        )
        .unwrap();
        let x0 = *ctx.compilation.bunch_x_map.get(&0).unwrap();
        let x1 = *ctx.compilation.bunch_x_map.get(&1).unwrap();

        ctx.hide_bunches_in_model(&[0], &mut model).unwrap();
        assert_eq!(
            model.variable_range_by_index(x0),
            Some(ospf_rust_core::variable::VariableRange::bounded(0.0, 0.0))
        );

        ctx.globally_fix_in_model(&HashSet::from([1]), &mut model)
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(x1),
            Some(ospf_rust_core::variable::VariableRange::bounded(1.0, 1.0))
        );

        ctx.restore_non_removed_ranges_in_model(&mut model).unwrap();
        assert_eq!(
            model.variable_range_by_index(x0),
            Some(ospf_rust_core::variable::VariableRange::bounded(0.0, 1.0))
        );
        assert_eq!(
            model.variable_range_by_index(x1),
            Some(ospf_rust_core::variable::VariableRange::bounded(0.0, 1.0))
        );
    }
}
