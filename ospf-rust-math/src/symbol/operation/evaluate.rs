//! 多项式求值 trait 定义
//! Polynomial evaluation trait definition
//!
//! 本模块定义多项式求值的 trait，具体实现在各多项式类型文件中。
//! This module defines the evaluation trait, implementations are in polynomial type files.
//!
//! # 实现位置 / Implementation Locations
//!
//! - `Linear`: `polynomial/linear.rs`
//! - `Quadratic`: `polynomial/quadratic.rs`
//! - `Canonical`: `polynomial/canonical.rs`

use std::collections::HashMap;
use std::ops::Add;
use num_traits::Zero;
use crate::operator::{MulRef, ZeroRef};
use crate::symbol::symbol::OwnedSymbol;

// ============================================================================
// Evaluatable - 可求值类型约束
// ============================================================================

/// 可求值类型约束 / Evaluatable type constraint
///
/// 封装多项式求值所需的核心约束。
/// Encapsulates core constraints required for polynomial evaluation.
///
/// # 约束说明 / Constraint Description
///
/// - `MulRef`: 支持引用乘法 `&T * &T -> T`
/// - `Mul<Output = T>`: 支持值乘法 `T * T -> T`
/// - `Add<Output = T>`: 支持加法 `T + T -> T`
/// - `Zero`: 支持零值
/// - `ZeroRef`: 支持零值引用 `T::zero_ref() -> &'static T`
///
/// # 注意 / Note
///
/// `Clone` 约束不在 `Evaluatable` 中，而是根据具体实现需要单独添加。
/// `Clone` constraint is not in `Evaluatable`, it's added separately based on implementation needs.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::Evaluatable;
///
/// fn evaluate_something<T: Evaluatable>(value: &T) -> T {
///     T::zero()  // 可以使用 Zero / Can use Zero
/// }
/// ```
pub trait Evaluatable:
    MulRef + std::ops::Mul<Output = Self> + Add<Output = Self> + Zero + ZeroRef + 'static
{
}

/// 为满足约束的类型自动实现 Evaluatable
/// Auto-implement Evaluatable for types satisfying constraints
impl<T: MulRef + std::ops::Mul<Output = T> + Add<Output = T> + Zero + ZeroRef + 'static> Evaluatable
    for T
{
}

/// 求值 trait / Evaluation trait
///
/// 给定符号值映射，计算多项式的值。
/// Evaluate polynomial given symbol value mapping.
///
/// # 示例 / Example
///
/// ```
/// use std::collections::HashMap;
/// use ospf_rust_math::symbol::{Linear, LinearMonomial, OwnedSymbol, DynSymbol, SymbolDynId, Evaluate};
/// use std::any::Any;
///
/// // 定义简单符号 / Define simple symbol
/// #[derive(Debug, Clone)]
/// struct SimpleSymbol { id: usize, name: String }
///
/// impl std::fmt::Display for SimpleSymbol {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.name) }
/// }
///
/// impl DynSymbol for SimpleSymbol {
///     fn name(&self) -> &str { &self.name }
///     fn display_name(&self) -> &str { &self.name }
///     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
///     fn as_any(&self) -> &dyn Any { self }
/// }
///
/// // 创建符号 / Create symbols
/// let x = OwnedSymbol::new(SimpleSymbol { id: 1, name: "x".to_string() });
/// let y = OwnedSymbol::new(SimpleSymbol { id: 2, name: "y".to_string() });
///
/// // 创建线性多项式 2x + 3y + 1 / Create linear polynomial 2x + 3y + 1
/// let poly = Linear::new(vec![
///     LinearMonomial::new(2.0, x.clone()),
///     LinearMonomial::new(3.0, y.clone()),
/// ], 1.0);
///
/// // 求值：x = 2, y = 3 / Evaluate: x = 2, y = 3
/// let values = HashMap::from([
///     (x, 2.0),
///     (y, 3.0),
/// ]);
///
/// let result = poly.evaluate(&values);
/// assert_eq!(result, 2.0 * 2.0 + 3.0 * 3.0 + 1.0);  // 14.0
/// ```
pub trait Evaluate<T> {
    /// 完全求值（所有符号都有值）
    /// Full evaluation (all symbols have values)
    ///
    /// # 参数 / Arguments
    /// - `values`: 符号到值的映射 / Symbol to value mapping
    ///
    /// # 返回 / Returns
    /// 多项式的值 / Value of the polynomial
    ///
    /// # 注意 / Note
    /// 如果符号不在映射中，其值视为零。
    /// If a symbol is not in the mapping, its value is treated as zero.
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable;

    /// 部分求值（返回可能简化的表达式）
    /// Partial evaluation (returns possibly simplified expression)
    ///
    /// 对已知符号进行代入，返回简化后的表达式。
    /// Substitutes known symbols and returns the simplified expression.
    ///
    /// # 参数 / Arguments
    /// - `values`: 符号到值的映射 / Symbol to value mapping
    ///
    /// # 返回 / Returns
    /// 简化后的多项式 / Simplified polynomial
    ///
    /// # 示例 / Example
    /// ```
    /// // 对于 2x + 3y + 1，给定 x = 2
    /// // 结果为 3y + 5
    /// ```
    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> Self
    where
        T: Evaluatable;
}

/// 有序求值 trait / Ordered evaluation trait
///
/// 支持从切片按顺序获取符号值。
/// Support getting symbol values from slice by order.
pub trait EvaluateOrdered<T> {
    /// 按符号顺序求值
    /// Evaluate with symbol values in order
    ///
    /// # 参数 / Arguments
    /// - `symbols`: 符号列表（定义顺序）/ Symbol list (defines order)
    /// - `values`: 值切片（按顺序对应符号）/ Value slice (corresponds to symbols by order)
    ///
    /// # 返回 / Returns
    /// 多项式的值 / Value of the polynomial
    ///
    /// # 注意 / Note
    /// 如果切片长度小于符号数量，缺失的符号视为零。
    /// If slice length is less than symbol count, missing symbols are treated as zero.
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable;
}
