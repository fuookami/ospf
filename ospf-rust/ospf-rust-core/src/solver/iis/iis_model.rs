//! IIS 模型定义 / IIS Model Definition

use super::{ConstraintSource, IISAlgorithm};
use crate::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, LinearTriadModelView};
use crate::solver::audit::LinearModelMapping;
use crate::solver::{
    InfeasibilityEvidence, InfeasibilityEvidenceMember, InfeasibilityEvidenceSource,
    InfeasibilityMinimality, ProofCompleteness, ProofReliability,
};
use std::collections::HashSet;

/// legacy IIS 分析证据等级 / Legacy IIS analysis evidence level.
///
/// 该类型明确区分算法可靠性和执行完整度；它不把 legacy 过滤结果伪装成
/// native exact IIS。/ This type separates algorithm reliability from execution
/// completeness; it never presents a legacy filtering result as a native exact IIS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IISAnalysis {
    /// 使用的 legacy 算法 / Legacy algorithm used.
    pub algorithm: IISAlgorithm,
    /// 证据可靠性 / Evidence reliability.
    pub reliability: ProofReliability,
    /// 执行完整度 / Execution completeness.
    pub completeness: ProofCompleteness,
    /// 冲突集合最小性 / Conflict-set minimality.
    pub minimality: InfeasibilityMinimality,
}

impl IISAnalysis {
    /// 创建删除过滤分析结果 / Create a deletion-filtering analysis result.
    pub fn deletion_filtering(completed: bool) -> Self {
        Self {
            algorithm: IISAlgorithm::DeletionFiltering,
            reliability: ProofReliability::Reliable,
            completeness: if completed {
                ProofCompleteness::Complete
            } else {
                ProofCompleteness::Partial
            },
            minimality: if completed {
                InfeasibilityMinimality::Irreducible
            } else {
                InfeasibilityMinimality::Partial
            },
        }
    }

    /// 创建弹性过滤分析结果 / Create an elastic-filtering analysis result.
    pub fn elastic_filtering(completed: bool) -> Self {
        let mut analysis = Self::deletion_filtering(completed);
        analysis.algorithm = IISAlgorithm::ElasticFiltering;
        analysis.reliability = ProofReliability::Heuristic;
        analysis
    }

    /// 创建默认的未知分析结果 / Create an unknown analysis result.
    pub fn unknown() -> Self {
        Self {
            algorithm: IISAlgorithm::DeletionFiltering,
            reliability: ProofReliability::Unknown,
            completeness: ProofCompleteness::Unavailable,
            minimality: InfeasibilityMinimality::NotChecked,
        }
    }
}

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
    /// 原始模型约束数量 / Original model constraint count
    pub original_num_constraints: usize,
    /// 原始模型变量数量 / Original model variable count
    pub original_num_variables: usize,
    /// 计算时间 / Computation time
    pub computation_time: std::time::Duration,
    /// legacy 分析证据等级 / Legacy analysis evidence level.
    pub analysis: IISAnalysis,
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
            analysis: IISAnalysis::unknown(),
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

    /// 设置分析证据等级 / Set the analysis evidence level.
    pub fn set_analysis(&mut self, analysis: IISAnalysis) {
        self.analysis = analysis;
    }

    /// 获取分析证据等级 / Get the analysis evidence level.
    pub fn analysis(&self) -> &IISAnalysis {
        &self.analysis
    }

    /// 转换为带准确等级的统一不可行证据 / Convert to a uniformly typed infeasibility evidence record.
    ///
    /// 删除过滤是可靠但可能部分完成的 legacy 证据，弹性过滤始终标为启发式。/ Deletion
    /// filtering is reliable but may be partial, while elastic filtering is always heuristic.
    pub fn to_infeasibility_evidence(
        &self,
        mapping: Option<&LinearModelMapping>,
    ) -> InfeasibilityEvidence {
        let mut constraint_ids = std::collections::BTreeSet::new();
        let mut members = std::collections::BTreeSet::new();
        for index in &self.constraint_indices {
            let id = mapping
                .and_then(|mapping| mapping.stable_constraint(*index))
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("c{index}"));
            constraint_ids.insert(id.clone());
            members.insert(InfeasibilityEvidenceMember::Constraint(id));
        }
        for index in &self.lower_bound_indices {
            let id = mapping
                .and_then(|mapping| mapping.stable_variable(*index))
                .map(|variable| format!("{}::lower-bound", variable.0))
                .unwrap_or_else(|| format!("lb{index}"));
            constraint_ids.insert(id.clone());
            members.insert(InfeasibilityEvidenceMember::LowerBound(id));
        }
        for index in &self.upper_bound_indices {
            let id = mapping
                .and_then(|mapping| mapping.stable_variable(*index))
                .map(|variable| format!("{}::upper-bound", variable.0))
                .unwrap_or_else(|| format!("ub{index}"));
            constraint_ids.insert(id.clone());
            members.insert(InfeasibilityEvidenceMember::UpperBound(id));
        }
        let source = match self.analysis.algorithm {
            IISAlgorithm::DeletionFiltering => InfeasibilityEvidenceSource::DeletionFilter,
            IISAlgorithm::ElasticFiltering => InfeasibilityEvidenceSource::ElasticFilter,
        };
        InfeasibilityEvidence {
            source,
            reliability: self.analysis.reliability,
            completeness: self.analysis.completeness,
            constraint_ids,
            members,
            minimality: self.analysis.minimality,
            computation_time: self.computation_time,
            unavailable_reason: None,
        }
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

