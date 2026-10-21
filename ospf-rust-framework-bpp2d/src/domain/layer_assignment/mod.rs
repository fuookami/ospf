//! 二维层分配领域 / 2D layer assignment domain
//!
//! 映射 Kotlin `bpp2d-domain-layer-assignment-context`。本层是 bpp2d 唯一允许出现求解器数值类型的
//! 建模边界：领域其余部分泛型于 `V`，这里统一收到 `f64`。
//! Maps the Kotlin `bpp2d-domain-layer-assignment-context` submodule. This layer is the only modelling
//! boundary in bpp2d where solver value types may appear: the rest of the domain is generic over `V` while
//! this layer normalises to `f64`.

pub mod context;
pub mod model;
pub mod service;

pub use context::{
    FinalAssignmentArtifacts, LayerAssignmentContext, RmpAssignmentArtifacts,
};
pub use model::*;
pub use service::*;
