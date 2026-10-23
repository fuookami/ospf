//! 仅组合建设、根树和覆盖组件，不创建 FlowContext。 / Compose construction, rooted-tree and coverage components without creating a FlowContext.

use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::design::NetworkConstruction;
use crate::error::Result;
use super::{FacilityCoverageComponent, NetworkStructureProblem, NetworkStructureSolution, RootedTreeComponent};

/// 每模型独立的结构聚合。 / Per-model independent structure aggregation.
#[derive(Debug, Clone)]
pub struct NetworkStructureAggregation<V: SolveValue + UnitConversionValue> {
    problem: NetworkStructureProblem<V>,
    construction: NetworkConstruction<V>,
    tree: RootedTreeComponent,
    coverage: FacilityCoverageComponent,
}

impl<V: SolveValue + UnitConversionValue> NetworkStructureAggregation<V> {
    pub(super) fn new(problem: NetworkStructureProblem<V>, name: &str) -> Result<Self> {
        let construction = NetworkConstruction::new(problem.design().clone(), name)?;
        Ok(Self { problem, construction, tree: Default::default(), coverage: Default::default() })
    }

    /// 只读业务问题 / Read-only business problem.
    pub fn problem(&self) -> &NetworkStructureProblem<V> { &self.problem }

    /// 不含商品流的建设组件 / Construction component without commodity flows.
    pub fn construction(&self) -> &NetworkConstruction<V> { &self.construction }

    /// 根树组件和公开索引 / Rooted-tree component and public indices.
    pub fn tree(&self) -> &RootedTreeComponent { &self.tree }

    /// 分区覆盖组件和公开表达式 / Partition-coverage component and public expressions.
    pub fn coverage(&self) -> &FacilityCoverageComponent { &self.coverage }

    /// 纯建设费用，不含名义运输或资源费用 / Construction-only cost, excluding nominal transportation and resource costs.
    pub fn cost_expression(&self) -> &[(usize, f64)] { self.construction.cost_expression() }

    /// 树弧表达式；已知但不合格的弧恒为零，未知弧失败。 / Tree-arc expression; known ineligible arcs are fixed zero, unknown arcs fail.
    pub fn selected_tree_arc(&self, arc: &crate::infrastructure::NetworkArcId) -> Result<Vec<(usize, f64)>> {
        if self.problem.design().flow().arc(arc).is_none() {
            return Err(crate::error::NetworkSchedulingError::structure("未知树弧 / unknown tree arc"));
        }
        if self.tree.active_variables().is_empty() {
            return Err(crate::error::NetworkSchedulingError::contract("树尚未注册 / tree is not registered"));
        }
        Ok(self.tree.arc_variables().get(arc).map(|index| vec![(*index, 1.0)]).unwrap_or_default())
    }

    pub(super) fn register(&mut self, model: &mut MetaModel<f64>, name: &str) -> Result<()> {
        self.construction.register(model)?;
        self.tree.register(model, &self.construction, &self.problem, name)?;
        self.coverage.register(model, &self.construction, &self.problem, name)?;
        Ok(())
    }

    pub(super) fn extract(&self, model: &MetaModel<f64>) -> Result<NetworkStructureSolution<V>> {
        let construction = self.construction.extract_solution(model)?;
        let tree = self.tree.extract(model, &self.problem)?;
        let partition_owners = self.coverage.extract(model, &self.problem)?;
        Ok(NetworkStructureSolution {
            construction, selected_tree_arcs: tree.arcs, connected_nodes: tree.nodes,
            parent_arcs: tree.parents, partition_owners,
        })
    }
}