/// 线性三角模型 IIS 视图 trait / Linear Triad Model IIS View Trait
///
/// 提供 IIS 算法所需的模型操作接口。
/// Provides model operation interface needed by IIS algorithms.
pub trait LinearTriadModelIISView: LinearTriadModelView {
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

/// 线性三角模型 IIS 输入视图 / Linear Triad Model IIS Input View
///
/// 用于把不同线性模型统一映射到 `BasicLinearTriadModel` 视图。
/// Used to map different linear model containers into `BasicLinearTriadModel` view.
pub trait LinearTriadModelIISSource {
    /// 获取基础线性三角模型引用 / Get basic linear triad model reference
    fn as_basic_linear_triad_model(&self) -> &BasicLinearTriadModel;
}

impl LinearTriadModelIISSource for BasicLinearTriadModel {
    fn as_basic_linear_triad_model(&self) -> &BasicLinearTriadModel {
        self
    }
}

impl LinearTriadModelIISSource for LinearTriadModel {
    fn as_basic_linear_triad_model(&self) -> &BasicLinearTriadModel {
        &self.basic
    }
}

/// 基本线性三角模型视图 trait（兼容层）/ Basic Linear Triad Model View Trait (compatibility)
#[doc(hidden)]
#[deprecated(note = "use LinearTriadModelIISView instead")]
pub trait BasicLinearTriadModelView: LinearTriadModelIISView {}

#[allow(deprecated)]
impl<T> BasicLinearTriadModelView for T where T: LinearTriadModelIISView {}

// 类型别名 / Type aliases
/// f64 精度的线性 IIS 模型（默认）/ Linear IIS model with f64 precision (default)
pub type LinearIISModelF64 = LinearIISModel;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_deletion_analysis_projects_typed_evidence_members() {
        let mut iis = LinearIISModel::new(2, 1);
        iis.add_constraint(1);
        iis.add_lower_bound(0);
        iis.set_analysis(IISAnalysis::deletion_filtering(true));

        let evidence = iis.to_infeasibility_evidence(None);

        assert_eq!(evidence.source, InfeasibilityEvidenceSource::DeletionFilter);
        assert_eq!(evidence.reliability, ProofReliability::Reliable);
        assert_eq!(evidence.completeness, ProofCompleteness::Complete);
        assert_eq!(evidence.minimality, InfeasibilityMinimality::Irreducible);
        assert!(
            evidence
                .members
                .contains(&InfeasibilityEvidenceMember::Constraint("c1".to_owned()))
        );
        assert!(
            evidence
                .members
                .contains(&InfeasibilityEvidenceMember::LowerBound("lb0".to_owned()))
        );
    }

    #[test]
    fn legacy_elastic_analysis_remains_heuristic_even_when_complete() {
        let mut iis = LinearIISModel::new(0, 0);
        iis.set_analysis(IISAnalysis::elastic_filtering(true));

        let evidence = iis.to_infeasibility_evidence(None);

        assert_eq!(evidence.source, InfeasibilityEvidenceSource::ElasticFilter);
        assert_eq!(evidence.reliability, ProofReliability::Heuristic);
        assert_eq!(evidence.completeness, ProofCompleteness::Complete);
    }
}
