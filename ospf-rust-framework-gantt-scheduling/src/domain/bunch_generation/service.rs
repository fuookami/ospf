//! 任务束生成服务 / Bunch generation services
//!
//! 提供基于时隙、已计划任务和未计划任务的列生成辅助服务。
//! Provides column-generation helper services for slot-based, planned, and unplanned tasks.

use std::collections::{HashMap, HashSet};

use crate::domain::bunch_compilation::model::{BunchEntry, SlotBasedBunchEntry};
use crate::domain::bunch_generation::model::{Graph, Node, TaskNode};
use crate::domain::bunch_generation::pricing::BunchPricingProblem;
use crate::domain::task::{AssignmentPolicyTrait, ExecutorTrait, TaskTrait};
use crate::infrastructure::{TimeRange, TimeSlot};
use crate::{GanttError, GanttResult};

/// 任务束生成配置 / Bunch generation configuration
#[derive(Debug, Clone)]
pub struct BunchGenerationConfig {
    /// 每个执行器最多生成的列数量 / Maximum columns generated per executor
    pub max_columns_per_executor: usize,
    /// 每个束最多包含的任务数量 / Maximum tasks contained by one bunch
    pub max_tasks_per_bunch: usize,
    /// 是否允许改变任务顺序 / Whether task order changes are allowed
    pub allow_order_change: bool,
    /// 是否允许空时隙生成结果 / Whether empty slot generation results are allowed
    pub allow_empty_slot: bool,
}

impl Default for BunchGenerationConfig {
    fn default() -> Self {
        Self {
            max_columns_per_executor: 60,
            max_tasks_per_bunch: 100,
            allow_order_change: false,
            allow_empty_slot: true,
        }
    }
}

/// 候选任务 / Candidate task
///
/// 将任务引用、稳定索引和生成状态聚合在一起，供生成器过滤。
/// Aggregates a task reference, stable index, and generation state for generator filtering.
#[derive(Debug)]
pub struct BunchTaskCandidate<'a, T> {
    /// 任务索引 / Task index
    pub index: usize,
    /// 任务引用 / Task reference
    pub task: &'a T,
    /// 是否已计划 / Whether the task is already planned
    pub planned: bool,
}

impl<T> Clone for BunchTaskCandidate<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for BunchTaskCandidate<'_, T> {}

impl<'a, T> BunchTaskCandidate<'a, T> {
    /// 创建候选任务 / Create candidate task
    pub fn new(index: usize, task: &'a T, planned: bool) -> Self {
        Self {
            index,
            task,
            planned,
        }
    }
}

/// 时隙约束 / Slot constraints
///
/// 提供通用过滤边界：可用执行器、排除任务、必选任务和每束任务数量上限。
/// Provides generic filtering boundaries: enabled executors,
/// excluded tasks, required tasks, and max task count per bunch.
#[derive(Debug, Clone, Default)]
pub struct SlotConstraints {
    /// 可用执行器 ID；为空表示不限制 / Enabled executor ids; empty means unrestricted
    pub enabled_executor_ids: HashSet<String>,
    /// 排除任务索引 / Excluded task indices
    pub excluded_task_indices: HashSet<usize>,
    /// 必须包含的任务索引 / Required task indices
    pub required_task_indices: HashSet<usize>,
    /// 每束任务数量上限 / Max tasks per bunch
    pub max_tasks_per_bunch: Option<usize>,
}

/// 束可行性策略 / Bunch feasibility policy
///
/// 允许将资源、产出、产能和业务规则统一注入到束生成流程。
/// Allows injecting resource, produce, capacity, and business rules into bunch generation.
pub trait BunchFeasibilityPolicy<E, T, A>: Send + Sync
where
    E: ExecutorTrait,
    T: TaskTrait<E, A>,
    A: AssignmentPolicyTrait<E>,
{
    /// 判断任务是否可用于当前执行器 / Check whether a task is feasible for the executor
    fn allow_task(
        &self,
        executor: &E,
        task: &T,
        slot: &TimeRange,
    ) -> bool;

    /// 判断候选束是否可行 / Check whether a candidate bunch is feasible
    fn allow_bunch(
        &self,
        executor: &E,
        bunch: &BunchEntry,
        slot: &TimeRange,
    ) -> bool;
}

