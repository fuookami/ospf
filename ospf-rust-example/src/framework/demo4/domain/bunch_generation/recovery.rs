//! 无损恢复束的网络路径桥接，不拥有或求解主模型。 / Network-path bridge for lossless recovery bunches, without owning or solving the master model.

use std::cmp::Ordering;
use std::collections::HashSet;
use ospf_rust_framework::model::ColumnGenerationScope;
use ospf_rust_framework_gantt_network::{NetworkBunchGenerationContext, NetworkBunchGenerationResult};
use ospf_rust_framework_network_scheduling::{NetworkSchedulingError, Result};
use ospf_rust_framework_network_scheduling::domain::path::{PathPolicy, PathProblem, PathSearchBudget, PathTrace};
use ospf_rust_framework_network_scheduling::infrastructure::{NetworkArc, NetworkGraph, NetworkNode};
use crate::framework::demo4::domain::rule::service::{BunchRecoveryRules, CostCalculator, DefaultRecoveryTransitionPolicy, FeasibilityJudger, RecoveryCostPolicy, RecoveryTransitionPolicy};
use crate::framework::demo4::domain::task::model::{Aircraft, FlightTaskBunch, FlightTaskRecovery};

/// 同一原任务可有多个恢复候选；稳定候选 ID 必须唯一。 / Multiple recovery candidates may refer to one original task; stable candidate IDs must be unique.
#[derive(Debug, Clone)]
pub struct RecoveryActivityCandidate {
    /// 稳定的恢复候选标识 / Stable recovery candidate identifier.
    pub id: String,
    /// 无损恢复分配 / Lossless recovery assignment.
    pub recovery: FlightTaskRecovery,
}

/// 带完整费用的实际恢复束 / Actual recovery bunch with complete cost.
#[derive(Debug, Clone)]
pub struct RecoveryBunchCandidate {
    /// 原任务、飞机、航线及逐活动实际时间 / Original tasks, aircraft, route and actual activity times.
    pub bunch: FlightTaskBunch,
    /// 包含终点贡献的完整费用 / Complete cost including terminal contributions.
    pub cost: f64,
}

/// 每次搜索独立的业务域与策略 / Independent per-search business domain and policy.
pub struct RecoveryPathRequest<'a, C> {
    /// 本域执行飞机 / Executor aircraft of this domain.
    pub aircraft: &'a Aircraft,
    /// 有限恢复候选集 / Finite recovery candidate set.
    pub candidates: &'a [RecoveryActivityCandidate],
    /// 实际恢复记录校验规则 / Actual recovery record validation rules.
    pub rules: BunchRecoveryRules<'a>,
    /// 完整费用策略，不要求可加 / Complete cost policy, not assumed additive.
    pub cost: &'a C,
    /// 上游请求作用域；不代替 LP 对偶证书 / Upstream request scope, not a replacement for an LP dual certificate.
    pub scope: ColumnGenerationScope,
    /// 只截取返回结果，不提前结束搜索 / Limit returned results only, without ending search early.
    pub maximum_results: Option<usize>,
}

/// 复用 Network 搜索和 Gantt 桥接的恢复束生成上下文 / Recovery bunch generation context reusing Network search and the Gantt bridge.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecoveryBunchGenerationContext;

impl RecoveryBunchGenerationContext {
    /// 返回完整业务记录、前驱 trace 和真实完成度；本入口不认证无改进或任何下界。 / Return complete records, predecessor traces and actual completion; this entry certifies neither no-improvement nor any bound.
    pub fn generate<C: RecoveryCostPolicy>(
        &self,
        request: &RecoveryPathRequest<'_, C>,
        budget: &PathSearchBudget<'_>,
    ) -> Result<NetworkBunchGenerationResult<RecoveryBunchCandidate, RecoveryBunchCandidate>> {
        self.generate_filtered(request, budget, |_| true)
    }

