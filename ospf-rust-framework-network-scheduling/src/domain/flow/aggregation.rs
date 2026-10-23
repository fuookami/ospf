//! flow 聚合 / Flow aggregation.

use std::collections::BTreeMap;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Continuous, UInteger, VariableRange};
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{FlowIndex, NetworkArcId, NetworkNodeId, NetworkSchedulingSolverValueAdapter};

use super::model::FlowProblem;

#[derive(Debug, Clone)]
struct PreparedFlowArc {
    step: f64,
    capacity: f64,
    cost: f64,
    upper: f64,
}

#[derive(Debug, Clone, Default)]
struct PreparedFlow {
    arcs: BTreeMap<(String, NetworkArcId), PreparedFlowArc>,
    resources: BTreeMap<String, BTreeMap<(String, NetworkArcId), f64>>,
}

fn finite(value: f64) -> Result<f64> {
    if value.is_finite() { Ok(value) } else {
        Err(NetworkSchedulingError::validation("流派生系数及变量界必须有限 / derived flow coefficients and variable bounds must be finite"))
    }
}

/// flow 领域聚合，持有 `(commodity, arc)` 到模型变量的稳定索引 / Flow aggregation holding stable `(commodity, arc)` model-variable indices.
#[derive(Debug, Clone)]
pub struct FlowAggregation<V: SolveValue + UnitConversionValue> {
    /// 不可变 flow 图 / Immutable flow graph.
    pub(super) graph: FlowProblem<V>,
    variable_indices: BTreeMap<(String, NetworkArcId), usize>,
    slack_indices: BTreeMap<(String, NetworkNodeId), (usize, usize)>,
    prepared: PreparedFlow,
}

