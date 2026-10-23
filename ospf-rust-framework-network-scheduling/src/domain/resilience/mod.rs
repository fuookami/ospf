//! 共享建设、显式故障与独立连通辅助流。 / Shared construction, explicit failures and independent connectivity witnesses.

mod aggregation;
mod connectivity;
mod context;
mod model;

pub use aggregation::NetworkResilienceAggregation;
pub use context::{NetworkResilienceContext, NetworkResiliencePipeline};
pub use model::{ConnectivityKind, ConnectivityRequirement, FailureScenario, NetworkResilienceProblem, NetworkResilienceSolution};

#[cfg(test)]
mod tests;
