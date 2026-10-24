//! 显式故障场景，不自动取消失效节点需求。 / Explicit failure scenarios without automatically waiving failed-node demand.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use crate::domain::design::{NetworkConstructionSolution, NetworkDesignProblem};
use crate::domain::flow::{CommodityId, FlowBalanceSlack, FlowSolution};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId};

/// 辅助流连通合同，不代表需求容量证明。 / Auxiliary-flow connectivity contract, not a demand-capacity proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityKind {
    /// 可达性 / Reachability.
    Reachable,
    /// 边不相交 / Edge-disjoint paths.
    EdgeDisjoint,
    /// 内部节点不相交 / Internally node-disjoint paths.
    NodeDisjoint,
}

/// 名义建设图上的业务连通要求。 / Business connectivity requirement on the nominal construction graph.
#[derive(Debug, Clone)]
pub struct ConnectivityRequirement {
    /// 起点 / Source.
    pub source: NetworkNodeId,
    /// 终点 / Target.
    pub target: NetworkNodeId,
    /// 路径数量，必须为正 / Positive path count.
    pub paths: usize,
    /// 不相交合同 / Disjointness contract.
    pub kind: ConnectivityKind,
}

/// 显式场景定义；空覆盖继承名义业务状态。 / Explicit scenario definition; absent overrides inherit nominal business state.
#[derive(Debug, Clone)]
pub struct FailureScenario<V> {
    /// 唯一场景 ID，nominal 保留 / Unique scenario ID; nominal is reserved.
    pub id: String,
    /// 关闭的弧 / Closed arcs.
    pub closed_arcs: BTreeSet<NetworkArcId>,
    /// 失效节点及其相邻弧 / Failed nodes and their incident arcs.
    pub failed_nodes: BTreeSet<NetworkNodeId>,
    /// 弧降级上界；不能超过名义值 / Degraded arc upper bounds, never above nominal values.
    pub upper_capacities: BTreeMap<NetworkArcId, Quantity<V, Unit>>,
    /// 按商品完整替换节点供需表 / Complete per-commodity node-balance replacements.
    pub balances: BTreeMap<CommodityId, HashMap<NetworkNodeId, Quantity<V, Unit>>>,
    /// 按商品完整替换松弛；未出现的商品继承名义配置 / Complete per-commodity slack replacements; absent commodities inherit nominal settings.
    pub slacks: BTreeMap<CommodityId, HashMap<NetworkNodeId, FlowBalanceSlack<V>>>,
}

impl<V> FailureScenario<V> {
    /// 创建没有隐含故障的空场景。 / Create an empty scenario without implicit failures.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), closed_arcs: BTreeSet::new(), failed_nodes: BTreeSet::new(), upper_capacities: BTreeMap::new(), balances: BTreeMap::new(), slacks: BTreeMap::new() }
    }
}

/// 不可变韧性问题，仅对显式场景作结论。 / Immutable resilience problem whose conclusions cover only explicit scenarios.
#[derive(Debug, Clone)]
pub struct NetworkResilienceProblem<V: SolveValue + UnitConversionValue> {
    design: NetworkDesignProblem<V>,
    scenarios: Vec<FailureScenario<V>>,
    connectivity: Vec<ConnectivityRequirement>,
}

impl<V: SolveValue + UnitConversionValue> NetworkResilienceProblem<V> {
    /// 在模型修改前验证全部场景和连通引用。 / Validate every scenario and connectivity reference before modifying a model.
    pub fn new(
        design: NetworkDesignProblem<V>,
        scenarios: Vec<FailureScenario<V>>,
        connectivity: Vec<ConnectivityRequirement>,
    ) -> Result<Self> {
        let mut ids = BTreeSet::new();
        for scenario in &scenarios {
            if scenario.id.is_empty() || scenario.id == "nominal" || !ids.insert(scenario.id.clone()) {
                return Err(NetworkSchedulingError::validation("故障场景 ID 非法或重复 / invalid or duplicate failure scenario ID"));
            }
            super::aggregation::prepare_scenario(&design, scenario)?;
        }
        for requirement in &connectivity {
            if requirement.paths == 0 || requirement.paths > u32::MAX as usize || requirement.source == requirement.target
                || design.flow().node(&requirement.source).is_none() || design.flow().node(&requirement.target).is_none()
            {
                return Err(NetworkSchedulingError::validation("连通要求引用或路径数量非法 / invalid connectivity reference or path count"));
            }
        }
        Ok(Self { design, scenarios, connectivity })
    }

    /// 共享建设问题 / Shared construction problem.
    pub fn design(&self) -> &NetworkDesignProblem<V> { &self.design }

    /// 显式故障集合 / Explicit failure set.
    pub fn scenarios(&self) -> &[FailureScenario<V>] { &self.scenarios }

    /// 名义图连通要求 / Nominal-graph connectivity requirements.
    pub fn connectivity(&self) -> &[ConnectivityRequirement] { &self.connectivity }
}

/// 共享建设、全部场景与独立连通见证。 / Shared construction, all scenarios and independent connectivity witnesses.
#[derive(Debug, Clone)]
pub struct NetworkResilienceSolution<V> {
    /// 建设选择 / Construction choices.
    pub construction: NetworkConstructionSolution<V>,
    /// 包含 nominal 的独立场景结果 / Independent scenario results including nominal.
    pub scenarios: BTreeMap<String, FlowSolution<V>>,
    /// 实际最大运行费用 / Actual maximum operating cost.
    pub worst_operating_cost: Quantity<V, Unit>,
    /// 求解模型的费用上包络；可行但非最优结果可能有余量 / Solver epigraph, potentially slack in feasible nonoptimal results.
    pub operating_cost_epigraph: Quantity<V, Unit>,
    /// 运行费用最大的场景 / Scenario with the maximum realized operating cost.
    pub worst_scenario: String,
    /// 各显式场景的建成且有效上界 / Built and effective upper bounds for each explicit scenario.
    pub effective_capacities: BTreeMap<String, BTreeMap<NetworkArcId, Quantity<V, Unit>>>,
    /// 与连通要求同序的辅助流 / Auxiliary flows in connectivity-requirement order.
    pub connectivity_flows: Vec<BTreeMap<NetworkArcId, V>>,
    /// 建设加实际最坏运行费用 / Construction plus actual worst operating cost.
    pub total_cost: Quantity<V, Unit>,
}
