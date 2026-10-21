//! 块装载服务 / Block loading services
//!
//! 简单块、复杂块、旋转守卫与两类空间切分搜索。
//! Simple blocks, complex blocks, the rotation guard and both space-splitting searches.

pub mod complex_block_generator;
pub mod rotation_guard;
pub mod simple_block_generator;
pub mod space_split_algorithms;

pub use complex_block_generator::ComplexBlockGenerator;
pub use rotation_guard::require_rotation_allowed;
pub use simple_block_generator::{SimpleBlockConfig, SimpleBlockGenerator};
pub use space_split_algorithms::{
    DepthFirstSearchAlgorithm2D, MultiRowHeuristicAlgorithm2D, SpaceSplit2Result,
};
