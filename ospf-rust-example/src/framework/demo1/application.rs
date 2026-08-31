use std::error::Error;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::ObjectiveCategory;
use crate::core::common::solve_typed as solve_meta_typed;
use crate::framework::demo1::bandwidth_context::BandwidthContext;
use crate::framework::demo1::infrastructure::dto::{Input, Output};
use crate::framework::demo1::route_context::RouteContext;

/// SSP 求解器 / SSP solver
pub struct Ssp {
    route_context: RouteContext,
    bandwidth_context: BandwidthContext,
}

impl Ssp {
    pub fn new() -> Self {
        Self {
            route_context: RouteContext::new(),
            bandwidth_context: BandwidthContext::new(),
        }
    }

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
