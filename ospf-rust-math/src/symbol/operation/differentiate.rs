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

// ============================================================================
// 测试 / Tests
// ============================================================================

/// 本模块只声明 trait，实现分散在 `polynomial/{linear,quadratic,canonical}.rs`。
/// 因此这里**通过真实实现**校验 trait 契约，而不是测一个空壳。
///
/// This module only declares traits; the implementations live in
/// `polynomial/{linear,quadratic,canonical}.rs`. The trait contract is therefore verified
/// **through the real implementations** rather than against an empty shell.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::operation::ToCanonical;
    use crate::symbol::test_utils::SimpleSymbol;
    use crate::symbol::{Canonical, Linear, LinearMonomial, Quadratic, QuadraticMonomial};

    /// 构造一个具名测试符号 / Build a named test symbol.
    fn sym(id: usize, name: &str) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol::with_id(id, name))
    }

    /// 把线性式摊平成 (符号名, 系数) 列表，便于顺序无关的比较。
    /// Flatten a linear form into (symbol name, coefficient) pairs for order-free comparison.
    fn linear_terms(poly: &Linear<f64>) -> Vec<(String, f64)> {
        poly.monomials
            .iter()
            .map(|m| (m.symbol.to_string(), m.coefficient))
            .collect()
    }

    /// 断言线性式包含给定项（顺序无关） / Assert a linear form contains a term, order-free.
    fn assert_has_term(poly: &Linear<f64>, name: &str, coefficient: f64, context: &str) {
        let terms = linear_terms(poly);
        assert!(
            terms.contains(&(name.to_string(), coefficient)),
            "{context}: 期望含 {coefficient}{name}，实际 {terms:?}"
        );
    }

    #[test]
    fn linear_partial_derivative_is_the_matching_coefficient() {
        // 线性式 ∂/∂x(2x + 3y + 1) = 2，且与常数项无关。
        // For a linear form ∂/∂x(2x + 3y + 1) = 2, independent of the constant term.
        let x = sym(1, "x");
        let y = sym(2, "y");
        let poly = Linear::new(
            vec![
                LinearMonomial::new(2.0_f64, x.clone()),
                LinearMonomial::new(3.0_f64, y.clone()),
            ],
            1.0,
        );

        assert_eq!(poly.partial_derivative(&x), 2.0);
        assert_eq!(poly.partial_derivative(&y), 3.0);
    }

    #[test]
    fn linear_partial_derivative_of_absent_symbol_is_zero() {
        // 对未出现的符号求偏导必须为 0，而不是 panic 或返回常数项。
        // Differentiating with respect to a symbol that does not occur must yield 0,
        // not panic and not return the constant term.
        let x = sym(1, "x");
        let absent = sym(9, "absent");
        let poly = Linear::new(vec![LinearMonomial::new(5.0_f64, x)], 7.0);

        assert_eq!(poly.partial_derivative(&absent), 0.0);
    }

    #[test]
    fn zero_linear_form_has_zero_derivative_everywhere() {
        // 零多项式的偏导恒为 0 / The zero form differentiates to zero everywhere.
        let x = sym(1, "x");
        let poly = Linear::<f64>::zero();

        assert_eq!(poly.partial_derivative(&x), 0.0);
    }

    #[test]
    fn linear_gradient_follows_the_symbol_order() {
        // 默认 gradient 必须按传入符号顺序逐个求偏导（本模块提供的默认实现）。
        // The default gradient must differentiate in the given symbol order; this is the
        // default implementation supplied by this module.
        let x = sym(1, "x");
        let y = sym(2, "y");
        let poly = Linear::new(
            vec![
                LinearMonomial::new(2.0_f64, x.clone()),
                LinearMonomial::new(3.0_f64, y.clone()),
            ],
            0.0,
        );

        assert_eq!(poly.gradient(&[x.clone(), y.clone()]), vec![2.0, 3.0]);
        assert_eq!(poly.gradient(&[y, x]), vec![3.0, 2.0], "顺序必须被尊重");
        assert_eq!(poly.gradient(&[]), Vec::<f64>::new(), "空符号表给出空梯度");
    }

    #[test]
    fn quadratic_partial_derivative_matches_hand_computation() {
        // 二次式 f = x² + 3xy + 2y + 5：
        //   ∂f/∂x = 2x + 3y
        //   ∂f/∂y = 3x + 2
        // 结果应是 Linear<T>（一阶导降次）。
        //
        // For f = x² + 3xy + 2y + 5:
        //   ∂f/∂x = 2x + 3y and ∂f/∂y = 3x + 2, each a Linear<T> (degree drops by one).
        let x = sym(1, "x");
        let y = sym(2, "y");
        let poly = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(1.0_f64, x.clone(), x.clone()),
                QuadraticMonomial::quadratic(3.0_f64, x.clone(), y.clone()),
                QuadraticMonomial::linear(2.0_f64, y.clone()),
            ],
            5.0,
        );

        let dx = poly.partial_derivative(&x);
        assert_eq!(dx.constant, 0.0, "∂f/∂x 不含常数项");
        assert_has_term(&dx, "x", 2.0, "∂f/∂x");
        assert_has_term(&dx, "y", 3.0, "∂f/∂x");

        let dy = poly.partial_derivative(&y);
        assert_eq!(dy.constant, 2.0, "∂f/∂y 的常数项应为 2");
        assert_has_term(&dy, "x", 3.0, "∂f/∂y");
    }

    #[test]
    fn quadratic_partial_derivative_of_absent_symbol_is_zero() {
        // 未出现符号的偏导必须是零线性式 / An absent symbol must yield the zero linear form.
        let x = sym(1, "x");
        let absent = sym(9, "absent");
        let poly = Quadratic::new(
            vec![QuadraticMonomial::quadratic(1.0_f64, x.clone(), x)],
            0.0,
        );

        let derivative = poly.partial_derivative(&absent);
        assert_eq!(derivative.constant, 0.0);
        assert!(
            derivative.monomials.is_empty(),
            "对未出现符号求导应得到零线性式，实际 {:?}",
            derivative.monomials
        );
    }

    #[test]
    fn quadratic_gradient_agrees_with_partial_derivatives() {
        // gradient 必须与逐个 partial_derivative 的结果一致。
        // gradient must agree with calling partial_derivative one symbol at a time.
        let x = sym(1, "x");
        let y = sym(2, "y");
        let poly = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(2.0_f64, x.clone(), x.clone()),
                QuadraticMonomial::quadratic(4.0_f64, x.clone(), y.clone()),
            ],
            0.0,
        );

        let symbols = [x, y];
        let gradient = poly.gradient(&symbols);
        assert_eq!(gradient.len(), 2);
        for (index, symbol) in symbols.iter().enumerate() {
            let direct = poly.partial_derivative(symbol);
            assert_eq!(
                gradient[index].constant, direct.constant,
                "第 {index} 个分量的常数项应与直接求偏导一致"
            );
            let from_gradient = linear_terms(&gradient[index]);
            let from_direct = linear_terms(&direct);
            assert_eq!(
                from_gradient.len(),
                from_direct.len(),
                "第 {index} 个分量的项数应一致"
            );
            for term in &from_direct {
                assert!(
                    from_gradient.contains(term),
                    "第 {index} 个分量缺少项 {term:?}"
                );
            }
        }
    }

    #[test]
    fn quadratic_partial_derivative_of_a_square_applies_the_power_rule() {
        // 回归：d/dx(c·x²) 必须是 2c·x。
        // 曾经的实现直接搬运系数，得到 c·x，且与 hessian 给出的 2c 自相矛盾。
        //
        // Regression: d/dx(c·x²) must be 2c·x. The previous implementation copied the
        // coefficient verbatim, yielding c·x and contradicting hessian's 2c.
        let x = sym(1, "x");

        for coefficient in [1.0_f64, 3.0, -2.5] {
            let poly = Quadratic::new(
                vec![QuadraticMonomial::quadratic(
                    coefficient,
                    x.clone(),
                    x.clone(),
                )],
                0.0,
            );

            let derivative = poly.partial_derivative(&x);
            assert_has_term(&derivative, "x", 2.0 * coefficient, "d/dx(c·x²)");
            assert_eq!(derivative.constant, 0.0, "d/dx(c·x²) 不含常数项");
            assert_eq!(derivative.monomials.len(), 1, "d/dx(c·x²) 只有一项");

            // 二阶导必须与一阶导一致：hessian 元素为 2c。
            // The second derivative must agree with the first: the Hessian entry is 2c.
            let hessian = poly.hessian(&[x.clone()]);
            assert_eq!(hessian[0][0], 2.0 * coefficient, "hessian 与一阶导必须一致");
        }
    }

    #[test]
    fn quadratic_mixed_term_is_not_doubled() {
        // 混合项 c·xy 的偏导不带幂次因子：∂/∂x = c·y，∂/∂y = c·x。
        // A mixed term c·xy carries no power factor: ∂/∂x = c·y and ∂/∂y = c·x.
        let x = sym(1, "x");
        let y = sym(2, "y");
        let poly = Quadratic::new(
            vec![QuadraticMonomial::quadratic(4.0_f64, x.clone(), y.clone())],
            0.0,
        );

        let dx = poly.partial_derivative(&x);
        assert_has_term(&dx, "y", 4.0, "∂(4xy)/∂x");
        assert_eq!(dx.monomials.len(), 1);

        let dy = poly.partial_derivative(&y);
        assert_has_term(&dy, "x", 4.0, "∂(4xy)/∂y");
        assert_eq!(dy.monomials.len(), 1);

        // 混合项的 hessian 非对角元素是 c（不是 2c）。
        // For a mixed term the off-diagonal Hessian entry is c, not 2c.
        let hessian = poly.hessian(&[x, y]);
        assert_eq!(hessian[0][1], 4.0);
        assert_eq!(hessian[1][0], 4.0);
    }

    #[test]
    fn canonical_partial_derivative_drops_degree_by_one() {
        // Canonical 的偏导仍是 Canonical；对 x² 求导应得到 2x（无常数项）。
        // A Canonical derivative stays Canonical; differentiating x² yields 2x with no
        // constant term.
        let x = sym(1, "x");
        let quadratic = Quadratic::new(
            vec![QuadraticMonomial::quadratic(1.0_f64, x.clone(), x.clone())],
            0.0,
        );
        let poly: Canonical<f64, i32> = quadratic.to_canonical();

        let derivative = poly.partial_derivative(&x);
        assert_eq!(derivative.constant, 0.0, "x² 的导数不含常数项");
        assert_eq!(derivative.monomials.len(), 1, "d/dx(x²) 只有一项");
        let monomial = &derivative.monomials[0];
        assert_eq!(monomial.coefficient, 2.0);
        let powers: Vec<(String, i32)> = monomial
            .powers
            .iter()
            .map(|(symbol, exponent)| (symbol.to_string(), *exponent))
            .collect();
        assert_eq!(powers, vec![("x".to_string(), 1)], "符号 x 的次数应降为 1");
    }

    #[test]
    fn canonical_partial_derivative_of_absent_symbol_is_zero() {
        // 未出现符号的偏导必须为零多项式 / An absent symbol differentiates to the zero form.
        let x = sym(1, "x");
        let absent = sym(9, "absent");
        let quadratic = Quadratic::new(
            vec![QuadraticMonomial::quadratic(1.0_f64, x.clone(), x)],
            0.0,
        );
        let poly: Canonical<f64, i32> = quadratic.to_canonical();

        let derivative = poly.partial_derivative(&absent);
        assert_eq!(derivative.constant, 0.0);
        assert!(
            derivative.monomials.is_empty(),
            "对未出现符号求导应得到零多项式，实际 {:?}",
            derivative.monomials
        );
    }
}
