//! BPP2D 领域层 / BPP2D domain layer
//!
//! 与 Kotlin 的 `bpp2d-domain-*-context` 子模块一一对应。领域层泛型于 `V` 并保持物理量化；
//! 求解器数值类型只出现在 layer-assignment 建模边界与应用层。
//! Corresponds one-to-one with the Kotlin `bpp2d-domain-*-context` submodules. The domain layer is generic
//! over `V` and physically quantified; solver value types appear only in the layer-assignment modelling
//! boundary and the application layer.

pub mod bla;
pub mod block_loading;
pub mod item;
pub mod layer_assignment;
pub mod layer_generation;
pub mod packing;
