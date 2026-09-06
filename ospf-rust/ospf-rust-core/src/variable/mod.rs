//! 变量系统模块 / Variable System Module
//!
//! 本模块提供优化问题中决策变量的定义和管理。
//! This module provides definition and management of decision variables in optimization problems.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`VariableType`] - 变量类型枚举 / Variable type enum
//! - [`VariableTypeTrait`] - 变量类型标记 trait / Variable type marker trait
//! - [`VariableId`] - 变量唯一标识符 / Variable unique identifier
//! - [`VariableRange`] - 变量取值范围 / Variable value range
//! - [`VariableItem`] - 泛型变量项 / Generic variable item
//! - [`VariableArena`] - 变量内存池 / Variable arena

pub mod variable_arena;
pub mod variable_combination;
pub mod variable_id;
pub mod variable_item;
pub mod variable_range;
pub mod variable_type;

pub use variable_arena::*;
pub use variable_combination::*;
pub use variable_id::*;
pub use variable_item::*;
pub use variable_range::*;
pub use variable_type::*;
