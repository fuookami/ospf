//! 时隙级任务束编译上下文 / Slot-based bunch compilation context
//!
//! 复用基础任务束编译上下文，并增加时隙、产能预求解和按时隙束管理。
//! Reuses the basic bunch compilation context and adds slots, capacity
//! pre-solving, and slot-wise bunch management.

use std::collections::{BTreeMap, HashMap, HashSet};

use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::column_generation_solver::LinearDualSolution;

use crate::domain::bunch_compilation::context::{
    BasicBunchCompilationContext, IterativeBunchCompilationContext,
};
use crate::domain::bunch_compilation::model::{BunchEntry, SlotBasedBunchEntry};
use crate::domain::bunch_generation::{CapacityIntermediateValues, SlotConstraints};
use crate::domain::common::{
    ConstraintIndexKey, ConstraintIndexMap, ExecutorId, ExecutorIdTrait,
    GanttDynamicModelLifecycle, GanttModelStateFacade,
};
use crate::infrastructure::TimeSlot;
use crate::{GanttError, GanttResult};

/// 产能预求解调用边界 / Capacity pre-solve callable boundary
pub type CapacityPreSolveSolver<'a> =
    dyn Fn(&MetaModel<f64>) -> GanttResult<Vec<f64>> + Send + Sync + 'a;

/// 时隙级产能预求解器 / Slot-based capacity pre-solver
pub trait SlotBasedCapacityPreSolver<S, I = ExecutorId>: Send + Sync + std::fmt::Debug
where
    S: TimeSlot + Clone,
    I: ExecutorIdTrait,
{
    /// 从预求解结果提取中间值 / Extract intermediate values from pre-solve result
    fn extract_intermediate_values(
        &self,
        solution: &[f64],
        slots: &[S],
    ) -> GanttResult<CapacityIntermediateValues<S, I>>;

    /// 执行产能预求解 / Execute capacity pre-solving
    fn solve(
        &self,
        model: &MetaModel<f64>,
        slots: &[S],
        solver: &CapacityPreSolveSolver<'_>,
    ) -> GanttResult<CapacityIntermediateValues<S, I>> {
        let solution = solver(model)?;
        self.extract_intermediate_values(&solution, slots)
    }
}

/// 静态时隙级产能预求解器 / Static slot-based capacity pre-solver
#[derive(Debug, Clone)]
pub struct StaticSlotBasedCapacityPreSolver<S, I = ExecutorId>
where
    S: TimeSlot + Clone,
    I: ExecutorIdTrait,
{
    /// 预置的中间值 / Predefined intermediate values
    pub intermediate_values: CapacityIntermediateValues<S, I>,
}

impl<S, I> StaticSlotBasedCapacityPreSolver<S, I>
where
    S: TimeSlot + Clone,
    I: ExecutorIdTrait,
{
    /// 创建静态预求解器 / Create static pre-solver
    pub fn new(intermediate_values: CapacityIntermediateValues<S, I>) -> Self {
        Self {
            intermediate_values,
        }
    }
}

impl<S, I> SlotBasedCapacityPreSolver<S, I> for StaticSlotBasedCapacityPreSolver<S, I>
where
    S: TimeSlot + Clone + std::fmt::Debug,
    I: ExecutorIdTrait,
{
    fn extract_intermediate_values(
        &self,
        _solution: &[f64],
        _slots: &[S],
    ) -> GanttResult<CapacityIntermediateValues<S, I>> {
        Ok(self.intermediate_values.clone())
    }
}

