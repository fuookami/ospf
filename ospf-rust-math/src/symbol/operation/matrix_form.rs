//! 矩阵形式 Trait 定义
//! Matrix form trait definitions
//!
//! 本模块提供多项式矩阵形式的 trait 和结构体定义。
//! This module provides trait and struct definitions for polynomial matrix forms.

use std::ops::{Add, AddAssign, Div, Mul};
use num_traits::Zero;
use crate::symbol::OwnedSymbol;
use crate::symbol::{Linear, Quadratic};

// ============================================================================
// 矩阵形式结构体 / Matrix Form Structures
// ============================================================================

/// 线性多项式的矩阵形式 / Matrix form of linear polynomial
///
/// 表示线性多项式 `c^T * x + b`，其中：
/// Represents linear polynomial `c^T * x + b`, where:
/// - `symbols` 是变量向量 / `symbols` is the variable vector
/// - `coefficients` 是系数向量 c / `coefficients` is the coefficient vector c
/// - `constant` 是常数项 b / `constant` is the constant term b
///
/// # 示例 / Examples
///
/// 对于线性多项式 `2x + 3y + 1`，其中 `symbols = [x, y]`：
/// For linear polynomial `2x + 3y + 1`, where `symbols = [x, y]`:
/// - `coefficients = [2, 3]`
/// - `constant = 1`
#[derive(Clone, Debug, PartialEq)]
pub struct LinearMatrixForm<T> {
    /// 变量列表 / Variable list
    pub symbols: Vec<OwnedSymbol>,
    /// 系数向量 / Coefficient vector
    pub coefficients: Vec<T>,
    /// 常数项 / Constant term
    pub constant: T,
}

impl<T> LinearMatrixForm<T> {
    /// 创建线性矩阵形式 / Create a linear matrix form
    pub fn new(coefficients: Vec<T>, constant: T, symbols: Vec<OwnedSymbol>) -> Self {
        Self {
            symbols,
            coefficients,
            constant,
        }
    }

    /// Kotlin 命名兼容构造 / Kotlin naming compatible constructor
    pub fn from_kotlin_parts(c: Vec<T>, d: T, order: Vec<OwnedSymbol>) -> Self {
        Self::new(c, d, order)
    }

    /// 系数向量 c / Coefficient vector c
    pub fn c(&self) -> &[T] {
        &self.coefficients
    }

    /// 常数项 d / Constant term d
    pub fn d(&self) -> &T {
        &self.constant
    }

    /// 符号顺序 / Symbol order
    pub fn order(&self) -> &[OwnedSymbol] {
        &self.symbols
    }

    /// 拆解为 Kotlin 命名顺序 / Decompose in Kotlin naming order
    pub fn into_kotlin_parts(self) -> (Vec<T>, T, Vec<OwnedSymbol>) {
        (self.coefficients, self.constant, self.symbols)
    }
}

/// 二次多项式的矩阵形式 / Matrix form of quadratic polynomial
///
/// 表示二次多项式 `x^T * Q * x + c^T * x + b`，其中：
/// Represents quadratic polynomial `x^T * Q * x + c^T * x + b`, where:
/// - `symbols` 是变量向量 / `symbols` is the variable vector
/// - `q_matrix` 是二次项矩阵 Q（对称矩阵） / `q_matrix` is the quadratic term matrix Q (symmetric)
/// - `c_vector` 是线性项系数向量 c / `c_vector` is the linear term coefficient vector c
/// - `constant` 是常数项 b / `constant` is the constant term b
///
/// # 示例 / Examples
///
/// 对于二次多项式 `x^2 + 2xy + y^2 + 3x + 4y + 5`，其中 `symbols = [x, y]`：
/// For quadratic polynomial `x^2 + 2xy + y^2 + 3x + 4y + 5`, where `symbols = [x, y]`:
/// - `q_matrix = [[1, 2], [2, 1]]`（对称矩阵）
/// - `c_vector = [3, 4]`
/// - `constant = 5`
///
/// # 注意 / Notes
///
/// Q 矩阵是对称的，即 `Q[i][j] = Q[j][i]`。
/// The Q matrix is symmetric, i.e., `Q[i][j] = Q[j][i]`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticMatrixForm<T> {
    /// 变量列表 / Variable list
    pub symbols: Vec<OwnedSymbol>,
    /// 二次项矩阵（对称矩阵） / Quadratic term matrix (symmetric)
    pub q_matrix: Vec<Vec<T>>,
    /// 线性项系数向量 / Linear term coefficient vector
    pub c_vector: Vec<T>,
    /// 常数项 / Constant term
    pub constant: T,
}

