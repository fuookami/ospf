//! Branch-and-Price 应用编排 / Branch-and-Price application orchestration.
//!
//! 应用层只管理节点、边界、分支和终态；单节点模型装配通过 provider 注入。
//! The application layer manages nodes, bounds, branching, and terminal states;
//! single-node model assembly is injected through a provider.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_core::solver::{
    ModelFingerprint, ProblemStatus, SolveDiagnostics, SolveHandle, SolveProof, SolveReport,
    SolveSolution, SolveStatistics, SolveTrace, TerminationReason,
    require_infeasibility_certificate_for_model,
};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::domain::route_generation::CancellationToken;
use crate::domain::vrp::{
    BranchMask, CustomerId, DefaultArcFeasibilityPolicy, DistanceArcCostCalculator,
    DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator, FixedPlusArcCostPolicy,
    ResourceArc, Route, RouteValidationPolicy, RouteValidator, VehicleTypeId, VrptwInstance,
    VrptwSolution,
};
use crate::error::{NetworkSchedulingError, Result};

mod branch_node_solver;

pub use branch_node_solver::{BranchNodeSolver, BranchNodeSolverConfig, LinearProgrammingSolver};

/// 不可变分支决策 / Immutable branching decision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BranchDecision {
    /// 禁止车辆类型服务客户 / Forbid a vehicle type from serving a customer.
    ForbidVehicleType {
        /// 车辆类型 / Vehicle type.
        vehicle_type_id: VehicleTypeId,
        /// 客户 / Customer.
        customer_id: CustomerId,
        /// 客户节点 / Customer node.
        customer_node_id: crate::infrastructure::NetworkNodeId,
    },
    /// 要求车辆类型服务客户 / Require a vehicle type to serve a customer.
    RequireVehicleType {
        /// 车辆类型 / Vehicle type.
        vehicle_type_id: VehicleTypeId,
        /// 客户 / Customer.
        customer_id: CustomerId,
        /// 客户节点 / Customer node.
        customer_node_id: crate::infrastructure::NetworkNodeId,
    },
    /// 禁止车辆类型使用一条弧 / Forbid a vehicle type from using an arc.
    ForbidArc {
        /// 车辆类型 / Vehicle type.
        vehicle_type_id: VehicleTypeId,
        /// 稳定弧 ID / Stable arc ID.
        arc_id: crate::infrastructure::NetworkArcId,
        /// 弧起点 / Arc origin.
        from: crate::infrastructure::NetworkNodeId,
        /// 弧终点 / Arc destination.
        to: crate::infrastructure::NetworkNodeId,
    },
    /// 要求车辆类型使用一条弧 / Require a vehicle type to use an arc.
    RequireArc {
        /// 车辆类型 / Vehicle type.
        vehicle_type_id: VehicleTypeId,
        /// 稳定弧 ID / Stable arc ID.
        arc_id: crate::infrastructure::NetworkArcId,
        /// 弧起点 / Arc origin.
        from: crate::infrastructure::NetworkNodeId,
        /// 弧终点 / Arc destination.
        to: crate::infrastructure::NetworkNodeId,
    },
}

impl BranchDecision {
    /// 返回互补分支决策 / Return the complementary branch decision.
    pub fn complementary(&self) -> Self {
        match self {
            Self::ForbidVehicleType {
                vehicle_type_id,
                customer_id,
                customer_node_id,
            } => Self::RequireVehicleType {
                vehicle_type_id: vehicle_type_id.clone(),
                customer_id: customer_id.clone(),
                customer_node_id: customer_node_id.clone(),
            },
            Self::RequireVehicleType {
                vehicle_type_id,
                customer_id,
                customer_node_id,
            } => Self::ForbidVehicleType {
                vehicle_type_id: vehicle_type_id.clone(),
                customer_id: customer_id.clone(),
                customer_node_id: customer_node_id.clone(),
            },
            Self::ForbidArc {
                vehicle_type_id,
                arc_id,
                from,
                to,
            } => Self::RequireArc {
                vehicle_type_id: vehicle_type_id.clone(),
                arc_id: arc_id.clone(),
                from: from.clone(),
                to: to.clone(),
            },
            Self::RequireArc {
                vehicle_type_id,
                arc_id,
                from,
                to,
            } => Self::ForbidArc {
                vehicle_type_id: vehicle_type_id.clone(),
                arc_id: arc_id.clone(),
                from: from.clone(),
                to: to.clone(),
            },
        }
    }
}

/// 从根到当前节点的不可变分支路径 / Immutable branch path from root to a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchPath {
    /// 起始仓库 / Start depot.
    pub start_depot: crate::infrastructure::NetworkNodeId,
    /// 结束仓库 / End depot.
    pub end_depot: crate::infrastructure::NetworkNodeId,
    /// 累积决策 / Accumulated decisions.
    pub decisions: Vec<BranchDecision>,
    /// 由决策累积得到的遮罩 / Mask accumulated from decisions.
    pub mask: BranchMask<VehicleTypeId>,
}

impl BranchPath {
    /// 创建根路径 / Create a root path.
    pub fn root(
        start_depot: crate::infrastructure::NetworkNodeId,
        end_depot: crate::infrastructure::NetworkNodeId,
    ) -> Result<Self> {
        let mask = BranchMask::empty(start_depot.clone(), end_depot.clone())?;
        Ok(Self {
            start_depot,
            end_depot,
            decisions: Vec::new(),
            mask,
        })
    }

    /// 添加一个决策并重新构造不可变遮罩 / Add one decision and rebuild the immutable mask.
    pub fn with_decision(&self, decision: BranchDecision) -> Result<Self> {
        let mut decisions = self.decisions.clone();
        decisions.push(decision);
        let mask = mask_from_decisions(&self.start_depot, &self.end_depot, &decisions)?;
        Ok(Self {
            start_depot: self.start_depot.clone(),
            end_depot: self.end_depot.clone(),
            decisions,
            mask,
        })
    }
}

/// 分支节点状态 / Branch-node status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNodeStatus {
    /// 等待求解 / Pending.
    Pending,
    /// 已求解 / Solved.
    Solved,
    /// 已剪枝 / Pruned.
    Pruned,
    /// 当前节点不可行 / Infeasible.
    Infeasible,
}

/// 分支树节点 / Branch-tree node.
#[derive(Debug, Clone)]
pub struct BranchNode {
    /// 节点 ID / Node ID.
    pub id: usize,
    /// 父节点 ID / Parent node ID.
    pub parent_id: Option<usize>,
    /// 节点深度 / Node depth.
    pub depth: usize,
    /// 分支路径 / Branch path.
    pub path: BranchPath,
    /// 继承的父节点有效下界 / Inherited parent effective lower bound.
    pub inherited_lower_bound: f64,
    /// 当前有效下界 / Current effective lower bound.
    pub lower_bound: f64,
    /// 节点状态 / Node status.
    pub status: BranchNodeStatus,
}

