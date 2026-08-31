//! 时隙级任务束编译上下文 / Slot-based bunch compilation context
//!
//! 复用基础任务束编译上下文，并增加时隙、产能预求解和按时隙束管理。
//! Reuses the basic bunch compilation context and adds slots, capacity
//! pre-solving, and slot-wise bunch management.

use std::collections::HashMap;

use ospf_rust_core::model::MetaModel;

use crate::domain::bunch_compilation::context::{
    BasicBunchCompilationContext, IterativeBunchCompilationContext,
};
use crate::domain::bunch_compilation::model::{BunchEntry, SlotBasedBunchEntry};
use crate::domain::bunch_generation::{CapacityIntermediateValues, SlotConstraints};
use crate::infrastructure::TimeSlot;
use crate::GanttResult;

/// 时隙级产能预求解器 / Slot-based capacity pre-solver
pub trait SlotBasedCapacityPreSolver<S>: Send + Sync + std::fmt::Debug
where
    S: TimeSlot + Clone,
{
    /// 执行产能预求解 / Execute capacity pre-solving
    fn pre_solve(
        &self,
        model: &mut MetaModel<f64>,
        slots: &[S],
    ) -> GanttResult<CapacityIntermediateValues<S>>;
}

/// 静态时隙级产能预求解器 / Static slot-based capacity pre-solver
#[derive(Debug, Clone)]
pub struct StaticSlotBasedCapacityPreSolver<S>
where
    S: TimeSlot + Clone,
{
    /// 预置的中间值 / Predefined intermediate values
    pub intermediate_values: CapacityIntermediateValues<S>,
}

impl<S> StaticSlotBasedCapacityPreSolver<S>
where
    S: TimeSlot + Clone,
{
    /// 创建静态预求解器 / Create static pre-solver
    pub fn new(intermediate_values: CapacityIntermediateValues<S>) -> Self {
        Self { intermediate_values }
    }
}

impl<S> SlotBasedCapacityPreSolver<S> for StaticSlotBasedCapacityPreSolver<S>
where
    S: TimeSlot + Clone + std::fmt::Debug,
{
    fn pre_solve(
        &self,
        _model: &mut MetaModel<f64>,
        _slots: &[S],
    ) -> GanttResult<CapacityIntermediateValues<S>> {
        Ok(self.intermediate_values.clone())
    }
}

