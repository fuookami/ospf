//! IIS (Irreducible Inconsistent Subsystem) 计算模块
//! IIS (Irreducible Inconsistent Subsystem) Computation Module
//!
//! 当优化模型不可行时，IIS 用于识别导致不可行的最小约束和变量边界集合。
//! When an optimization model is infeasible, IIS is used to identify the minimal
//! set of constraints and variable bounds causing infeasibility.
//!
//! # 核心概念 / Core Concepts
//!
//! - **IIS**: 不可约不一致子系统，即删除其中任何一个约束或边界后系统变为可行
//! - **IIS**: Irreducible Inconsistent Subsystem, meaning removing any constraint
//!   or bound from it makes the system feasible
//!
//! # 算法 / Algorithms
//!
//! - **弹性过滤 (Elastic Filtering)**: 通过添加松弛变量来识别 IIS
//! - **Elastic Filtering**: Identifies IIS by adding slack variables
//! - **删除过滤 (Deletion Filtering)**: 逐个删除约束来识别 IIS
//! - **Deletion Filtering**: Identifies IIS by removing constraints one by one

pub mod deletion_filtering;
pub mod elastic_filtering;
pub mod iis_config;
pub mod iis_model;

pub use deletion_filtering::*;
pub use elastic_filtering::*;
pub use iis_config::*;
pub use iis_model::*;

use crate::error::Result;
use crate::model::intermediate::BasicLinearTriadModel;

/// 计算 IIS / Compute IIS
///
/// 对给定的线性模型计算不可约不一致子系统。
/// Computes the irreducible inconsistent subsystem for the given linear model.
///
/// # 参数 / Parameters
/// - `model`: 线性三角模型 / Linear triad model
/// - `config`: IIS 配置 / IIS configuration
///
/// # 返回 / Returns
/// IIS 模型 / IIS model
///
/// # 示例 / Example
///
/// ```rust,no_run
/// use ospf_rust_core::solver::iis::{compute_iis, IISConfig};
/// use ospf_rust_core::model::intermediate::BasicLinearTriadModel;
///
/// // 假设有一个不可行的模型 / Assume an infeasible model
/// let model: BasicLinearTriadModel = unimplemented!();
///
/// let iis = compute_iis(&model, &IISConfig::default()).unwrap();
/// println!("IIS contains {} constraints", iis.num_constraints());
/// ```
pub fn compute_iis(model: &BasicLinearTriadModel, config: &IISConfig) -> Result<LinearIISModel> {
    match config.algorithm {
        IISAlgorithm::ElasticFiltering => elastic_filtering::compute_iis_elastic(model, config),
        IISAlgorithm::DeletionFiltering => deletion_filtering::compute_iis_deletion(model, config),
    }
}
