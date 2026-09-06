//! 符号微分 trait 定义
//! Symbolic differentiation trait definition
//!
//! 本模块定义符号微分的 trait，具体实现在各多项式类型文件中。
//! This module defines the differentiation trait, implementations are in polynomial type files.
//!
//! # 实现位置 / Implementation Locations
//!
//! - `Linear`: `polynomial/linear.rs`
//! - `Quadratic`: `polynomial/quadratic.rs`
//! - `Canonical`: `polynomial/canonical.rs`

use crate::symbol::OwnedSymbol;
use num_traits::Zero;

/// 微分 trait / Differentiation trait
///
/// 对多项式求偏导数。
/// Compute partial derivatives of polynomials.
///
/// # 类型关系 / Type Relationships
///
/// - `Linear` 的偏导是常数 `T`
/// - `Quadratic` 的偏导是 `Linear<T>`
/// - `Canonical` 的偏导是 `Canonical<T, E>`
///
/// # 示例 / Example
///
/// ```
/// use std::collections::HashMap;
/// use ospf_rust_math::symbol::{Linear, LinearMonomial, OwnedSymbol, DynSymbol, SymbolDynId, Differentiate};
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
/// // 对 x 求偏导 / Partial derivative with respect to x
/// let dx = poly.partial_derivative(&x);
/// // 结果: 2.0 (常数) / Result: 2.0 (constant)
///
/// // 对 y 求偏导 / Partial derivative with respect to y
/// let dy = poly.partial_derivative(&y);
/// // 结果: 3.0 (常数) / Result: 3.0 (constant)
/// ```
pub trait Differentiate<T> {
    /// 偏导数的类型 / Type of partial derivative
    ///
    /// - 对于 `Linear<T>`，偏导是 `T`（常数）
    /// - 对于 `Quadratic<T>`，偏导是 `Linear<T>`
    /// - 对于 `Canonical<T, E>`，偏导是 `Canonical<T, E>`
    type Derivative;

    /// 对指定符号求偏导
    /// Partial derivative with respect to the given symbol
    ///
    /// # 参数 / Arguments
    /// - `symbol`: 要求导的符号 / Symbol to differentiate with respect to
    ///
    /// # 返回 / Returns
    /// 偏导数
    /// Partial derivative
    fn partial_derivative(&self, symbol: &OwnedSymbol) -> Self::Derivative
    where
        T: Zero + for<'a> std::ops::AddAssign<&'a T>;

    /// 对所有符号求梯度
    /// Gradient with respect to all symbols
    ///
    /// # 参数 / Arguments
    /// - `symbols`: 符号列表 / Symbol list
    ///
    /// # 返回 / Returns
    /// 梯度向量（每个符号对应一个偏导数）
    /// Gradient vector (one partial derivative per symbol)
    fn gradient(&self, symbols: &[OwnedSymbol]) -> Vec<Self::Derivative>
    where
        T: Zero + for<'a> std::ops::AddAssign<&'a T>,
    {
        symbols.iter().map(|s| self.partial_derivative(s)).collect()
    }
}

/// 二阶微分 trait / Second-order differentiation trait
///
/// 支持计算 Hessian 矩阵。
/// Supports computing Hessian matrix.
pub trait SecondOrderDifferentiate<T>: Differentiate<T> {
    /// 计算 Hessian 矩阵
    /// Compute Hessian matrix
    ///
    /// # 参数 / Arguments
    /// - `symbols`: 符号列表 / Symbol list
    ///
    /// # 返回 / Returns
    /// Hessian 矩阵（二维向量，H[i][j] = ∂²f/∂xᵢ∂xⱼ）
    /// Hessian matrix (2D vector, H[i][j] = ∂²f/∂xᵢ∂xⱼ)
    ///
    /// 注意：对于 Quadratic，二阶导数是常数，返回 `Vec<Vec<T>>`
    /// Note: For Quadratic, second derivative is constant, returns `Vec<Vec<T>>`
    fn hessian(&self, symbols: &[OwnedSymbol]) -> Vec<Vec<T>>
    where
        T: Zero + for<'a> std::ops::AddAssign<&'a T>;
}
