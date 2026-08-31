//! VRPTW 领域层 / VRPTW domain layer.

mod branching;
mod model;
mod policy;
mod pricing_duals;
mod validator;

pub use branching::{BranchMask, ResourceArc, VehicleAssignment};
pub use model::{
    Coordinate, Customer, CustomerId, Depot, Route, RouteStop, ServiceTimeWindow, TravelTime,
    VehicleType, VehicleTypeId, VrpNode, VrptwArc, VrptwInstance, VrptwSolution, VrptwTolerances,
    VrptwUnits, coordinate_node, default_arc_id, scheduling_window,
};
pub use policy::{
    ArcCostCalculator, ArcFeasibilityPolicy, DefaultArcFeasibilityPolicy, Demo17CostPolicy,
    DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, DistanceCalculator,
    EuclideanDistanceCalculator, FixedPlusArcCostPolicy, RouteCostPolicy,
    SolomonDistanceCalculator, TravelTimeCalculator,
};
pub use pricing_duals::{PricingDuals, PricingPhase, VrpShadowPriceMap};
pub use validator::{RouteValidationPolicy, RouteValidator, VrptwValidator};