/// 时隙级任务束编译上下文 / Slot-based bunch compilation context
pub trait SlotBasedBunchCompilationContext<S>: Send + Sync
where
    S: TimeSlot + Clone,
{
    /// 注册到模型 / Register to model
    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()>;

    /// 获取时隙列表 / Get slots
    fn slots(&self) -> &[S];

    /// 获取产能中间值 / Get capacity intermediate values
    fn intermediate_values(&self) -> Option<&CapacityIntermediateValues<S>>;

    /// 执行产能预求解 / Execute capacity pre-solving
    fn pre_solve_capacity(
        &mut self,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<&CapacityIntermediateValues<S>>;

    /// 获取指定时隙约束 / Get constraints for specified slot
    fn slot_constraints(&self, slot_index: usize) -> Option<&SlotConstraints>;

    /// 获取所有时隙约束 / Get all slot constraints
    fn all_slot_constraints(&self) -> HashMap<usize, SlotConstraints>;

    /// 按时隙添加列 / Add columns by slot
    fn add_columns_by_slot(
        &mut self,
        iteration: usize,
        new_bunches: Vec<SlotBasedBunchEntry>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashMap<usize, Vec<usize>>>;

    /// 获取指定时隙的束 / Get bunches for specified slot
    fn bunches_in_slot(&self, slot_index: usize) -> Vec<SlotBasedBunchEntry>;
}

/// 基础时隙级任务束编译上下文 / Basic slot-based bunch compilation context
#[derive(Debug)]
pub struct BasicSlotBasedBunchCompilationContext<S>
where
    S: TimeSlot + Clone + std::fmt::Debug,
{
    /// 基础任务束编译上下文 / Basic bunch compilation context
    pub base: BasicBunchCompilationContext,
    slots: Vec<S>,
    capacity_pre_solver: Box<dyn SlotBasedCapacityPreSolver<S>>,
    intermediate_values: Option<CapacityIntermediateValues<S>>,
    slot_constraints: HashMap<usize, SlotConstraints>,
    slot_bunches: HashMap<usize, Vec<SlotBasedBunchEntry>>,
}

impl<S> BasicSlotBasedBunchCompilationContext<S>
where
    S: TimeSlot + Clone + std::fmt::Debug,
{
    /// 创建基础时隙级任务束编译上下文 / Create basic slot-based bunch compilation context
    pub fn new(
        base: BasicBunchCompilationContext,
        slots: Vec<S>,
        capacity_pre_solver: Box<dyn SlotBasedCapacityPreSolver<S>>,
    ) -> Self {
        Self {
            base,
            slots,
            capacity_pre_solver,
            intermediate_values: None,
            slot_constraints: HashMap::new(),
            slot_bunches: HashMap::new(),
        }
    }

    /// 设置时隙约束 / Set slot constraints
    pub fn with_slot_constraints(
        mut self,
        slot_constraints: HashMap<usize, SlotConstraints>,
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

impl<S> SlotBasedBunchCompilationContext<S> for BasicSlotBasedBunchCompilationContext<S>
where
    S: TimeSlot + Clone + std::fmt::Debug,
{
    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.base.register(model)
    }

    fn slots(&self) -> &[S] {
        &self.slots
    }

    fn intermediate_values(&self) -> Option<&CapacityIntermediateValues<S>> {
        self.intermediate_values.as_ref()
    }

    fn pre_solve_capacity(
        &mut self,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<&CapacityIntermediateValues<S>> {
        let values = self.capacity_pre_solver.pre_solve(model, &self.slots)?;
        self.intermediate_values = Some(values);
        self.refresh_slot_constraints();
        Ok(self.intermediate_values.as_ref().expect("intermediate values must exist"))
    }

    fn slot_constraints(&self, slot_index: usize) -> Option<&SlotConstraints> {
        self.intermediate_values
            .as_ref()
            .and_then(|values| values.slot_constraints(slot_index))
            .or_else(|| self.slot_constraints.get(&slot_index))
    }

    fn all_slot_constraints(&self) -> HashMap<usize, SlotConstraints> {
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
        new_bunches: Vec<SlotBasedBunchEntry>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashMap<usize, Vec<usize>>> {
        let mut grouped: HashMap<usize, Vec<SlotBasedBunchEntry>> = HashMap::new();
        for mut entry in new_bunches {
            entry.bunch.iteration = iteration;
            grouped.entry(entry.slot_index).or_default().push(entry);
        }

        let mut added_by_slot = HashMap::new();
        for (slot_index, entries) in grouped {
            let bunches: Vec<BunchEntry> = entries.iter().map(|entry| entry.bunch.clone()).collect();
            let added = self.base.add_columns(iteration, bunches, model)?;
            let added_set = added.iter().copied().collect::<std::collections::HashSet<_>>();
            let added_entries = entries
                .into_iter()
                .filter(|entry| added_set.contains(&entry.bunch.index));
            self.slot_bunches
                .entry(slot_index)
                .or_default()
                .extend(added_entries);
            added_by_slot.insert(slot_index, added);
        }

        Ok(added_by_slot)
    }

    fn bunches_in_slot(&self, slot_index: usize) -> Vec<SlotBasedBunchEntry> {
        self.slot_bunches
            .get(&slot_index)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::bunch_compilation::context::BasicBunchCompilationContext;
    use crate::domain::bunch_generation::SlotConstraints;
    use crate::domain::task::BasicExecutor;
    use crate::infrastructure::TimeRange;
    use time::macros::datetime;
    use time::Duration;

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
        fn pre_solve(
            &self,
            _model: &mut MetaModel<f64>,
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

        ctx.register(&mut model).unwrap();
        let values = ctx.pre_solve_capacity(&mut model).unwrap();

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
    fn test_slot_based_context_add_columns_by_slot() {
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

        let added = ctx
            .add_columns_by_slot(
                0,
                vec![
                    SlotBasedBunchEntry {
                        bunch: BunchEntry {
                            index: 0,
                            executor_id: "exec_1".to_string(),
                            task_indices: vec![0],
                            cost: 1.0,
                            iteration: 0,
                        },
                        slot_index: 0,
                    },
                    SlotBasedBunchEntry {
                        bunch: BunchEntry {
                            index: 1,
                            executor_id: "exec_1".to_string(),
                            task_indices: vec![1],
                            cost: 2.0,
                            iteration: 0,
                        },
                        slot_index: 1,
                    },
                ],
                &mut model,
            )
            .unwrap();

        assert_eq!(added.len(), 2);
        assert_eq!(ctx.bunches_in_slot(0).len(), 1);
        assert_eq!(ctx.bunches_in_slot(1).len(), 1);
    }
}
