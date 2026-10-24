//! 物理量输入、候选生成、分配与完整结果。 / Quantity inputs, candidate generation, assignment and complete results.

mod assignment;
mod discrete_loading;
mod generation;
mod model;
mod packing;

pub use assignment::*;
pub use discrete_loading::*;
pub use generation::*;
pub use model::*;
pub use packing::*;
