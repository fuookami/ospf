//! # ospf-rust-quantities
//! 
//! 物理量、量纲和单位系统
//! Physical quantities, dimensions and units system
//! 
//! 支持运行时和编译时的量纲运算
//! Supports both runtime and compile-time dimension operations

pub mod scale;
pub mod dimension;
pub mod unit;
pub mod quantity;
pub mod error;

// 重导出常用类型 / Re-export common types
pub use error::{DimensionMismatchError, UnitConversionError};
