//! Unit - 单位
//! Unit - Physical units
//!
//! 提供运行时和编译时的单位表示，支持单位运算和转换。
//! Provides runtime and compile-time unit representations, supporting unit operations and conversions.
//!
//! # 模块结构 / Module Structure
//! - `concept`: 单位 super trait (`UnitTrait`)
//! - `physical_unit`: 核心单位类型（运行时 `Unit` 和编译时 `CTUnit`）
//! - `system`: 单位制定义（SI、MKS、CGS 等）
//! - `derived`: 预定义的导出单位（米、千克、秒等）
//!
//! # 核心特性 / Key Features
//! - 零成本抽象：编译时单位类型在编译期完成所有计算
//! - 类型安全：编译时检查量纲匹配
//! - 灵活转换：支持运行时和编译时单位转换
//! - 统一接口：`UnitTrait` 统一编译时和运行时单位

pub mod concept;
pub mod derived;
pub mod physical_unit;
pub mod system;

// ============================================================================
// 重导出核心类型 / Re-export core types
// ============================================================================

// 从 concept 模块重导出
pub use concept::UnitTrait;

// 从 physical_unit 模块重导出
pub use physical_unit::{
    ct_conversion_factor, CTUnit, CTUnitDiv, CTUnitMul, CTUnitPow,
    CTUnitReciprocal, Unit, UnitBuilder, UnitInner,
};

// 从 system 模块重导出
pub use system::{ConcreteUnitSystem, UnitSystem, UnitSystemBuilder};

// 从 derived 模块重导出所有单位
pub use derived::*;
