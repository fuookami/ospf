//! 模型框架模块
//! Model Framework Module
//!
//! 本模块提供运筹学建模框架的高级管道和 Shadow Price 管理。
//! This module provides high-level pipelines and shadow price management for operations research modeling.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`Pipeline`] - 基础管道 trait / Basic pipeline trait
//! - [`CGPipeline`] - 列生成管道 trait / Column generation pipeline trait
//! - [`HAPipeline`] - 启发式算法管道 trait / Heuristic algorithm pipeline trait
//! - [`ShadowPrice`] - Shadow Price 数据结构 / Shadow Price data structure
//! - [`ShadowPriceMap`] - Shadow Price 映射表 / Shadow Price map
//! - [`IndexedVariableCombination1`] - 一维索引变量组合 / 1D indexed variable combination
//! - [`IndexedVariableCombination2`] - 二维索引变量组合 / 2D indexed variable combination
//! - [`IndexedLinearExpressionSymbols1`] - 一维索引符号组合 / 1D indexed symbol combination
//! - [`IndexedLinearExpressionSymbols2`] - 二维索引符号组合 / 2D indexed symbol combination
//! - [`OptionalIndexedVariableArray`] - 稀疏索引变量数组 / Sparse indexed variable array
//! - [`OptionalIndexedLinearExpressionSymbols`] - 稀疏索引符号组合 / Sparse indexed symbol combination
//! - [`AppendableVariablePool`] - 可追加变量池 / Appendable variable pool
//! - [`AppendableSymbolPool`] - 可追加符号池 / Appendable symbol pool

pub mod appendable_pool;
pub mod dynamic_lifecycle;
pub mod indexed_combination;
pub mod optional_array;
pub mod pipeline;
pub mod shadow_price;

pub use appendable_pool::*;
pub use dynamic_lifecycle::*;
pub use indexed_combination::*;
pub use optional_array::*;
pub use pipeline::*;
pub use shadow_price::*;
