//! 二次四角模型视图 trait（Kotlin 对齐）
//! Quadratic tetrad model view trait (Kotlin-aligned)

use super::{QuadraticTetradModel, SparseMatrix};

pub trait QuadraticTetradModelView {
    fn name(&self) -> &str;
    fn num_variables(&self) -> usize;
    fn num_constraints(&self) -> usize;
    fn num_quadratic_constraints(&self) -> usize;
    fn linear_objective(&self) -> &[f64];
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
