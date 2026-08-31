//! 矩阵形式 Trait 定义
//! Matrix form trait definitions
//!
//! 本模块提供多项式矩阵形式的 trait 和结构体定义。
//! This module provides trait and struct definitions for polynomial matrix forms.

use crate::symbol::OwnedSymbol;

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
