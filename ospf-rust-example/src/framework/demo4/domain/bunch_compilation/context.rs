//! 无损恢复记录与 Gantt 受管主问题的注册边界。 / Registration boundary between lossless recoveries and the managed Gantt master.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::column_generation_solver::LinearDualSolution;
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::domain::bunch_compilation::{
    BasicBunchCompilationContext, BunchEntry, BunchExecutorSelection, BunchMasterConfiguration, BunchSolution,
    IterativeBunchCompilationContext,
};
use ospf_rust_framework_gantt_scheduling::domain::common::{BunchId, ExecutorId};
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::passenger::recovery::{RecoveryPassengerComponent, RecoveryPassengerLegResult, RecoveryPassengerRecovery};
use crate::framework::demo4::domain::task::model::{Aircraft, Airport, FlightTaskImpl, FlightTaskRecovery, FlightTaskStatus};
use super::rules::{RecoveryRuleComponent, RecoveryRuleRow};
use super::fleet_balance::{RecoveryFleetBalance, RecoveryFleetBalanceResult, RecoveryFleetComponent};
use super::operating_costs::{RecoveryOperatingCostComponent, RecoveryOperatingCostResult, RecoveryOperatingCosts};
use super::restrictions::{RecoveryRestrictionComponent, RecoveryRestrictionResult, RecoveryRestrictions};
use super::flow_control::{RecoveryFlowControlComponent, RecoveryFlowControlResult, RecoveryFlowControls};

/// 主问题的原始任务与飞机定义；规则可行性仍由恢复生成策略校验。 / Original task and aircraft definitions; recovery generation policies remain responsible for business-rule feasibility.
#[derive(Debug, Clone)]
pub struct RecoveryMasterConfiguration {
    /// 稳定任务顺序，取消费用与该顺序一致。 / Stable task order aligned with cancellation costs.
    pub tasks: Vec<FlightTaskImpl>,
    /// 可用的完整飞机配置。 / Available complete aircraft configurations.
    pub aircraft: Vec<Aircraft>,
    /// 每任务取消费用，None 为硬覆盖。 / Per-task cancellation costs, with None requiring hard coverage.
    pub cancellation_costs: Vec<Option<f64>>,
}

/// 从实际求解变量恢复的调度，不从任务 ID 重构航线或时间。 / Schedule recovered from actual solver variables, never reconstructing routes or times from task IDs.
#[derive(Debug, Clone)]
pub struct RecoverySchedule {
    /// 选中的完整业务候选，按模型历史列索引排序。 / Selected complete business candidates ordered by historical model column index.
    pub selected: Vec<RecoveryBunchCandidate>,
    /// 实际取消变量对应的原任务。 / Original tasks corresponding to actual cancellation variables.
    pub canceled: Vec<FlightTaskImpl>,
    /// 从实际恢复记录和模型缺口变量复核的机队分布。 / Fleet distribution checked against actual recovery records and modeled shortage variables.
    pub fleet_balance: Vec<RecoveryFleetBalanceResult>,
    /// 实际取消、舱位/航班改签及运输人数，硬旅客模式下为空。 / Actual cancellation, cabin/flight changes and transported counts, empty in hard-passenger mode.
    pub passengers: Vec<RecoveryPassengerLegResult>,
    /// 软链接拆分和飞机闲置的实际模型费用。 / Actual modeled soft-link split and aircraft-idle costs.
    pub operating_costs: RecoveryOperatingCostResult,
    /// 硬限制复核及软限制的实际使用量与费用。 / Hard-restriction checks and actual soft-restriction usage and costs.
    pub restrictions: Vec<RecoveryRestrictionResult>,
    /// 按实际内部事件与停场间隔复核的机场容量使用量。 / Airport capacity usage checked against actual internal events and stay intervals.
    pub flow_controls: Vec<RecoveryFlowControlResult>,
}

/// 只读恢复目录；不得保存模型注册态或从任务 ID 猜测实际恢复记录。 / Read-only recovery directory; it must neither hold model registration state nor infer actual recoveries from task IDs.
pub trait RecoveryBunchResolver: std::fmt::Debug + Send + Sync {
    /// 从完整候选身份恢复不可变记录及完整费用，未知身份必须失败。 / Resolve immutable records and complete costs from full candidate identity, rejecting unknown identities.
    fn resolve(&self, entry: &BunchEntry) -> GanttResult<RecoveryBunchCandidate>;

