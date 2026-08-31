//! 迭代任务编译上下文 / Iterative task compilation context
//!
//! 定义迭代列生成中任务编译上下文的接口和影子价格提取。
//! Defines interface and shadow price extraction for task compilation context
//! in iterative column generation.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::column_generation_solver::LinearDualSolution;

use crate::GanttResult;
use crate::domain::common::{ConstraintIndexMap, GanttDynamicModelLifecycle};
use crate::domain::task::Cost;
use crate::domain::task_compilation::iterative::{AddedTaskColumn, IterativeTaskCompilation};

/// 迭代任务编译上下文 trait / Iterative task compilation context trait
///
/// 定义迭代列生成中任务编译上下文的接口。
/// Defines interface for task compilation context in iterative column generation.
pub trait IterativeTaskCompilationContext: Send + Sync {
    /// 注册到模型 / Register to model
    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()>;

    /// 添加列 / Add columns
    fn add_columns(
        &mut self,
        iteration: usize,
        new_pairs: Vec<(usize, usize, Cost<f64>)>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<AddedTaskColumn>>;

    /// 移除列 / Remove columns
    fn remove_columns(&mut self, column_indices: &[usize]);

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

    /// 提取已固定的列 / Extract fixed columns
    fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize>;

    /// 提取保留的列 / Extract kept columns
    fn extract_kept(&self, solution: &[f64]) -> HashSet<usize>;

    /// 活跃列数量 / Active column count
    fn column_count(&self) -> usize;
}

/// 基础任务编译上下文实现 / Basic task compilation context implementation
pub struct BasicTaskCompilationContext {
    /// 迭代任务编译 / Iterative task compilation
    pub compilation: IterativeTaskCompilation,
}

impl BasicTaskCompilationContext {
    /// 创建基础任务编译上下文 / Create basic task compilation context
    pub fn new(
        n_tasks: usize,
        n_executors: usize,
        task_cancel_enabled: bool,
        with_executor_leisure: bool,
    ) -> Self {
        Self {
            compilation: IterativeTaskCompilation::new(
                n_tasks,
                n_executors,
                task_cancel_enabled,
                with_executor_leisure,
            ),
        }
    }
}

impl IterativeTaskCompilationContext for BasicTaskCompilationContext {
    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.compilation.register(model)
    }

    fn add_columns(
        &mut self,
        iteration: usize,
        new_pairs: Vec<(usize, usize, Cost<f64>)>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<AddedTaskColumn>> {
        self.compilation.add_columns(iteration, new_pairs, model)
    }

    fn remove_columns(&mut self, column_indices: &[usize]) {
        self.compilation.remove_columns(column_indices);
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
        for ti in 0..self.compilation.n_tasks {
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
        self.compilation.n_tasks
    }

    fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize> {
        self.compilation.extract_fixed(solution)
    }

    fn extract_kept(&self, solution: &[f64]) -> HashSet<usize> {
        self.compilation.extract_kept(solution)
    }

    fn column_count(&self) -> usize {
        self.compilation.active_column_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_task_compilation_context_register() {
        let mut model = MetaModel::<f64>::new("test_task_context_register");
        let mut ctx = BasicTaskCompilationContext::new(2, 1, true, false);
        ctx.register(&mut model).unwrap();
        assert_eq!(ctx.compilation.y_indices.len(), 2);
        assert_eq!(ctx.column_count(), 0); // 没有列
    }

    #[test]
    fn test_basic_task_compilation_context_add_columns() {
        let mut model = MetaModel::<f64>::new("test_task_context_add");
        let mut ctx = BasicTaskCompilationContext::new(2, 1, false, false);
        ctx.register(&mut model).unwrap();

        let new_pairs = vec![(0, 0, Cost::empty()), (1, 0, Cost::empty())];
        let added = ctx.add_columns(0, new_pairs, &mut model).unwrap();
        assert_eq!(added.len(), 2);
        assert_eq!(ctx.column_count(), 2);
    }

    #[test]
    fn test_basic_task_compilation_context_shadow_price() {
        let mut model = MetaModel::<f64>::new("test_task_context_shadow");
        let mut ctx = BasicTaskCompilationContext::new(2, 1, true, false);
        ctx.register(&mut model).unwrap();

        // 构造假的约束名到索引映射
        let mut name_to_idx = HashMap::new();
        name_to_idx.insert("task_compilation_0".to_string(), 0);
        name_to_idx.insert("task_compilation_1".to_string(), 1);

        // 构造假的对偶解
        let dual = LinearDualSolution::new(
            vec![1.5, 2.5], // constraint duals
            vec![],         // variable duals
        );

        let prices = ctx.extract_shadow_price(&dual, &name_to_idx);
        assert!((prices[&0] - 1.5).abs() < f64::EPSILON);
        assert!((prices[&1] - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_basic_task_compilation_context_builds_constraint_index_map() {
        let ctx = BasicTaskCompilationContext::new(3, 1, true, false);
        let mut name_to_idx = HashMap::new();
        name_to_idx.insert("task_compilation_0".to_string(), 2);
        name_to_idx.insert("task_compilation_2".to_string(), 4);

        let map = ctx.build_constraint_index_map(&name_to_idx);
        let dual = LinearDualSolution::new(vec![0.0, 0.0, 1.25, 0.0, 3.5], vec![]);
        let prices = ctx.extract_shadow_price_with_index_map(&dual, &map, &HashMap::new());

        assert_eq!(map.len(), 2);
        assert_eq!(prices.get(&0), Some(&1.25));
        assert_eq!(prices.get(&2), Some(&3.5));
        assert!(!prices.contains_key(&1));
    }
}
