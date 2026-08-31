//! 表达式平展系统模块 / Expression Flatten System Module
//!
//! 本模块提供将复杂表达式展开为纯变量多项式的能力。
//! This module provides the ability to expand complex expressions into pure variable polynomials.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`CacheKey`] - 缓存键，用于标识平展结果
//! - [`Cacheable`] - 可缓存 trait，拥有稳定内存地址的对象
//! - [`LinearMonomial`] - 线性单项式
//! - [`Linear`] - 线性多项式
//! - [`QuadraticMonomial`] - 二次单项式
//! - [`Quadratic`] - 二次多项式
//! - [`FlattenContextTrait`] - 平展上下文 trait
//! - [`Flattenable`] - 可平展 trait

pub mod cache_key;
pub mod flatten_context;
pub mod flattenable;
pub mod inner_box;
pub mod lazy_context;

pub use cache_key::*;
pub use flatten_context::*;
pub use flattenable::*;
pub use inner_box::*;
pub use lazy_context::*;
