//! 多目标算子 / Multi-objective operators
//!
//! 多目标优化没有全序，只有「支配」这一偏序；本模块以支配关系为唯一底层判据，向上构造非支配排序、拥挤距离、
//! Pareto 档案与参考点。
//! Multi-objective optimization has no total order, only the dominance partial order. This module treats dominance as the
//! single bottom-most predicate and builds non-dominated sorting, crowding distance, the Pareto archive, and reference
//! points on top of it.

pub mod dominance;
pub mod non_dominated_sort;
pub mod pareto_archive;
pub mod reference_points;

pub use dominance::{Dominance, MinimizationDominance, ParetoDominance};
pub use non_dominated_sort::{CrowdingDistance, FastNonDominatedSort};
pub use pareto_archive::ParetoArchive;
pub use reference_points::ReferencePoints;
