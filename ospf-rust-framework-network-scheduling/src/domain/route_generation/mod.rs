//! 路线生成领域层 / Route-generation domain layer.

#[cfg(feature = "big-decimal")]
mod exact;
mod graph;
mod initial_routes;
mod model;
mod policy;
mod pricer;

#[cfg(feature = "big-decimal")]
pub use exact::{
    ExactBigDecimalEspprcPricer, ExactBigDecimalPricingArc, ExactBigDecimalPricingDuals,
    ExactBigDecimalPricingGraph, ExactBigDecimalPricingNode, ExactBigDecimalPricingRequest,
    ExactBigDecimalPricingResult, ExactBigDecimalRouteGraphBuilder,
};
pub use graph::{PricingArc, PricingGraph, PricingNode, RouteGraphBuilder};
pub use initial_routes::InitialRouteGenerator;
pub use model::{
    CancellationToken, EspprcLabel, ForbiddenCustomers, LabelStatistics, PricingDiagnostic,
    PricingRequest, PricingResult, TruncationReason, VehiclePricingDiagnostic, VisitedCustomers,
};
pub use policy::{
    DefaultLabelDominancePolicy, DefaultPricingColumnSelector, LabelDominancePolicy,
    PricingColumnSelector,
};
pub use pricer::EspprcPricer;
