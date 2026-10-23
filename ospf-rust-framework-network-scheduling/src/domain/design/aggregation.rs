//! 建设与独立流的组合，不重复实现流内核。 / Composition of construction and independent flows without duplicating the flow kernel.

use std::collections::BTreeMap;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::domain::flow::{FlowContext, FlowRegistration};
use crate::error::Result;
use crate::infrastructure::NetworkSchedulingSolverValueAdapter;
use super::construction::NetworkConstruction;
use super::model::{NetworkDesignProblem, NetworkDesignSolution};
use super::pipeline::{LinearRow, embedded_flow, link_flow, validate_rows};

/// 每模型独立的建设和名义流聚合。 / Per-model construction and nominal-flow aggregation.
#[derive(Debug, Clone)]
pub struct NetworkDesignAggregation<V: SolveValue + UnitConversionValue> {
    construction: NetworkConstruction<V>,
    flow: FlowContext<V>,
    rows: Vec<LinearRow>,
}

impl<V: SolveValue + UnitConversionValue> NetworkDesignAggregation<V> {
    /// 注册前完成单位化准备。 / Complete unit-normalization preparation before registration.
    pub fn new(problem: NetworkDesignProblem<V>, name: &str) -> Result<Self> {
        let flow = FlowContext::new(embedded_flow(problem.flow())?).with_name(format!("{name}_nominal"))?;
        Ok(Self { construction: NetworkConstruction::new(problem, name)?, flow, rows: Vec::new() })
    }

    /// 共享建设组件 / Shared construction component.
    pub fn construction(&self) -> &NetworkConstruction<V> { &self.construction }

    /// 独立名义流 context / Independent nominal flow context.
    pub fn flow(&self) -> &FlowContext<V> { &self.flow }

    /// 建设加运行费用表达式 / Construction plus operating-cost expression.
    pub fn cost_expression(&self) -> Result<Vec<(usize, f64)>> {
        let mut terms = self.construction.cost_expression().to_vec();
        terms.extend(self.flow.aggregation.cost_expression()?);
        Ok(terms)
    }

    pub(super) fn register(&mut self, model: &mut MetaModel<f64>, name: &str) -> Result<()> {
        self.construction.register(model)?;
        self.flow.register_with_options(model, FlowRegistration { minimize_cost: false })?;
        self.rows = link_flow(model, &self.construction, &self.flow, self.construction.problem().flow(), &BTreeMap::new(), name)?;
        Ok(())
    }

    pub(super) fn extract(&mut self, model: &MetaModel<f64>) -> Result<NetworkDesignSolution<V>> {
        let construction = self.construction.extract_solution(model)?;
        validate_rows(model, &self.rows)?;
        let flow = self.flow.extract_solution(model)?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let cost = adapter.to_solver(&construction.cost.value)? + adapter.to_solver(&flow.total_cost.value)?;
        Ok(NetworkDesignSolution { construction, flow,
            total_cost: Quantity::new(adapter.from_solver(cost)?, self.construction.problem().cost_unit().clone()) })
    }
}