impl<V> FlowAggregation<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建聚合 / Create an aggregation.
    pub fn new(graph: FlowProblem<V>) -> Self {
        Self {
            graph,
            variable_indices: BTreeMap::new(),
            slack_indices: BTreeMap::new(),
            prepared: PreparedFlow::default(),
        }
    }

    /// 注册非负流变量 / Register non-negative flow variables.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.register_named(model, "flow")
    }

    pub(super) fn register_named(&mut self, model: &mut MetaModel<f64>, name: &str) -> Result<()> {
        let mut next = self.clone();
        // 全部步长缩放在任何模型写入前完成，嵌入式关闭目标也不绕过校验。 / Prepare all step scaling before model writes, even when embedded objectives are disabled.
        next.prepared = self.prepare()?;
        model.transaction(|model| next.register_prepared(model, name))?;
        *self = next;
        Ok(())
    }

    fn prepare(&self) -> Result<PreparedFlow> {
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let mut prepared = PreparedFlow::default();
        for commodity in self.graph.commodities() {
            let step = adapter.to_solver(&self.graph.step(commodity)?)?;
            for balance in commodity.balances.values() { adapter.to_solver(&balance.value)?; }
            for slack in commodity.slacks.values() {
                adapter.to_solver(&slack.spill_limit.value)?;
                adapter.to_solver(&slack.shortage_limit.value)?;
                adapter.to_solver(&slack.spill_cost.quantity.value)?;
                adapter.to_solver(&slack.shortage_cost.quantity.value)?;
            }
            for arc in self.graph.arcs().iter().filter(|arc| self.graph.is_allowed(commodity, &arc.id)) {
                let capacity = finite(adapter.to_solver(&self.graph.capacity_coefficient(commodity, &arc.id)?)? * step)?;
                if capacity <= 0.0 {
                    return Err(NetworkSchedulingError::validation("流容量缩放必须为有限正数 / flow capacity scaling must be finite and positive"));
                }
                let upper = finite(adapter.to_solver(&arc.capacity.upper.value)? / capacity)?;
                let cost = finite(adapter.to_solver(&self.graph.cost_coefficient(commodity, arc)?)? * step)?;
                prepared.arcs.insert((commodity.commodity.as_str().to_owned(), arc.id.clone()),
                    PreparedFlowArc { step, capacity, cost, upper });
            }
        }
        for resource in self.graph.resources() {
            let mut terms = BTreeMap::new();
            for ((commodity, arc), usage) in &resource.consumptions {
                let key = (commodity.as_str().to_owned(), arc.clone());
                if let Some(flow) = prepared.arcs.get(&key) {
                    terms.insert(key, finite(adapter.to_solver(&usage.value)? * flow.step)?);
                }
            }
            prepared.resources.insert(resource.id.clone(), terms);
        }
        Ok(prepared)
    }

    fn register_prepared(&mut self, model: &mut MetaModel<f64>, name: &str) -> Result<()> {
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        for commodity in &self.graph.commodities {
            for arc in self.graph.arcs.iter().filter(|arc| self.graph.is_allowed(commodity, &arc.id)) {
                let upper = self.prepared.arcs[&(commodity.commodity.as_str().to_owned(), arc.id.clone())].upper;
                let name = format!("{name}_{}_{}", commodity.commodity, arc.id);
                let index = if commodity.integer_step.is_some() {
                    model.register_auto_variable_with_range::<UInteger>(&name, VariableRange::bounded(0.0, upper.floor()))
                } else {
                    model.register_auto_variable_with_range::<Continuous>(&name, VariableRange::bounded(0.0, upper))
                }.map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.variable_indices.insert(
                    (commodity.commodity.as_str().to_owned(), arc.id.clone()),
                    index,
                );
            }
            for (node, slack) in &commodity.slacks {
                let spill = model.register_auto_variable_with_range::<Continuous>(
                    &format!("{name}_{}_{}_spill", commodity.commodity, node),
                    VariableRange::bounded(0.0, adapter.to_solver(&slack.spill_limit.value)?),
                ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                let shortage = model.register_auto_variable_with_range::<Continuous>(
                    &format!("{name}_{}_{}_shortage", commodity.commodity, node),
                    VariableRange::bounded(0.0, adapter.to_solver(&slack.shortage_limit.value)?),
                ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.slack_indices.insert((commodity.commodity.as_str().to_owned(), node.clone()), (spill, shortage));
            }
        }
        Ok(())
    }

    /// 不可变领域问题 / Immutable domain problem.
    pub fn problem(&self) -> &FlowProblem<V> { &self.graph }

    /// 弃供/缺货变量索引，顺序为 `(spill, shortage)` / Slack variable indices ordered as `(spill, shortage)`.
    pub fn slack_indices(&self) -> &BTreeMap<(String, NetworkNodeId), (usize, usize)> { &self.slack_indices }

    /// 按商品单位计的流量表达式，已包含整数步长 / Flow expression in commodity units, including integer-step scaling.
    pub fn flow_expression(&self, commodity: &str, arc: &NetworkArcId) -> Result<Vec<(usize, f64)>> {
        let item = self.graph.commodities.iter().find(|item| item.commodity.as_str() == commodity)
            .ok_or_else(|| NetworkSchedulingError::structure("未知商品 / unknown commodity"))?;
        let index = self.variable_index(commodity, arc)
            .ok_or_else(|| NetworkSchedulingError::structure("商品弧不可用 / commodity arc is unavailable"))?;
        let step = self.prepared.arcs.get(&(item.commodity.as_str().to_owned(), arc.clone()))
            .ok_or_else(|| NetworkSchedulingError::contract("流系数尚未准备 / flow coefficients are not prepared"))?.step;
        Ok(vec![(index, step)])
    }

    /// 跨商品的物理弧容量用量表达式。 / Physical arc-capacity usage across commodities.
    pub fn capacity_expression(&self, arc: &NetworkArcId) -> Result<Vec<(usize, f64)>> {
        self.graph.commodities().iter().filter(|item| self.graph.is_allowed(item, arc))
            .map(|item| {
                let (index, _) = self.flow_expression(item.commodity.as_str(), arc)?[0];
                Ok((index, self.prepared.arcs[&(item.commodity.as_str().to_owned(), arc.clone())].capacity))
            }).collect()
    }

    /// 共享资源的归一化用量表达式。 / Normalized shared-resource usage expression.
    pub fn resource_expression(&self, id: &str) -> Result<Vec<(usize, f64)>> {
        let resource = self.prepared.resources.get(id)
            .ok_or_else(|| NetworkSchedulingError::structure("未知资源 / unknown resource"))?;
        let mut expression = Vec::new();
        for ((commodity, arc), coefficient) in resource {
            let (index, _) = self.flow_expression(commodity, arc)?[0];
            expression.push((index, *coefficient));
        }
        Ok(expression)
    }

    /// 包含运输与显式松弛的总运行费用表达式。 / Total operating cost including transport and explicit slacks.
    pub fn cost_expression(&self) -> Result<Vec<(usize, f64)>> {
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let mut expression = Vec::new();
        for item in self.graph.commodities() {
            for arc in self.graph.arcs().iter().filter(|arc| self.graph.is_allowed(item, &arc.id)) {
                let (index, _) = self.flow_expression(item.commodity.as_str(), &arc.id)?[0];
                expression.push((index, self.prepared.arcs[&(item.commodity.as_str().to_owned(), arc.id.clone())].cost));
            }
            for (node, slack) in &item.slacks {
                let (spill, shortage) = self.slack_indices[&(item.commodity.as_str().to_owned(), node.clone())];
                expression.push((spill, adapter.to_solver(&slack.spill_cost.quantity.value)?));
                expression.push((shortage, adapter.to_solver(&slack.shortage_cost.quantity.value)?));
            }
        }
        if expression.iter().any(|(_, value)| !value.is_finite()) {
            return Err(NetworkSchedulingError::validation("费用系数必须有限 / cost coefficients must be finite"));
        }
        Ok(expression)
    }

    /// 获取流变量索引 / Get a flow-variable index.
    pub fn variable_index(&self, commodity: &str, arc: &NetworkArcId) -> Option<usize> {
        self.variable_indices
            .get(&(commodity.to_owned(), arc.clone()))
            .copied()
    }

    /// 获取全部变量索引 / Get all flow-variable indices.
    pub fn variable_indices(&self) -> &BTreeMap<(String, NetworkArcId), usize> {
        &self.variable_indices
    }

    /// 创建 flow index / Create a typed flow index.
    pub fn flow_index(
        &self,
        commodity: &str,
        arc: &NetworkArcId,
    ) -> Option<FlowIndex<String, NetworkArcId>> {
        self.variable_index(commodity, arc)
            .map(|_| FlowIndex::new(commodity.to_owned(), arc.clone()))
    }
}
