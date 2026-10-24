//! 只读恢复活动域；完整候选身份可回放，不共享任何模型注册态。 / Read-only recovery activity domains with replayable complete candidate identities and no shared model registration state.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;
use time::Duration;
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::domain::bunch_compilation::BunchEntry;
use crate::framework::demo4::domain::bunch_compilation::context::{
    BunchCompilationContext, RecoveryBunchResolver, RecoveryMasterConfiguration, RecoverySchedule,
};
use crate::framework::demo4::domain::bunch_compilation::rules::{RecoveryRuleRow, passenger_rows};
use crate::framework::demo4::domain::bunch_compilation::fleet_balance::{RecoveryFleetBalance, RecoveryFleetCheckpoint};
use crate::framework::demo4::domain::bunch_compilation::operating_costs::RecoveryOperatingCosts;
use crate::framework::demo4::domain::bunch_compilation::restrictions::RecoveryRestrictions;
use crate::framework::demo4::domain::bunch_compilation::flow_control::RecoveryFlowControls;
use crate::framework::demo4::domain::passenger::model::Passenger;
use crate::framework::demo4::domain::passenger::recovery::{RecoveryPassengerConfiguration, RecoveryPassengerRecovery};
use crate::framework::demo4::domain::rule::model::{FlowControl, Link, Lock, Restriction};
use crate::framework::demo4::domain::rule::service::{BunchRecoveryRules, CostCalculator, DefaultRecoveryTransitionPolicy, FeasibilityJudger, RecoveryCostPolicy, RecoveryTransitionPolicy};
use crate::framework::demo4::domain::task::model::{Aircraft, AircraftUsability, FlightTaskBunch, FlightTaskRecovery};
use super::recovery::{RecoveryActivityCandidate, RecoveryBunchCandidate};

/// 单机的有限恢复活动域与已执行前序。 / Finite per-aircraft recovery activity domain and executed predecessor.
#[derive(Debug, Clone)]
pub struct RecoveryAircraftDomain {
    /// 执行飞机的完整配置。 / Complete executor aircraft configuration.
    pub aircraft: Aircraft,
    /// 初始位置、可用时间和剩余飞行额度。 / Initial location, availability and remaining flight allowances.
    pub usability: AircraftUsability,
    /// 已执行的前序活动，不作为待选列重复注册。 / Executed predecessor, never re-registered as a selectable activity.
    pub previous: Option<FlightTaskRecovery>,
    /// 每个候选 ID 对应唯一的实际恢复记录。 / Each candidate ID identifies a unique actual recovery record.
    pub candidates: Vec<RecoveryActivityCandidate>,
}

/// 可在定价策略与独立 context 间共享的不可变业务定义。 / Immutable business definition shared by pricing policies and independent contexts.
pub struct RecoveryProblem<C> {
    configuration: RecoveryMasterConfiguration,
    domains: Vec<RecoveryAircraftDomain>,
    links: Vec<Link>,
    cost: C,
    projection: BunchCompilationContext,
    passengers: Vec<Passenger>,
    passenger_recovery: Option<RecoveryPassengerRecovery>,
    rows: Vec<RecoveryRuleRow>,
    fleet: Option<RecoveryFleetBalance>,
    locked: BTreeMap<String, BTreeSet<String>>,
    departure_locks: BTreeMap<String, Lock>,
    cancellation_costs: Vec<Option<f64>>,
    operating_costs: RecoveryOperatingCosts,
    restrictions: RecoveryRestrictions,
    flow_controls: RecoveryFlowControls,
    transition: Arc<dyn RecoveryTransitionPolicy>,
}

impl<C> std::fmt::Debug for RecoveryProblem<C> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("RecoveryProblem").field("configuration", &self.configuration)
            .field("domains", &self.domains).field("links", &self.links).finish_non_exhaustive()
    }
}