/// 时隙级任务束编译上下文 / Slot-based bunch compilation context
pub trait SlotBasedBunchCompilationContext<S>: IterativeBunchCompilationContext
where
    S: TimeSlot + Clone,
{
    /// 获取时隙列表 / Get slots
    fn slots(&self) -> &[S];

    /// 获取产能中间值 / Get capacity intermediate values
    fn intermediate_values(&self) -> Option<&CapacityIntermediateValues<S, Self::ExecutorId>>;

    /// 执行产能预求解 / Execute capacity pre-solving
    fn pre_solve_capacity(
        &mut self,
        model: &mut MetaModel<f64>,
        solver: &CapacityPreSolveSolver<'_>,
    ) -> GanttResult<&CapacityIntermediateValues<S, Self::ExecutorId>>;

    /// 获取指定时隙约束 / Get constraints for specified slot
    fn slot_constraints(&self, slot_index: usize) -> Option<&SlotConstraints<Self::ExecutorId>>;

    /// 获取所有时隙约束 / Get all slot constraints
    fn all_slot_constraints(&self) -> HashMap<usize, SlotConstraints<Self::ExecutorId>>;

    /// 按时隙添加列 / Add columns by slot
    fn add_columns_by_slot(
        &mut self,
        iteration: usize,
        new_bunches: Vec<SlotBasedBunchEntry<Self::ExecutorId>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashMap<usize, Vec<usize>>>;

    /// 获取指定时隙的束 / Get bunches for specified slot
    fn bunches_in_slot(&self, slot_index: usize) -> Vec<SlotBasedBunchEntry<Self::ExecutorId>>;

    /// 获取时隙到模型变量索引的完整映射 / Get complete slot-to-model-variable mapping
    fn x_by_slot(&self) -> HashMap<usize, Vec<usize>>;

    /// 获取执行器-时隙到模型变量索引的完整映射 / Get executor-slot-to-model-variable mapping
    fn x_by_executor_slot(&self) -> HashMap<(Self::ExecutorId, usize), Vec<usize>>;
}

/// 基础时隙级任务束编译上下文 / Basic slot-based bunch compilation context
#[derive(Debug)]
pub struct BasicSlotBasedBunchCompilationContext<S, I = ExecutorId>
where
    S: TimeSlot + Clone + std::fmt::Debug,
    I: ExecutorIdTrait,
{
    /// 基础任务束编译上下文 / Basic bunch compilation context
    pub base: BasicBunchCompilationContext<I>,
    slots: Vec<S>,
    capacity_pre_solver: Box<dyn SlotBasedCapacityPreSolver<S, I>>,
    intermediate_values: Option<CapacityIntermediateValues<S, I>>,
    slot_constraints: HashMap<usize, SlotConstraints<I>>,
    slot_bunches: HashMap<usize, Vec<SlotBasedBunchEntry<I>>>,
    x_by_slot: HashMap<usize, Vec<usize>>,
    x_by_executor_slot: HashMap<(I, usize), Vec<usize>>,
}

impl<S, I> BasicSlotBasedBunchCompilationContext<S, I>
where
    S: TimeSlot + Clone + std::fmt::Debug,
    I: ExecutorIdTrait,
{
    /// 创建基础时隙级任务束编译上下文 / Create basic slot-based bunch compilation context
    pub fn new(
        base: BasicBunchCompilationContext<I>,
        slots: Vec<S>,
        capacity_pre_solver: Box<dyn SlotBasedCapacityPreSolver<S, I>>,
    ) -> Self {
        let x_by_executor_slot = base
            .compilation
            .base
            .executor_ids
            .iter()
            .cloned()
            .flat_map(|executor_id| {
                (0..slots.len())
                    .map(move |slot_index| ((executor_id.clone(), slot_index), Vec::new()))
            })
            .collect();
        Self {
            base,
            slots,
            capacity_pre_solver,
            intermediate_values: None,
            slot_constraints: HashMap::new(),
            slot_bunches: HashMap::new(),
            x_by_slot: HashMap::new(),
            x_by_executor_slot,
        }
    }

    /// 设置时隙约束 / Set slot constraints
    pub fn with_slot_constraints(
        mut self,
        slot_constraints: HashMap<usize, SlotConstraints<I>>,
    ) -> Self {
        self.slot_constraints = slot_constraints;
        self
    }

    fn refresh_slot_constraints(&mut self) {
        if let Some(values) = &self.intermediate_values {
            self.slot_constraints = values.slot_constraints.clone();
        }
    }
}

impl<S, I> IterativeBunchCompilationContext for BasicSlotBasedBunchCompilationContext<S, I>
where
    S: TimeSlot + Clone + std::fmt::Debug,
    I: ExecutorIdTrait,
{
    type ExecutorId = I;

    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.base.register(model)
    }

    fn add_columns(
        &mut self,
        iteration: usize,
        new_bunches: Vec<BunchEntry<I>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        let slot_bunches = new_bunches
            .into_iter()
            .map(|bunch| {
                let slot_index = bunch.slot_index.ok_or_else(|| GanttError::Calculation {
                    message: format!(
                        "slot index is required for bunch {} in slot-based context",
                        bunch.index
                    ),
                })?;
                Ok(SlotBasedBunchEntry { bunch, slot_index })
            })
            .collect::<GanttResult<Vec<_>>>()?;
        let added_by_slot = SlotBasedBunchCompilationContext::add_columns_by_slot(
            self,
            iteration,
            slot_bunches,
            model,
        )?;
        let mut slots = added_by_slot.into_iter().collect::<Vec<_>>();
        slots.sort_by_key(|(slot_index, _)| *slot_index);
        Ok(slots
            .into_iter()
            .flat_map(|(_, bunch_indices)| bunch_indices)
            .collect())
    }

    fn remove_columns(&mut self, bunch_indices: &[usize]) {
        let removed_model_indices = bunch_indices
            .iter()
            .filter_map(|bunch_index| self.base.compilation.bunch_x_map.get(bunch_index).copied())
            .collect::<HashSet<_>>();
        self.base.remove_columns(bunch_indices);
        let removed = bunch_indices.iter().copied().collect::<HashSet<_>>();
        for bunches in self.slot_bunches.values_mut() {
            bunches.retain(|entry| !removed.contains(&entry.bunch.index));
        }
        for model_indices in self.x_by_slot.values_mut() {
            model_indices.retain(|model_index| !removed_model_indices.contains(model_index));
        }
        for model_indices in self.x_by_executor_slot.values_mut() {
            model_indices.retain(|model_index| !removed_model_indices.contains(model_index));
        }
    }

    fn hide_bunches_in_model(
        &mut self,
        bunch_indices: &[usize],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.base.hide_bunches_in_model(bunch_indices, model)
    }

    fn globally_fix_in_model(
        &mut self,
        bunch_indices: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.base.globally_fix_in_model(bunch_indices, model)
    }

    fn locally_fix_in_model(
        &mut self,
        iteration: usize,
        threshold: f64,
        solution: &[f64],
        current_fixed: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        self.base
            .locally_fix_in_model(iteration, threshold, solution, current_fixed, model)
    }

    fn restore_non_removed_ranges_in_model(
        &mut self,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.base.restore_non_removed_ranges_in_model(model)
    }

    fn flush(&mut self) {
        self.base.flush();
    }

    fn apply_lifecycle(&mut self, lifecycle: &mut GanttDynamicModelLifecycle) {
        self.base.apply_lifecycle(lifecycle);
    }

    fn extract_shadow_price(
        &self,
        dual_solution: &LinearDualSolution,
        constraint_name_to_index: &HashMap<String, usize>,
    ) -> HashMap<usize, f64> {
        self.base
            .extract_shadow_price(dual_solution, constraint_name_to_index)
    }

    fn build_constraint_index_map(
        &self,
        constraint_name_to_index: &HashMap<String, usize>,
    ) -> ConstraintIndexMap {
        let mut map = self
            .base
            .build_constraint_index_map(constraint_name_to_index);
        map.register_executor_slot_compilation_constraints(
            self.base.compilation.base.executor_ids.iter(),
            self.slots.len(),
            constraint_name_to_index,
        );
        map
    }

    fn extract_executor_slot_shadow_prices(
        &self,
        dual_solution: &LinearDualSolution,
        constraint_index_map: &ConstraintIndexMap,
    ) -> HashMap<(Self::ExecutorId, usize), f64> {
        let mut prices = HashMap::new();
        for executor_id in &self.base.compilation.base.executor_ids {
            for slot_index in 0..self.slots.len() {
                let key = ConstraintIndexKey::executor_slot_compilation(
                    executor_id.to_string(),
                    slot_index,
                );
                if let Some(price) =
                    constraint_index_map.dual_value(&key, &dual_solution.constraints)
                {
                    prices.insert((executor_id.clone(), slot_index), price);
                }
            }
        }
        prices
    }

    fn task_count_for_shadow_price(&self) -> usize {
        self.base.task_count_for_shadow_price()
    }

    fn extract_fixed(&self, solution: &[f64]) -> HashSet<usize> {
        self.base.extract_fixed(solution)
    }

    fn extract_kept(&self, solution: &[f64]) -> HashSet<usize> {
        self.base.extract_kept(solution)
    }

    fn extract_hidden_executors(&self, solution: &[f64]) -> HashSet<I> {
        self.base.extract_hidden_executors(solution)
    }

    fn analyze_solution(
        &self,
        solution: &[f64],
    ) -> crate::domain::bunch_compilation::BunchSolution {
        self.base.analyze_solution(solution)
    }

    fn analyze_solution_with_state(
        &self,
        solution: &[f64],
        model_state: &GanttModelStateFacade,
    ) -> crate::domain::bunch_compilation::BunchSolution {
        self.base.analyze_solution_with_state(solution, model_state)
    }

    fn column_count(&self) -> usize {
        self.base.column_count()
    }

    fn get_bunch_entry(&self, bunch_index: usize) -> Option<BunchEntry<I>> {
        self.base.get_bunch_entry(bunch_index)
    }
}

