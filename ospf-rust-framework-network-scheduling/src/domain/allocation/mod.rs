//! 多请求运输分配、兼容、层级、共享配额与枢纽建设。 / Multi-request transportation allocation, compatibility, layers, shared quotas and hub construction.

mod context;
mod model;

pub use context::AllocationContext;
pub use model::{AllocationFlowKey, AllocationHub, AllocationLocation, AllocationOptions, AllocationProblem,
    AllocationQuota, AllocationRequest, AllocationService, AllocationSolution};

#[cfg(test)]
pub(crate) mod tests;