impl<C: RecoveryCostPolicy> RecoveryProblem<C> {
    /// 校验有限域及全部原始任务引用，不预枚举路线，也不启动任何求解器。 / Validate finite domains and all original task references without enumerating routes or starting a solver.
    pub fn new(
        configuration: RecoveryMasterConfiguration,
        domains: Vec<RecoveryAircraftDomain>,
        links: Vec<Link>,
        cost: C,
    ) -> GanttResult<Self> {
        let projection = BunchCompilationContext::new(configuration.clone())?;
        let expected = configuration.aircraft.iter().map(|aircraft| &aircraft.reg_no.0).collect::<HashSet<_>>();
        let actual = domains.iter().map(|domain| &domain.aircraft.reg_no.0).collect::<HashSet<_>>();
        if domains.len() != expected.len() || actual != expected {
            return Err(invalid("recovery domains must cover each declared aircraft exactly once"));
        }
        let tasks = configuration.tasks.iter().map(|task| task.plan.id.as_str()).collect::<HashSet<_>>();
        let mut previous = HashSet::new();
        for activity in domains.iter().filter_map(|domain| domain.previous.as_ref()) {
            let id = activity.original().plan.id.as_str();
            if id.is_empty() || tasks.contains(id) || !previous.insert(id) {
                return Err(invalid("empty, repeated or still-selectable executed predecessor"));
            }
        }
        let mut pairs = HashSet::new();
        for link in &links {
            if link.from_task == link.to_task || !tasks.contains(link.to_task.as_str())
                || !(tasks.contains(link.from_task.as_str()) || previous.contains(link.from_task.as_str()))
                || link.min_connection_time() < Duration::ZERO
                || link.split_cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0 || !tasks.contains(link.from_task.as_str()))
                || !pairs.insert((&link.from_task, &link.to_task))
            {
                return Err(invalid("unknown, repeated or invalid recovery link"));
            }
        }
        for domain in &domains {
            let mut ids = HashSet::new();
            let idle = FlightTaskBunch::new("idle".into(), domain.aircraft.clone(), vec![]).map_err(invalid)?;
            projection.entry(&RecoveryBunchCandidate { bunch: idle.clone(), cost: 0.0 }, 0, 0)?;
            CostCalculator.calculate_bunch(&idle, &cost).map_err(invalid)?;
            FeasibilityJudger.judge_bunch(&idle, &BunchRecoveryRules {
                usability: &domain.usability, previous: domain.previous.as_ref(), links: &links,
            }).map_err(invalid)?;
            for candidate in &domain.candidates {
                if candidate.id.is_empty() || !ids.insert(&candidate.id) {
                    return Err(invalid("empty or duplicate recovery activity candidate ID"));
                }
                let bunch = FlightTaskBunch::new("validate".into(), domain.aircraft.clone(), vec![candidate.recovery.clone()]).map_err(invalid)?;
                projection.entry(&RecoveryBunchCandidate { bunch, cost: 0.0 }, 0, 0)?;
                let activity_cost = cost.activity_cost(&candidate.recovery).map_err(invalid)?;
                if !activity_cost.is_finite() { return Err(invalid("non-finite recovery activity cost")); }
            }
        }
        let cancellation_costs = configuration.cancellation_costs.clone();
        let operating_costs = RecoveryOperatingCosts::new(&links);
        Ok(Self { configuration, domains, links, cost, projection, passengers: vec![], passenger_recovery: None,
            rows: vec![], fleet: None, locked: BTreeMap::new(), departure_locks: BTreeMap::new(), cancellation_costs, operating_costs,
            restrictions: Default::default(), flow_controls: Default::default(), transition: Arc::new(DefaultRecoveryTransitionPolicy) })
    }

    /// 替换无模型状态的转移规则和连接时间策略；全部生成及独立生产回放共用该策略。 / Replace model-free transition and turnaround rules, shared by all generation and independent production replay.
    pub fn with_transition_policy(mut self, policy: Arc<dyn RecoveryTransitionPolicy>) -> Self {
        self.transition = policy;
        self
    }

    /// 添加硬旅客行程；非法引用失败，合法但无法满足的行程保留为模型不可行。 / Add hard passenger itineraries; reject invalid references but retain valid unsatisfiable itineraries as model infeasibility.
    pub fn with_passengers(mut self, passengers: Vec<Passenger>) -> GanttResult<Self> {
        let tasks = self.configuration.tasks.iter().map(|task| task.plan.id.as_str()).collect();
        let variants = self.domains.iter().flat_map(|domain| domain.candidates.iter().map(|candidate| candidate.recovery.clone())).collect::<Vec<_>>();
        self.rows = passenger_rows(&tasks, &variants, &passengers)?;
        self.passengers = passengers;
        self.passenger_recovery = None;
        Ok(self)
    }

    /// 以显式航段政策替换硬旅客模式，允许有界取消和舱位改签。 / Replace hard-passenger mode with explicit leg policies allowing bounded cancellation and cabin changes.
    pub fn with_passenger_recovery(mut self, configuration: RecoveryPassengerConfiguration) -> GanttResult<Self> {
        let variants = self.domains.iter().flat_map(|domain| domain.candidates.iter().map(|candidate| candidate.recovery.clone())).collect::<Vec<_>>();
        self.passenger_recovery = Some(RecoveryPassengerRecovery::new(configuration, &self.configuration.tasks, &variants)?);
        self.passengers.clear();
        self.rows.clear();
        Ok(self)
    }

    /// 以硬下界或有界缺口费用配置终点机队，不改变旅客规则。 / Configure terminal fleet with hard minima or bounded shortage costs without changing passenger rules.
    pub fn with_fleet_balance(mut self, checkpoints: Vec<RecoveryFleetCheckpoint>) -> GanttResult<Self> {
        self.fleet = Some(RecoveryFleetBalance::new(checkpoints, &self.domains)?);
        Ok(self)
    }

    /// 配置有限非负的单机闲置费用；空束不表示飞机已使用。 / Configure finite nonnegative per-aircraft idle costs; empty bunches do not count as aircraft use.
    pub fn with_aircraft_idle_costs(mut self, costs: BTreeMap<String, f64>) -> GanttResult<Self> {
        let aircraft = self.configuration.aircraft.iter().map(|aircraft| aircraft.reg_no.0.as_str()).collect();
        self.operating_costs.configure_idle(costs, &aircraft)?;
        Ok(self)
    }

    /// 添加已验证的实际分配限制；硬限制贯穿生成和回放，软限制费用进入同一主模型。 / Add validated actual-assignment restrictions; hard restrictions persist through generation/replay and soft costs enter the same master.
    pub fn with_restrictions(mut self, restrictions: Vec<Restriction>) -> GanttResult<Self> {
        self.restrictions = RecoveryRestrictions::new(restrictions, &self.configuration.tasks, &self.domains)?;
        Ok(self)
    }

    /// 声明有限容量窗及实际事件使用量，不新增全局时空模型或求解器。 / Declare finite capacity windows and actual event usage without a global spacetime model or another solver.
    pub fn with_flow_controls(mut self, controls: Vec<FlowControl>) -> GanttResult<Self> {
        self.flow_controls = RecoveryFlowControls::new(controls, &self.configuration.tasks, &self.domains)?;
        Ok(self)
    }

    /// 从完整原始束推导期望终点；原始束仅定义基准，不必已是恢复域中的可行列。 / Derive desired terminals from full original bunches; these define a baseline and need not be feasible columns in the recovery domain.
    pub fn with_original_fleet_balance(
        mut self,
        originals: &[RecoveryBunchCandidate],
        shortage_cost: Option<f64>,
    ) -> GanttResult<Self> {
        for original in originals { self.entry(original)?; }
        self.fleet = Some(RecoveryFleetBalance::from_original(&self.domains, originals, shortage_cost)?);
        Ok(self)
    }

    /// 锁定任务到实际飞机；同时禁止取消和其他飞机执行，不只是初始列偏好。 / Lock tasks to actual aircraft, forbidding both cancellation and other executors rather than merely preferring initial columns.
    pub fn with_locked_tasks(mut self, locked: BTreeMap<String, Vec<String>>) -> GanttResult<Self> {
        let mut seen = HashSet::new();
        let mut normalized = BTreeMap::new();
        for (executor, tasks) in locked {
            if !self.configuration.aircraft.iter().any(|aircraft| aircraft.reg_no.0 == executor) { return Err(invalid("unknown locked executor")); }
            let mut ids = BTreeSet::new();
            for task in tasks {
                if !self.configuration.tasks.iter().any(|original| original.plan.id == task) || !seen.insert(task.clone()) {
                    return Err(invalid("unknown or multiply locked task"));
                }
                ids.insert(task);
            }
            normalized.insert(executor, ids);
        }
        self.locked = normalized;
        self.refresh_cancellation_locks()?;
        Ok(self)
    }

    /// 替换实际出发时刻锁定；无匹配候选保留为合法不可行域，不构造或猜测新的时间。 / Replace actual departure locks; missing matching candidates remain a valid infeasible domain without constructing or guessing times.
    pub fn with_departure_locks(mut self, locks: Vec<Lock>) -> GanttResult<Self> {
        let mut ids = HashSet::new();
        let mut normalized = BTreeMap::new();
        for lock in locks {
            if lock.id.is_empty() || !ids.insert(lock.id.clone())
                || !self.configuration.tasks.iter().any(|task| task.plan.id == lock.task_id)
                || normalized.contains_key(&lock.task_id)
            { return Err(invalid("empty, unknown or duplicate departure lock")); }
            normalized.insert(lock.task_id.clone(), lock);
        }
        self.departure_locks = normalized;
        self.refresh_cancellation_locks()?;
        Ok(self)
    }

    /// 查询原任务锁定的实际开始时刻。 / Query the locked actual start time of an original task.
    pub fn locked_departure(&self, task_id: &str) -> Option<time::OffsetDateTime> {
        self.departure_locks.get(task_id).map(|lock| lock.departure_time)
    }

    fn refresh_cancellation_locks(&mut self) -> GanttResult<()> {
        self.configuration.cancellation_costs = self.cancellation_costs.clone();
        for (index, task) in self.configuration.tasks.iter().enumerate() {
            if self.departure_locks.contains_key(&task.plan.id) || self.locked.values().any(|tasks| tasks.contains(&task.plan.id)) {
                self.configuration.cancellation_costs[index] = None;
            }
        }
        self.projection = BunchCompilationContext::new(self.configuration.clone())?;
        Ok(())
    }

    /// 检查不可变锁定域和硬限制；完整记录及其他航空可行性仍独立校验。 / Check immutable locks and hard restrictions; full records and other aviation feasibility still require independent validation.
    pub(crate) fn accepts_bunch(&self, bunch: &FlightTaskBunch) -> bool {
        FeasibilityJudger.judge_restrictions(bunch, self.restrictions.definitions()).feasible
            && bunch.activities().iter().all(|activity| self.locked_departure(&activity.original().plan.id)
                .is_none_or(|time| time == activity.time().0))
            && self.locked.iter().all(|(executor, tasks)| {
            if executor == &bunch.aircraft().reg_no.0 { tasks.iter().all(|task| bunch.contains(task)) }
            else { tasks.iter().all(|task| !bunch.contains(task)) }
        })
    }

    /// 包含旅客扩展的完整行名，零对偶不可省略。 / Complete row names including passenger extensions; zero duals cannot be omitted.
    pub fn row_names(&self) -> std::collections::BTreeSet<String> {
        let mut names = self.configuration.tasks.iter().enumerate().map(|(index, _)| format!("task_compilation_{index}")).collect::<std::collections::BTreeSet<_>>();
        names.extend(self.configuration.aircraft.iter().map(|aircraft| format!("executor_compilation_{}", aircraft.reg_no.0)));
        names.extend(self.rows.iter().map(|row| row.name().to_string()));
        if let Some(fleet) = &self.fleet { names.extend(fleet.row_names()); }
        if let Some(passengers) = &self.passenger_recovery { names.extend(passengers.row_names()); }
        names.extend(self.operating_costs.row_names());
        names.extend(self.restrictions.row_names());
        names.extend(self.flow_controls.row_names());
        names
    }

    /// 只读主问题定义。 / Read-only master definition.
    pub fn configuration(&self) -> &RecoveryMasterConfiguration { &self.configuration }

    /// 完整有限飞机域。 / Complete finite aircraft domains.
    pub fn domains(&self) -> &[RecoveryAircraftDomain] { &self.domains }

    /// 硬相邻或付费拆分链接规则。 / Hard-adjacency or penalized-split link rules.
    pub fn links(&self) -> &[Link] { &self.links }

    /// 完整费用策略。 / Complete cost policy.
    pub fn cost(&self) -> &C { &self.cost }

    /// 当前只读转移策略，不包含模型或分支注册状态。 / Current read-only transition policy, without model or branch registration state.
    pub fn transition_policy(&self) -> &dyn RecoveryTransitionPolicy { self.transition.as_ref() }

    /// 将完整候选投影为稳定 Gantt 身份，不接触注册态。 / Project a complete candidate into stable Gantt identity without touching registration state.
    pub fn entry(&self, candidate: &RecoveryBunchCandidate) -> GanttResult<BunchEntry> {
        self.projection.entry(candidate, 0, 0)
    }

    /// 全部声明主问题行的完整列系数，包含显式零；与受管主问题的行身份一致。 / Complete column coefficients for every declared master row, including explicit zeros and matching managed-master row identities.
    pub fn column_coefficients(&self, candidate: &RecoveryBunchCandidate) -> GanttResult<BTreeMap<String, f64>> {
        let entry = self.entry(candidate)?;
        let mut rows = self.configuration.tasks.iter().enumerate().map(|(index, _)| {
            (format!("task_compilation_{index}"), if entry.task_indices.contains(&index) { 1.0 } else { 0.0 })
        }).collect::<BTreeMap<_, _>>();
        rows.extend(self.configuration.aircraft.iter().map(|aircraft| {
            (format!("executor_compilation_{}", aircraft.reg_no.0), if candidate.bunch.aircraft() == aircraft { 1.0 } else { 0.0 })
        }));
        rows.extend(self.rows.iter().map(|row| (row.name().to_string(), row.coefficient(candidate))));
        if let Some(fleet) = &self.fleet { rows.extend(fleet.column_coefficients(candidate)?); }
        if let Some(passengers) = &self.passenger_recovery { rows.extend(passengers.column_coefficients(candidate)); }
        rows.extend(self.operating_costs.column_coefficients(candidate));
        rows.extend(self.restrictions.column_coefficients(candidate));
        if !self.flow_controls.controls().is_empty() { rows.extend(self.flow_controls.column_coefficients(candidate)?); }
        Ok(rows)
    }

    /// 回放活动候选身份，保留原任务、实际飞机、机场及逐活动时间。 / Replay activity candidate identities, retaining original tasks, actual aircraft, airports and activity times.
    pub fn resolve_candidate(&self, entry: &BunchEntry) -> GanttResult<RecoveryBunchCandidate> {
        let domain = self.domains.iter().find(|domain| domain.aircraft.reg_no.0 == entry.executor_id)
            .ok_or_else(|| invalid("unknown recovery executor"))?;
        let executor = &domain.aircraft.reg_no.0;
        let prefix = format!("{}:{}", executor.len(), executor);
        let encoded = entry.id.as_str().strip_prefix(&prefix).ok_or_else(|| invalid("invalid aircraft candidate namespace"))?;
        let (length, encoded) = encoded.split_once(':').ok_or_else(|| invalid("missing bunch identity length"))?;
        if length != encoded.len().to_string() { return Err(invalid("noncanonical bunch identity length")); }
        let mut rest = encoded.strip_prefix("idle").ok_or_else(|| invalid("unknown bunch identity prefix"))?;
        let mut id = "idle".to_string();
        let mut activities = Vec::new();
        while !rest.is_empty() {
            let suffix = rest.strip_prefix(':').ok_or_else(|| invalid("invalid activity identity separator"))?;
            let (length, suffix) = suffix.split_once(':').ok_or_else(|| invalid("missing activity identity length"))?;
            let count = length.parse::<usize>().map_err(invalid)?;
            if count == 0 || length != count.to_string() { return Err(invalid("noncanonical activity identity length")); }
            let key = suffix.get(..count).ok_or_else(|| invalid("invalid UTF-8 activity identity length"))?;
            rest = suffix.get(count..).ok_or_else(|| invalid("invalid activity identity tail"))?;
            let candidate = domain.candidates.iter().find(|candidate| candidate.id == key).ok_or_else(|| invalid("unknown recovery activity candidate"))?;
            id.push_str(&format!(":{}:{}", key.len(), key));
            activities.push(candidate.recovery.clone());
        }
        let bunch = FlightTaskBunch::new(id, domain.aircraft.clone(), activities).map_err(invalid)?;
        if !self.accepts_bunch(&bunch) { return Err(invalid("recovery candidate violates locked task allocation or a hard restriction")); }
        let feasible = FeasibilityJudger.judge_bunch_with_policy(&bunch, &BunchRecoveryRules {
            usability: &domain.usability, previous: domain.previous.as_ref(), links: &self.links,
        }, self.transition_policy()).map_err(invalid)?;
        if !feasible.feasible { return Err(invalid(format!("infeasible recovery candidate: {:?}", feasible.violations))); }
        let cost = CostCalculator.calculate_bunch(&bunch, &self.cost).map_err(invalid)?;
        let candidate = RecoveryBunchCandidate { bunch, cost };
        let canonical = self.entry(&candidate)?;
        if canonical.id != entry.id || canonical.executor_id != entry.executor_id || canonical.task_indices != entry.task_indices
            || canonical.cost != entry.cost || canonical.slot_index != entry.slot_index
        { return Err(invalid("candidate projection differs from immutable recovery domain")); }
        Ok(candidate)
    }
}

