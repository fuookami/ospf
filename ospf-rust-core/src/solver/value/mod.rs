//! 求解值模块根（Kotlin 对齐）/ Solve value module root (Kotlin-aligned)
//!
//! 定义求解值的类型约束、转换策略、边界处理和校验辅助。
//! Defines solve value type constraints, conversion policies, boundary handling, and validation helpers.

/// 边界转换 / Boundary conversion
pub mod boundary;
/// 转换上下文 / Conversion context
pub mod conversion_context;
/// 转换策略 / Conversion policy
pub mod conversion_policy;
/// 求解值类型约束 / Solve value type constraints
pub mod solve_value;
/// 校验辅助 / Validation helpers
pub mod validation;

pub use boundary::*;
pub use conversion_context::*;
pub use solve_value::*;
pub use validation::*;