    /// 领域终止谓词共用完整路径校验，不在搜索后丢弃锁定规则。 / Apply the domain terminal predicate through full-path validation rather than discarding locking rules after search.
    pub fn generate_filtered<C, F>(
        &self,
        request: &RecoveryPathRequest<'_, C>,
        budget: &PathSearchBudget<'_>,
        accepts: F,
    ) -> Result<NetworkBunchGenerationResult<RecoveryBunchCandidate, RecoveryBunchCandidate>>
    where
        C: RecoveryCostPolicy,
        F: Fn(&FlightTaskBunch) -> bool,
    {
        self.generate_with_policy(request, budget, accepts, &DefaultRecoveryTransitionPolicy)
    }

    /// 使用同一只读转移策略执行完整终止及独立 trace 校验，不新增搜索或求解内核。 / Use one read-only transition policy for terminal and independent trace checks without another search or solve kernel.
    pub fn generate_with_policy<C, F>(
        &self,
        request: &RecoveryPathRequest<'_, C>,
        budget: &PathSearchBudget<'_>,
        accepts: F,
        policy: &dyn RecoveryTransitionPolicy,
    ) -> Result<NetworkBunchGenerationResult<RecoveryBunchCandidate, RecoveryBunchCandidate>>
    where
        C: RecoveryCostPolicy,
        F: Fn(&FlightTaskBunch) -> bool,
    {
        let mut ids = HashSet::new();
        if request.aircraft.reg_no.0.is_empty() || request.scope.groups != [request.aircraft.reg_no.0.clone()].into()
            || !request.scope.executor_slots.is_empty()
        {
            return Err(NetworkSchedulingError::validation("recovery path scope must describe exactly one unslotted aircraft"));
        }
        let mut nodes = vec![NetworkNode::new("source", None), NetworkNode::new("sink", None)];
        let mut arcs = vec![NetworkArc::new("idle", "source", "sink", ())];
        for candidate in request.candidates {
            if candidate.id.is_empty() || !ids.insert(candidate.id.as_str()) {
                return Err(NetworkSchedulingError::validation(format!("empty or duplicate recovery candidate {}", candidate.id)));
            }
            if !candidate.recovery.aircraft().same_configuration(request.aircraft) {
                return Err(NetworkSchedulingError::validation(format!("recovery aircraft mismatch {}", candidate.id)));
            }
            let id = format!("activity:{}", candidate.id);
            nodes.push(NetworkNode::new(id.clone(), Some(candidate.clone())));
            arcs.push(NetworkArc::new(format!("start:{}", candidate.id), "source", id.clone(), ()));
            arcs.push(NetworkArc::new(format!("finish:{}", candidate.id), id, "sink", ()));
        }
        for from in request.candidates {
            for to in request.candidates {
                if from.id != to.id && from.recovery.route().arr == to.recovery.route().dep
                    && from.recovery.time().1 <= to.recovery.time().0
                {
                    // 序号而非用户字符串拼接形成弧身份，避免分隔符碰撞。 / Use ordinal arc identities to avoid delimiter collisions in user IDs.
                    arcs.push(NetworkArc::new(format!("connection:{}", arcs.len()),
                        format!("activity:{}", from.id), format!("activity:{}", to.id), ()));
                }
            }
        }
        let graph = NetworkGraph::new(nodes, arcs)?;
        let mut problem = PathProblem::new(&graph, RecoveryPathPolicy { request, accepts: &accepts, transition: policy });
        problem.starts = vec!["source".into()];
        problem.terminals = ["sink".into()].into();
        problem.options.maximum_results = request.maximum_results;
        let mut result = NetworkBunchGenerationContext::default().generate(
            &problem,
            budget,
            request.scope.clone(),
            |path, _| Ok(Some(path.state.clone())),
            |_| Ok(false),
        )?;
        // 此层只证明路径域搜索完成，不从原费用推导 reduced cost 证明。 / This layer reports path-domain completion only, never deriving reduced-cost proof from original costs.
        result.batch.no_improving_columns = None;
        Ok(result)
    }
}

