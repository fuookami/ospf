//! 分配 context 委托同一建设/流内核，不复制建模规则。 / Allocation context delegates to the same design/flow kernel without copying modeling rules.

use std::sync::Arc;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::design::{NetworkDesignAggregation, NetworkDesignContext, NetworkDesignPipeline, NetworkDesignRegistration};
use crate::error::Result;
use super::{AllocationFlowKey, AllocationProblem, AllocationSolution};

/// 每模型独立的运输分配入口。 / Per-model transportation-allocation entry point.
#[derive(Debug, Clone)]
pub struct AllocationContext<V: SolveValue + UnitConversionValue> {
    problem: AllocationProblem<V>,
    design: NetworkDesignContext<V>,
}

impl<V: SolveValue + UnitConversionValue> AllocationContext<V> {
    /// 使用经验证问题创建独立建设/流 context。 / Create an independent design/flow context from a validated problem.
    pub fn new(problem: AllocationProblem<V>, name: impl Into<String>) -> Result<Self> {
        let design = NetworkDesignContext::new(problem.design().clone(), name)?;
        Ok(Self { problem, design })
    }

    /// 业务输入快照，包含完整地点与服务属性 / Business-input snapshot including complete location and service attributes.
    pub fn problem(&self) -> &AllocationProblem<V> { &self.problem }

    /// 直接公开下层建设、流和费用表达式，不创建平行聚合 / Directly expose underlying construction, flow and cost expressions without a parallel aggregation.
    pub fn aggregation(&self) -> &NetworkDesignAggregation<V> { self.design.aggregation() }

    /// 使用现有 Design pipeline 追加业务约束或目标。 / Append business constraints or objectives with existing Design pipelines.
    pub fn add_pipeline(&mut self, pipeline: Arc<dyn NetworkDesignPipeline<V>>) -> Result<()> { self.design.add_pipeline(pipeline) }

    /// 同一路径事务注册运输、配额、建设及可选总费用。 / Transactionally register transport, quotas, construction and optional total cost through the same path.
    pub fn register(&mut self, model: &mut MetaModel<f64>, options: NetworkDesignRegistration) -> Result<()> {
        self.design.register(model, options)
    }

    /// 下层生产提取复核全部守恒、整数步长、容量、配额与建设联动。 / Underlying production extraction validates all balances, integer steps, capacities, quotas and construction links.
    pub fn extract_solution(&mut self, model: &MetaModel<f64>) -> Result<AllocationSolution<V>> {
        let result = self.design.extract_solution(model)?;
        let shipments = result.flow.flows.iter().map(|((request, service), amount)| (
            AllocationFlowKey { request: request.clone(), service: service.clone() }, amount.clone(),
        )).collect();
        Ok(AllocationSolution { shipments, flow: result.flow, open_hubs: result.construction.open_facilities,
            service_levels: result.construction.service_levels, construction_cost: result.construction.cost, total_cost: result.total_cost })
    }
}
