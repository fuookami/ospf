//! 启发式算法模块
//! Heuristic Algorithm Module
//!
//! 本模块提供进化算法的基础设施，包括个体、种群、遗传算子等。
//! This module provides infrastructure for evolutionary algorithms,
//! including individuals, populations, and genetic operators.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`Individual`] - 个体 trait
//! - [`Population`] - 种群
//! - [`SolutionWithFitness`] - 带适应度的解
//! - 交叉算子 / Crossover operators
//! - 变异算子 / Mutation operators
//! - 选择算子 / Selection operators
//! - 迁移算子 / Migration operators

pub mod algorithm;
pub mod cross;
pub mod individual;
pub mod iteration;
pub mod migration;
pub mod mutation;
pub mod normalization;
pub mod policy;
pub mod population;
pub mod selection;
pub mod solution_fitness;

pub use algorithm::*;
pub use cross::*;
pub use individual::*;
pub use iteration::*;
pub use migration::*;
pub use mutation::*;
pub use normalization::*;
pub use policy::*;
pub use population::*;
pub use selection::*;
pub use solution_fitness::*;
