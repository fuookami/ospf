//! 维度无关的共享装载建模。 / Dimension-independent shared loading models.

pub mod domain;
pub mod infrastructure;
pub mod application;

pub use domain::discrete_loading::*;
pub use domain::partition_loading::*;
pub use infrastructure::*;