impl BranchNode {
    /// 创建根节点 / Create the root node.
    pub fn root(
        start_depot: crate::infrastructure::NetworkNodeId,
        end_depot: crate::infrastructure::NetworkNodeId,
    ) -> Result<Self> {
        let path = BranchPath::root(start_depot, end_depot)?;
        Ok(Self {
            id: 0,
            parent_id: None,
            depth: 0,
            path,
            inherited_lower_bound: f64::NEG_INFINITY,
            lower_bound: f64::NEG_INFINITY,
            status: BranchNodeStatus::Pending,
        })
    }

    /// 创建继承父节点下界的子节点 / Create a child inheriting the parent's effective lower bound.
    pub fn child(&self, id: usize, decision: BranchDecision) -> Result<Self> {
        let path = self.path.with_decision(decision)?;
        Ok(Self {
            id,
            parent_id: Some(self.id),
            depth: self.depth + 1,
            path,
            inherited_lower_bound: self.lower_bound,
            lower_bound: self.lower_bound,
            status: BranchNodeStatus::Pending,
        })
    }

    /// 标记节点已求解 / Mark a node solved.
    pub fn solve(&mut self, lower_bound: f64) {
        self.lower_bound = lower_bound;
        self.status = BranchNodeStatus::Solved;
    }

    /// 标记节点不可行 / Mark a node infeasible.
    pub fn mark_infeasible(&mut self) {
        self.status = BranchNodeStatus::Infeasible;
    }

    /// 标记节点剪枝 / Mark a node pruned.
    pub fn prune(&mut self) {
        self.status = BranchNodeStatus::Pruned;
    }

    /// 判断节点是否还能改善 incumbent / Check whether this node can improve the incumbent.
    pub fn can_improve(&self, incumbent: Option<f64>, tolerance: f64) -> bool {
        incumbent.is_none_or(|value| self.lower_bound < value - tolerance)
    }
}

/// 单节点求解状态 / Single-node solve status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNodeSolveStatus {
    /// 已获得精确 LP 证书 / Exact LP certificate obtained.
    Optimal,
    /// 当前节点不可行 / Node infeasible.
    Infeasible,
    /// 命中时间上限 / Time limit reached.
    TimeLimit,
    /// 被外部求解器或取消令牌停止 / Stopped by an external solver or cancellation.
    SolverStopped,
}

/// 单次 LP 求解状态 / Single LP-solve status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearProgrammingStatus {
    /// 已获得最优 LP 解 / Optimal LP solution obtained.
    Optimal,
    /// RMP 不可行 / RMP is infeasible.
    Infeasible,
    /// LP 求解命中时间上限 / LP solve reached a time limit.
    TimeLimit,
    /// LP 求解被外部停止 / LP solve was stopped externally.
    SolverStopped,
}

/// LP 求解请求 / LP-solving request.
pub struct LinearProgrammingRequest<'a> {
    /// 分支节点 ID / Branch-node ID.
    pub node_id: usize,
    /// 当前列生成阶段 / Current column-generation phase.
    pub phase: crate::domain::vrp::PricingPhase,
    /// 当前阶段迭代号 / Iteration within the current phase.
    pub iteration: usize,
    /// 当前 RMP / Current restricted master problem.
    pub model: &'a MetaModel<f64>,
    /// 绝对截止时间 / Absolute deadline.
    pub deadline: Option<Instant>,
    /// 统一取消句柄 / Unified cancellation handle.
    pub cancellation: Option<SolveHandle>,
}

/// LP 求解结果 / LP-solving result.
#[derive(Debug, Clone)]
pub struct LinearProgrammingResult {
    /// 求解状态 / Solve status.
    pub status: LinearProgrammingStatus,
    /// LP 目标值 / LP objective value.
    pub objective: Option<f64>,
    /// 按 solver 顺序排列的变量值 / Variable values in solver order.
    pub solution: Option<Vec<f64>>,
    /// 按模型约束顺序排列的对偶值 / Dual values in model-constraint order.
    pub dual_solution: Option<Vec<f64>>,
    /// LP 的统一求解报告 / Unified solve report for the LP.
    ///
    /// 不可行节点必须从该报告重新通过证书门禁，不能由布尔标记代替。
    /// An infeasible node must pass the certificate gate again from this report; a boolean flag
    /// is intentionally not accepted as a proof.
    pub report: Option<SolveReport<f64>>,
}

impl LinearProgrammingResult {
    /// 创建时间中断结果 / Create a time-limit result.
    pub fn time_limit() -> Self {
        Self {
            status: LinearProgrammingStatus::TimeLimit,
            objective: None,
            solution: None,
            dual_solution: None,
            report: None,
        }
    }

    /// 创建外部停止结果 / Create an externally stopped result.
    pub fn solver_stopped() -> Self {
        Self {
            status: LinearProgrammingStatus::SolverStopped,
            objective: None,
            solution: None,
            dual_solution: None,
            report: None,
        }
    }
}

/// 单节点求解结果 / Single-node solve result.
#[derive(Debug, Clone)]
pub struct BranchNodeSolveResult<V: SolveValue + UnitConversionValue> {
    /// 单节点状态 / Node status.
    pub status: BranchNodeSolveStatus,
    /// 精确定价完成标记 / Exact-pricing completion flag.
    pub pricing_complete: bool,
    /// 当前节点是否可行 / Whether the node is feasible.
    pub is_feasible: bool,
    /// 路线变量是否满足整数性 / Whether route variables are integral.
    pub is_integer: bool,
    /// 节点 LP 下界 / Node LP lower bound.
    pub lower_bound: f64,
    /// LP 目标值 / LP objective value.
    pub lp_objective: f64,
    /// 路线变量值 / Route-variable values.
    pub route_values: Vec<(Route<V>, f64)>,
    /// 当前节点产生的所有列 / All columns generated at this node.
    pub columns: Vec<Route<V>>,
    /// 列生成迭代数 / Column-generation iterations.
    pub iterations: usize,
    /// 定价调用次数 / Pricing calls.
    pub pricing_calls: usize,
    /// 生成列数 / Generated-column count.
    pub generated_columns: usize,
    /// 是否由内部截止条件中断 / Whether internal interruption occurred.
    pub interrupted: bool,
    /// 节点求解报告 / Structured report produced by the node solver.
    ///
    /// 应用层会重新校验证书和报告状态后才允许形成全局不可行证明。
    /// The application revalidates the proof and report status before forming a global
    /// infeasibility proof.
    pub solver_report: Option<SolveReport<f64>>,
    /// 产生节点报告的实际 LP 模型指纹 / Fingerprint of the actual LP model producing the report.
    ///
    /// 应用层必须用它绑定不可行证书；缺少该绑定时，注入 provider 的报告不能升级为全局证明。
    /// The application must use this binding for infeasibility certificates; an injected
    /// provider report without it cannot be promoted to a global proof.
    pub solver_model_fingerprint: Option<ModelFingerprint>,
}

impl<V> BranchNodeSolveResult<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建不可行节点结果 / Create an infeasible-node result.
    pub fn infeasible() -> Self {
        Self {
            status: BranchNodeSolveStatus::Infeasible,
            pricing_complete: true,
            is_feasible: false,
            is_integer: false,
            lower_bound: f64::INFINITY,
            lp_objective: f64::INFINITY,
            route_values: Vec::new(),
            columns: Vec::new(),
            iterations: 0,
            pricing_calls: 0,
            generated_columns: 0,
            interrupted: false,
            solver_report: None,
            solver_model_fingerprint: None,
        }
    }
}

