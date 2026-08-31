//! IIS 模型定义
//! IIS Model Definition

use super::ConstraintSource;
use std::collections::HashSet;

/// 线性 IIS 模型 / Linear IIS Model
///
/// 存储不可约不一致子系统的结果。
/// Stores the result of irreducible inconsistent subsystem.
#[derive(Debug, Clone)]
pub struct LinearIISModel {
    /// IIS 中的约束来源 / Constraint sources in IIS
    pub sources: HashSet<ConstraintSource>,
    /// 约束索引列表 / Constraint index list
    pub constraint_indices: Vec<usize>,
    /// 变量下界索引列表 / Variable lower bound index list
    pub lower_bound_indices: Vec<usize>,
    /// 变量上界索引列表 / Variable upper bound index list
    pub upper_bound_indices: Vec<usize>,
    /// 原始模型的引用信息 / Reference info to original model
    pub original_num_constraints: usize,
    pub original_num_variables: usize,
    /// 计算时间 / Computation time
    pub computation_time: std::time::Duration,
}

impl LinearIISModel {
    /// 创建空 IIS 模型 / Create empty IIS model
    pub fn new(num_constraints: usize, num_variables: usize) -> Self {
        Self {
            sources: HashSet::new(),
            constraint_indices: Vec::new(),
            lower_bound_indices: Vec::new(),
            upper_bound_indices: Vec::new(),
            original_num_constraints: num_constraints,
            original_num_variables: num_variables,
            computation_time: std::time::Duration::ZERO,
        }
    }

    /// 添加约束 / Add constraint
    pub fn add_constraint(&mut self, index: usize) {
        self.sources.insert(ConstraintSource::Constraint(index));
        if !self.constraint_indices.contains(&index) {
            self.constraint_indices.push(index);
        }
    }

    /// 添加变量下界 / Add variable lower bound
    pub fn add_lower_bound(&mut self, index: usize) {
        self.sources.insert(ConstraintSource::LowerBound(index));
        if !self.lower_bound_indices.contains(&index) {
            self.lower_bound_indices.push(index);
        }
    }

    /// 添加变量上界 / Add variable upper bound
    pub fn add_upper_bound(&mut self, index: usize) {
        self.sources.insert(ConstraintSource::UpperBound(index));
        if !self.upper_bound_indices.contains(&index) {
            self.upper_bound_indices.push(index);
        }
    }

    /// 获取 IIS 元素总数 / Get total IIS elements count
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// 是否为空 / Is empty
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.constraint_indices.len()
    }

    /// 获取边界数量 / Get bound count
    pub fn num_bounds(&self) -> usize {
        self.lower_bound_indices.len() + self.upper_bound_indices.len()
    }

    /// 检查约束是否在 IIS 中 / Check if constraint is in IIS
    pub fn contains_constraint(&self, index: usize) -> bool {
        self.sources.contains(&ConstraintSource::Constraint(index))
    }

    /// 检查边界是否在 IIS 中 / Check if bound is in IIS
    pub fn contains_bound(&self, source: &ConstraintSource) -> bool {
        self.sources.contains(source)
    }

    /// 设置计算时间 / Set computation time
    pub fn set_computation_time(&mut self, duration: std::time::Duration) {
        self.computation_time = duration;
    }

    /// 合并另一个 IIS 模型 / Merge another IIS model
    pub fn merge(&mut self, other: &LinearIISModel) {
        for &idx in &other.constraint_indices {
            self.add_constraint(idx);
        }
        for &idx in &other.lower_bound_indices {
            self.add_lower_bound(idx);
        }
        for &idx in &other.upper_bound_indices {
            self.add_upper_bound(idx);
        }
    }

    /// 生成报告 / Generate report
    pub fn report(&self) -> IISReport {
        IISReport {
            total_elements: self.len(),
            constraints: self.num_constraints(),
            lower_bounds: self.lower_bound_indices.len(),
            upper_bounds: self.upper_bound_indices.len(),
            computation_time: self.computation_time,
        }
    }
}

impl Default for LinearIISModel {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// IIS 报告 / IIS Report
#[derive(Debug, Clone)]
pub struct IISReport {
    /// 总元素数 / Total elements
    pub total_elements: usize,
    /// 约束数 / Constraints
    pub constraints: usize,
    /// 下界数 / Lower bounds
    pub lower_bounds: usize,
    /// 上界数 / Upper bounds
    pub upper_bounds: usize,
    /// 计算时间 / Computation time
    pub computation_time: std::time::Duration,
}

impl std::fmt::Display for IISReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IIS Report:")?;
        writeln!(f, "  Total elements: {}", self.total_elements)?;
        writeln!(f, "  Constraints: {}", self.constraints)?;
        writeln!(f, "  Lower bounds: {}", self.lower_bounds)?;
        writeln!(f, "  Upper bounds: {}", self.upper_bounds)?;
        writeln!(f, "  Computation time: {:?}", self.computation_time)?;
        Ok(())
    }
}

/// 基本线性三角模型视图 trait / Basic Linear Triad Model View Trait
///
/// 提供 IIS 算法所需的模型操作接口。
/// Provides model operation interface needed by IIS algorithms.
pub trait BasicLinearTriadModelView {
    /// 获取变量数量 / Get variable count
    fn num_variables(&self) -> usize;

    /// 获取约束数量 / Get constraint count
    fn num_constraints(&self) -> usize;

    /// 检查模型是否可行 / Check if model is feasible
    fn is_feasible(&self) -> bool;

    /// 移除约束并返回是否改变了可行性 / Remove constraint and return if feasibility changed
    fn remove_constraint(&mut self, index: usize) -> bool;

    /// 恢复约束 / Restore constraint
    fn restore_constraint(&mut self, index: usize);

    /// 移除变量边界并返回是否改变了可行性 / Remove variable bound and return if feasibility changed
    fn remove_bound(&mut self, var_index: usize, is_lower: bool) -> bool;

    /// 恢复变量边界 / Restore variable bound
    fn restore_bound(&mut self, var_index: usize, is_lower: bool);
}

// 类型别名 / Type aliases
/// f64 精度的线性 IIS 模型（默认）/ Linear IIS model with f64 precision (default)
pub type LinearIISModelF64 = LinearIISModel;
