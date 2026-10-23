//! 通用网络流领域层 / Generic network-flow domain layer.

mod aggregation;
mod context;
mod extraction;
mod model;
pub mod oracle;
pub mod pipeline;

pub use aggregation::FlowAggregation;
pub use context::{FlowContext, FlowContextExtension, FlowRegistration};
pub use model::{
    CommodityId, FlowArc, FlowBalanceSlack, FlowCommodity, FlowProblem, FlowNode,
    FlowNodeCapacity, FlowResource, FlowSolution, FlowThroughput, FlowUnits,
};
pub use oracle::{IntegralFlowOracleResult, solve_bounded_integral_min_cost_flow};
pub use pipeline::{
    CapacityBoundsPipeline, FlowConservationPipeline, MinCostFlowObjectivePipeline,
};

#[cfg(test)]
mod tests;
