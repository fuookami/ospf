//! 中间符号系统
//! Intermediate Symbol System

pub mod expression_symbol;
pub mod flatten;
pub mod function;
pub mod function_symbol;
#[doc(hidden)]
pub mod functions;
mod intermediate_symbol;
pub mod monomial_cell;
pub mod symbol_combination;
pub mod symbol_combination_factory;

pub use expression_symbol::*;
pub use function_symbol::*;
pub use intermediate_symbol::*;
pub use monomial_cell::*;
pub use symbol_combination::*;
pub use symbol_combination_factory::*;

// 函数符号主导出走 `function`，`functions` 仅保留兼容模块路径
// Function symbols are exported via `function`; `functions` stays as compatibility path only.
pub use function::*;