impl<T> QuadraticMatrixForm<T> {
    /// 创建二次矩阵形式 / Create a quadratic matrix form
    pub fn new(
        q_matrix: Vec<Vec<T>>,
        c_vector: Vec<T>,
        constant: T,
        symbols: Vec<OwnedSymbol>,
    ) -> Self {
        Self {
            symbols,
            q_matrix,
            c_vector,
            constant,
        }
    }

    /// Kotlin 命名兼容构造 / Kotlin naming compatible constructor
    pub fn from_kotlin_parts(q: Vec<Vec<T>>, c: Vec<T>, d: T, order: Vec<OwnedSymbol>) -> Self {
        Self::new(q, c, d, order)
    }

    /// 二次项矩阵 q / Quadratic matrix q
    pub fn q(&self) -> &[Vec<T>] {
        &self.q_matrix
    }

    /// 线性项系数向量 c / Linear coefficient vector c
    pub fn c(&self) -> &[T] {
        &self.c_vector
    }

    /// 常数项 d / Constant term d
    pub fn d(&self) -> &T {
        &self.constant
    }

    /// 符号顺序 / Symbol order
    pub fn order(&self) -> &[OwnedSymbol] {
        &self.symbols
    }

    /// 拆解为 Kotlin 命名顺序 / Decompose in Kotlin naming order
    pub fn into_kotlin_parts(self) -> (Vec<Vec<T>>, Vec<T>, T, Vec<OwnedSymbol>) {
        (self.q_matrix, self.c_vector, self.constant, self.symbols)
    }
}

// ============================================================================
// 矩阵形式 Trait / Matrix Form Trait
// ============================================================================

/// 转换为矩阵形式 / Convert to matrix form
///
/// 将多项式转换为矩阵表示形式，便于数值计算。
/// Converts polynomial to matrix representation for numerical computation.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
///
/// # 设计说明 / Design Notes
///
/// 这个 trait 使用关联类型 `MatrixForm` 来指定转换后的矩阵形式类型，
/// 使得 `Linear` 和 `Quadratic` 可以有不同的矩阵形式。
/// This trait uses associated type `MatrixForm` to specify the converted matrix form type,
/// allowing `Linear` and `Quadratic` to have different matrix forms.
pub trait ToMatrixForm<T>: Sized {
    /// 矩阵形式类型 / Matrix form type
    type MatrixForm;

    /// 转换为矩阵形式
    /// Convert to matrix form
    ///
    /// # 参数 / Arguments
    ///
    /// - `symbols`: 变量顺序，决定了矩阵的维度和排列
    /// - `symbols`: Variable order, determines matrix dimension and arrangement
    fn to_matrix_form(&self, symbols: &[OwnedSymbol]) -> Self::MatrixForm;

    /// 从矩阵形式构建
    /// Build from matrix form
    ///
    /// # 参数 / Arguments
    ///
    /// - `form`: 矩阵形式
    /// - `form`: Matrix form
    fn from_matrix_form(form: &Self::MatrixForm) -> Self;
}

/// 从线性矩阵形式还原线性多项式 / Reconstruct linear polynomial from matrix form
pub fn linear_polynomial_from_matrix_form<T>(c: Vec<T>, d: T, order: Vec<OwnedSymbol>) -> Linear<T>
where
    T: Clone + Zero + PartialEq + for<'a> AddAssign<&'a T>,
{
    let form = LinearMatrixForm::from_kotlin_parts(c, d, order);
    <Linear<T> as ToMatrixForm<T>>::from_matrix_form(&form)
}