    /// 模型行和定价使用同一不可变规则定义。 / Model rows and pricing share the same immutable rule definitions.
    fn master_rows(&self) -> &[RecoveryRuleRow] { &[] }

    /// 不可变机队终点定义，注册状态由 context 独占。 / Immutable terminal fleet definition with registration state exclusively owned by the context.
    fn fleet_balance(&self) -> Option<&RecoveryFleetBalance> { None }

    /// 不可变旅客恢复定义，不共享模型注册态。 / Immutable passenger recovery definition without shared model registration state.
    fn passenger_recovery(&self) -> Option<&RecoveryPassengerRecovery> { None }

    /// 不可变运行费用定义，不共享模型注册态。 / Immutable operating cost definition without shared registration state.
    fn operating_costs(&self) -> Option<&RecoveryOperatingCosts> { None }

    /// 只读限制目录，不跨模型共享注册状态。 / Read-only restriction directory without cross-model registration state.
    fn restrictions(&self) -> Option<&RecoveryRestrictions> { None }

    /// 只读有限流控目录，不共享模型注册态。 / Read-only finite flow-control directory without shared model registration state.
    fn flow_controls(&self) -> Option<&RecoveryFlowControls> { None }

    /// 完整调度的独立生产复核。 / Independent production validation of a complete schedule.
    fn validate_schedule(&self, _: &RecoverySchedule) -> GanttResult<()> { Ok(()) }
}

/// 复用 Gantt 的变量、约束、目标和列生命周期，不实现第二套主模型。 / Reuse Gantt variables, rows, objectives and column lifecycle without a second master-model implementation.
#[derive(Debug, Clone)]
pub struct BunchCompilationContext {
    configuration: RecoveryMasterConfiguration,
    task_indices: HashMap<String, usize>,
    aircraft_indices: HashMap<String, usize>,
    candidates: HashMap<BunchId, RecoveryBunchCandidate>,
    resolver: Option<Arc<dyn RecoveryBunchResolver>>,
    rules: RecoveryRuleComponent,
    fleet: RecoveryFleetComponent,
    passengers: RecoveryPassengerComponent,
    operating_costs: RecoveryOperatingCostComponent,
    restrictions: RecoveryRestrictionComponent,
    flow_controls: RecoveryFlowControlComponent,
    master: BasicBunchCompilationContext,
}

impl BunchCompilationContext {
    /// 定义有限任务和飞机；未知引用、重复定义和冲突的硬取消规则作为输入错误。 / Define finite tasks and aircraft; unknown references, duplicates and conflicting hard cancellation rules are input errors.
    pub fn new(configuration: RecoveryMasterConfiguration) -> GanttResult<Self> {
        let mut task_indices = HashMap::new();
        let mut aircraft_indices = HashMap::new();
        for (index, task) in configuration.tasks.iter().enumerate() {
            if task.plan.id.is_empty() || task_indices.insert(task.plan.id.clone(), index).is_some() {
                return Err(invalid("empty or duplicate original task ID"));
            }
            if task.plan.flight_task_status.contains(&FlightTaskStatus::NotCancel)
                && configuration.cancellation_costs.get(index).is_some_and(Option::is_some)
            {
                return Err(invalid("NotCancel task must use hard coverage"));
            }
        }
        for (index, aircraft) in configuration.aircraft.iter().enumerate() {
            if aircraft.reg_no.0.is_empty() || aircraft_indices.insert(aircraft.reg_no.0.clone(), index).is_some() {
                return Err(invalid("empty or duplicate aircraft registration"));
            }
        }
        let mut master = BasicBunchCompilationContext::new(
            configuration.tasks.len(),
            configuration.aircraft.iter().map(|aircraft| aircraft.reg_no.0.clone()).collect::<Vec<_>>(),
            false,
        );
        master.compilation.configure_master(BunchMasterConfiguration {
            cancellation_costs: configuration.cancellation_costs.clone(),
            executor_selection: BunchExecutorSelection::AtMostOne,
        })?;
        Ok(Self { configuration, task_indices, aircraft_indices, candidates: HashMap::new(), resolver: None,
            rules: Default::default(), fleet: Default::default(), passengers: Default::default(), operating_costs: Default::default(),
            restrictions: Default::default(), flow_controls: Default::default(), master })
    }

