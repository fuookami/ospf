//! 有限候选活动排程，不启动任务束列生成。 / Finite-candidate activity scheduling, without bunch column generation.

pub mod aggregation;
pub mod compilation;
pub mod context;
pub mod extraction;
pub mod inventory;
pub mod model;
pub mod numeric;
pub mod pipeline;
pub mod validation;

pub use aggregation::{ActivityCompilation, ActivitySchedulingAggregation};
pub use compilation::{ActivityPreparation, PreparedActivityChoice};
pub use context::{ActivitySchedulingConfiguration, ActivitySchedulingContext};
pub use extraction::{ActivityCompiledModel, ActivitySolveExecution};
pub use model::*;

#[cfg(test)]
mod tests;
