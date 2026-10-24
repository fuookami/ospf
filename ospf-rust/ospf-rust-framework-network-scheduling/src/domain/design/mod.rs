//! 设施、通道等级与共享建设决策。 / Facilities, service levels and shared construction decisions.

mod aggregation;
mod construction;
mod context;
mod model;
pub(crate) mod pipeline;

pub use aggregation::NetworkDesignAggregation;
pub use construction::NetworkConstruction;
pub use context::{NetworkDesignContext, NetworkDesignPipeline, NetworkDesignRegistration};
pub use model::{CapacityLevel, NetworkConstructionSolution, NetworkDesignProblem, NetworkDesignSolution, NetworkFacility, NetworkService};

#[cfg(test)]
pub(crate) mod tests;