    /// 每个 context 独立持有注册态，只共享无模型状态的不可变业务目录。 / Each context owns independent registration state, sharing only an immutable model-free business directory.
    pub fn with_resolver(
        configuration: RecoveryMasterConfiguration,
        resolver: Arc<dyn RecoveryBunchResolver>,
    ) -> GanttResult<Self> {
        let mut context = Self::new(configuration)?;
        context.rules.rows = resolver.master_rows().to_vec();
        context.fleet.definition = resolver.fleet_balance().cloned();
        context.passengers.definition = resolver.passenger_recovery().cloned();
        context.operating_costs.definition = resolver.operating_costs().cloned();
        context.restrictions.definition = resolver.restrictions().cloned();
        context.flow_controls.definition = resolver.flow_controls().cloned();
        context.resolver = Some(resolver);
        Ok(context)
    }

    /// 只读访问共享 Gantt 编译及其稳定变量映射。 / Read-only access to the shared Gantt compilation and stable variable mapping.
    pub fn master(&self) -> &BasicBunchCompilationContext { &self.master }

    /// 注册同一 Gantt 主问题，允许下游直接追加模型约束和目标。 / Register the same Gantt master, allowing downstream model rows and objectives.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        let mut next = self.clone();
        model.transaction(|model| {
            next.master.register(model)?;
            next.refresh_rules(model)
        })?;
        *self = next;
        Ok(())
    }

    fn refresh_rules(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        let columns = self.master.compilation.base.aggregation.bunches().iter().filter_map(|entry| {
            self.master.compilation.bunch_x_map.get(&entry.index).map(|index| (entry, *index))
        }).map(|(entry, index)| self.candidates.get(&entry.id).cloned().map(|candidate| (index, candidate))
            .ok_or_else(|| invalid("missing recovery payload for rule refresh"))).collect::<GanttResult<Vec<_>>>()?;
        self.rules.refresh(&columns, model)?;
        self.fleet.refresh(&columns, model)?;
        self.passengers.refresh(&columns, model)?;
        let cancellations = self.configuration.tasks.iter().zip(&self.master.compilation.base.y_indices)
            .map(|(task, variable)| (task.plan.id.clone(), *variable)).collect();
        self.operating_costs.refresh(&columns, &cancellations, model)?;
        self.restrictions.refresh(&columns, model)?;
        self.flow_controls.refresh(&columns, model)
    }

    /// 原子登记完整恢复记录并增列；同业务身份必须保持完整 payload 和费用一致。 / Atomically record complete recoveries and add columns; a business identity must retain its complete payload and cost.
    pub fn add_candidates(
        &mut self,
        iteration: usize,
        candidates: Vec<RecoveryBunchCandidate>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        let mut next = self.clone();
        let offset = self.master.next_column_index()?;
        let mut entries = Vec::new();
        for (position, candidate) in candidates.into_iter().enumerate() {
            let index = offset.checked_add(position).ok_or_else(|| invalid("column index exhausted"))?;
            let entry = next.entry(&candidate, iteration, index)?;
            if let Some(existing) = next.candidates.get(&entry.id)
                && !same_candidate(existing, &candidate)
            {
                return Err(invalid("conflicting recovery payload for the same bunch ID"));
            }
            next.candidates.entry(entry.id.clone()).or_insert(candidate);
            entries.push(entry);
        }
        let added = IterativeBunchCompilationContext::add_columns(&mut next, iteration, entries, model)?;
        *self = next;
        Ok(added)
    }

    /// 原子退休共享主问题中的列，保留不可变业务历史供一致性检查和重加。 / Atomically retire shared master columns while retaining immutable business history for consistency checks and re-addition.
    pub fn remove_columns(&mut self, indices: &[usize], model: &mut MetaModel<f64>) -> GanttResult<()> {
        let mut next = self.clone();
        model.transaction(|model| {
            next.master.remove_columns(indices, model)?;
            next.refresh_rules(model)
        })?;
        *self = next;
        Ok(())
    }

    /// 先在生产入口校验完整当前模型，再返回原始恢复记录。 / Validate the complete current model at the production entry before returning original recovery records.
    pub fn extract_solution(&self, model: &MetaModel<f64>, values: &[f64]) -> GanttResult<RecoverySchedule> {
        let solution = self.master.analyze_solution_checked(model, values)?;
        let selected = solution.selected_bunches.iter().map(|index| {
            let entry = self.master.get_bunch_entry(*index).ok_or_else(|| invalid("missing registered bunch"))?;
            self.candidates.get(&entry.id).cloned().ok_or_else(|| invalid("missing recovery payload"))
        }).collect::<GanttResult<Vec<_>>>()?;
        let canceled = solution.canceled_tasks.into_iter().map(|index| self.configuration.tasks[index].clone()).collect::<Vec<_>>();
        let fleet_balance = self.fleet.extract(&selected, values)?;
        let passengers = self.passengers.extract(&selected, values)?;
        let canceled_ids = canceled.iter().map(|task| task.plan.id.as_str()).collect();
        let operating_costs = self.operating_costs.extract(&selected, &canceled_ids, values)?;
        let restrictions = self.restrictions.extract(&selected, values)?;
        let flow_controls = self.flow_controls.extract(&selected, values)?;
        let schedule = RecoverySchedule { selected, canceled, fleet_balance, passengers, operating_costs, restrictions, flow_controls };
        if let Some(resolver) = &self.resolver { resolver.validate_schedule(&schedule)?; }
        Ok(schedule)
    }

    /// 校验完整记录并投影为 Gantt 候选；索引及轮次不属于业务身份。 / Validate full records and project a Gantt candidate; index and iteration are not business identity.
    pub fn entry(
        &self,
        candidate: &RecoveryBunchCandidate,
        iteration: usize,
        index: usize,
    ) -> GanttResult<BunchEntry> {
        let bunch = &candidate.bunch;
        if bunch.id().is_empty() || !candidate.cost.is_finite() { return Err(invalid("empty bunch ID or non-finite cost")); }
        let executor = &bunch.aircraft().reg_no.0;
        let aircraft = self.aircraft_indices.get(executor).map(|index| &self.configuration.aircraft[*index])
            .ok_or_else(|| invalid("unknown executor aircraft"))?;
        if !same_aircraft(aircraft, bunch.aircraft()) { return Err(invalid("changed executor aircraft configuration")); }
        let mut task_indices = Vec::new();
        for activity in bunch.activities() {
            let task = activity.original();
            let task_index = *self.task_indices.get(&task.plan.id).ok_or_else(|| invalid("unknown original task"))?;
            if !same_task(task, &self.configuration.tasks[task_index]) { return Err(invalid("changed original task definition")); }
            task_indices.push(task_index);
        }
        // 飞机命名空间及长度编码避免跨飞机和分隔符碰撞。 / Aircraft namespaces and length encoding avoid cross-aircraft and delimiter collisions.
        let id = format!("{}:{}{}:{}", executor.len(), executor, bunch.id().len(), bunch.id());
        Ok(BunchEntry { id: id.into(), index, executor_id: executor.clone().into(), task_indices, cost: candidate.cost, iteration, slot_index: None })
    }
}

