//! 线性三角模型视图 trait（Kotlin 对齐）
//! Linear triad model view trait (Kotlin-aligned)

use super::LinearTriadModel;

pub trait LinearTriadModelView {
    fn name(&self) -> &str;
    fn num_variables(&self) -> usize;
    fn num_constraints(&self) -> usize;
    fn objective(&self) -> &[f64];
}

impl LinearTriadModelView for LinearTriadModel {
    fn name(&self) -> &str {
        &self.basic.name
    }

    fn num_variables(&self) -> usize {
        self.basic.num_variables()
    }

    fn num_constraints(&self) -> usize {
        self.basic.num_constraints()
    }

    fn objective(&self) -> &[f64] {
        &self.c
    }
}
