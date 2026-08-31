//! 模型框架模块
//! Model Framework Module
//!
//! 本模块提供运筹学建模框架的高级管道和 Shadow Price 管理。
//! This module provides high-level pipelines and shadow price management for operations research modeling.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`Pipeline`] - 基础管道 trait
//! - [`CGPipeline`] - 列生成管道 trait
//! - [`HAPipeline`] - 启发式算法管道 trait
//! - [`ShadowPrice`] - Shadow Price 数据结构
//! - [`ShadowPriceMap`] - Shadow Price 映射表

pub mod dynamic_lifecycle;
pub mod pipeline;
pub mod shadow_price;

pub use dynamic_lifecycle::*;
pub use pipeline::*;
pub use shadow_price::*;