impl IterativeBunchCompilationContext for BunchCompilationContext {
    type ExecutorId = ExecutorId;

    fn pricing_domain(&self) -> Option<HashSet<(ExecutorId, Option<usize>)>> { self.master.pricing_domain() }

    fn pricing_revision(&self) -> Option<String> { self.master.pricing_revision() }

    fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> { BunchCompilationContext::register(self, model) }

    fn add_columns(
        &mut self,
        iteration: usize,
        entries: Vec<BunchEntry>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        let mut next = self.clone();
        for entry in &entries {
            let candidate = if let Some(resolver) = &next.resolver {
                resolver.resolve(entry)?
            } else {
                next.candidates.get(&entry.id).cloned().ok_or_else(|| invalid("missing complete recovery payload resolver"))?
            };
            let canonical = next.entry(&candidate, iteration, entry.index)?;
            if canonical.id != entry.id || canonical.executor_id != entry.executor_id
                || canonical.task_indices != entry.task_indices || canonical.cost != entry.cost || canonical.slot_index != entry.slot_index
                || next.candidates.get(&entry.id).is_some_and(|existing| !same_candidate(existing, &candidate))
            {
                return Err(invalid("Gantt candidate differs from complete recovery payload"));
            }
            next.candidates.entry(entry.id.clone()).or_insert(candidate);
        }
        let added = model.transaction(|model| {
            let added = next.master.add_columns(iteration, entries, model)?;
            next.refresh_rules(model)?;
            GanttResult::Ok(added)
        })?;
        *self = next;
        Ok(added)
    }

    fn remove_columns(&mut self, indices: &[usize], model: &mut MetaModel<f64>) -> GanttResult<()> {
        BunchCompilationContext::remove_columns(self, indices, model)
    }

    fn next_column_index(&self) -> GanttResult<usize> { self.master.next_column_index() }

    fn hide_bunches_in_model(&mut self, indices: &[usize], model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.master.hide_bunches_in_model(indices, model)
    }

    fn globally_fix_in_model(&mut self, indices: &HashSet<usize>, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.master.globally_fix_in_model(indices, model)
    }

