//! 通用网络流领域层 / Generic network-flow domain layer.

mod aggregation;
mod context;
mod model;
pub mod oracle;
pub mod pipeline;

pub use aggregation::FlowAggregation;
pub use context::{FlowContext, FlowContextExtension};
pub use model::{
    CommodityId, FlowArc, FlowCommodity, FlowGraph, FlowNode, FlowUnits, SupplyDemand,
};
pub use oracle::{IntegralFlowOracleResult, solve_bounded_integral_min_cost_flow};
pub use pipeline::{
    CapacityBoundsPipeline, FlowConservationPipeline, MinCostFlowObjectivePipeline,
};
