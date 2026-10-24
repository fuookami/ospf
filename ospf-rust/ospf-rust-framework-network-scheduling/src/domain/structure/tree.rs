//! 唯一父弧与有限深度建立根有向树。 / Unique parent arcs and finite depths establish a rooted directed tree.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Binary, Continuous, VariableRange};
use crate::domain::design::NetworkConstruction;
use crate::domain::design::pipeline::{LinearRow, register_row, validate_rows};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId};
use super::model::NetworkStructureProblem;

pub(super) fn result(model: &MetaModel<f64>, index: usize) -> Result<f64> {
    model.tokens().get(index).and_then(|token| token.get_result()).filter(|value| value.is_finite())
        .ok_or_else(|| NetworkSchedulingError::contract("缺失或非有限结构结果 / missing or nonfinite structure result"))
}

pub(super) fn binary(model: &MetaModel<f64>, index: usize) -> Result<bool> {
    let value = result(model, index)?;
    if !(-1e-7..=1.0 + 1e-7).contains(&value) || (value - value.round()).abs() > 1e-7 {
        return Err(NetworkSchedulingError::contract("结构选择结果不是二元值 / structure selection is not binary"));
    }
    Ok(value.round() == 1.0)
}

pub(super) struct RootedTreeSelection {
    pub arcs: BTreeSet<NetworkArcId>,
    pub nodes: BTreeSet<NetworkNodeId>,
    pub parents: BTreeMap<NetworkNodeId, NetworkArcId>,
}

/// 不拥有求解器的根树变量与约束组件。 / Rooted-tree variables and constraints without solver ownership.
#[derive(Debug, Clone, Default)]
pub struct RootedTreeComponent {
    arcs: BTreeMap<NetworkArcId, usize>,
    active: BTreeMap<NetworkNodeId, usize>,
    depths: BTreeMap<NetworkNodeId, usize>,
    incoming: BTreeMap<NetworkNodeId, Vec<NetworkArcId>>,
    rows: Vec<LinearRow>,
    binding: Option<u64>,
}

impl RootedTreeComponent {
    /// 树弧变量；自环或树候选域外的已知弧没有变量，恒为零 / Tree-arc variables; known self-loops or out-of-domain arcs have no variable and are fixed zero.
    pub fn arc_variables(&self) -> &BTreeMap<NetworkArcId, usize> { &self.arcs }

    /// 节点启用变量；根和终端固定为一 / Node-activation variables; root and terminals are fixed one.
    pub fn active_variables(&self) -> &BTreeMap<NetworkNodeId, usize> { &self.active }

    /// 有限、无量纲的 solver 深度变量 / Finite, dimensionless solver depth variables.
    pub fn depth_variables(&self) -> &BTreeMap<NetworkNodeId, usize> { &self.depths }

    /// 已注册节点的父弧选择和表达式。 / Parent-arc selection-sum expression for a registered node.
    pub fn parent_expression(&self, node: &NetworkNodeId) -> Result<Vec<(usize, f64)>> {
        let incoming = self.incoming.get(node)
            .ok_or_else(|| NetworkSchedulingError::structure("节点不是已注册树候选 / node is not a registered tree candidate"))?;
        Ok(incoming.iter().map(|id| (self.arcs[id], 1.0)).collect())
    }

