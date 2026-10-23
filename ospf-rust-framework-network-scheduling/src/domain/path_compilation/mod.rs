//! 完整路径的有限二元主问题、列生命周期和来源可核对的定价对偶。 / Finite binary whole-path masters, column lifecycle and provenance-checked pricing duals.

mod aggregation;
mod compilation;
mod context;
mod model;
mod pricing;

pub use aggregation::PathCompilationAggregation;
pub use compilation::PathCompilation;
pub use context::{PathCompilationContext, PathCompilationPipeline};
pub use model::{CompiledPathColumn, PathColumn, PathCompilationProblem, PathCompilationRow, PathCompilationSolution};
pub use pricing::PathCompilationDuals;

#[cfg(test)]
pub(crate) mod tests;
