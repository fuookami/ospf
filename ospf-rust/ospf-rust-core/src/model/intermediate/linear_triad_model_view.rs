//! 线性三角模型视图 trait（Kotlin 对齐）
//! Linear triad model view trait (Kotlin-aligned)

use super::LinearTriadModel;

/// 线性三角模型视图 / Linear triad model view
///
/// 提供对线性三角模型的只读访问接口
/// Read-only access interface for a linear triad model
pub trait LinearTriadModelView {
    /// 模型名称 / Model name
    fn name(&self) -> &str;

    /// 变量数量 / Number of variables
    fn num_variables(&self) -> usize;

    /// 约束数量 / Number of constraints
    fn num_constraints(&self) -> usize;

    /// 目标函数系数向量 / Objective function coefficient vector
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