/// 单节点求解上下文 / Single-node solve context.
#[derive(Debug, Clone)]
pub struct BranchNodeSolveContext {
    /// 每个节点允许的最大列生成迭代数 / Maximum column-generation iterations per node.
    pub max_cg_iterations: usize,
    /// 每次定价最多返回列数 / Maximum columns returned per pricing call.
    pub max_columns_per_pricing: usize,
    /// 定价标签上限 / Pricing label limit.
    pub max_labels: Option<usize>,
    /// 绝对截止时间 / Absolute deadline.
    pub deadline: Option<Instant>,
    /// 取消令牌 / Cancellation token.
    pub cancellation: Option<CancellationToken>,
}

/// 单节点求解器注入边界 / Injectable single-node solver boundary.
pub trait BranchNodeSolverProvider<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 独立求解一个分支节点 / Solve one branch node independently.
    fn solve_node(
        &self,
        node: &BranchNode,
        inherited_columns: &[Route<V>],
        context: &BranchNodeSolveContext,
    ) -> Result<BranchNodeSolveResult<V>>;

    /// 独立复核候选路线集合 / Independently validate a candidate route set.
    ///
    /// 具体 provider 可以覆盖此方法，使用业务策略完成完整重放；默认实现使用框架默认策略，
    /// 因而不会只凭 provider 返回的路线字段形成 incumbent。
    /// A provider may override this method with business policies; the default implementation
    /// replays the framework defaults instead of trusting provider-supplied route fields alone.
    fn validate_routes(
        &self,
        instance: &VrptwInstance<V>,
        branch_mask: &BranchMask<VehicleTypeId>,
        routes: &[Route<V>],
    ) -> Result<()> {
        validate_solution_structure(instance, branch_mask, routes)?;
        let distance_calculator = EuclideanDistanceCalculator;
        let travel_time_calculator =
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator);
        let arc_cost_calculator = DistanceArcCostCalculator;
        let route_cost_policy = FixedPlusArcCostPolicy;
        for route in routes {
            RouteValidator::validate_with_policy(
                instance,
                route,
                RouteValidationPolicy {
                    distance_calculator: &distance_calculator,
                    travel_time_calculator: &travel_time_calculator,
                    arc_cost_calculator: &arc_cost_calculator,
                    route_cost_policy: &route_cost_policy,
                    arc_feasibility_policy: &DefaultArcFeasibilityPolicy,
                    branch_mask: Some(branch_mask),
                },
            )?;
        }
        Ok(())
    }
}

/// Branch-and-Price 配置 / Branch-and-Price configuration.
#[derive(Debug, Clone)]
pub struct BranchAndPriceConfig {
    /// 总时间上限 / Overall time limit.
    pub time_limit: Option<Duration>,
    /// 节点上限 / Node limit.
    pub node_limit: usize,
    /// 相对 gap 容差 / Relative-gap tolerance.
    pub relative_gap_tolerance: f64,
    /// 定价返回列上限 / Maximum columns returned by pricing.
    pub max_columns_per_pricing: usize,
    /// 每节点列生成迭代上限 / Maximum column-generation iterations per node.
    pub max_cg_iterations_per_node: usize,
    /// 定价标签上限 / Pricing label limit.
    pub max_labels: Option<usize>,
    /// 外部取消令牌 / External cancellation token.
    pub cancellation: Option<CancellationToken>,
}

impl Default for BranchAndPriceConfig {
    fn default() -> Self {
        Self {
            time_limit: None,
            node_limit: usize::MAX,
            relative_gap_tolerance: 1e-4,
            max_columns_per_pricing: usize::MAX,
            max_cg_iterations_per_node: 1000,
            max_labels: None,
            cancellation: None,
        }
    }
}

/// Branch-and-Price trace 兼容别名 / Branch-and-Price trace compatibility alias.
///
/// 终态已经统一进入 [`SolveReport`]；该别名只保留算法 trace 的语义，不再承载状态。
/// Terminal states now live in [`SolveReport`]; this alias only preserves algorithm-trace semantics.
pub type BranchAndPriceTrace = SolveTrace;

/// Branch-and-Price trace 监听器 / Branch-and-Price trace listener.
pub trait TraceListener: Send + Sync {
    /// 接收一个 trace 快照 / Receive one trace snapshot.
    fn on_trace(&self, trace: &BranchAndPriceTrace) -> Result<()>;
}

impl<F> TraceListener for F
where
    F: Fn(&BranchAndPriceTrace) -> Result<()> + Send + Sync,
{
    fn on_trace(&self, trace: &BranchAndPriceTrace) -> Result<()> {
        self(trace)
    }
}

/// 无操作监听器 / No-op listener.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopTraceListener;

impl TraceListener for NoopTraceListener {
    fn on_trace(&self, _trace: &BranchAndPriceTrace) -> Result<()> {
        Ok(())
    }
}

/// Branch-and-Price best-bound 编排器 / Branch-and-Price best-bound orchestrator.
pub struct BranchAndPriceAlgorithm<V, P>
where
    V: SolveValue + UnitConversionValue,
    P: BranchNodeSolverProvider<V>,
{
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<V>>,
    /// 单节点求解器 / Single-node solver provider.
    pub provider: P,
    /// 算法配置 / Algorithm configuration.
    pub config: BranchAndPriceConfig,
    /// trace 监听器 / Trace listener.
    pub trace_listener: Arc<dyn TraceListener>,
}