    fn locally_fix_in_model(
        &mut self,
        iteration: usize,
        threshold: f64,
        solution: &[f64],
        fixed: &HashSet<usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        self.master.locally_fix_in_model(iteration, threshold, solution, fixed, model)
    }

    fn restore_non_removed_ranges_in_model(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.master.restore_non_removed_ranges_in_model(model)
    }

    fn flush(&mut self) { self.master.flush(); }

    fn extract_shadow_price(&self, dual: &LinearDualSolution, names: &HashMap<String, usize>) -> HashMap<usize, f64> {
        self.master.extract_shadow_price(dual, names)
    }

    fn task_count_for_shadow_price(&self) -> usize { self.master.task_count_for_shadow_price() }

    fn extract_fixed(&self, _: &[f64]) -> HashSet<usize> {
        // 初始 incumbent 是可改进的恢复方案，不是已执行活动或分支决策。 / An initial incumbent is an improvable recovery, not an executed activity or branch decision.
        HashSet::new()
    }

    fn extract_kept(&self, solution: &[f64]) -> HashSet<usize> { self.master.extract_kept(solution) }

    fn extract_hidden_executors(&self, solution: &[f64]) -> HashSet<ExecutorId> { self.master.extract_hidden_executors(solution) }

    fn analyze_solution(&self, solution: &[f64]) -> BunchSolution { self.master.analyze_solution(solution) }

    fn analyze_solution_checked(&self, model: &MetaModel<f64>, values: &[f64]) -> GanttResult<BunchSolution> {
        let result = self.master.analyze_solution_checked(model, values)?;
        self.extract_solution(model, values)?;
        Ok(result)
    }

    fn column_count(&self) -> usize { self.master.column_count() }

    fn get_bunch_entry(&self, index: usize) -> Option<BunchEntry> { self.master.get_bunch_entry(index) }
}

fn same_candidate(left: &RecoveryBunchCandidate, right: &RecoveryBunchCandidate) -> bool {
    left.cost == right.cost && same_aircraft(left.bunch.aircraft(), right.bunch.aircraft())
        && left.bunch.activities().len() == right.bunch.activities().len()
        && left.bunch.activities().iter().zip(right.bunch.activities()).all(|(left, right)| same_recovery(left, right))
}

pub(crate) fn same_recovery(left: &FlightTaskRecovery, right: &FlightTaskRecovery) -> bool {
    same_task(left.original(), right.original()) && same_aircraft(left.aircraft(), right.aircraft())
        && same_airport(&left.route().dep, &right.route().dep) && same_airport(&left.route().arr, &right.route().arr)
        && left.time() == right.time()
}

fn same_task(left: &FlightTaskImpl, right: &FlightTaskImpl) -> bool {
    let same_optional_aircraft = |left: &Option<Aircraft>, right: &Option<Aircraft>| match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => same_aircraft(left, right),
        _ => false,
    };
    left.plan.id == right.plan.id && left.plan.name == right.plan.name
        && same_optional_aircraft(&left.plan.aircraft, &right.plan.aircraft)
        && same_airport(&left.plan.dep, &right.plan.dep) && same_airport(&left.plan.arr, &right.plan.arr)
        && left.plan.time == right.plan.time && left.plan.scheduled_time == right.plan.scheduled_time
        && left.plan.flight_task_status == right.plan.flight_task_status
        && left.task_type.category == right.task_type.category && left.task_type.name == right.task_type.name
        && same_optional_aircraft(&left.recovery_aircraft, &right.recovery_aircraft) && left.recovery_time == right.recovery_time
}

fn same_aircraft(left: &Aircraft, right: &Aircraft) -> bool {
    left.same_configuration(right)
        && left.minor_type.connection_time.keys().all(|airport| right.minor_type.connection_time.get_key_value(airport)
            .is_some_and(|(other, _)| same_airport(airport, other)))
        && left.minor_type.route_fly_time.keys().all(|route| right.minor_type.route_fly_time.get_key_value(route)
            .is_some_and(|(other, _)| same_airport(&route.dep, &other.dep) && same_airport(&route.arr, &other.arr)))
}

fn same_airport(left: &Airport, right: &Airport) -> bool {
    left == right && left.airport_type == right.airport_type && left.passenger_transfer_time == right.passenger_transfer_time
        && left.cargo_transfer_time == right.cargo_transfer_time && left.base == right.base
}

fn invalid(message: &str) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复主问题输入或结果 / invalid recovery master input or result: {message}") }
}