impl<C: RecoveryCostPolicy + Send + Sync> RecoveryBunchResolver for RecoveryProblem<C> {
    fn resolve(&self, entry: &BunchEntry) -> GanttResult<RecoveryBunchCandidate> { self.resolve_candidate(entry) }

    fn master_rows(&self) -> &[RecoveryRuleRow] { &self.rows }

    fn fleet_balance(&self) -> Option<&RecoveryFleetBalance> { self.fleet.as_ref() }

    fn passenger_recovery(&self) -> Option<&RecoveryPassengerRecovery> { self.passenger_recovery.as_ref() }

    fn operating_costs(&self) -> Option<&RecoveryOperatingCosts> { Some(&self.operating_costs) }

    fn restrictions(&self) -> Option<&RecoveryRestrictions> { Some(&self.restrictions) }

    fn flow_controls(&self) -> Option<&RecoveryFlowControls> { Some(&self.flow_controls) }

    fn validate_schedule(&self, schedule: &RecoverySchedule) -> GanttResult<()> {
        for candidate in &schedule.selected { self.resolve_candidate(&self.entry(candidate)?)?; }
        let canceled = schedule.canceled.iter().map(|task| task.plan.id.as_str()).collect();
        self.operating_costs.validate_results(&schedule.selected, &canceled, &schedule.operating_costs)?;
        self.restrictions.validate_results(&schedule.selected, &schedule.restrictions)?;
        self.flow_controls.validate_results(&schedule.selected, &schedule.flow_controls)?;
        for (executor, tasks) in &self.locked {
            if tasks.iter().any(|task| !schedule.selected.iter().any(|candidate| &candidate.bunch.aircraft().reg_no.0 == executor && candidate.bunch.contains(task))) {
                return Err(invalid("locked task is not executed by its required aircraft"));
            }
        }
        for lock in self.departure_locks.values() {
            if !schedule.selected.iter().any(|candidate| candidate.bunch.recovery(&lock.task_id)
                .is_some_and(|activity| activity.time().0 == lock.departure_time))
            { return Err(invalid("departure-locked task is absent or executed at a different actual time")); }
        }
        let bunches = schedule.selected.iter().map(|candidate| candidate.bunch.clone()).collect::<Vec<_>>();
        let checked = FeasibilityJudger.judge_passengers(&bunches, &self.passengers).map_err(invalid)?;
        if !checked.feasible { return Err(invalid(format!("invalid passenger incumbent: {:?}", checked.violations))); }
        if let Some(passengers) = &self.passenger_recovery {
            passengers.validate_results(&schedule.selected, &schedule.passengers)?;
        } else if !schedule.passengers.is_empty() { return Err(invalid("unexpected passenger extraction")); }
        if let Some(fleet) = &self.fleet {
            let modeled = schedule.fleet_balance.iter().map(|result| result.modeled_shortage.map(|value| value as f64)).collect::<Vec<_>>();
            if fleet.evaluate(&schedule.selected, &modeled)? != schedule.fleet_balance {
                return Err(invalid("fleet extraction differs from the complete recovery schedule"));
            }
        } else if !schedule.fleet_balance.is_empty() { return Err(invalid("unexpected fleet extraction")); }
        Ok(())
    }
}

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复活动域 / invalid recovery activity domain: {error}") }
}