impl<V, P> BranchAndPriceAlgorithm<V, P>
where
    V: SolveValue + UnitConversionValue,
    P: BranchNodeSolverProvider<V>,
{
    /// 创建 Branch-and-Price 编排器 / Create a Branch-and-Price orchestrator.
    pub fn new(
        instance: Arc<VrptwInstance<V>>,
        provider: P,
        config: BranchAndPriceConfig,
    ) -> Result<Self> {
        validate_config(&config)?;
        Ok(Self {
            instance,
            provider,
            config,
            trace_listener: Arc::new(NoopTraceListener),
        })
    }

    /// 设置 trace 监听器 / Set the trace listener.
    pub fn with_trace_listener(mut self, listener: Arc<dyn TraceListener>) -> Self {
        self.trace_listener = listener;
        self
    }

    /// 执行 best-bound Branch-and-Price / Run best-bound Branch-and-Price.
    pub fn solve(&self) -> Result<SolveReport<VrptwSolution<V>>> {
        let begin = Instant::now();
        let root = BranchNode::root(
            self.instance.start_depot.node.id.clone(),
            self.instance.end_depot.node.id.clone(),
        )?;
        let mut queue: BinaryHeap<QueueEntry<V>> = BinaryHeap::new();
        let mut sequence = 0usize;
        queue.push(QueueEntry::new(root, sequence));
        let mut inherited_columns = Vec::new();
        let mut column_signatures = BTreeSet::new();
        let mut incumbent: Option<(Vec<Route<V>>, f64)> = None;
        let mut nodes_explored = 0usize;
        let mut nodes_pruned = 0usize;
        let mut total_iterations = 0usize;
        let mut pricing_calls = 0usize;
        let mut generated_columns = 0usize;
        let mut closed_lower_bound = None;
        let mut saw_infeasible_node = false;
        let mut all_infeasible_nodes_verified = true;

        loop {
            let global_lower_bound = min_bound(queue_lower_bound(&queue), closed_lower_bound);
            validate_bound_relationship(
                global_lower_bound,
                incumbent.as_ref().map(|(_, value)| *value),
                self.instance.tolerances.feasibility,
            )?;
            let elapsed = begin.elapsed();
            if self.config.time_limit.is_some_and(|limit| elapsed >= limit) {
                return self.assemble_result(
                    if incumbent.is_some() {
                        ProblemStatus::Feasible
                    } else {
                        ProblemStatus::Unknown
                    },
                    TerminationReason::TimeLimit,
                    None,
                    incumbent,
                    global_lower_bound,
                    begin.elapsed(),
                    nodes_explored,
                    nodes_pruned,
                    queue.len(),
                    total_iterations,
                    pricing_calls,
                    generated_columns,
                );
            }
            if nodes_explored >= self.config.node_limit {
                return self.assemble_result(
                    if incumbent.is_some() {
                        ProblemStatus::Feasible
                    } else {
                        ProblemStatus::Unknown
                    },
                    TerminationReason::NodeLimit,
                    None,
                    incumbent,
                    global_lower_bound,
                    begin.elapsed(),
                    nodes_explored,
                    nodes_pruned,
                    queue.len(),
                    total_iterations,
                    pricing_calls,
                    generated_columns,
                );
            }
            if let Some((_, upper_bound)) = &incumbent
                && let Some(gap) = global_lower_bound
                    .and_then(|lower_bound| relative_gap(*upper_bound, lower_bound))
                && gap <= self.config.relative_gap_tolerance
            {
                return self.assemble_result(
                    ProblemStatus::Feasible,
                    TerminationReason::Completed,
                    Some(SolveProof::optimality()),
                    incumbent,
                    global_lower_bound,
                    begin.elapsed(),
                    nodes_explored,
                    nodes_pruned,
                    queue.len(),
                    total_iterations,
                    pricing_calls,
                    generated_columns,
                );
            }
            let Some(mut entry) = queue.pop() else {
                let (problem_status, proof) = match incumbent {
                    None if saw_infeasible_node && all_infeasible_nodes_verified => {
                        (ProblemStatus::Infeasible, Some(SolveProof::infeasibility()))
                    }
                    None => (ProblemStatus::Unknown, None),
                    Some(_) => {
                        let gap = global_lower_bound
                            .zip(incumbent.as_ref().map(|(_, value)| *value))
                            .and_then(|(lower, upper)| relative_gap(upper, lower));
                        if gap.is_some_and(|gap| gap <= self.config.relative_gap_tolerance) {
                            (ProblemStatus::Feasible, Some(SolveProof::optimality()))
                        } else {
                            (ProblemStatus::Feasible, None)
                        }
                    }
                };
                return self.assemble_result(
                    problem_status,
                    TerminationReason::Completed,
                    proof,
                    incumbent,
                    global_lower_bound,
                    begin.elapsed(),
                    nodes_explored,
                    nodes_pruned,
                    0,
                    total_iterations,
                    pricing_calls,
                    generated_columns,
                );
            };
            if !entry.node.can_improve(
                incumbent.as_ref().map(|(_, cost)| *cost),
                self.instance.tolerances.pricing,
            ) {
                // 被 incumbent 剪枝的节点仍然有一个已知的有效下界；将它并入已关闭区域 / A pruned node still has a known valid lower bound; include it in the closed region.
                closed_lower_bound =
                    min_bound(closed_lower_bound, finite_bound(entry.node.lower_bound));
                entry.node.prune();
                nodes_pruned += 1;
                continue;
            }

            let context = BranchNodeSolveContext {
                max_cg_iterations: self.config.max_cg_iterations_per_node,
                max_columns_per_pricing: self.config.max_columns_per_pricing,
                max_labels: self.config.max_labels,
                deadline: self.config.time_limit.map(|limit| begin + limit),
                cancellation: self.config.cancellation.clone(),
            };
            let result = self
                .provider
                .solve_node(&entry.node, &inherited_columns, &context)?;
            validate_node_result_bounds(&result, self.instance.tolerances.feasibility)?;
            nodes_explored += 1;
            total_iterations += result.iterations;
            pricing_calls += result.pricing_calls;
            generated_columns += result.generated_columns;
            for route in &result.columns {
                if column_signatures.insert(route.signature()) {
                    inherited_columns.push(route.clone());
                }
            }

            let integer_incumbent =
                self.consider_integer_incumbent(&entry.node, &result, &mut incumbent)?;
            validate_bound_relationship(
                finite_bound(result.lower_bound),
                incumbent.as_ref().map(|(_, value)| *value),
                self.instance.tolerances.feasibility,
            )?;

            match result.status {
                BranchNodeSolveStatus::TimeLimit => {
                    return self.assemble_result(
                        if incumbent.is_some() {
                            ProblemStatus::Feasible
                        } else {
                            ProblemStatus::Unknown
                        },
                        TerminationReason::TimeLimit,
                        None,
                        incumbent,
                        min_bound(
                            global_lower_bound,
                            finite_bound(entry.node.inherited_lower_bound),
                        ),
                        begin.elapsed(),
                        nodes_explored,
                        nodes_pruned,
                        queue.len(),
                        total_iterations,
                        pricing_calls,
                        generated_columns,
                    );
                }
                BranchNodeSolveStatus::SolverStopped => {
                    return self.assemble_result(
                        if incumbent.is_some() {
                            ProblemStatus::Feasible
                        } else {
                            ProblemStatus::Unknown
                        },
                        self.cancellation_termination_reason(),
                        None,
                        incumbent,
                        min_bound(
                            global_lower_bound,
                            finite_bound(entry.node.inherited_lower_bound),
                        ),
                        begin.elapsed(),
                        nodes_explored,
                        nodes_pruned,
                        queue.len(),
                        total_iterations,
                        pricing_calls,
                        generated_columns,
                    );
                }
                BranchNodeSolveStatus::Infeasible => {
                    saw_infeasible_node = true;
                    all_infeasible_nodes_verified &= verified_infeasibility_report(
                        result.solver_report.as_ref(),
                        result.solver_model_fingerprint.as_ref(),
                    );
                    entry.node.mark_infeasible();
                    continue;
                }
                BranchNodeSolveStatus::Optimal => {}
            }
            if !result.pricing_complete {
                return Err(NetworkSchedulingError::interrupted(format!(
                    "节点 {} 定价未完成，不能更新有效下界 / pricing at node {} is incomplete and cannot update a valid bound",
                    entry.node.id, entry.node.id
                )));
            }
            if !result.is_feasible {
                saw_infeasible_node = true;
                all_infeasible_nodes_verified &= verified_infeasibility_report(
                    result.solver_report.as_ref(),
                    result.solver_model_fingerprint.as_ref(),
                );
                entry.node.mark_infeasible();
                continue;
            }
            if !result.lower_bound.is_finite() {
                return Err(NetworkSchedulingError::contract(format!(
                    "节点 {} 缺少有限 LP 下界 / node {} has no finite LP lower bound",
                    entry.node.id, entry.node.id
                )));
            }
            entry.node.solve(result.lower_bound);

            if result.is_integer {
                if !integer_incumbent {
                    return Err(NetworkSchedulingError::contract(format!(
                        "节点 {} 声明整数解但未形成合法 incumbent / node {} declared an integer solution without a valid incumbent",
                        entry.node.id, entry.node.id
                    )));
                }
                closed_lower_bound =
                    min_bound(closed_lower_bound, finite_bound(entry.node.lower_bound));
                self.emit_trace(
                    begin,
                    incumbent.as_ref().map(|(_, value)| *value),
                    min_bound(queue_lower_bound(&queue), closed_lower_bound),
                    nodes_explored,
                    nodes_pruned,
                    queue.len(),
                    total_iterations,
                    pricing_calls,
                    generated_columns,
                )?;
                continue;
            }

            let decision = select_branch_decision(
                &result.route_values,
                &self.instance,
                self.instance.tolerances.integrality,
            )?
            .ok_or_else(|| NetworkSchedulingError::contract(format!(
                "节点 {} 的 assignment、edge 和 route 整数性合同不一致 / node {} violates assignment-edge-route integrality contract",
                entry.node.id, entry.node.id
            )))?;
            let left = entry.node.child(sequence + 1, decision.clone());
            sequence += 1;
            let right = entry.node.child(sequence + 1, decision.complementary());
            sequence += 1;
            if let Ok(left) = left {
                queue.push(QueueEntry::new(left, sequence));
            }
            if let Ok(right) = right {
                queue.push(QueueEntry::new(right, sequence + 1));
            }
            self.emit_trace(
                begin,
                incumbent.as_ref().map(|(_, value)| *value),
                min_bound(queue_lower_bound(&queue), closed_lower_bound),
                nodes_explored,
                nodes_pruned,
                queue.len(),
                total_iterations,
                pricing_calls,
                generated_columns,
            )?;
        }
    }

    fn consider_integer_incumbent(
        &self,
        node: &BranchNode,
        result: &BranchNodeSolveResult<V>,
        incumbent: &mut Option<(Vec<Route<V>>, f64)>,
    ) -> Result<bool> {
        if !result.is_feasible || matches!(result.status, BranchNodeSolveStatus::Infeasible) {
            return Ok(false);
        }
        for (_, value) in &result.route_values {
            validate_numeric(*value, "route value")?;
            if *value < -self.instance.tolerances.integrality
                || *value > 1.0 + self.instance.tolerances.integrality
            {
                return Err(NetworkSchedulingError::contract(
                    "路线变量超出二元范围 / route variable is outside the binary range",
                ));
            }
        }
        let all_integer = !result.route_values.is_empty()
            && result.route_values.iter().all(|(_, value)| {
                value.is_finite()
                    && (*value - value.round()).abs() <= self.instance.tolerances.integrality
            });
        if !all_integer {
            return Ok(false);
        }
        let selected = result
            .route_values
            .iter()
            .filter(|(_, value)| *value >= 1.0 - self.instance.tolerances.integrality)
            .map(|(route, _)| route.clone())
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Ok(false);
        }
        // application 自身先复核组合合同，不能把最终解安全性完全委托给 provider。
        // The application validates the aggregate contract before consulting the provider.
        let validation = validate_solution_structure(&self.instance, &node.path.mask, &selected)
            .and_then(|_| {
                self.provider
                    .validate_routes(&self.instance, &node.path.mask, &selected)
            });
        if let Err(error) = validation {
            if matches!(
                result.status,
                BranchNodeSolveStatus::TimeLimit | BranchNodeSolveStatus::SolverStopped
            ) {
                // 中断时的部分整数 RMP 不是业务 incumbent；保留状态但丢弃非法候选。
                // A partial integer RMP on interruption is not a business incumbent; retain the
                // terminal status while discarding the invalid candidate.
                return Ok(false);
            }
            return Err(error);
        }
        let cost = solution_cost(&selected, &self.instance)?;
        if incumbent
            .as_ref()
            .is_none_or(|(_, best)| cost < *best - self.instance.tolerances.cost_validation)
        {
            *incumbent = Some((selected, cost));
        }
        Ok(true)
    }

    fn cancellation_termination_reason(&self) -> TerminationReason {
        if self
            .config
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            TerminationReason::Cancelled
        } else {
            TerminationReason::Interrupted
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_trace(
        &self,
        begin: Instant,
        upper_bound: Option<f64>,
        lower_bound: Option<f64>,
        nodes_explored: usize,
        nodes_pruned: usize,
        active_nodes: usize,
        total_iterations: usize,
        pricing_calls: usize,
        generated_columns: usize,
    ) -> Result<()> {
        let trace = BranchAndPriceTrace {
            nodes_explored,
            nodes_pruned,
            active_nodes,
            global_lower_bound: lower_bound,
            upper_bound,
            relative_gap: upper_bound
                .zip(lower_bound)
                .and_then(|(upper, lower)| relative_gap(upper, lower)),
            total_iterations,
            pricing_calls,
            generated_columns,
            elapsed: begin.elapsed(),
            iteration_snapshots: Vec::new(),
        };
        self.trace_listener.on_trace(&trace)
    }

    #[allow(clippy::too_many_arguments)]
    fn assemble_result(
        &self,
        problem_status: ProblemStatus,
        termination_reason: TerminationReason,
        proof: Option<SolveProof<VrptwSolution<V>>>,
        incumbent: Option<(Vec<Route<V>>, f64)>,
        lower_bound: Option<f64>,
        elapsed: Duration,
        nodes_explored: usize,
        nodes_pruned: usize,
        active_nodes: usize,
        total_iterations: usize,
        pricing_calls: usize,
        generated_columns: usize,
    ) -> Result<SolveReport<VrptwSolution<V>>> {
        let upper_bound = incumbent.as_ref().map(|(_, value)| *value);
        validate_bound_relationship(
            lower_bound,
            upper_bound,
            self.instance.tolerances.feasibility,
        )?;
        let solution = incumbent
            .map(|(routes, _)| build_solution(routes, &self.instance))
            .transpose()?;
        let trace = BranchAndPriceTrace {
            nodes_explored,
            nodes_pruned,
            active_nodes,
            global_lower_bound: lower_bound,
            upper_bound,
            relative_gap: upper_bound
                .zip(lower_bound)
                .and_then(|(upper, lower)| relative_gap(upper, lower)),
            total_iterations,
            pricing_calls,
            generated_columns,
            elapsed,
            iteration_snapshots: Vec::new(),
        };
        let solution = solution.map(|solution| {
            let mut report_solution = SolveSolution::incumbent(solution);
            report_solution.objective_value = upper_bound;
            report_solution
        });
        let statistics = SolveStatistics {
            solve_time: elapsed,
            iterations: Some(total_iterations),
            nodes: Some(nodes_explored),
            best_bound: None,
            best_bound_value: lower_bound,
            absolute_gap: upper_bound
                .zip(lower_bound)
                .map(|(upper, lower)| (upper - lower).max(0.0)),
            relative_gap: trace.relative_gap,
            solution_count: solution.as_ref().map(|_| 1),
            extensions: BTreeMap::new(),
        };
        let mut builder = SolveReport::builder(problem_status, termination_reason)
            .statistics(statistics)
            .trace(trace);
        let mut diagnostics = SolveDiagnostics::default();
        if let Some(cancellation) = self
            .config
            .cancellation
            .as_ref()
            .and_then(CancellationToken::cancellation)
        {
            diagnostics.extensions.insert(
                "cancellation.origin".to_owned(),
                cancellation.origin.to_string(),
            );
            diagnostics.extensions.insert(
                "cancellation.requestedAtEpochMs".to_owned(),
                cancellation.requested_at_epoch_ms.to_string(),
            );
        }
        builder = builder.diagnostics(diagnostics);
        if let Some(solution) = solution {
            builder = builder.solution(solution);
        }
        if let Some(proof) = proof {
            builder = builder.proof(proof);
        }
        builder
            .build()
            .map_err(|error| NetworkSchedulingError::contract(error.to_string()))
    }
}