struct RecoveryPathPolicy<'a, 'b, C, F> {
    request: &'a RecoveryPathRequest<'b, C>,
    accepts: &'a F,
    transition: &'a dyn RecoveryTransitionPolicy,
}

impl<C, F> PathPolicy<Option<RecoveryActivityCandidate>, (), RecoveryBunchCandidate>
    for RecoveryPathPolicy<'_, '_, C, F>
where
    C: RecoveryCostPolicy,
    F: Fn(&FlightTaskBunch) -> bool,
{
    fn start(&self, _: &NetworkNode<Option<RecoveryActivityCandidate>>) -> Result<Option<RecoveryBunchCandidate>> {
        let bunch = FlightTaskBunch::new("idle".into(), self.request.aircraft.clone(), Vec::new())
            .map_err(NetworkSchedulingError::dependency)?;
        Ok(Some(RecoveryBunchCandidate { bunch, cost: 0.0 }))
    }

    fn extend(
        &self,
        state: &RecoveryBunchCandidate,
        _: &NetworkArc<()>,
        destination: &NetworkNode<Option<RecoveryActivityCandidate>>,
    ) -> Result<Option<RecoveryBunchCandidate>> {
        let Some(candidate) = &destination.payload else { return Ok(Some(state.clone())); };
        if state.bunch.contains(&candidate.recovery.original().plan.id) { return Ok(None); }
        let mut activities = state.bunch.activities().to_vec();
        activities.push(candidate.recovery.clone());
        let id = format!("{}:{}", state.bunch.id(), candidate.id.len());
        let id = format!("{id}:{}", candidate.id);
        let bunch = FlightTaskBunch::new(id, self.request.aircraft.clone(), activities)
            .map_err(NetworkSchedulingError::dependency)?;
        Ok(Some(RecoveryBunchCandidate { bunch, cost: 0.0 }))
    }

    fn finish(&self, trace: &PathTrace<'_, Option<RecoveryActivityCandidate>, (), RecoveryBunchCandidate>) -> Result<Option<RecoveryBunchCandidate>> {
        let Some(state) = trace.states.last() else { return Ok(None); };
        if !(self.accepts)(&state.bunch) || !FeasibilityJudger.judge_bunch_with_policy(&state.bunch, &self.request.rules, self.transition)
            .map_err(NetworkSchedulingError::dependency)?.feasible
        {
            return Ok(None);
        }
        let cost = CostCalculator.calculate_bunch(&state.bunch, self.request.cost)
            .map_err(NetworkSchedulingError::dependency)?;
        Ok(Some(RecoveryBunchCandidate { bunch: state.bunch.clone(), cost }))
    }

    fn validate(
        &self,
        trace: &PathTrace<'_, Option<RecoveryActivityCandidate>, (), RecoveryBunchCandidate>,
        finished: &RecoveryBunchCandidate,
    ) -> Result<bool> {
        let activities = trace.nodes.iter().filter_map(|node| node.payload.as_ref())
            .map(|candidate| candidate.recovery.clone()).collect();
        let reconstructed = FlightTaskBunch::new(finished.bunch.id().into(), self.request.aircraft.clone(), activities)
            .map_err(NetworkSchedulingError::dependency)?;
        let cost = CostCalculator.calculate_bunch(&reconstructed, self.request.cost)
            .map_err(NetworkSchedulingError::dependency)?;
        Ok(cost == finished.cost && reconstructed.task_ids().eq(finished.bunch.task_ids()) && (self.accepts)(&reconstructed)
            && FeasibilityJudger.judge_bunch_with_policy(&reconstructed, &self.request.rules, self.transition)
                .map_err(NetworkSchedulingError::dependency)?.feasible)
    }

    fn compare(&self, left: &RecoveryBunchCandidate, right: &RecoveryBunchCandidate) -> Ordering {
        left.cost.total_cmp(&right.cost).then_with(|| left.bunch.id().cmp(right.bunch.id()))
    }
}
