//! 构造宏模块 / Construction macros module
//!
//! 提供简化单项式、多项式、不等式构造的宏系统。
//! Provides macro system for simplifying monomial, polynomial, and inequality construction.
//!
//! **注 / Note**: 宏通过 `#[macro_export]` 自动导出到 crate 根，无需 `pub use`。
//! Macros are automatically exported to crate root via `#[macro_export]`, no `pub use` needed.
//!
//! # 数学表达式宏 / Math Expression Macros
//!
//! 推荐使用更接近数学语言的宏：
//! Recommended macros with syntax closer to mathematical language:
//!
//! ## 多项式 / Polynomials
//!
//! - `lin!` - 线性多项式：`lin!(2 * x + 3 * y + 1)`
//! - `quad!` - 二次多项式：`quad!(x ^ 2 + 2 * x * y)`
//!
//! ## 不等式 / Inequalities
//!
//! - `ineq!` - 线性不等式：`ineq!(lin!(2 * x) <= 5.0)`
//! - `qineq!` - 二次不等式：`qineq!(quad!(x ^ 2) <= 1.0)`
//! - `cineq!` - 标准不等式：`cineq!(poly >= 0.0)`
//!
//! ## 约束集合 / Constraint Sets
//!
//! - `constraints!` - 约束集合：`constraints![ineq!(...), ineq!(...)]`
//!
//! # 旧版构造宏（仍可使用）/ Legacy Construction Macros (Still Available)
//!
//! - `linear_monomial!` - 构造线性单项式
//! - `quadratic_monomial!` - 构造二次单项式
//! - `linear!` - 构造线性多项式
//! - `quadratic!` - 构造二次多项式

mod inequality;
mod monomial;
mod ops;
mod polynomial;
mod symbol;