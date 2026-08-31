//! Quantity - 物理量
//! Quantity - Physical quantities
//!
//! 提供运行时和编译时的物理量表示和运算
//! Provides runtime and compile-time physical quantity representations and operations
//!
//! # 模块结构 / Module Structure
//! - `quantity`: 运行时物理量 `Quantity`
//! - `ct_quantity`: 编译时物理量 `CTQuantity`
//!
//! # 核心特性 / Key Features
//! - 类型安全的单位运算
//! - 编译时量纲检查（`CTQuantity`）
//! - 运行时单位转换（`Quantity`）

pub mod quantity;
pub mod ct_quantity;

// ============================================================================
// 重导出 / Re-exports
// ============================================================================

pub use quantity::Quantity;
pub use ct_quantity::CTQuantity;