/// 默认束可行性策略 / Default bunch feasibility policy
#[derive(Debug, Clone, Default)]
pub struct DefaultBunchFeasibilityPolicy;

impl<E, T, A> BunchFeasibilityPolicy<E, T, A> for DefaultBunchFeasibilityPolicy
where
    E: ExecutorTrait,
    T: TaskTrait<E, A>,
    A: AssignmentPolicyTrait<E>,
{
    fn allow_task(&self, _executor: &E, _task: &T, _slot: &TimeRange) -> bool {
        true
    }

    fn allow_bunch(&self, _executor: &E, _bunch: &BunchEntry, _slot: &TimeRange) -> bool {
        true
    }
}

impl SlotConstraints {
    /// 判断执行器是否可用 / Check whether an executor is enabled
    pub fn supports_executor(&self, executor_id: &str) -> bool {
        self.enabled_executor_ids.is_empty() || self.enabled_executor_ids.contains(executor_id)
    }

    /// 判断任务是否允许参与生成 / Check whether a task is allowed for generation
    pub fn allows_task(&self, task_index: usize) -> bool {
        !self.excluded_task_indices.contains(&task_index)
    }

    /// 校验生成的束是否满足约束 / Validate generated bunch against constraints
    pub fn accepts_bunch(&self, bunch: &BunchEntry) -> bool {
        if !self.supports_executor(&bunch.executor_id) {
            return false;
        }

        if let Some(max_tasks) = self.max_tasks_per_bunch {
            if bunch.task_indices.len() > max_tasks {
                return false;
            }
        }

        self.required_task_indices
            .iter()
            .all(|idx| bunch.task_indices.contains(idx))
    }
}

/// 产能中间值 / Capacity intermediate values
///
/// 将 Kotlin 侧 `CapacityIntermediateValues` 的生成器所需部分收敛为时隙和约束映射。
/// Narrows Kotlin-side `CapacityIntermediateValues` to the parts required by generators:
/// slots and slot-constraint mapping.
#[derive(Debug, Clone)]
pub struct CapacityIntermediateValues<S>
where
    S: TimeSlot,
{
    /// 时隙列表 / Slot list
    pub slots: Vec<S>,
    /// 时隙约束映射 / Slot constraints by slot index
    pub slot_constraints: HashMap<usize, SlotConstraints>,
}

impl<S> CapacityIntermediateValues<S>
where
    S: TimeSlot,
{
    /// 创建产能中间值 / Create capacity intermediate values
    pub fn new(slots: Vec<S>, slot_constraints: HashMap<usize, SlotConstraints>) -> Self {
        Self {
            slots,
            slot_constraints,
        }
    }

    /// 获取时隙约束 / Get slot constraints
    pub fn slot_constraints(&self, slot_index: usize) -> Option<&SlotConstraints> {
        self.slot_constraints.get(&slot_index)
    }
}

/// 基于时隙的任务束生成器 / Slot-based bunch generator
///
/// 基于候选任务、时隙约束和影子价格构建定价图，并生成 reduced cost 为负的束列。
/// Builds pricing graphs from candidate tasks, slot constraints, and shadow prices,
/// then generates bunch columns with negative reduced cost.
#[derive(Debug, Clone)]
pub struct SlotBasedBunchGenerator<E>
where
    E: ExecutorTrait,
{
    /// 支持的执行器列表 / Supported executors
    pub executors: Vec<E>,
    /// 生成配置 / Generation configuration
    pub config: BunchGenerationConfig,
}

