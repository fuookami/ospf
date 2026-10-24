//! 根有向树、Steiner 中转与设施分区归属。 / Rooted directed trees, Steiner relays and facility-partition ownership.

mod aggregation;
mod context;
mod coverage;
mod model;
mod tree;

pub use aggregation::NetworkStructureAggregation;
pub use context::{NetworkStructureContext, NetworkStructurePipeline, NetworkStructureRegistration};
pub use coverage::FacilityCoverageComponent;
pub use model::{FacilityAssignmentQuota, FacilityPartitionRequest, NetworkStructureProblem, NetworkStructureSolution};
pub use tree::RootedTreeComponent;

#[cfg(test)]
pub(crate) mod tests;