/// VRPTW 应用服务 / VRPTW application service.
pub struct VrptwApplicationService<V, P>
where
    V: SolveValue + UnitConversionValue,
    P: BranchNodeSolverProvider<V>,
{
    /// 内部 Branch-and-Price 算法 / Internal Branch-and-Price algorithm.
    pub algorithm: BranchAndPriceAlgorithm<V, P>,
}

impl<V, P> VrptwApplicationService<V, P>
where
    V: SolveValue + UnitConversionValue,
    P: BranchNodeSolverProvider<V>,
{
    /// 创建应用服务 / Create the application service.
    pub fn new(
        instance: Arc<VrptwInstance<V>>,
        provider: P,
        config: BranchAndPriceConfig,
    ) -> Result<Self> {
        Ok(Self {
            algorithm: BranchAndPriceAlgorithm::new(instance, provider, config)?,
        })
    }

    /// 校验输入并执行求解 / Validate input and solve.
    pub fn solve(&self) -> Result<SolveReport<VrptwSolution<V>>> {
        crate::domain::vrp::VrptwValidator::validate(&self.algorithm.instance)?;
        self.algorithm.solve()
    }
}

/// 按 assignment/edge 两级规则选择分支 / Select a branch using assignment then edge values.
pub fn select_branch_decision<V>(
    route_values: &[(Route<V>, f64)],
    instance: &VrptwInstance<V>,
    tolerance: f64,
) -> Result<Option<BranchDecision>>
where
    V: SolveValue + UnitConversionValue,
{
    let mut assignment = BTreeMap::<(VehicleTypeId, CustomerId), f64>::new();
    for (route, value) in route_values {
        validate_numeric(*value, "route value")?;
        if *value < -tolerance || *value > 1.0 + tolerance {
            return Err(NetworkSchedulingError::contract(
                "路线变量超出二元范围 / route variable is outside the binary range",
            ));
        }
        for customer_id in route.customer_ids() {
            *assignment
                .entry((route.vehicle_type_id.clone(), customer_id))
                .or_default() += *value;
        }
    }
    if let Some(((vehicle_type_id, customer_id), _)) =
        closest_fractional_assignment(&assignment, tolerance)
    {
        let customer_node_id = instance
            .customer(&customer_id)
            .map(|customer| customer.node.id.clone())
            .ok_or_else(|| {
                NetworkSchedulingError::contract(format!(
                    "分支客户不存在：{} / branching customer does not exist: {}",
                    customer_id, customer_id
                ))
            })?;
        return Ok(Some(BranchDecision::ForbidVehicleType {
            vehicle_type_id,
            customer_id,
            customer_node_id,
        }));
    }

    let mut edges = BTreeMap::<
        (
            VehicleTypeId,
            crate::infrastructure::NetworkArcId,
            crate::infrastructure::NetworkNodeId,
            crate::infrastructure::NetworkNodeId,
        ),
        f64,
    >::new();
    for (route, value) in route_values {
        let arc_ids = route.effective_arc_ids();
        for (position, pair) in route.stops.windows(2).enumerate() {
            let arc_id = arc_ids.get(position).ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "路线弧索引长度不一致 / route arc-index length is inconsistent",
                )
            })?;
            let key = (
                route.vehicle_type_id.clone(),
                arc_id.clone(),
                pair[0].node_id.clone(),
                pair[1].node_id.clone(),
            );
            *edges.entry(key).or_default() += *value;
        }
    }
    if let Some(((vehicle_type_id, arc_id, from, to), _)) =
        closest_fractional_edge(&edges, tolerance)
    {
        return Ok(Some(BranchDecision::ForbidArc {
            vehicle_type_id,
            arc_id,
            from,
            to,
        }));
    }
    if route_values.iter().any(|(_, value)| {
        validate_numeric(*value, "route value").is_ok()
            && *value > tolerance
            && (*value - value.round()).abs() > tolerance
    }) {
        return Err(NetworkSchedulingError::contract(
            "assignment 和 edge 已整数但 route 变量仍为分数 / assignment and edge are integral but route variable remains fractional",
        ));
    }
    Ok(None)
}

