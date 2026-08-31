//! 代数结构模块
//! Algebraic structure module
//!
//! 本模块提供完整的代数结构层次定义，从半群到域，以及向量空间、赋范空间等。
//! This module provides complete algebraic structure hierarchy definitions,
//! from semigroups to fields, as well as vector spaces, normed spaces, etc.
//!
//! # 子模块 / Submodules
//!
//! - [`concept`] - 代数概念 traits（群、环、域、向量空间等）
//!   Algebraic concept traits (groups, rings, fields, vector spaces, etc.)
//! - [`law`] - 代数定律采样验证器（结合律、交换律、分配律等）
//!   Algebraic law sampling validators (associativity, commutativity, distributivity, etc.)
//! - [`value_range`] - 值空间与区间定义
//!   Value ranges and interval definitions

pub mod concept;
pub mod law;
pub mod value_range;

pub use concept::*;
pub use law::*;
