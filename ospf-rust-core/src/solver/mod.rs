//! 求解器接口模块 / Solver Interface Module
//!
//! 本模块提供优化求解器的统一接口。
//! This module provides unified interface for optimization solvers.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`SolverInfo`] - 求解器基础信息 trait
//! - [`LinearSolver`] - 线性模型求解 trait
//! - [`QuadraticSolver`] - 二次模型求解 trait
//! - [`Solver`] - 组合求解器 trait（线性 + 二次）
//! - [`SolverConfig`] - 求解器配置
//! - [`SolverOutput`] - 求解结果
//! - [`SolverStatus`] - 求解状态
//!
//! # 支持的求解器 / Supported Solvers
//!
//! - [`solvers::GurobiSolver`] - Gurobi 求解器（需要 `gurobi` feature）
//! - [`solvers::SCIPSolver`] - SCIP 求解器（需要 `scip` feature）
//!
//! # IIS 计算 / IIS Computation
//!
//! - [`iis::compute_iis`] - 计算 IIS
//! - [`iis::IISConfig`] - IIS 配置
//! - [`iis::LinearIISModel`] - IIS 模型
//!
//! # 启发式算法 / Heuristic Algorithms
//!
//! - [`heuristic::Individual`] - 个体 trait
//! - [`heuristic::Population`] - 种群

pub mod heuristic;
pub mod iis;
pub mod solver;
pub mod solver_config;
pub mod solver_ext;
pub mod solver_output;
pub mod solvers;
pub mod value;

pub use solver::*;
pub use solver_config::*;
pub use solver_ext::*;
pub use solver_output::*;
pub use value::*;