fn closest_fractional_assignment(
    values: &BTreeMap<(VehicleTypeId, CustomerId), f64>,
    tolerance: f64,
) -> Option<((VehicleTypeId, CustomerId), f64)> {
    values
        .iter()
        .filter(|(_, value)| **value > tolerance && **value < 1.0 - tolerance)
        .min_by(|(_, left), (_, right)| {
            (**left - 0.5)
                .abs()
                .partial_cmp(&(**right - 0.5).abs())
                .unwrap_or(Ordering::Equal)
        })
        .map(|(key, value)| (key.clone(), *value))
}

fn closest_fractional_edge(
    values: &BTreeMap<
        (
            VehicleTypeId,
            crate::infrastructure::NetworkArcId,
            crate::infrastructure::NetworkNodeId,
            crate::infrastructure::NetworkNodeId,
        ),
        f64,
    >,
    tolerance: f64,
) -> Option<(
    (
        VehicleTypeId,
        crate::infrastructure::NetworkArcId,
        crate::infrastructure::NetworkNodeId,
        crate::infrastructure::NetworkNodeId,
    ),
    f64,
)> {
    values
        .iter()
        .filter(|(_, value)| **value > tolerance && **value < 1.0 - tolerance)
        .min_by(|(_, left), (_, right)| {
            (**left - 0.5)
                .abs()
                .partial_cmp(&(**right - 0.5).abs())
                .unwrap_or(Ordering::Equal)
        })
        .map(|(key, value)| (key.clone(), *value))
}

