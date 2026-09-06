//! # 组合数学模块 / Combinatorics Module
//!
//! 提供组合数学相关算法：
//! Provides combinatorics algorithms:
//!
//! - [`combinations`] - 生成所有子集组合 / Generate all subset combinations
//! - [`cross`] - 多列表笛卡尔积 / Cartesian product of multiple lists
//! - [`permutations`] - 生成所有排列 / Generate all permutations

mod combinations;
mod cross;
mod permutations;

pub use combinations::*;
pub use cross::*;
pub use permutations::*;
