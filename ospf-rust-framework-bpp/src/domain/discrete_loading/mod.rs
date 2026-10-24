//! 稀疏分配、整体占位与独立复核。 / Sparse assignment, whole-position occupancy and independent validation.

mod context;
mod model;
mod limits;

pub use context::*;
pub use model::*;
pub use limits::*;

#[cfg(test)]
mod tests;