fn mask_from_decisions(
    start_depot: &crate::infrastructure::NetworkNodeId,
    end_depot: &crate::infrastructure::NetworkNodeId,
    decisions: &[BranchDecision],
) -> Result<BranchMask<VehicleTypeId>> {
    let mut forbidden_nodes = BTreeMap::<VehicleTypeId, BTreeSet<_>>::new();
    let mut required_resources =
        BTreeMap::<crate::infrastructure::NetworkNodeId, VehicleTypeId>::new();
    let mut forbidden_arcs = BTreeSet::new();
    let mut required_arcs = BTreeSet::new();
    for decision in decisions {
        match decision {
            BranchDecision::ForbidVehicleType {
                vehicle_type_id,
                customer_node_id,
                ..
            } => {
                forbidden_nodes
                    .entry(vehicle_type_id.clone())
                    .or_default()
                    .insert(customer_node_id.clone());
            }
            BranchDecision::RequireVehicleType {
                vehicle_type_id,
                customer_node_id,
                ..
            } => {
                if required_resources
                    .insert(customer_node_id.clone(), vehicle_type_id.clone())
                    .is_some_and(|previous| previous != *vehicle_type_id)
                {
                    return Err(NetworkSchedulingError::validation(
                        "客户的必选车辆类型冲突 / required vehicle types conflict for a customer",
                    ));
                }
            }
            BranchDecision::ForbidArc {
                vehicle_type_id,
                arc_id,
                from,
                to,
            } => {
                forbidden_arcs.insert(ResourceArc::new(
                    vehicle_type_id.clone(),
                    arc_id.clone(),
                    from.clone(),
                    to.clone(),
                ));
            }
            BranchDecision::RequireArc {
                vehicle_type_id,
                arc_id,
                from,
                to,
            } => {
                required_arcs.insert(ResourceArc::new(
                    vehicle_type_id.clone(),
                    arc_id.clone(),
                    from.clone(),
                    to.clone(),
                ));
            }
        }
    }
    BranchMask::new(
        start_depot.clone(),
        end_depot.clone(),
        forbidden_nodes,
        required_resources,
        forbidden_arcs,
        required_arcs,
    )
}

fn solution_cost<V>(routes: &[Route<V>], instance: &VrptwInstance<V>) -> Result<f64>
where
    V: SolveValue + UnitConversionValue,
{
    routes.iter().try_fold(0.0, |sum, route| {
        let cost = route
            .cost
            .to_unit(&instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        Ok(sum + cost)
    })
}

fn validate_solution_structure<V>(
    instance: &VrptwInstance<V>,
    branch_mask: &BranchMask<VehicleTypeId>,
    routes: &[Route<V>],
) -> Result<()>
where
    V: SolveValue + UnitConversionValue,
{
    if routes.is_empty() {
        return Err(NetworkSchedulingError::validation(
            "候选解没有路线 / candidate solution contains no routes",
        ));
    }
    let tolerance = instance.tolerances.feasibility;
    let mut covered = BTreeSet::new();
    let mut vehicle_counts = BTreeMap::<VehicleTypeId, usize>::new();
    for route in routes {
        let vehicle_type = instance
            .vehicle_type(&route.vehicle_type_id)
            .ok_or_else(|| {
                NetworkSchedulingError::validation(format!(
                    "路线车辆类型不存在：{} / route vehicle type does not exist: {}",
                    route.vehicle_type_id, route.vehicle_type_id
                ))
            })?;
        if route.stops.len() < 3 {
            return Err(NetworkSchedulingError::validation(
                "路线至少需要一个客户停靠点 / route must contain at least one customer stop",
            ));
        }
        let route_distance = route
            .distance
            .to_unit(&instance.units.distance_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let route_cost = route
            .cost
            .to_unit(&instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if !route_distance.is_finite()
            || !route_cost.is_finite()
            || route_distance < -tolerance
            || route_cost < -tolerance
        {
            return Err(NetworkSchedulingError::validation(
                "路线距离或成本不是有限非负值 / route distance or cost is not finite and non-negative",
            ));
        }
        let first = route.stops.first().ok_or_else(|| {
            NetworkSchedulingError::contract("路线首停靠点缺失 / route first stop is missing")
        })?;
        let last = route.stops.last().ok_or_else(|| {
            NetworkSchedulingError::contract("路线末停靠点缺失 / route last stop is missing")
        })?;
        if first.node_id != instance.start_depot.node.id
            || first.customer_id.is_some()
            || last.node_id != instance.end_depot.node.id
            || last.customer_id.is_some()
        {
            return Err(NetworkSchedulingError::validation(
                "路线仓库端点无效 / route depot endpoints are invalid",
            ));
        }
        instance.validate_route_arc_ids(route)?;
        if !instance.arcs.is_empty() {
            for (index, pair) in route.stops.windows(2).enumerate() {
                let arc = instance
                    .arc_for_route(
                        &route.effective_arc_ids()[index],
                        &pair[0].node_id,
                        &pair[1].node_id,
                    )
                    .ok_or_else(|| {
                        NetworkSchedulingError::validation(
                            "路线引用不存在或有歧义的基础网络弧 / route references a missing or ambiguous base-network arc",
                        )
                    })?;
                if !arc.feasible {
                    return Err(NetworkSchedulingError::validation(
                        "路线使用了不可行基础网络弧 / route uses an infeasible base-network arc",
                    ));
                }
            }
        }
        let stops = route
            .stops
            .iter()
            .map(|stop| stop.node_id.clone())
            .collect::<Vec<_>>();
        if !branch_mask.is_route_compatible(
            &route.vehicle_type_id,
            &stops,
            &route.effective_arc_ids(),
        ) {
            return Err(NetworkSchedulingError::validation(
                "路线不兼容当前分支遮罩 / route is incompatible with the current branch mask",
            ));
        }
        let capacity = vehicle_type
            .capacity
            .to_unit(&instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if !capacity.is_finite() || capacity < -tolerance {
            return Err(NetworkSchedulingError::validation(
                "车辆容量不是有限非负值 / vehicle capacity is not finite and non-negative",
            ));
        }
        let mut previous_departure = None;
        let mut previous_load = 0.0;
        for stop in &route.stops {
            let window = instance.time_window_of(&stop.node_id).ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线节点时间窗不存在 / route node time window does not exist",
                )
            })?;
            if stop.arrival > stop.service_start
                || stop.service_start > stop.departure
                || !window.contains(stop.service_start)
                || previous_departure.is_some_and(|departure| stop.arrival < departure)
                || stop.departure != stop.service_start + instance.service_time_of(&stop.node_id)
            {
                return Err(NetworkSchedulingError::validation(
                    "路线时间递推或时间窗无效 / route time recurrence or time window is invalid",
                ));
            }
            let load = stop
                .accumulated_load
                .to_unit(&instance.units.load_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            if !load.is_finite() || load < -tolerance || load > capacity + tolerance {
                return Err(NetworkSchedulingError::validation(
                    "路线累计负载不是有限值或超出容量 / route accumulated load is non-finite or exceeds capacity",
                ));
            }
            let demand = instance
                .demand_of(&stop.node_id)
                .map(|value| value.to_unit(&instance.units.load_unit))
                .transpose()
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .map(|value| {
                    value
                        .value
                        .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                })
                .transpose()
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .unwrap_or(0.0);
            if !demand.is_finite() || (load - (previous_load + demand)).abs() > tolerance {
                return Err(NetworkSchedulingError::validation(
                    "路线累计负载递推无效 / route accumulated-load recurrence is invalid",
                ));
            }
            previous_departure = Some(stop.departure);
            previous_load = load;
        }
        for stop in &route.stops[1..route.stops.len() - 1] {
            let customer_id = stop.customer_id.as_ref().ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "客户停靠点缺少客户 ID / customer stop is missing customer ID",
                )
            })?;
            let customer = instance.customer(customer_id).ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线引用不存在客户 / route references a missing customer",
                )
            })?;
            if customer.node.id != stop.node_id || !covered.insert(customer_id.clone()) {
                return Err(NetworkSchedulingError::validation(
                    "客户重复或节点不匹配 / customer is repeated or has a mismatched node",
                ));
            }
        }
        let count = vehicle_counts
            .entry(route.vehicle_type_id.clone())
            .or_default();
        *count += 1;
        if *count > vehicle_type.amount {
            return Err(NetworkSchedulingError::validation(
                "路线数量超过车队上限 / route count exceeds fleet capacity",
            ));
        }
    }
    let expected = instance
        .customers
        .iter()
        .map(|customer| customer.id.clone())
        .collect::<BTreeSet<_>>();
    if covered != expected {
        return Err(NetworkSchedulingError::validation(
            "客户没有被精确覆盖 / customers are not covered exactly once",
        ));
    }
    Ok(())
}