impl<S, I> SlotBasedBunchCompilationContext<S> for BasicSlotBasedBunchCompilationContext<S, I>
where
    S: TimeSlot + Clone + std::fmt::Debug,
    I: ExecutorIdTrait,
{
    fn slots(&self) -> &[S] {
        &self.slots
    }

    fn intermediate_values(&self) -> Option<&CapacityIntermediateValues<S, I>> {
        self.intermediate_values.as_ref()
    }

    fn pre_solve_capacity(
        &mut self,
        model: &mut MetaModel<f64>,
        solver: &CapacityPreSolveSolver<'_>,
    ) -> GanttResult<&CapacityIntermediateValues<S, I>> {
        let values = self.capacity_pre_solver.solve(model, &self.slots, solver)?;
        self.intermediate_values = Some(values);
        self.refresh_slot_constraints();
        Ok(self
            .intermediate_values
            .as_ref()
            .expect("intermediate values must exist"))
    }

    fn slot_constraints(&self, slot_index: usize) -> Option<&SlotConstraints<I>> {
        self.intermediate_values
            .as_ref()
            .and_then(|values| values.slot_constraints(slot_index))
            .or_else(|| self.slot_constraints.get(&slot_index))
    }

    fn all_slot_constraints(&self) -> HashMap<usize, SlotConstraints<I>> {
        let mut constraints = self.slot_constraints.clone();
        if let Some(values) = &self.intermediate_values {
            for (slot_index, slot_constraints) in &values.slot_constraints {
                constraints.insert(*slot_index, slot_constraints.clone());
            }
        }

        if constraints.is_empty() {
            for slot_index in 0..self.slots.len() {
                if let Some(slot_constraints) = self.slot_constraints.get(&slot_index) {
                    constraints.insert(slot_index, slot_constraints.clone());
                }
            }
        }
        constraints
    }

    fn add_columns_by_slot(
        &mut self,
        iteration: usize,
        new_bunches: Vec<SlotBasedBunchEntry<I>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashMap<usize, Vec<usize>>> {
        let mut grouped: BTreeMap<usize, Vec<SlotBasedBunchEntry<I>>> = BTreeMap::new();
        for mut entry in new_bunches {
            entry.bunch.iteration = iteration;
            entry.bunch.slot_index = Some(entry.slot_index);
            grouped.entry(entry.slot_index).or_default().push(entry);
        }

        let mut added_by_slot = HashMap::new();
        for (slot_index, entries) in grouped {
            let bunches: Vec<BunchEntry<I>> =
                entries.iter().map(|entry| entry.bunch.clone()).collect();
            let added = self.base.add_columns(iteration, bunches, model)?;
            let added_set = added
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            let added_entries = entries
                .into_iter()
                .filter(|entry| added_set.contains(&entry.bunch.index));
            self.slot_bunches
                .entry(slot_index)
                .or_default()
                .extend(added_entries);
            let slot_variables = self.x_by_slot.entry(slot_index).or_default();
            for bunch_index in &added {
                if let Some(model_index) = self.base.compilation.bunch_x_map.get(bunch_index) {
                    slot_variables.push(*model_index);
                    if let Some(entry) = self.base.compilation.get_bunch_entry(*bunch_index) {
                        self.x_by_executor_slot
                            .entry((entry.executor_id, slot_index))
                            .or_default()
                            .push(*model_index);
                    }
                }
            }
            added_by_slot.insert(slot_index, added);
        }

        Ok(added_by_slot)
    }

    fn bunches_in_slot(&self, slot_index: usize) -> Vec<SlotBasedBunchEntry<I>> {
        self.slot_bunches
            .get(&slot_index)
            .cloned()
            .unwrap_or_default()
    }

    fn x_by_slot(&self) -> HashMap<usize, Vec<usize>> {
        self.x_by_slot.clone()
    }

    fn x_by_executor_slot(&self) -> HashMap<(I, usize), Vec<usize>> {
        self.x_by_executor_slot.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::bunch_compilation::context::BasicBunchCompilationContext;
    use crate::domain::bunch_generation::SlotConstraints;
    use crate::domain::task::BasicExecutor;
    use crate::infrastructure::TimeRange;
    use time::Duration;
    use time::macros::datetime;

    #[derive(Debug, Clone)]
    struct TestSlot {
        time: TimeRange,
    }

    impl TimeSlot for TestSlot {
        fn time(&self) -> &TimeRange {
            &self.time
        }

        fn sub_of(&self, sub_time: &TimeRange) -> Option<Self> {
            self.time.intersection(sub_time).map(|time| Self { time })
        }
    }

    #[derive(Debug, Clone)]
    struct StaticSolver<S>
    where
        S: TimeSlot + Clone,
    {
        values: CapacityIntermediateValues<S>,
    }

    impl<S> SlotBasedCapacityPreSolver<S> for StaticSolver<S>
    where
        S: TimeSlot + Clone + std::fmt::Debug,
    {
        fn extract_intermediate_values(
            &self,
            _solution: &[f64],
            _slots: &[S],
        ) -> GanttResult<CapacityIntermediateValues<S>> {
            Ok(self.values.clone())
        }
    }

    fn slot(hour_start: i64, hour_end: i64) -> TestSlot {
        TestSlot {
            time: TimeRange::new(
                datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour_start),
                datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour_end),
            ),
        }
    }

    #[test]
    fn test_slot_based_context_pre_solve_and_constraints() {
        let slots = vec![slot(8, 12), slot(12, 18)];
        let mut slot_constraints = HashMap::new();
        slot_constraints.insert(0, SlotConstraints::default());
        slot_constraints.insert(1, SlotConstraints::default());
        let values = CapacityIntermediateValues::new(slots.clone(), slot_constraints.clone());
        let solver = Box::new(StaticSolver { values });
        let base = BasicBunchCompilationContext::new(
            1,
            vec![BasicExecutor::new("exec_1", "Executor 1").id.clone()],
            false,
        );
        let mut ctx = BasicSlotBasedBunchCompilationContext::new(base, slots, solver)
            .with_slot_constraints(slot_constraints);
        let mut model = MetaModel::<f64>::new("slot_based_context");
        let solve = |_model: &MetaModel<f64>| Ok(Vec::new());

        ctx.register(&mut model).unwrap();
        let values = ctx.pre_solve_capacity(&mut model, &solve).unwrap();

        assert_eq!(values.slots.len(), 2);
        assert!(ctx.intermediate_values().is_some());
        assert!(ctx.slot_constraints(0).is_some());
        assert_eq!(ctx.all_slot_constraints().len(), 2);
    }

    #[test]
    fn test_slot_based_context_without_intermediate_values_returns_empty_constraints() {
        let slots = vec![slot(8, 12)];
        let values = CapacityIntermediateValues::new(slots.clone(), HashMap::new());
        let solver = Box::new(StaticSolver { values });
        let base = BasicBunchCompilationContext::new(
            1,
            vec![BasicExecutor::new("exec_1", "Executor 1").id.clone()],
            false,
        );
        let ctx = BasicSlotBasedBunchCompilationContext::new(base, slots, solver);

        assert!(ctx.intermediate_values().is_none());
        assert!(ctx.slot_constraints(0).is_none());
        assert!(ctx.all_slot_constraints().is_empty());
    }

    #[test]
    fn test_executor_slot_shadow_price_preserves_zero_dual() {
        let slots = vec![slot(8, 12)];
        let values = CapacityIntermediateValues::new(
            slots.clone(),
            HashMap::from([(0, SlotConstraints::default())]),
        );
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let base = BasicBunchCompilationContext::new(0, vec![executor.id.clone()], false);
        let ctx = BasicSlotBasedBunchCompilationContext::new(
            base,
            slots,
            Box::new(StaticSolver { values }),
        );
        let mut indexes = ConstraintIndexMap::new();
        indexes.register(
            ConstraintIndexKey::executor_slot_compilation(executor.id.to_string(), 0),
            "executor_slot_compilation_exec_1_0",
            0,
        );

        let prices = ctx.extract_executor_slot_shadow_prices(
            &LinearDualSolution::new(vec![0.0], Vec::new()),
            &indexes,
        );

        assert_eq!(prices.get(&(executor.id, 0)), Some(&0.0));
    }

    #[test]
    fn test_slot_based_context_delegates_add_columns_and_maps_full_slot_batch() {
        let slots = vec![slot(8, 12), slot(12, 18)];
        let values = CapacityIntermediateValues::new(slots.clone(), HashMap::new());
        let solver = Box::new(StaticSolver { values });
        let base = BasicBunchCompilationContext::new(
            2,
            vec![BasicExecutor::new("exec_1", "Executor 1").id.clone()],
            false,
        );
        let mut ctx = BasicSlotBasedBunchCompilationContext::new(base, slots, solver);
        let mut model = MetaModel::<f64>::new("slot_based_context_add");
        ctx.register(&mut model).unwrap();

        let added = IterativeBunchCompilationContext::add_columns(
            &mut ctx,
            0,
            vec![
                BunchEntry {
                    index: 0,
                    executor_id: "exec_1".into(),
                    task_indices: vec![0],
                    cost: 1.0,
                    iteration: 0,
                    slot_index: Some(0),
                },
                BunchEntry {
                    index: 1,
                    executor_id: "exec_1".into(),
                    task_indices: vec![1],
                    cost: 2.0,
                    iteration: 0,
                    slot_index: Some(0),
                },
                BunchEntry {
                    index: 2,
                    executor_id: "exec_1".into(),
                    task_indices: vec![1],
                    cost: 3.0,
                    iteration: 0,
                    slot_index: Some(1),
                },
            ],
            &mut model,
        )
        .unwrap();

        let x_by_slot = ctx.x_by_slot();
        let expected_slot_zero = vec![
            *ctx.base.compilation.bunch_x_map.get(&0).unwrap(),
            *ctx.base.compilation.bunch_x_map.get(&1).unwrap(),
        ];

        assert_eq!(added, vec![0, 1, 2]);
        assert_eq!(ctx.bunches_in_slot(0).len(), 2);
        assert_eq!(ctx.bunches_in_slot(1).len(), 1);
        assert_eq!(x_by_slot.get(&0), Some(&expected_slot_zero));
        assert_eq!(x_by_slot.get(&1).map(Vec::len), Some(1));
    }

    #[test]
    fn test_slot_based_context_rejects_bunch_without_slot() {
        let slots = vec![slot(8, 12)];
        let values = CapacityIntermediateValues::new(slots.clone(), HashMap::new());
        let solver = Box::new(StaticSolver { values });
        let base = BasicBunchCompilationContext::new(
            1,
            vec![BasicExecutor::new("exec_1", "Executor 1").id.clone()],
            false,
        );
        let mut ctx = BasicSlotBasedBunchCompilationContext::new(base, slots, solver);
        let mut model = MetaModel::<f64>::new("slot_based_context_missing_slot");
        ctx.register(&mut model).unwrap();

        let error = IterativeBunchCompilationContext::add_columns(
            &mut ctx,
            0,
            vec![BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0],
                cost: 1.0,
                iteration: 0,
                slot_index: None,
            }],
            &mut model,
        )
        .unwrap_err();

        assert!(format!("{error}").contains("slot index is required"));
    }
}