/// 从线性矩阵形式结构还原线性多项式
/// Reconstruct linear polynomial from a linear matrix form struct
pub fn linear_polynomial_from_linear_matrix_form<T>(form: &LinearMatrixForm<T>) -> Linear<T>
where
    T: Clone + Zero + PartialEq + for<'a> AddAssign<&'a T>,
{
    <Linear<T> as ToMatrixForm<T>>::from_matrix_form(form)
}

/// 从二次矩阵形式还原二次多项式 / Reconstruct quadratic polynomial from matrix form
pub fn quadratic_polynomial_from_matrix_form<T>(
    q: Vec<Vec<T>>,
    c: Vec<T>,
    d: T,
    order: Vec<OwnedSymbol>,
) -> Quadratic<T>
where
    T: Clone
        + Zero
        + PartialEq
        + for<'a> AddAssign<&'a T>
        + Add<Output = T>
        + Mul<Output = T>
        + Div<Output = T>,
{
    let form = QuadraticMatrixForm::from_kotlin_parts(q, c, d, order);
    <Quadratic<T> as ToMatrixForm<T>>::from_matrix_form(&form)
}

/// 从二次矩阵形式结构还原二次多项式
/// Reconstruct quadratic polynomial from a quadratic matrix form struct
pub fn quadratic_polynomial_from_quadratic_matrix_form<T>(
    form: &QuadraticMatrixForm<T>,
) -> Quadratic<T>
where
    T: Clone
        + Zero
        + PartialEq
        + for<'a> AddAssign<&'a T>
        + Add<Output = T>
        + Mul<Output = T>
        + Div<Output = T>,
{
    <Quadratic<T> as ToMatrixForm<T>>::from_matrix_form(form)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{LinearMonomial, QuadraticMonomial, test_utils::SimpleSymbol};

    fn make_symbol(id: usize, name: &str) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol::with_id(id, name))
    }

    #[test]
    fn linear_matrix_form_exposes_kotlin_names() {
        let x = make_symbol(1, "x");
        let y = make_symbol(2, "y");
        let form = LinearMatrixForm::from_kotlin_parts(vec![2.0, 3.0], 1.0, vec![x, y]);

        assert_eq!(form.c(), &[2.0, 3.0]);
        assert_eq!(*form.d(), 1.0);
        assert_eq!(form.order().len(), 2);
    }

    #[test]
    fn linear_polynomial_can_be_restored_from_kotlin_parts() {
        let x = make_symbol(1, "x");
        let y = make_symbol(2, "y");
        let polynomial =
            linear_polynomial_from_matrix_form(vec![2.0, 0.0], 1.0, vec![x.clone(), y]);

        assert_eq!(polynomial.constant, 1.0);
        assert_eq!(polynomial.monomials, vec![LinearMonomial::new(2.0, x)]);
    }

    #[test]
    fn quadratic_matrix_form_exposes_kotlin_names() {
        let x = make_symbol(1, "x");
        let y = make_symbol(2, "y");
        let form = QuadraticMatrixForm::from_kotlin_parts(
            vec![vec![1.0, 2.0], vec![2.0, 3.0]],
            vec![4.0, 5.0],
            6.0,
            vec![x, y],
        );

        assert_eq!(form.q()[0], vec![1.0, 2.0]);
        assert_eq!(form.c(), &[4.0, 5.0]);
        assert_eq!(*form.d(), 6.0);
        assert_eq!(form.order().len(), 2);
    }

    #[test]
    fn quadratic_polynomial_can_be_restored_from_kotlin_parts() {
        let x = make_symbol(1, "x");
        let y = make_symbol(2, "y");
        let polynomial = quadratic_polynomial_from_matrix_form(
            vec![vec![1.0, 2.0], vec![0.0, 3.0]],
            vec![4.0, 0.0],
            5.0,
            vec![x.clone(), y.clone()],
        );

        assert_eq!(polynomial.constant, 5.0);
        assert_eq!(
            polynomial.monomials,
            vec![
                QuadraticMonomial::quadratic(1.0, x.clone(), x),
                QuadraticMonomial::quadratic(2.0, make_symbol(1, "x"), y.clone()),
                QuadraticMonomial::quadratic(3.0, y.clone(), y),
                QuadraticMonomial::linear(4.0, make_symbol(1, "x")),
            ]
        );
    }
}