fn build_solution<V>(routes: Vec<Route<V>>, instance: &VrptwInstance<V>) -> Result<VrptwSolution<V>>
where
    V: SolveValue + UnitConversionValue,
{
    let distance_unit = instance.units.distance_unit.clone();
    let cost_unit = instance.units.cost_unit.clone();
    let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding).map_err(
        |error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        },
    )?;
    let mut total_distance = Quantity::new(zero.clone(), distance_unit.clone());
    let mut total_cost = Quantity::new(zero, cost_unit.clone());
    for route in &routes {
        let distance = route.distance.to_unit(&distance_unit).map_err(|error| {
            NetworkSchedulingError::Conversion {
                message: error.to_string(),
            }
        })?;
        let cost =
            route
                .cost
                .to_unit(&cost_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
        total_distance.value = total_distance.value.clone() + distance.value;
        total_cost.value = total_cost.value.clone() + cost.value;
    }
    Ok(VrptwSolution {
        routes,
        total_distance,
        total_cost,
    })
}

fn relative_gap(upper_bound: f64, lower_bound: f64) -> Option<f64> {
    if !upper_bound.is_finite() || !lower_bound.is_finite() {
        return None;
    }
    (lower_bound <= upper_bound).then(|| (upper_bound - lower_bound) / upper_bound.abs().max(1.0))
}

fn validate_node_result_bounds<V>(result: &BranchNodeSolveResult<V>, tolerance: f64) -> Result<()>
where
    V: SolveValue + UnitConversionValue,
{
    if result.status != BranchNodeSolveStatus::Optimal || !result.is_feasible {
        return Ok(());
    }
    validate_numeric(result.lower_bound, "node lower bound")?;
    validate_numeric(result.lp_objective, "node LP objective")?;
    if result.lower_bound > result.lp_objective + tolerance {
        return Err(NetworkSchedulingError::contract(format!(
            "节点下界超过 LP 目标：{} > {} / node lower bound exceeds LP objective: {} > {}",
            result.lower_bound, result.lp_objective, result.lower_bound, result.lp_objective
        )));
    }
    Ok(())
}

fn verified_infeasibility_report(
    report: Option<&SolveReport<f64>>,
    expected_model: Option<&ModelFingerprint>,
) -> bool {
    let (Some(report), Some(expected_model)) = (report, expected_model) else {
        return false;
    };
    require_infeasibility_certificate_for_model(report, expected_model).is_ok()
}

fn validate_bound_relationship(
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    tolerance: f64,
) -> Result<()> {
    if lower_bound.is_some_and(|value| !value.is_finite())
        || upper_bound.is_some_and(|value| !value.is_finite())
    {
        return Err(NetworkSchedulingError::contract(
            "上下界必须是有限值 / lower and upper bounds must be finite",
        ));
    }
    if let (Some(lower_bound), Some(upper_bound)) = (lower_bound, upper_bound)
        && lower_bound > upper_bound + tolerance
    {
        return Err(NetworkSchedulingError::contract(format!(
            "下界超过可行解上界：{} > {} / lower bound exceeds incumbent upper bound: {} > {}",
            lower_bound, upper_bound, lower_bound, upper_bound
        )));
    }
    Ok(())
}

fn validate_config(config: &BranchAndPriceConfig) -> Result<()> {
    if !config.relative_gap_tolerance.is_finite()
        || config.relative_gap_tolerance < 0.0
        || config.max_cg_iterations_per_node == 0
        || config.max_columns_per_pricing == 0
    {
        return Err(NetworkSchedulingError::validation(
            "Branch-and-Price 配置无效 / Branch-and-Price configuration is invalid",
        ));
    }
    if config.time_limit.is_some_and(|limit| limit.is_zero()) {
        return Err(NetworkSchedulingError::validation(
            "时间上限必须为正 / time limit must be positive",
        ));
    }
    Ok(())
}

fn validate_numeric(value: f64, name: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(NetworkSchedulingError::contract(format!(
            "{} 不是有限数值 / {} is not finite",
            name, name
        )))
    }
}

fn finite_bound(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn min_bound(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn queue_lower_bound<V>(queue: &BinaryHeap<QueueEntry<V>>) -> Option<f64>
where
    V: SolveValue + UnitConversionValue,
{
    queue
        .iter()
        .filter_map(|entry| finite_bound(entry.node.lower_bound))
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal))
}

#[derive(Debug)]
struct QueueEntry<V: SolveValue + UnitConversionValue> {
    node: BranchNode,
    sequence: usize,
    marker: std::marker::PhantomData<V>,
}

impl<V> QueueEntry<V>
where
    V: SolveValue + UnitConversionValue,
{
    fn new(node: BranchNode, sequence: usize) -> Self {
        Self {
            node,
            sequence,
            marker: std::marker::PhantomData,
        }
    }
}

impl<V> PartialEq for QueueEntry<V>
where
    V: SolveValue + UnitConversionValue,
{
    fn eq(&self, other: &Self) -> bool {
        self.node.lower_bound == other.node.lower_bound && self.sequence == other.sequence
    }
}

impl<V> Eq for QueueEntry<V> where V: SolveValue + UnitConversionValue {}

impl<V> PartialOrd for QueueEntry<V>
where
    V: SolveValue + UnitConversionValue,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<V> Ord for QueueEntry<V>
where
    V: SolveValue + UnitConversionValue,
{
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .node
            .lower_bound
            .partial_cmp(&self.node.lower_bound)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}
