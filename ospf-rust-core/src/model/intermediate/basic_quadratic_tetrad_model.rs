//! 基本二次四角模型
//! Basic Quadratic Tetrad Model

use super::{BasicLinearTriadModel, SparseMatrix};

/// 基本二次四角模型 / Basic Quadratic Tetrad Model
///
/// 只包含变量和约束的标准形式，不包含目标函数。
/// Standard form with only variables and constraints, without objective.
///
/// 标准形式: Ax ≤ b, x ∈ [lb, ub]
/// Standard form: Ax ≤ b, x ∈ [lb, ub]
#[derive(Debug, Clone)]
pub struct BasicQuadraticTetradModel {
    /// 基本线性部分 / Basic linear part
    pub linear: BasicLinearTriadModel,
}

impl BasicQuadraticTetradModel {
    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            linear: BasicLinearTriadModel::new(name),
        }
    }

    /// 从基本线性三角模型创建 / Create from basic linear triad model
    pub fn from_linear(linear: BasicLinearTriadModel) -> Self {
        Self { linear }
    }

    /// 获取变量数量 / Get variable count
    pub fn num_variables(&self) -> usize {
        self.linear.num_variables()
    }

    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.linear.num_constraints()
    }

    /// 克隆约束结构 / Clone constraint structure
    pub fn clone_constraints(&self) -> (SparseMatrix<f64>, Vec<f64>) {
        self.linear.clone_constraints()
    }
}

impl Default for BasicQuadraticTetradModel {
    fn default() -> Self {
        Self::new("default")
    }
}

/// f64 精度的基本二次四角模型 / Basic quadratic tetrad model with f64 precision
pub type BasicQuadraticTetradModelF64 = BasicQuadraticTetradModel;