impl<E> SlotBasedBunchGenerator<E>
where
    E: ExecutorTrait,
{
    /// 创建生成器 / Create generator
    pub fn new(executors: Vec<E>, config: BunchGenerationConfig) -> Self {
        Self { executors, config }
    }

    /// 判断是否支持执行器 / Check whether an executor is supported
    pub fn supports_executor(&self, executor: &E) -> bool {
        self.executors.iter().any(|item| item == executor)
    }

    /// 为指定时隙生成束 / Generate bunches for a specified slot
    pub fn generate<T, A, S>(
        &self,
        iteration: usize,
        slot_index: usize,
        slot: &S,
        constraints: &SlotConstraints,
        candidates: &[BunchTaskCandidate<'_, T>],
        shadow_prices: &HashMap<usize, f64>,
    ) -> GanttResult<Vec<SlotBasedBunchEntry>>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
        S: TimeSlot,
    {
        self.generate_with_policy(
            iteration,
            slot_index,
            slot,
            constraints,
            candidates,
            shadow_prices,
            &DefaultBunchFeasibilityPolicy,
        )
    }

    /// 使用可行性策略生成指定时隙的束 / Generate bunches for a slot with feasibility policy
    pub fn generate_with_policy<T, A, S, P>(
        &self,
        iteration: usize,
        slot_index: usize,
        slot: &S,
        constraints: &SlotConstraints,
        candidates: &[BunchTaskCandidate<'_, T>],
        shadow_prices: &HashMap<usize, f64>,
        feasibility_policy: &P,
    ) -> GanttResult<Vec<SlotBasedBunchEntry>>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
        S: TimeSlot,
        P: BunchFeasibilityPolicy<E, T, A>,
    {
        if slot.time().is_empty() {
            return Err(GanttError::InvalidTimeRange {
                message: "slot time range is empty".to_string(),
            });
        }

        let mut result = Vec::new();
        for executor in &self.executors {
            if !constraints.supports_executor(executor.id()) {
                continue;
            }

            let graph = self.build_pricing_graph(
                slot.time(),
                executor,
                constraints,
                candidates,
                feasibility_policy,
            )?;
            let mut pricing = BunchPricingProblem::new(executor.id().to_string(), graph);
            pricing.set_shadow_prices(shadow_prices.clone());

            let max_tasks = constraints
                .max_tasks_per_bunch
                .unwrap_or(self.config.max_tasks_per_bunch);
            let mut generated: Vec<BunchEntry> = pricing
                .solve(0)
                .into_iter()
                .filter(|bunch| {
                    bunch.task_indices.len() <= max_tasks
                        && constraints.accepts_bunch(bunch)
                        && feasibility_policy.allow_bunch(executor, bunch, slot.time())
                })
                .take(self.config.max_columns_per_executor)
                .map(|mut bunch| {
                    bunch.iteration = iteration;
                    bunch
                })
                .collect();

            generated.sort_by(|a, b| a.cost.total_cmp(&b.cost));
            result.extend(generated.into_iter().map(|bunch| SlotBasedBunchEntry {
                bunch,
                slot_index,
            }));
        }

        if result.is_empty() && !self.config.allow_empty_slot {
            return Err(GanttError::EmptyResult {
                message: format!("no bunch generated for slot {}", slot_index),
            });
        }

        Ok(result)
    }

    /// 批量生成所有时隙的束 / Generate bunches for all slots in batch
    pub fn generate_all<T, A, S>(
        &self,
        iteration: usize,
        intermediate_values: &CapacityIntermediateValues<S>,
        candidates: &[BunchTaskCandidate<'_, T>],
        shadow_prices: &HashMap<usize, f64>,
    ) -> GanttResult<Vec<SlotBasedBunchEntry>>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
        S: TimeSlot,
    {
        self.generate_all_with_policy(
            iteration,
            intermediate_values,
            candidates,
            shadow_prices,
            &DefaultBunchFeasibilityPolicy,
        )
    }

    /// 使用可行性策略批量生成所有时隙的束 / Generate all slots with feasibility policy
    pub fn generate_all_with_policy<T, A, S, P>(
        &self,
        iteration: usize,
        intermediate_values: &CapacityIntermediateValues<S>,
        candidates: &[BunchTaskCandidate<'_, T>],
        shadow_prices: &HashMap<usize, f64>,
        feasibility_policy: &P,
    ) -> GanttResult<Vec<SlotBasedBunchEntry>>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
        S: TimeSlot,
        P: BunchFeasibilityPolicy<E, T, A>,
    {
        let mut all_bunches = Vec::new();
        for (slot_index, slot) in intermediate_values.slots.iter().enumerate() {
            if let Some(constraints) = intermediate_values.slot_constraints(slot_index) {
                all_bunches.extend(self.generate_with_policy(
                    iteration,
                    slot_index,
                    slot,
                    constraints,
                    candidates,
                    shadow_prices,
                    feasibility_policy,
                )?);
            }
        }
        Ok(all_bunches)
    }

    fn build_pricing_graph<T, A>(
        &self,
        slot_time: &TimeRange,
        executor: &E,
        constraints: &SlotConstraints,
        candidates: &[BunchTaskCandidate<'_, T>],
        feasibility_policy: &impl BunchFeasibilityPolicy<E, T, A>,
    ) -> GanttResult<Graph>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
    {
        let mut graph = Graph::new();
        let mut task_nodes = Vec::new();

        for candidate in candidates {
            if !constraints.allows_task(candidate.index) {
                continue;
            }
            if !task_available_for_slot(candidate.task, executor, slot_time)
                || !feasibility_policy.allow_task(executor, candidate.task, slot_time)
            {
                continue;
            }

            let task_time = candidate
                .task
                .time()
                .or_else(|| candidate.task.scheduled_time())
                .or_else(|| candidate.task.time_window());
            let node_time = task_time.map(|time| time.start).unwrap_or(slot_time.start);
            let node = Node::Task(TaskNode::new(
                candidate.index,
                candidate.index as u64 + 1,
                node_time.unix_timestamp_nanos() as f64,
            ));

            graph.add_node(node.clone());
            task_nodes.push((node, node_time));
        }

        task_nodes.sort_by_key(|(_, time)| *time);
        for (node, _) in &task_nodes {
            graph.add_edge(graph.root().clone(), node.clone());
            graph.add_edge(node.clone(), graph.end().clone());
        }

        for (from_pos, (from_node, from_time)) in task_nodes.iter().enumerate() {
            for (to_node, to_time) in task_nodes.iter().skip(from_pos + 1) {
                if self.config.allow_order_change || from_time <= to_time {
                    graph.add_edge(from_node.clone(), to_node.clone());
                }
            }
        }

        Ok(graph)
    }
}

impl<E> Default for SlotBasedBunchGenerator<E>
where
    E: ExecutorTrait,
{
    fn default() -> Self {
        Self {
            executors: Vec::new(),
            config: BunchGenerationConfig::default(),
        }
    }
}

/// 已计划任务束生成器 / Planned task bunch generator
///
/// 仅使用 `planned = true` 的候选任务。
/// Uses only candidates with `planned = true`.
#[derive(Debug, Clone)]
pub struct PlannedTaskBunchGenerator<E>
where
    E: ExecutorTrait,
{
    /// 底层时隙生成器 / Inner slot-based generator
    pub inner: SlotBasedBunchGenerator<E>,
}

impl<E> PlannedTaskBunchGenerator<E>
where
    E: ExecutorTrait,
{
    /// 创建已计划任务束生成器 / Create planned task bunch generator
    pub fn new(inner: SlotBasedBunchGenerator<E>) -> Self {
        Self { inner }
    }

    /// 生成已计划任务束 / Generate planned task bunches
    pub fn generate<T, A, S>(
        &self,
        iteration: usize,
        slot_index: usize,
        slot: &S,
        constraints: &SlotConstraints,
        candidates: &[BunchTaskCandidate<'_, T>],
        shadow_prices: &HashMap<usize, f64>,
    ) -> GanttResult<Vec<SlotBasedBunchEntry>>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
        S: TimeSlot,
    {
        let planned = candidates
            .iter()
            .copied()
            .filter(|candidate| candidate.planned)
            .collect::<Vec<_>>();
        self.inner.generate(
            iteration,
            slot_index,
            slot,
            constraints,
            &planned,
            shadow_prices,
        )
    }
}

/// 未计划任务束生成器 / Unplanned task bunch generator
///
/// 仅使用 `planned = false` 的候选任务。
/// Uses only candidates with `planned = false`.
#[derive(Debug, Clone)]
pub struct UnplannedTaskBunchGenerator<E>
where
    E: ExecutorTrait,
{
    /// 底层时隙生成器 / Inner slot-based generator
    pub inner: SlotBasedBunchGenerator<E>,
}

impl<E> UnplannedTaskBunchGenerator<E>
where
    E: ExecutorTrait,
{
    /// 创建未计划任务束生成器 / Create unplanned task bunch generator
    pub fn new(inner: SlotBasedBunchGenerator<E>) -> Self {
        Self { inner }
    }

    /// 生成未计划任务束 / Generate unplanned task bunches
    pub fn generate<T, A, S>(
        &self,
        iteration: usize,
        slot_index: usize,
        slot: &S,
        constraints: &SlotConstraints,
        candidates: &[BunchTaskCandidate<'_, T>],
        shadow_prices: &HashMap<usize, f64>,
    ) -> GanttResult<Vec<SlotBasedBunchEntry>>
    where
        T: TaskTrait<E, A>,
        A: AssignmentPolicyTrait<E>,
        S: TimeSlot,
    {
        let unplanned = candidates
            .iter()
            .copied()
            .filter(|candidate| !candidate.planned)
            .collect::<Vec<_>>();
        self.inner.generate(
            iteration,
            slot_index,
            slot,
            constraints,
            &unplanned,
            shadow_prices,
        )
    }
}

fn task_available_for_slot<T, E, A>(
    task: &T,
    executor: &E,
    slot_time: &TimeRange,
) -> bool
where
    T: TaskTrait<E, A>,
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
{
    if !task.enabled_executors().is_empty()
        && !task.enabled_executors().iter().any(|item| item.id() == executor.id())
    {
        return false;
    }

    if let Some(assigned_executor) = task.executor() {
        if assigned_executor.id() != executor.id() && !task.executor_change_enabled() {
            return false;
        }
    }

    if let Some(time) = task.time().or_else(|| task.scheduled_time()) {
        return slot_time.intersects(time) || slot_time.contains_range(time);
    }

    if let Some(time_window) = task.time_window() {
        return slot_time.intersects(time_window) || time_window.contains_range(slot_time);
    }

    task.schedule_enabled(slot_time) || task.schedule_needed(slot_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{BasicAssignmentPolicy, BasicExecutor};
    use time::macros::datetime;
    use time::{Duration, OffsetDateTime};

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
    struct TestTask {
        id: String,
        name: String,
        time: Option<TimeRange>,
        window: Option<TimeRange>,
        executor: Option<BasicExecutor>,
        schedule_needed: bool,
    }

    impl TestTask {
        fn new(id: &str, start: OffsetDateTime, duration: Duration) -> Self {
            Self {
                id: id.to_string(),
                name: id.to_string(),
                time: Some(TimeRange::new(start, start + duration)),
                window: None,
                executor: None,
                schedule_needed: false,
            }
        }

        fn unplanned(id: &str, window: TimeRange) -> Self {
            Self {
                id: id.to_string(),
                name: id.to_string(),
                time: None,
                window: Some(window),
                executor: None,
                schedule_needed: true,
            }
        }
    }

    impl TaskTrait<BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> for TestTask {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn executor(&self) -> Option<&BasicExecutor> {
            self.executor.as_ref()
        }

        fn time(&self) -> Option<&TimeRange> {
            self.time.as_ref()
        }

        fn scheduled_time(&self) -> Option<&TimeRange> {
            self.time.as_ref()
        }

        fn time_window(&self) -> Option<&TimeRange> {
            self.window.as_ref()
        }

        fn schedule_enabled(&self, time_window: &TimeRange) -> bool {
            self.window
                .as_ref()
                .map(|window| window.intersects(time_window))
                .unwrap_or(self.schedule_needed)
        }

        fn schedule_needed(&self, _time_window: &TimeRange) -> bool {
            self.schedule_needed
        }
    }

    fn h(hour: i64) -> OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour)
    }

    #[test]
    fn test_slot_based_generator_generates_negative_reduced_cost_bunches() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig {
                max_columns_per_executor: 4,
                max_tasks_per_bunch: 2,
                allow_order_change: false,
                allow_empty_slot: false,
            },
        );
        let slot = TestSlot {
            time: TimeRange::new(h(8), h(18)),
        };
        let tasks = [
            TestTask::new("t0", h(9), Duration::hours(1)),
            TestTask::new("t1", h(11), Duration::hours(1)),
        ];
        let candidates = tasks
            .iter()
            .enumerate()
            .map(|(index, task)| BunchTaskCandidate::new(index, task, true))
            .collect::<Vec<_>>();
        let constraints = SlotConstraints {
            max_tasks_per_bunch: Some(2),
            ..Default::default()
        };
        let shadow_prices = HashMap::from([(0, 2.0), (1, 3.0)]);

        let bunches = generator
            .generate(1, 0, &slot, &constraints, &candidates, &shadow_prices)
            .unwrap();

        assert!(!bunches.is_empty());
        assert!(bunches.iter().all(|entry| entry.slot_index == 0));
        assert!(bunches
            .iter()
            .all(|entry| entry.bunch.executor_id == "exec_1"));
        assert!(bunches
            .iter()
            .all(|entry| entry.bunch.task_indices.len() <= 2));
    }

    #[test]
    fn test_planned_and_unplanned_generators_filter_candidates() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let inner = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig {
                max_columns_per_executor: 8,
                max_tasks_per_bunch: 1,
                allow_order_change: false,
                allow_empty_slot: true,
            },
        );
        let planned_generator = PlannedTaskBunchGenerator::new(inner.clone());
        let unplanned_generator = UnplannedTaskBunchGenerator::new(inner);
        let slot = TestSlot {
            time: TimeRange::new(h(8), h(18)),
        };
        let tasks = [
            TestTask::new("planned", h(9), Duration::hours(1)),
            TestTask::unplanned("unplanned", TimeRange::new(h(10), h(12))),
        ];
        let candidates = vec![
            BunchTaskCandidate::new(0, &tasks[0], true),
            BunchTaskCandidate::new(1, &tasks[1], false),
        ];
        let constraints = SlotConstraints {
            max_tasks_per_bunch: Some(1),
            ..Default::default()
        };
        let shadow_prices = HashMap::from([(0, 3.0), (1, 4.0)]);

        let planned = planned_generator
            .generate(1, 0, &slot, &constraints, &candidates, &shadow_prices)
            .unwrap();
        let unplanned = unplanned_generator
            .generate(1, 0, &slot, &constraints, &candidates, &shadow_prices)
            .unwrap();

        assert!(planned
            .iter()
            .all(|entry| entry.bunch.task_indices == vec![0]));
        assert!(unplanned
            .iter()
            .all(|entry| entry.bunch.task_indices == vec![1]));
    }

    #[test]
    fn test_generate_all_uses_slot_constraints() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig::default(),
        );
        let slots = vec![
            TestSlot {
                time: TimeRange::new(h(8), h(12)),
            },
            TestSlot {
                time: TimeRange::new(h(12), h(18)),
            },
        ];
        let mut slot_constraints = HashMap::new();
        slot_constraints.insert(1, SlotConstraints::default());
        let intermediate_values = CapacityIntermediateValues::new(slots, slot_constraints);
        let tasks = [TestTask::new("t0", h(13), Duration::hours(1))];
        let candidates = [BunchTaskCandidate::new(0, &tasks[0], true)];
        let shadow_prices = HashMap::from([(0, 2.0)]);

        let bunches = generator
            .generate_all(2, &intermediate_values, &candidates, &shadow_prices)
            .unwrap();

        assert!(!bunches.is_empty());
        assert!(bunches.iter().all(|entry| entry.slot_index == 1));
    }

    #[derive(Debug, Clone, Default)]
    struct RejectingPolicy;

    impl BunchFeasibilityPolicy<BasicExecutor, TestTask, BasicAssignmentPolicy<BasicExecutor>>
        for RejectingPolicy
    {
        fn allow_task(
            &self,
            _executor: &BasicExecutor,
            _task: &TestTask,
            _slot: &TimeRange,
        ) -> bool {
            false
        }

        fn allow_bunch(
            &self,
            _executor: &BasicExecutor,
            _bunch: &BunchEntry,
            _slot: &TimeRange,
        ) -> bool {
            false
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeResourcePolicy {
        blocked_task_ids: HashSet<String>,
    }

    impl BunchFeasibilityPolicy<BasicExecutor, TestTask, BasicAssignmentPolicy<BasicExecutor>>
        for FakeResourcePolicy
    {
        fn allow_task(
            &self,
            _executor: &BasicExecutor,
            task: &TestTask,
            _slot: &TimeRange,
        ) -> bool {
            !self.blocked_task_ids.contains(task.id())
        }

        fn allow_bunch(
            &self,
            _executor: &BasicExecutor,
            _bunch: &BunchEntry,
            _slot: &TimeRange,
        ) -> bool {
            true
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeProducePolicy {
        blocked_task_ids: HashSet<String>,
    }

    impl BunchFeasibilityPolicy<BasicExecutor, TestTask, BasicAssignmentPolicy<BasicExecutor>>
        for FakeProducePolicy
    {
        fn allow_task(
            &self,
            _executor: &BasicExecutor,
            task: &TestTask,
            _slot: &TimeRange,
        ) -> bool {
            !self.blocked_task_ids.contains(task.id())
        }

        fn allow_bunch(
            &self,
            _executor: &BasicExecutor,
            _bunch: &BunchEntry,
            _slot: &TimeRange,
        ) -> bool {
            true
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeCapacityPolicy {
        blocked_task_indices: HashSet<usize>,
    }

    impl BunchFeasibilityPolicy<BasicExecutor, TestTask, BasicAssignmentPolicy<BasicExecutor>>
        for FakeCapacityPolicy
    {
        fn allow_task(
            &self,
            _executor: &BasicExecutor,
            _task: &TestTask,
            _slot: &TimeRange,
        ) -> bool {
            true
        }

        fn allow_bunch(
            &self,
            _executor: &BasicExecutor,
            bunch: &BunchEntry,
            _slot: &TimeRange,
        ) -> bool {
            bunch
                .task_indices
                .iter()
                .all(|task_index| !self.blocked_task_indices.contains(task_index))
        }
    }

    #[test]
    fn test_feasibility_policy_can_block_generation() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig::default(),
        );
        let slot = TestSlot {
            time: TimeRange::new(h(8), h(18)),
        };
        let task = TestTask::new("t0", h(9), Duration::hours(1));
        let candidates = [BunchTaskCandidate::new(0, &task, true)];
        let constraints = SlotConstraints::default();
        let shadow_prices = HashMap::from([(0, 1.0)]);

        let bunches = generator
            .generate_with_policy(
                0,
                0,
                &slot,
                &constraints,
                &candidates,
                &shadow_prices,
                &RejectingPolicy,
            )
            .unwrap();
        assert!(bunches.is_empty());
    }

    #[test]
    fn test_resource_policy_can_block_infeasible_tasks() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig {
                max_columns_per_executor: 8,
                max_tasks_per_bunch: 1,
                allow_order_change: false,
                allow_empty_slot: true,
            },
        );
        let slot = TestSlot {
            time: TimeRange::new(h(8), h(18)),
        };
        let tasks = [
            TestTask::new("resource_blocked", h(9), Duration::hours(1)),
            TestTask::new("resource_ok", h(10), Duration::hours(1)),
        ];
        let candidates = tasks
            .iter()
            .enumerate()
            .map(|(index, task)| BunchTaskCandidate::new(index, task, true))
            .collect::<Vec<_>>();
        let constraints = SlotConstraints {
            max_tasks_per_bunch: Some(1),
            ..Default::default()
        };
        let policy = FakeResourcePolicy {
            blocked_task_ids: HashSet::from(["resource_blocked".to_string()]),
        };

        let bunches = generator
            .generate_with_policy(
                0,
                0,
                &slot,
                &constraints,
                &candidates,
                &HashMap::from([(0, 3.0), (1, 3.0)]),
                &policy,
            )
            .unwrap();

        assert!(!bunches.is_empty());
        assert!(bunches
            .iter()
            .all(|entry| !entry.bunch.task_indices.contains(&0)));
        assert!(bunches
            .iter()
            .any(|entry| entry.bunch.task_indices == vec![1]));
    }

    #[test]
    fn test_produce_policy_can_block_infeasible_tasks() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig {
                max_columns_per_executor: 8,
                max_tasks_per_bunch: 1,
                allow_order_change: false,
                allow_empty_slot: true,
            },
        );
        let slot = TestSlot {
            time: TimeRange::new(h(8), h(18)),
        };
        let tasks = [
            TestTask::new("produce_blocked", h(9), Duration::hours(1)),
            TestTask::new("produce_ok", h(10), Duration::hours(1)),
        ];
        let candidates = tasks
            .iter()
            .enumerate()
            .map(|(index, task)| BunchTaskCandidate::new(index, task, true))
            .collect::<Vec<_>>();
        let constraints = SlotConstraints {
            max_tasks_per_bunch: Some(1),
            ..Default::default()
        };
        let policy = FakeProducePolicy {
            blocked_task_ids: HashSet::from(["produce_blocked".to_string()]),
        };

        let bunches = generator
            .generate_with_policy(
                0,
                0,
                &slot,
                &constraints,
                &candidates,
                &HashMap::from([(0, 3.0), (1, 3.0)]),
                &policy,
            )
            .unwrap();

        assert!(!bunches.is_empty());
        assert!(bunches
            .iter()
            .all(|entry| !entry.bunch.task_indices.contains(&0)));
        assert!(bunches
            .iter()
            .any(|entry| entry.bunch.task_indices == vec![1]));
    }

    #[test]
    fn test_capacity_policy_can_block_generated_bunches() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig {
                max_columns_per_executor: 8,
                max_tasks_per_bunch: 1,
                allow_order_change: false,
                allow_empty_slot: true,
            },
        );
        let slot = TestSlot {
            time: TimeRange::new(h(8), h(18)),
        };
        let task = TestTask::new("capacity_blocked", h(9), Duration::hours(1));
        let candidates = [BunchTaskCandidate::new(0, &task, true)];
        let constraints = SlotConstraints {
            max_tasks_per_bunch: Some(1),
            ..Default::default()
        };
        let policy = FakeCapacityPolicy {
            blocked_task_indices: HashSet::from([0]),
        };

        let bunches = generator
            .generate_with_policy(
                0,
                0,
                &slot,
                &constraints,
                &candidates,
                &HashMap::from([(0, 3.0)]),
                &policy,
            )
            .unwrap();

        assert!(bunches.is_empty());
    }
}
