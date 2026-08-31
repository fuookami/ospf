//! 应用层：SSP 求解器 / Application layer: SSP solver

use crate::core::common::solve_typed as solve_meta_typed;
use crate::framework::demo1::bandwidth_context::BandwidthContext;
use crate::framework::demo1::infrastructure::dto::{Input, Output};
use crate::framework::demo1::route_context::RouteContext;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::ObjectiveCategory;
use std::error::Error;

/// SSP 求解器 / SSP solver
pub struct Ssp {
    /// 路由上下文 / Route context
    route_context: RouteContext,
    /// 带宽上下文 / Bandwidth context
    bandwidth_context: BandwidthContext,
}

impl Ssp {
    /// 创建新的 SSP 求解器 / Create a new SSP solver
    pub fn new() -> Self {
        Self {
            route_context: RouteContext::new(),
            bandwidth_context: BandwidthContext::new(),
        }
    }

    /// 求解带宽/路由优化问题 / Solve the bandwidth/route optimization problem
    pub fn solve(&mut self, input: Input) -> Result<Output, Box<dyn Error>> {
        self.route_context.init(&input)?;

        let mut model = MetaModel::<f64>::new("framework_demo1");
        model.set_objective_category(ObjectiveCategory::Minimum);

        self.route_context.register(&mut model)?;
        self.bandwidth_context
            .register(&mut model, &self.route_context)?;

        self.route_context.construct(&mut model)?;
        self.bandwidth_context
            .construct(&mut model, &self.route_context)?;

        let output = solve_meta_typed(model)?;
        let solution = output.solution;
        let links = self
            .bandwidth_context
            .analyze(&self.route_context, &solution)?;
        Ok(Output { links })
    }
}
