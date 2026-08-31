//! 中间模型层模块 / Intermediate Model Layer Module
//!
//! 本模块提供求解器可理解的标准形式模型。
//! This module provides standard form models that solvers can understand.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`BasicLinearTriadModel`] - 基本线性三角模型（无目标函数）
//! - [`LinearTriadModel`] - 线性三角模型（带目标函数）
//! - [`BasicQuadraticTetradModel`] - 基本二次四角模型（无目标函数）
//! - [`QuadraticTetradModel`] - 二次四角模型（带目标函数）

pub mod basic_linear_triad_model;
pub mod basic_quadratic_tetrad_model;
pub mod elastic;
pub mod linear_triad_model;
pub mod linear_triad_model_view;
pub mod lp_export;
pub mod quadratic_tetrad_model;
pub mod quadratic_tetrad_model_view;

pub use basic_linear_triad_model::{
    BasicLinearTriadModel, BasicLinearTriadModelF64, SparseMatrix, SparseVector,
};
pub use basic_quadratic_tetrad_model::{BasicQuadraticTetradModel, BasicQuadraticTetradModelF64};
pub use elastic::{LinearElasticBuilder, QuadraticElasticBuilder};
pub use linear_triad_model::{LinearTriadModel, LinearTriadModelF64};
pub use linear_triad_model_view::LinearTriadModelView;
pub use lp_export::{
    DumpOptions, LPExportableModel, ModelFileFormat, dump_batch, dump_lp_batch, dump_opm_batch,
};
pub use quadratic_tetrad_model::{QuadraticTetradModel, QuadraticTetradModelF64};
pub use quadratic_tetrad_model_view::QuadraticTetradModelView;