    pub(super) fn register<V>(
        &mut self,
        model: &mut MetaModel<f64>,
        construction: &NetworkConstruction<V>,
        problem: &NetworkStructureProblem<V>,
        name: &str,
    ) -> Result<()>
    where
        V: SolveValue + UnitConversionValue,
    {
        let maximum_depth = (problem.tree_nodes().len() - 1) as f64;
        let relaxation = maximum_depth + 1.0;
        for node in problem.tree_nodes() {
            let required = node == problem.root() || problem.terminals().contains(node);
            let active = model.register_auto_variable_with_range::<Binary>(
                &format!("{name}_active_{node}"), VariableRange::bounded(if required { 1.0 } else { 0.0 }, 1.0),
            ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            let depth = model.register_auto_variable_with_range::<Continuous>(
                &format!("{name}_depth_{node}"), VariableRange::bounded(0.0, if node == problem.root() { 0.0 } else { maximum_depth }),
            ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            self.active.insert(node.clone(), active);
            self.depths.insert(node.clone(), depth);
            self.incoming.insert(node.clone(), Vec::new());
            self.rows.push(register_row(model, vec![(depth, 1.0), (active, -maximum_depth)],
                ConstraintRelation::LessEqual, 0.0, &format!("{name}_depth_bound_{node}"))?);
            if let Some(facility) = construction.facility_variable(node) {
                self.rows.push(register_row(model, vec![(active, 1.0), (facility, -1.0)],
                    ConstraintRelation::LessEqual, 0.0, &format!("{name}_facility_{node}"))?);
            }
        }
        for arc in problem.design().flow().arcs().iter().filter(|arc| arc.from != arc.to
            && problem.tree_nodes().contains(&arc.from) && problem.tree_nodes().contains(&arc.to))
        {
            let index = model.register_auto_variable_with_range::<Binary>(
                &format!("{name}_arc_{}", arc.id), VariableRange::bounded(0.0, 1.0),
            ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            self.arcs.insert(arc.id.clone(), index);
            self.incoming.get_mut(&arc.to).ok_or_else(|| NetworkSchedulingError::contract("缺少树节点 / missing tree node"))?.push(arc.id.clone());
            if let Some(levels) = construction.service_variables(&arc.id) {
                let mut terms = vec![(index, 1.0)];
                terms.extend(levels.iter().map(|level| (*level, -1.0)));
                self.rows.push(register_row(model, terms, ConstraintRelation::LessEqual, 0.0,
                    &format!("{name}_service_{}", arc.id))?);
            }
            for node in [&arc.from, &arc.to] {
                self.rows.push(register_row(model, vec![(index, 1.0), (self.active[node], -1.0)],
                    ConstraintRelation::LessEqual, 0.0, &format!("{name}_endpoint_{}_{node}", arc.id))?);
            }
            self.rows.push(register_row(model,
                vec![(self.depths[&arc.to], 1.0), (self.depths[&arc.from], -1.0), (index, -relaxation)],
                ConstraintRelation::GreaterEqual, 1.0 - relaxation, &format!("{name}_order_{}", arc.id))?);
        }
        for node in problem.tree_nodes() {
            let mut terms = self.parent_expression(node)?;
            if node != problem.root() { terms.push((self.active[node], -1.0)); }
            self.rows.push(register_row(model, terms, ConstraintRelation::Equal, 0.0, &format!("{name}_parent_{node}"))?);
        }
        self.binding = Some(model.model_identity());
        Ok(())
    }

    pub(super) fn extract<V>(
        &self,
        model: &MetaModel<f64>,
        problem: &NetworkStructureProblem<V>,
    ) -> Result<RootedTreeSelection>
    where
        V: SolveValue + UnitConversionValue,
    {
        if self.binding != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("根树尚未注册或模型不匹配 / unregistered rooted tree or model mismatch"));
        }
        validate_rows(model, &self.rows)?;
        let mut nodes = BTreeSet::new();
        for (node, index) in &self.active {
            let active = binary(model, *index)?;
            if (node == problem.root() || problem.terminals().contains(node)) && !active {
                return Err(NetworkSchedulingError::contract("必选树节点未启用 / required tree node is inactive"));
            }
            let depth = result(model, self.depths[node])?;
            let maximum = if active && node != problem.root() { (problem.tree_nodes().len() - 1) as f64 } else { 0.0 };
            if depth < -1e-7 || depth > maximum + 1e-7 {
                return Err(NetworkSchedulingError::contract("树深度结果越界 / tree depth result out of bounds"));
            }
            if active { nodes.insert(node.clone()); }
        }
        let mut arcs = BTreeSet::new();
        let mut parents = BTreeMap::new();
        for (id, index) in &self.arcs {
            if !binary(model, *index)? { continue; }
            let arc = problem.design().flow().arc(id)
                .ok_or_else(|| NetworkSchedulingError::contract("未知树弧结果 / unknown tree-arc result"))?;
            if arc.to == *problem.root() || !nodes.contains(&arc.from) || !nodes.contains(&arc.to)
                || parents.insert(arc.to.clone(), id.clone()).is_some()
            {
                return Err(NetworkSchedulingError::contract("树父弧或活动端点结果非法 / invalid tree parent or active endpoints"));
            }
            arcs.insert(id.clone());
        }
        for node in nodes.iter().filter(|node| *node != problem.root()) {
            let mut current = node;
            let mut visited = BTreeSet::new();
            while current != problem.root() {
                if !visited.insert(current.clone()) {
                    return Err(NetworkSchedulingError::contract("树结果包含无根有向环 / tree result contains a rootless directed cycle"));
                }
                let arc = parents.get(current).and_then(|id| problem.design().flow().arc(id))
                    .ok_or_else(|| NetworkSchedulingError::contract("活动树节点没有根路径 / active tree node has no path from the root"))?;
                current = &arc.from;
            }
        }
        Ok(RootedTreeSelection { arcs, nodes, parents })
    }
}
