//! 广义流不可变问题与领域结果。 / Immutable generalized-flow problem and domain results.

use std::collections::BTreeMap;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{CapacityBounds, NetworkArcId, NetworkCost, NetworkNodeId};

/// 节点计量及硬净供给；正数为供给。 / Node measurement and hard net supply; positive means supply.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowNode<V> {
    /// 稳定节点 ID / Stable node ID.
    pub id: NetworkNodeId,
    /// 供需及该节点的计量单位 / Supply-demand and the node measurement unit.
    pub net_supply: Quantity<V, Unit>,
}

/// 起点投入容量与投入资费；不支持多输入 BOM。 / Origin input capacity and tariff; multi-input BOM is unsupported.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowArc<V> {
    /// 稳定弧 ID / Stable arc ID.
    pub id: NetworkArcId,
    /// 投入节点 / Input node.
    pub from: NetworkNodeId,
    /// 产出到达节点 / Arrival-output node.
    pub to: NetworkNodeId,
    /// 按起点单位计的有限投入范围 / Finite input bounds measured in origin units.
    pub input_capacity: CapacityBounds<V, Unit>,
    /// 每起点单位投入费用 / Cost per origin-unit input.
    pub cost: NetworkCost<V, Unit>,
    /// 无量纲、非负业务产率 / Dimensionless nonnegative business yield.
    pub production_yield: Quantity<V, Unit>,
    /// 终点单位/起点单位转换；跨量纲时必须显式给定 / Destination-unit per origin-unit conversion, required across dimensions.
    pub input_to_output_factor: Option<Quantity<V, Unit>>,
}

/// 不含注册状态的问题快照。 / Problem snapshot without registration state.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowProblem<V> {
    nodes: Vec<GeneralizedFlowNode<V>>,
    arcs: Vec<GeneralizedFlowArc<V>>,
    cost_unit: Unit,
}

impl<V> GeneralizedFlowProblem<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建并预检完整问题；不平衡供需由模型判断可行性。 / Create and preflight the problem; feasibility of unbalanced supply is left to the model.
    pub fn new(
        nodes: Vec<GeneralizedFlowNode<V>>,
        arcs: Vec<GeneralizedFlowArc<V>>,
        cost_unit: Unit,
    ) -> Result<Self> {
        let problem = Self { nodes, arcs, cost_unit };
        super::aggregation::prepare(&problem)?;
        Ok(problem)
    }

    /// 节点快照 / Node snapshot.
    pub fn nodes(&self) -> &[GeneralizedFlowNode<V>] { &self.nodes }

    /// 弧快照 / Arc snapshot.
    pub fn arcs(&self) -> &[GeneralizedFlowArc<V>] { &self.arcs }

    /// 总费用单位 / Total-cost unit.
    pub fn cost_unit(&self) -> &Unit { &self.cost_unit }
}

/// 归一化 solver 域的绝对/相对容差。 / Absolute/relative tolerance in normalized solver space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneralizedFlowSolverTolerance {
    /// 绝对容差，必须有限非负 / Finite nonnegative absolute tolerance.
    pub absolute: f64,
    /// 相对容差，必须有限非负 / Finite nonnegative relative tolerance.
    pub relative: f64,
}

impl Default for GeneralizedFlowSolverTolerance {
    fn default() -> Self { Self { absolute: 1e-7, relative: 1e-7 } }
}

impl GeneralizedFlowSolverTolerance {
    /// 拒绝非法配置与容差计算溢出。 / Reject invalid configuration and tolerance overflow.
    pub fn for_values(&self, values: &[f64]) -> Result<f64> {
        if !self.absolute.is_finite() || self.absolute < 0.0
            || !self.relative.is_finite() || self.relative < 0.0
            || values.iter().any(|value| !value.is_finite())
        {
            return Err(NetworkSchedulingError::validation(
                "广义流容差及比较值必须有限且容差非负 / generalized-flow tolerances and compared values must be finite, with nonnegative tolerances",
            ));
        }
        let scale = values.iter().fold(0.0_f64, |scale, value| scale.max(value.abs()));
        let tolerance = self.absolute + self.relative * scale;
        if !tolerance.is_finite() {
            return Err(NetworkSchedulingError::contract(
                "广义流容差计算溢出 / generalized-flow tolerance overflow",
            ));
        }
        Ok(tolerance)
    }
}

/// 一条弧的投入、产出及费用。 / Input, output and cost of an arc.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowArcResult<V> {
    /// 起点单位投入 / Input in origin units.
    pub input: Quantity<V, Unit>,
    /// 终点单位到达产出 / Arrival output in destination units.
    pub output: Quantity<V, Unit>,
    /// 先单位转换后、未计产率的产出减实际产出 / Converted pre-yield output minus actual output.
    pub loss_or_gain: Quantity<V, Unit>,
    /// 弧费用 / Arc cost.
    pub cost: Quantity<V, Unit>,
}

/// 提取入口独立复核的节点净供给。 / Node net supply independently checked by extraction.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowNodeResult<V> {
    /// 实际净供给 / Actual net supply.
    pub actual_net_supply: Quantity<V, Unit>,
    /// 实际减声明净供给 / Actual minus declared net supply.
    pub residual: Quantity<V, Unit>,
}

/// 通过生产校验的广义流解。 / Generalized-flow solution passing production validation.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowSolution<V> {
    /// 稳定弧结果 / Stable arc results.
    pub arcs: BTreeMap<NetworkArcId, GeneralizedFlowArcResult<V>>,
    /// 稳定节点结果 / Stable node results.
    pub nodes: BTreeMap<NetworkNodeId, GeneralizedFlowNodeResult<V>>,
    /// 总费用 / Total cost.
    pub total_cost: Quantity<V, Unit>,
}
