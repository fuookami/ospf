//! 符号运算模块
//! Symbolic computation module
//!
//! 本模块提供符号运算功能，支持线性规划、二次规划等优化问题。
//! This module provides symbolic computation capabilities for optimization problems
//! such as linear programming and quadratic programming.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`symbol`] - 符号定义（SymbolId, DynSymbol, Symbol, OwnedSymbol）
//! - [`monomial`] - 单项式（LinearMonomial, QuadraticMonomial, CanonicalMonomial）
//! - [`polynomial`] - 多项式（Linear, Quadratic, Canonical）
//! - [`operation`] - 运算操作（ToLinear, ToQuadratic, ToCanonical）
//! - [`inequality`] - 不等式（Comparison, LinearInequality, QuadraticInequality, CanonicalInequality）
//!
//! # 构造宏 / Construction Macros
//!
//! 本模块提供一系列宏用于简化单项式、多项式、不等式的构造：
//! This module provides macros to simplify construction of monomials, polynomials, and inequalities:
//!
//! - `symbols!` - 定义符号 / Define symbols
//! - `linear_monomial!` - 构造线性单项式 / Construct linear monomial
//! - `quadratic_monomial!` - 构造二次单项式 / Construct quadratic monomial
//! - `canonical_monomial!` - 构造标准单项式 / Construct canonical monomial
//! - `linear!` - 构造线性多项式 / Construct linear polynomial
//! - `quadratic!` - 构造二次多项式 / Construct quadratic polynomial
//! - `canonical!` - 构造标准多项式 / Construct canonical polynomial
//! - `linear_inequality!` - 构造线性不等式 / Construct linear inequality
//! - `quadratic_inequality!` - 构造二次不等式 / Construct quadratic inequality
//! - `canonical_inequality!` - 构造标准不等式 / Construct canonical inequality
//!
//! 注意：`Exponent` trait 定义在 [`crate::operator::Exponent`]。
//! Note: The `Exponent` trait is defined in [`crate::operator::Exponent`].

pub mod category;
pub mod expression;
pub mod inequality;
pub mod monomial;
pub mod operation;
pub mod polynomial;
pub mod symbol;

// 序列化模块（可选，需要启用 serde feature）
// Serialization module (optional, requires serde feature)
#[cfg(feature = "serde")]
pub mod serde;

// 解析器模块（可选，需要启用 parser feature）
// Parser module (optional, requires parser feature)
#[cfg(feature = "parser")]
pub mod parser;

// 宏模块（宏通过 #[macro_export] 自动导出到 crate 根）
// Macros module (macros are automatically exported to crate root via #[macro_export])
#[macro_use]
pub mod macros;

pub use category::*;
pub use expression::*;
pub use symbol::*;
// 允许模糊的 glob 重导出（monomial 和 inequality 模块都有 linear/quadratic/canonical 子模块）
// Allow ambiguous glob re-exports (monomial and inequality modules both have linear/quadratic/canonical submodules)
#[allow(ambiguous_glob_reexports)]
pub use inequality::*;
#[allow(ambiguous_glob_reexports)]
pub use monomial::*;
pub use operation::*;
pub use polynomial::*;

/// 测试工具模块 / Test utilities module
///
/// 提供测试用的 SimpleSymbol 实现。
/// Provides SimpleSymbol implementation for testing.
pub mod test_utils {
    use crate::symbol::{DynSymbol, SymbolDynId};
    use std::any::Any;
    use std::fmt::{Display, Formatter, Result};

    /// 简单符号（仅用于测试）/ Simple symbol (for testing only)
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SimpleSymbol {
        id: usize,
        name: String,
    }

    impl Display for SimpleSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "{}", self.name)
        }
    }

    impl SimpleSymbol {
        /// 创建新的简单符号
        /// Create a new simple symbol
        pub fn new(name: &str) -> Self {
            Self {
                id: 0,
                name: name.to_string(),
            }
        }

        /// 创建带 id 的简单符号
        /// Create a simple symbol with id
        pub fn with_id(id: usize, name: &str) -> Self {
            Self {
                id,
                name: name.to_string(),
            }
        }
    }

    impl DynSymbol for SimpleSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }
}
