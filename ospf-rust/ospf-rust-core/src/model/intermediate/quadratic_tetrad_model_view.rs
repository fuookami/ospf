//! 二次四角模型视图 trait（Kotlin 对齐）
//! Quadratic tetrad model view trait (Kotlin-aligned)

use super::{QuadraticTetradModel, SparseMatrix};

/// 二次四角模型视图 / Quadratic tetrad model view
///
/// 提供对二次四角模型的只读访问接口
/// Read-only access interface for a quadratic tetrad model
pub trait QuadraticTetradModelView {
    /// 模型名称 / Model name
    fn name(&self) -> &str;

    /// 变量数量 / Number of variables
    fn num_variables(&self) -> usize;

    /// 约束数量 / Number of constraints
    fn num_constraints(&self) -> usize;

    /// 二次约束数量 / Number of quadratic constraints
    fn num_quadratic_constraints(&self) -> usize;

    /// 线性目标函数系数向量 / Linear objective coefficient vector
    fn linear_objective(&self) -> &[f64];

    /// 二次目标函数矩阵 / Quadratic objective matrix
    fn quadratic_objective(&self) -> &SparseMatrix<f64>;
}

impl QuadraticTetradModelView for QuadraticTetradModel {
    fn name(&self) -> &str {
        &self.basic.linear.name
    }

    fn num_variables(&self) -> usize {
        self.basic.num_variables()
    }

    fn num_constraints(&self) -> usize {
        self.basic.num_constraints()
    }

    fn num_quadratic_constraints(&self) -> usize {
        self.quadratic_constraints.len()
    }

    fn linear_objective(&self) -> &[f64] {
        &self.c
    }

    fn quadratic_objective(&self) -> &SparseMatrix<f64> {
        &self.Q
    }
}
