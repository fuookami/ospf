//! 不可变建设问题及共享决策结果。 / Immutable construction problem and shared-decision results.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use crate::domain::flow::{FlowProblem, FlowSolution};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId, NetworkSchedulingSolverValueAdapter, validate_linear_units};

/// 互斥的通道建设等级。 / Mutually exclusive service construction level.
#[derive(Debug, Clone)]
pub struct CapacityLevel<V> {
    /// 稳定等级 ID / Stable level ID.
    pub id: String,
    /// 建成后的总容量，不是增量 / Total built capacity, not an increment.
    pub capacity: Quantity<V, Unit>,
    /// 建设费用 / Construction cost.
    pub cost: Quantity<V, Unit>,
}

/// 可选通道；没有等级被选中即关闭。 / Optional service, closed when no level is selected.
#[derive(Debug, Clone)]
pub struct NetworkService<V> {
    /// 已声明的稳定弧 / Declared stable arc.
    pub arc: NetworkArcId,
    /// 互斥容量等级 / Exclusive capacity levels.
    pub levels: Vec<CapacityLevel<V>>,
    /// 必须恰选一个等级 / Exactly one level must be selected.
    pub required: bool,
}

/// 控制相邻通道及指定处理行的候选设施。 / Candidate facility controlling incident services and declared processing rows.
#[derive(Debug, Clone)]
pub struct NetworkFacility<V> {
    /// 设施节点 / Facility node.
    pub node: NetworkNodeId,
    /// 开设费用 / Opening cost.
    pub cost: Quantity<V, Unit>,
    /// 是否必须开设 / Whether opening is required.
    pub required: bool,
    /// 随设施启用的共享资源行 / Shared resource rows gated by this facility.
    pub capacity_rows: BTreeSet<String>,
}

/// 经校验的不可变建设问题。 / Validated immutable construction problem.
#[derive(Debug, Clone)]
pub struct NetworkDesignProblem<V: SolveValue + UnitConversionValue> {
    flow: FlowProblem<V>,
    services: Vec<NetworkService<V>>,
    facilities: Vec<NetworkFacility<V>>,
    budget: Option<Quantity<V, Unit>>,
    cost_unit: Unit,
}

impl<V: SolveValue + UnitConversionValue> NetworkDesignProblem<V> {
    /// 在任何模型修改之前校验单位、引用、等级包络和预算。 / Validate units, references, level envelopes and budget before modifying any model.
    pub fn new(
        flow: FlowProblem<V>,
        services: Vec<NetworkService<V>>,
        facilities: Vec<NetworkFacility<V>>,
        budget: Option<Quantity<V, Unit>>,
    ) -> Result<Self> {
        validate_linear_units([&flow.units().flow_unit, &flow.units().cost_unit])?;
        let cost_unit = (&flow.units().flow_unit * &flow.units().cost_unit).build();
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let cost = |quantity: &Quantity<V, Unit>| -> Result<()> {
            if adapter.normalize(quantity, &cost_unit)? < 0.0 {
                return Err(NetworkSchedulingError::validation("建设费用和预算不能为负 / construction costs and budget must be nonnegative"));
            }
            Ok(())
        };
        let mut arcs = BTreeSet::new();
        for service in &services {
            let arc = flow.arcs().iter().find(|arc| arc.id == service.arc)
                .ok_or_else(|| NetworkSchedulingError::structure("建设通道不存在 / unknown construction service"))?;
            if !arcs.insert(service.arc.clone()) || service.levels.is_empty() {
                return Err(NetworkSchedulingError::validation("建设通道重复或没有等级 / duplicate construction service or no levels"));
            }
            let upper = adapter.to_solver(&arc.capacity.upper.value)?;
            let mut levels = BTreeSet::new();
            for level in &service.levels {
                let capacity = adapter.normalize(&level.capacity, &flow.units().flow_unit)?;
                cost(&level.cost)?;
                if level.id.is_empty() || !levels.insert(&level.id) || capacity < 0.0 || capacity > upper {
                    return Err(NetworkSchedulingError::validation("等级 ID 或总容量包络非法 / invalid level ID or total-capacity envelope"));
                }
            }
        }
        let mut nodes = BTreeSet::new();
        for facility in &facilities {
            if !nodes.insert(facility.node.clone()) || !flow.nodes().iter().any(|node| node.id == facility.node) {
                return Err(NetworkSchedulingError::structure("设施节点未知或重复 / unknown or duplicate facility node"));
            }
            cost(&facility.cost)?;
            for id in &facility.capacity_rows {
                if !flow.resources().iter().any(|resource| &resource.id == id) {
                    return Err(NetworkSchedulingError::structure("设施资源行不存在 / unknown facility resource row"));
                }
            }
        }
        if let Some(budget) = &budget { cost(budget)?; }
        Ok(Self { flow, services, facilities, budget, cost_unit })
    }

    /// 名义流问题 / Nominal flow problem.
    pub fn flow(&self) -> &FlowProblem<V> { &self.flow }

    /// 通道建设选择 / Service construction choices.
    pub fn services(&self) -> &[NetworkService<V>] { &self.services }

    /// 候选设施 / Candidate facilities.
    pub fn facilities(&self) -> &[NetworkFacility<V>] { &self.facilities }

    /// 可选建设预算 / Optional construction budget.
    pub fn budget(&self) -> Option<&Quantity<V, Unit>> { self.budget.as_ref() }

    /// 总费用单位 / Total-cost unit.
    pub fn cost_unit(&self) -> &Unit { &self.cost_unit }
}

/// 不含名义流的共享建设结果。 / Shared construction result without nominal flows.
#[derive(Debug, Clone)]
pub struct NetworkConstructionSolution<V> {
    /// 已开设施 / Open facilities.
    pub open_facilities: BTreeSet<NetworkNodeId>,
    /// 每通道选中的等级 / Selected level per service.
    pub service_levels: BTreeMap<NetworkArcId, String>,
    /// 建设费用 / Construction cost.
    pub cost: Quantity<V, Unit>,
}

/// 建设与名义运行结果。 / Construction and nominal-operation result.
#[derive(Debug, Clone)]
pub struct NetworkDesignSolution<V> {
    /// 共享建设选择 / Shared construction choices.
    pub construction: NetworkConstructionSolution<V>,
    /// 名义流结果 / Nominal flow result.
    pub flow: FlowSolution<V>,
    /// 建设加运行费用 / Construction plus operating cost.
    pub total_cost: Quantity<V, Unit>,
}
