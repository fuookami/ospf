//! 约束活动性分析 / Constraint-activity analysis.
//!
//! 该模块只在原始模型元素边界报告活动性。线性模型复用 `solver::audit` 的独立求值，
//! CP global constraint 使用原始 AST 语义求值；没有自然 margin 的约束不会被伪造出 slack。
//! This module reports activity only at the original-model boundary. Linear models reuse the
//! independent evaluation in `solver::audit`, while CP global constraints use source-AST
//! semantics; constraints without a natural margin never receive fabricated slack.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::{
    ConstraintProgrammingConstraint, ConstraintProgrammingSnapshot, IntegerRelation, IntervalValue,
};
use crate::model::intermediate::LinearTriadModel;
use crate::solver::audit::{evaluate_linear_solution, linear_model_mapping};
use crate::solver::report::StableVariableId;

use super::{BoundSide, ConstraintId, DiagnosticSource};

/// 活动性报告 schema 版本 / Activity-report schema version.
pub const ACTIVITY_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 活动性状态 / Activity status.
///
/// `Active` 和 `NearlyActive` 只描述有数值 margin 的元素。没有自然 margin 的已满足 CP
/// global constraint 使用 `SatisfiedWithoutSlackMetric`；语义求值为假的约束使用
/// `Violated`，避免把违反约束误解为“不活跃”。
/// `Active` and `NearlyActive` describe elements with a numeric margin. A satisfied CP global
/// constraint without a natural margin uses `SatisfiedWithoutSlackMetric`; a false semantic
/// evaluation uses `Violated` so a violation cannot be mistaken for inactivity.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActivityStatus {
    /// 处于容差内的贴边元素 / Element at the boundary within active tolerance.
    Active,
    /// 接近边界但超出 active 容差 / Close to the boundary but outside active tolerance.
    NearlyActive,
    /// 有明显正 margin 的满足元素 / Satisfied element with a material positive margin.
    Inactive,
    /// 满足但没有自然 slack 指标 / Satisfied without a natural slack metric.
    SatisfiedWithoutSlackMetric,
    /// 原始语义求值失败 / Element violates its original semantics.
    Violated,
    /// 无法形成活动性结论 / No activity conclusion is available.
    #[default]
    Unknown,
}

impl ActivityStatus {
    /// 是否为 active / Whether this is active.
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// 是否为 nearly active / Whether this is nearly active.
    pub const fn is_nearly_active(self) -> bool {
        matches!(self, Self::NearlyActive)
    }

    /// 是否为 inactive / Whether this is inactive.
    pub const fn is_inactive(self) -> bool {
        matches!(self, Self::Inactive)
    }

    /// 是否为明确违反 / Whether this is a violation.
    pub const fn is_violated(self) -> bool {
        matches!(self, Self::Violated)
    }

    /// 是否为没有 slack 指标的已满足元素 / Whether this is satisfied without a slack metric.
    pub const fn is_without_slack_metric(self) -> bool {
        matches!(self, Self::SatisfiedWithoutSlackMetric)
    }
}

/// 活动性分析配置 / Activity-analysis configuration.
///
/// `tolerance` 是绝对容差，`relative_tolerance` 按 `max(1, |rhs|)` 或 `max(1, |bound|)`
/// 缩放。`nearly_active_tolerance` 是绝对的近边界上限；两类容差都会包含相对容差。
/// `tolerance` is the absolute tolerance, while `relative_tolerance` is scaled by
/// `max(1, |rhs|)` or `max(1, |bound|)`. `nearly_active_tolerance` is the absolute near-boundary
/// limit; both limits also include the relative tolerance.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivityConfig {
    /// active 判定的绝对容差 / Absolute tolerance for an active classification.
    pub tolerance: f64,
    /// nearly-active 判定的绝对上限 / Absolute upper limit for a nearly-active classification.
    pub nearly_active_tolerance: f64,
    /// 相对容差系数 / Relative tolerance coefficient.
    pub relative_tolerance: f64,
    /// 是否输出有限变量边界 / Whether finite variable bounds are reported.
    pub include_variable_bounds: bool,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-7,
            nearly_active_tolerance: 1e-5,
            relative_tolerance: 0.0,
            include_variable_bounds: true,
        }
    }
}

impl ActivityConfig {
    /// 创建活动性配置 / Create an activity configuration.
    pub const fn new(tolerance: f64, nearly_active_tolerance: f64) -> Self {
        Self {
            tolerance,
            nearly_active_tolerance,
            relative_tolerance: 0.0,
            include_variable_bounds: true,
        }
    }

    /// 创建仅指定 active 容差的配置 / Create a configuration with only active tolerance.
    pub const fn with_tolerance(tolerance: f64) -> Self {
        Self::new(tolerance, tolerance * 10.0)
    }

    /// 校验配置 / Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if !self.tolerance.is_finite()
            || self.tolerance < 0.0
            || !self.nearly_active_tolerance.is_finite()
            || self.nearly_active_tolerance < self.tolerance
            || !self.relative_tolerance.is_finite()
            || self.relative_tolerance < 0.0
        {
            return Err(invalid_activity(
                "activity tolerances must be finite, non-negative, and ordered",
            ));
        }
        Ok(())
    }

    /// 设置相对容差 / Set the relative tolerance.
    pub const fn with_relative_tolerance(mut self, tolerance: f64) -> Self {
        self.relative_tolerance = tolerance;
        self
    }

    /// 设置是否输出变量边界 / Set whether variable bounds are reported.
    pub const fn with_variable_bounds(mut self, include: bool) -> Self {
        self.include_variable_bounds = include;
        self
    }

    /// 返回给定尺度下的 active 容差 / Return the active tolerance at a scale.
    pub fn effective_tolerance(&self, scale: f64) -> f64 {
        self.tolerance
            .max(self.relative_tolerance * scale.abs().max(1.0))
    }

    /// 返回给定尺度下的 nearly-active 容差 / Return the nearly-active tolerance at a scale.
    pub fn effective_nearly_active_tolerance(&self, scale: f64) -> f64 {
        self.nearly_active_tolerance
            .max(self.relative_tolerance * scale.abs().max(1.0))
    }

    /// 按 signed slack 分类 / Classify a signed slack.
    pub fn classify(&self, slack: f64, scale: f64) -> ActivityStatus {
        if !slack.is_finite() || !scale.is_finite() {
            return ActivityStatus::Unknown;
        }
        let active_tolerance = self.effective_tolerance(scale);
        let nearly_active_tolerance = self
            .effective_nearly_active_tolerance(scale)
            .max(active_tolerance);
        if !active_tolerance.is_finite() || !nearly_active_tolerance.is_finite() {
            return ActivityStatus::Unknown;
        }
        if slack < -active_tolerance {
            ActivityStatus::Violated
        } else if slack.abs() <= active_tolerance {
            ActivityStatus::Active
        } else if slack.abs() <= nearly_active_tolerance {
            ActivityStatus::NearlyActive
        } else {
            ActivityStatus::Inactive
        }
    }
}

/// 活动性证据 / Activity evidence.
///
/// 证据只包含原始模型身份和值，不包含 solver row、column 或 lowering auxiliary element。
/// Evidence contains only original-model identities and values; solver rows, columns, and
/// lowering auxiliary elements are deliberately absent.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityEvidence {
    /// 线性 `lhs <= rhs` 证据 / Linear `lhs <= rhs` evidence.
    LinearConstraint {
        /// 左侧值 / Left-hand value.
        lhs: f64,
        /// 右侧值 / Right-hand value.
        rhs: f64,
    },
    /// 变量边界证据 / Variable-bound evidence.
    VariableBound {
        /// 稳定变量身份 / Stable variable identity.
        variable_id: StableVariableId,
        /// 边界方向 / Bound side.
        side: BoundSide,
        /// 变量值 / Variable value.
        value: f64,
        /// 边界值 / Bound value.
        bound: f64,
    },
    /// CP 整数比较的自然 margin / Natural margin for a CP integer comparison.
    IntegerComparison {
        /// 左侧整数值 / Integer left-hand value.
        lhs: i64,
        /// 右侧整数值 / Integer right-hand value.
        rhs: i64,
        /// 比较关系 / Comparison relation.
        relation: IntegerRelation,
    },
    /// 有语义满足性但没有 slack 的约束 / Semantic constraint without a slack metric.
    Semantic {
        /// 是否满足 / Whether the constraint is satisfied.
        satisfied: bool,
        /// 可选的自然 margin / Optional natural margin.
        margin: Option<f64>,
    },
    /// 无法求值的证据 / Evidence that could not be evaluated.
    Unknown {
        /// 失败原因 / Failure reason.
        reason: String,
    },
}

impl ActivityEvidence {
    /// 返回证据是否有数值 slack / Whether this evidence has a numeric slack.
    pub const fn has_slack_metric(&self) -> bool {
        matches!(
            self,
            Self::LinearConstraint { .. }
                | Self::VariableBound { .. }
                | Self::IntegerComparison { .. }
                | Self::Semantic {
                    margin: Some(_),
                    ..
                }
        )
    }

    /// 返回语义满足性（若证据可提供）/ Return semantic satisfaction when available.
    pub const fn satisfied(&self) -> Option<bool> {
        match self {
            Self::Semantic { satisfied, .. } => Some(*satisfied),
            Self::Unknown { .. } => None,
            _ => None,
        }
    }
}

/// 单条原始约束活动性 / Activity of one original constraint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintActivity {
    /// 稳定约束身份 / Stable constraint identity.
    pub constraint_id: ConstraintId,
    /// 约束组身份或名称 / Constraint-group identity or name.
    pub group: Option<String>,
    /// 活动性状态 / Activity status.
    pub status: ActivityStatus,
    /// 可选左侧值 / Optional left-hand value.
    pub lhs: Option<f64>,
    /// 可选右侧值 / Optional right-hand value.
    pub rhs: Option<f64>,
    /// signed slack，正值表示满足 / Signed slack; positive means satisfied.
    pub slack: Option<f64>,
    /// 按 rhs 尺度归一化的 slack / Slack normalized by rhs scale.
    pub normalized_slack: Option<f64>,
    /// 使用的有效容差 / Effective tolerance used.
    pub tolerance: Option<f64>,
    /// 语义满足性 / Semantic satisfaction when available.
    pub satisfied: Option<bool>,
    /// 原始模型证据 / Original-model evidence.
    pub evidence: ActivityEvidence,
}

impl ConstraintActivity {
    /// 返回稳定诊断来源 / Return the stable diagnostic source.
    pub fn source(&self) -> DiagnosticSource {
        DiagnosticSource::Constraint {
            id: self.constraint_id.clone(),
        }
    }
}

/// 单个变量边界活动性 / Activity of one variable bound.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct VariableBoundActivity {
    /// 稳定变量身份 / Stable variable identity.
    pub variable_id: StableVariableId,
    /// 边界方向 / Bound side.
    pub side: BoundSide,
    /// 变量值 / Variable value.
    pub value: f64,
    /// 边界值 / Bound value.
    pub bound: f64,
    /// signed slack，正值表示在可行侧 / Signed slack; positive means feasible side.
    pub slack: f64,
    /// 归一化 slack / Normalized slack.
    pub normalized_slack: f64,
    /// 使用的有效容差 / Effective tolerance used.
    pub tolerance: f64,
    /// 是否满足边界 / Whether the bound is satisfied.
    pub satisfied: bool,
    /// 活动性状态 / Activity status.
    pub status: ActivityStatus,
    /// 原始模型证据 / Original-model evidence.
    pub evidence: ActivityEvidence,
}

impl VariableBoundActivity {
    /// 返回稳定诊断来源 / Return the stable diagnostic source.
    pub fn source(&self) -> DiagnosticSource {
        DiagnosticSource::VariableBound {
            bound: super::VariableBoundRef {
                variable_id: self.variable_id.clone(),
                side: self.side,
            },
        }
    }
}

/// 活动性计数 / Activity counts.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActivitySummary {
    /// 元素总数 / Total element count.
    pub total: usize,
    /// active 数量 / Active count.
    pub active: usize,
    /// nearly-active 数量 / Nearly-active count.
    pub nearly_active: usize,
    /// inactive 数量 / Inactive count.
    pub inactive: usize,
    /// 无 slack 指标的满足元素数量 / Count satisfied without a slack metric.
    pub satisfied_without_slack_metric: usize,
    /// 违反元素数量 / Violation count.
    pub violated: usize,
    /// 未知元素数量 / Unknown count.
    pub unknown: usize,
}

impl ActivitySummary {
    /// 累加一个活动性状态 / Accumulate one activity status.
    pub fn add(&mut self, status: ActivityStatus) {
        self.total += 1;
        match status {
            ActivityStatus::Active => self.active += 1,
            ActivityStatus::NearlyActive => self.nearly_active += 1,
            ActivityStatus::Inactive => self.inactive += 1,
            ActivityStatus::SatisfiedWithoutSlackMetric => self.satisfied_without_slack_metric += 1,
            ActivityStatus::Violated => self.violated += 1,
            ActivityStatus::Unknown => self.unknown += 1,
        }
    }
}

/// 活动性 group 汇总 / Activity summary for one group.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityGroupSummary {
    /// group 身份；`None` 表示未分组 / Group identity; `None` means ungrouped.
    pub group: Option<String>,
    /// 组内活动性计数 / Activity counts within the group.
    pub summary: ActivitySummary,
}

/// 约束活动性报告 / Constraint-activity report.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintActivityReport {
    /// 报告 schema 版本 / Report schema version.
    pub schema_version: String,
    /// 原始约束活动性 / Original constraint activities.
    pub constraints: Vec<ConstraintActivity>,
    /// 原始变量边界活动性 / Original variable-bound activities.
    pub variable_bounds: Vec<VariableBoundActivity>,
    /// 约束总计 / Constraint totals.
    pub summary: ActivitySummary,
    /// 变量边界总计 / Variable-bound totals.
    pub variable_bound_summary: ActivitySummary,
    /// group 汇总 / Group summaries.
    pub groups: Vec<ActivityGroupSummary>,
    /// active 判定容差 / Active classification tolerance.
    pub tolerance: f64,
    /// nearly-active 判定容差 / Nearly-active classification tolerance.
    pub nearly_active_tolerance: f64,
}

impl ConstraintActivityReport {
    /// 返回约束记录（兼容 `activities` 命名）/ Return constraint records.
    pub fn activities(&self) -> &[ConstraintActivity] {
        &self.constraints
    }

    /// 返回约束活动性记录 / Return constraint activities.
    pub fn constraint_activities(&self) -> &[ConstraintActivity] {
        &self.constraints
    }

    /// 返回变量边界记录 / Return variable-bound activities.
    pub fn variable_bound_activities(&self) -> &[VariableBoundActivity] {
        &self.variable_bounds
    }

    /// 校验报告计数和数值不变量 / Validate report counts and numeric invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty()
            || !self.tolerance.is_finite()
            || self.tolerance < 0.0
            || !self.nearly_active_tolerance.is_finite()
            || self.nearly_active_tolerance < self.tolerance
        {
            return Err(invalid_activity("activity report metadata is invalid"));
        }
        let summary = summarize_constraints(&self.constraints);
        let variable_bound_summary = self.variable_bounds.iter().fold(
            ActivitySummary::default(),
            |mut summary, activity| {
                summary.add(activity.status);
                summary
            },
        );
        if self.summary != summary || self.variable_bound_summary != variable_bound_summary {
            return Err(invalid_activity(
                "activity report summary does not match its records",
            ));
        }
        let mut expected_groups = BTreeMap::<Option<String>, ActivitySummary>::new();
        for activity in &self.constraints {
            expected_groups
                .entry(activity.group.clone())
                .or_default()
                .add(activity.status);
        }
        let actual_groups = self
            .groups
            .iter()
            .map(|group| (group.group.clone(), group.summary))
            .collect::<BTreeMap<_, _>>();
        if actual_groups.len() != self.groups.len() || actual_groups != expected_groups {
            return Err(invalid_activity(
                "activity group summary does not match its records",
            ));
        }
        for activity in &self.constraints {
            if activity.constraint_id.0.trim().is_empty() {
                return Err(invalid_activity("activity constraint ID must not be blank"));
            }
            validate_optional_finite(activity.lhs)?;
            validate_optional_finite(activity.rhs)?;
            validate_optional_finite(activity.slack)?;
            validate_optional_finite(activity.normalized_slack)?;
            validate_optional_finite(activity.tolerance)?;
            if activity
                .group
                .as_ref()
                .is_some_and(|group| group.trim().is_empty())
            {
                return Err(invalid_activity("activity group must not be blank"));
            }
        }
        let constraint_ids = self
            .constraints
            .iter()
            .map(|activity| activity.constraint_id.clone())
            .collect::<BTreeSet<_>>();
        if constraint_ids.len() != self.constraints.len() {
            return Err(invalid_activity(
                "activity report contains duplicate constraint IDs",
            ));
        }
        for activity in &self.variable_bounds {
            validate_finite(activity.value)?;
            validate_finite(activity.bound)?;
            validate_finite(activity.slack)?;
            validate_finite(activity.normalized_slack)?;
            validate_finite(activity.tolerance)?;
        }
        let variable_bounds = self
            .variable_bounds
            .iter()
            .map(|activity| (activity.variable_id.clone(), activity.side))
            .collect::<BTreeSet<_>>();
        if variable_bounds.len() != self.variable_bounds.len() {
            return Err(invalid_activity(
                "activity report contains duplicate variable bounds",
            ));
        }
        Ok(())
    }
}

/// 可扩展的 CP/语义活动性适配器 / Extensible CP/semantic activity adapter.
///
/// 外部模型可以只实现原始约束记录生成，报告聚合仍由公共 analyzer 完成。适配器返回的
/// 记录必须使用原始稳定身份；不得把 lowering 生成的行或列写入 `ConstraintActivity`。
/// External models can implement only original-constraint record generation while the common
/// analyzer performs report aggregation. Adapter records must use stable original identities and
/// must not put lowering-generated rows or columns into `ConstraintActivity`.
pub trait ConstraintActivityAdapter {
    /// 生成约束活动性记录 / Build constraint activity records.
    fn constraint_activities(
        &self,
        values: &BTreeMap<StableVariableId, i64>,
        config: &ActivityConfig,
    ) -> Result<Vec<ConstraintActivity>>;

    /// 生成变量边界活动性记录 / Build variable-bound activity records.
    fn variable_bound_activities(
        &self,
        _values: &BTreeMap<StableVariableId, i64>,
        _config: &ActivityConfig,
    ) -> Result<Vec<VariableBoundActivity>> {
        Ok(Vec::new())
    }
}

/// 约束活动性分析器 / Constraint-activity analyzer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintActivityAnalyzer {
    /// 活动性配置 / Activity configuration.
    config: ActivityConfig,
}

impl Default for ConstraintActivityAnalyzer {
    fn default() -> Self {
        Self {
            config: ActivityConfig::default(),
        }
    }
}

impl ConstraintActivityAnalyzer {
    /// 创建分析器 / Create an analyzer.
    pub fn new(config: ActivityConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// 创建默认分析器 / Create an analyzer with default tolerances.
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// 返回分析配置 / Return the analysis configuration.
    pub const fn config(&self) -> ActivityConfig {
        self.config
    }

    /// 分析线性模型的 incumbent / Analyze a linear-model incumbent.
    pub fn analyze_linear(
        &self,
        model: &LinearTriadModel,
        values: &[f64],
    ) -> Result<ConstraintActivityReport> {
        self.config.validate()?;
        let mapping = linear_model_mapping(model)?;
        let diagnostics = evaluate_linear_solution(model, values, self.config.tolerance)?;
        let constraints = diagnostics
            .constraint_evaluations
            .into_iter()
            .enumerate()
            .map(|(row, evaluation)| {
                let rhs_scale = evaluation.rhs.abs().max(1.0);
                let group = model
                    .basic
                    .constraint_group_ids
                    .get(row)
                    .and_then(|group| *group)
                    .map(|group| group.to_string());
                Ok(ConstraintActivity {
                    constraint_id: crate::solver::StableConstraintId(
                        mapping
                            .constraints_by_row
                            .get(row)
                            .cloned()
                            .ok_or_else(|| {
                                invalid_activity("linear audit omitted a constraint row")
                            })?,
                    ),
                    group,
                    status: self.config.classify(evaluation.residual, rhs_scale),
                    lhs: Some(evaluation.lhs),
                    rhs: Some(evaluation.rhs),
                    slack: Some(evaluation.residual),
                    normalized_slack: Some(evaluation.residual / rhs_scale),
                    tolerance: Some(self.config.effective_tolerance(rhs_scale)),
                    satisfied: Some(
                        evaluation.residual + self.config.effective_tolerance(rhs_scale) >= 0.0,
                    ),
                    evidence: ActivityEvidence::LinearConstraint {
                        lhs: evaluation.lhs,
                        rhs: evaluation.rhs,
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let variable_bounds = if self.config.include_variable_bounds {
            diagnostics
                .variable_bound_evaluations
                .into_iter()
                .flat_map(|evaluation| {
                    let mut activities = Vec::with_capacity(2);
                    if let Some(bound) = evaluation.lower {
                        activities.push(self.variable_bound_activity(
                            evaluation.variable_id.clone(),
                            BoundSide::Lower,
                            evaluation.value,
                            bound,
                        ));
                    }
                    if let Some(bound) = evaluation.upper {
                        activities.push(self.variable_bound_activity(
                            evaluation.variable_id.clone(),
                            BoundSide::Upper,
                            evaluation.value,
                            bound,
                        ));
                    }
                    activities
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };

        build_report(&self.config, constraints, variable_bounds)
    }

    /// 从 solver 报告分析线性 incumbent / Analyze a linear incumbent from a solver report.
    pub fn analyze_linear_report(
        &self,
        model: &LinearTriadModel,
        report: &crate::solver::report::SolveReport<f64>,
    ) -> Result<ConstraintActivityReport> {
        report.validate()?;
        let solution = report
            .solution
            .as_ref()
            .ok_or_else(|| invalid_activity("linear activity analysis requires an incumbent"))?;
        if !solution.values.is_empty() {
            return self.analyze_linear(model, &solution.values);
        }
        let mapping = linear_model_mapping(model)?;
        let values = mapping
            .variables_by_column
            .iter()
            .map(|id| {
                solution
                    .stable_values
                    .get(id)
                    .copied()
                    .ok_or_else(|| invalid_activity("stable incumbent is missing a variable"))
            })
            .collect::<Result<Vec<_>>>()?;
        self.analyze_linear(model, &values)
    }

    /// 分析 CP snapshot 的完整整数赋值 / Analyze a complete integer assignment on a CP snapshot.
    pub fn analyze_cp(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        values: &BTreeMap<StableVariableId, i64>,
    ) -> Result<ConstraintActivityReport> {
        self.analyze_adapter(snapshot, values)
    }

    /// 使用自定义适配器分析语义模型 / Analyze a semantic model through a custom adapter.
    pub fn analyze_adapter<A: ConstraintActivityAdapter + ?Sized>(
        &self,
        adapter: &A,
        values: &BTreeMap<StableVariableId, i64>,
    ) -> Result<ConstraintActivityReport> {
        self.config.validate()?;
        let constraints = adapter.constraint_activities(values, &self.config)?;
        let variable_bounds = if self.config.include_variable_bounds {
            adapter.variable_bound_activities(values, &self.config)?
        } else {
            Vec::new()
        };
        build_report(&self.config, constraints, variable_bounds)
    }

    fn variable_bound_activity(
        &self,
        variable_id: StableVariableId,
        side: BoundSide,
        value: f64,
        bound: f64,
    ) -> Result<VariableBoundActivity> {
        validate_finite(value)?;
        validate_finite(bound)?;
        let slack = match side {
            BoundSide::Lower => value - bound,
            BoundSide::Upper => bound - value,
        };
        let scale = bound.abs().max(1.0);
        let tolerance = self.config.effective_tolerance(scale);
        let status = self.config.classify(slack, scale);
        Ok(VariableBoundActivity {
            variable_id: variable_id.clone(),
            side,
            value,
            bound,
            slack,
            normalized_slack: slack / scale,
            tolerance,
            satisfied: slack + tolerance >= 0.0,
            status,
            evidence: ActivityEvidence::VariableBound {
                variable_id,
                side,
                value,
                bound,
            },
        })
    }
}

impl ConstraintActivityAdapter for ConstraintProgrammingSnapshot {
    fn constraint_activities(
        &self,
        values: &BTreeMap<StableVariableId, i64>,
        config: &ActivityConfig,
    ) -> Result<Vec<ConstraintActivity>> {
        self.validate_identity()?;
        validate_complete_cp_assignment(self, values)?;
        let intervals = evaluated_intervals(self, values)?;
        self.constraints
            .iter()
            .map(|constraint| cp_constraint_activity(constraint, values, &intervals, config))
            .collect()
    }

    fn variable_bound_activities(
        &self,
        values: &BTreeMap<StableVariableId, i64>,
        config: &ActivityConfig,
    ) -> Result<Vec<VariableBoundActivity>> {
        validate_complete_cp_assignment(self, values)?;
        self.variables
            .iter()
            .flat_map(|variable| {
                let value = values
                    .get(&variable.variable.stable_id)
                    .copied()
                    .unwrap_or_default() as f64;
                let mut records = Vec::with_capacity(2);
                let lower = variable.domain.lower() as f64;
                let upper = variable.domain.upper() as f64;
                records.push(cp_bound_activity(
                    variable.variable.stable_id.clone(),
                    BoundSide::Lower,
                    value,
                    lower,
                    config,
                ));
                if upper != lower {
                    records.push(cp_bound_activity(
                        variable.variable.stable_id.clone(),
                        BoundSide::Upper,
                        value,
                        upper,
                        config,
                    ));
                }
                records
            })
            .collect()
    }
}

fn cp_constraint_activity(
    constraint: &crate::model::constraint_programming::ConstraintSnapshot,
    values: &BTreeMap<StableVariableId, i64>,
    intervals: &BTreeMap<crate::model::constraint_programming::IntervalVariableId, IntervalValue>,
    config: &ActivityConfig,
) -> Result<ConstraintActivity> {
    let group = constraint.group.clone();
    match &constraint.constraint {
        ConstraintProgrammingConstraint::Integer {
            expression,
            relation,
            rhs,
        } => {
            let lhs = expression.evaluate(values)?;
            let slack = integer_comparison_slack(*relation, lhs, *rhs);
            let scale = (*rhs as f64).abs().max(1.0);
            let satisfied = relation.evaluate(lhs, *rhs);
            let status = if satisfied {
                config.classify(slack, scale)
            } else {
                ActivityStatus::Violated
            };
            Ok(ConstraintActivity {
                constraint_id: constraint.id.clone(),
                group,
                status,
                lhs: Some(lhs as f64),
                rhs: Some(*rhs as f64),
                slack: Some(slack),
                normalized_slack: Some(slack / scale),
                tolerance: Some(config.effective_tolerance(scale)),
                satisfied: Some(satisfied),
                evidence: ActivityEvidence::IntegerComparison {
                    lhs,
                    rhs: *rhs,
                    relation: *relation,
                },
            })
        }
        _ => {
            let satisfied = constraint.constraint.evaluate(values, intervals)?;
            // 只有在语义上存在无歧义标量"边界距离"的 global constraint 才给出 margin；
            // 其余（表约束、Circuit、Automaton、Reservoir、区间类等）按计划 2.2 保持
            // `SatisfiedWithoutSlackMetric`，绝不伪造 slack。
            //
            // Only global constraints with an unambiguous scalar boundary distance receive a margin.
            // The rest (table constraints, Circuit, Automaton, Reservoir, interval constraints, …)
            // keep `SatisfiedWithoutSlackMetric` per plan 2.2 and never receive fabricated slack.
            return match cp_global_margin(&constraint.constraint, values)? {
                Some(margin) => {
                    let scale = 1.0_f64;
                    Ok(ConstraintActivity {
                        constraint_id: constraint.id.clone(),
                        group,
                        status: config.classify(margin, scale),
                        lhs: None,
                        rhs: None,
                        slack: Some(margin),
                        normalized_slack: Some(margin),
                        tolerance: Some(config.effective_tolerance(scale)),
                        satisfied: Some(satisfied),
                        evidence: ActivityEvidence::Semantic {
                            satisfied,
                            margin: Some(margin),
                        },
                    })
                }
                None => Ok(ConstraintActivity {
                    constraint_id: constraint.id.clone(),
                    group,
                    status: if satisfied {
                        ActivityStatus::SatisfiedWithoutSlackMetric
                    } else {
                        ActivityStatus::Violated
                    },
                    lhs: None,
                    rhs: None,
                    slack: None,
                    normalized_slack: None,
                    tolerance: None,
                    satisfied: Some(satisfied),
                    evidence: ActivityEvidence::Semantic {
                        satisfied,
                        margin: None,
                    },
                }),
            };
        }
    }
}

/// CP global constraint 的自然 margin / Natural margin for a CP global constraint.
///
/// 只有在**语义上无歧义**时才返回 `Some`：目前只有 `AllDifferent`。
/// 整数取值下它的自然标量距离是"最小的两两间距减一"——即还需要缩小多少间距才会违反约束：
/// 全异时 `min_gap ≥ 1`，margin `≥ 0`（`0` 表示已贴边，即存在相邻取值）；出现重复时
/// `min_gap = 0`，margin `= -1`。该度量与 `ActivityConfig::classify` 的有符号 slack 语义一致，
/// 因此可以直接走同一套 Active / NearlyActive / Inactive 判定。
///
/// Returns `Some` only when the metric is **semantically unambiguous**, which today means
/// `AllDifferent` alone. For integer values its natural scalar distance is "smallest pairwise gap
/// minus one", i.e. how much the gap would have to shrink before the constraint is violated: when
/// all values differ, `min_gap ≥ 1` and the margin is `≥ 0` (`0` means already at the boundary, i.e.
/// adjacent values); with a duplicate, `min_gap = 0` and the margin is `-1`. This matches the signed
/// slack semantics of `ActivityConfig::classify`, so the same Active / NearlyActive / Inactive
/// classification applies.
fn cp_global_margin(
    constraint: &ConstraintProgrammingConstraint,
    values: &BTreeMap<StableVariableId, i64>,
) -> Result<Option<f64>> {
    match constraint {
        ConstraintProgrammingConstraint::AllDifferent { expressions } => {
            if expressions.len() < 2 {
                // 少于两个表达式时不存在两两间距。 / Fewer than two expressions means no pairwise gap.
                return Ok(None);
            }
            let mut evaluated = Vec::with_capacity(expressions.len());
            for expression in expressions {
                evaluated.push(expression.evaluate(values)?);
            }
            evaluated.sort_unstable();
            let min_gap = evaluated
                .windows(2)
                .map(|window| i128::from(window[1]) - i128::from(window[0]))
                .min()
                .unwrap_or(0);
            Ok(Some((min_gap - 1) as f64))
        }
        _ => Ok(None),
    }
}

fn cp_bound_activity(
    variable_id: StableVariableId,
    side: BoundSide,
    value: f64,
    bound: f64,
    config: &ActivityConfig,
) -> Result<VariableBoundActivity> {
    let slack = match side {
        BoundSide::Lower => value - bound,
        BoundSide::Upper => bound - value,
    };
    let scale = bound.abs().max(1.0);
    let tolerance = config.effective_tolerance(scale);
    Ok(VariableBoundActivity {
        variable_id: variable_id.clone(),
        side,
        value,
        bound,
        slack,
        normalized_slack: slack / scale,
        tolerance,
        satisfied: slack + tolerance >= 0.0,
        status: config.classify(slack, scale),
        evidence: ActivityEvidence::VariableBound {
            variable_id,
            side,
            value,
            bound,
        },
    })
}

fn integer_comparison_slack(relation: IntegerRelation, lhs: i64, rhs: i64) -> f64 {
    let difference = i128::from(rhs) - i128::from(lhs);
    match relation {
        IntegerRelation::Equal if difference == 0 => 0.0,
        IntegerRelation::Equal => -(difference.unsigned_abs() as f64),
        IntegerRelation::NotEqual => difference.unsigned_abs() as f64,
        IntegerRelation::LessOrEqual => difference as f64,
        IntegerRelation::GreaterOrEqual => -difference as f64,
    }
}

fn validate_complete_cp_assignment(
    snapshot: &ConstraintProgrammingSnapshot,
    values: &BTreeMap<StableVariableId, i64>,
) -> Result<()> {
    if values.len() != snapshot.variables.len()
        || snapshot
            .variables
            .iter()
            .any(|variable| !values.contains_key(&variable.variable.stable_id))
    {
        return Err(invalid_activity(
            "CP activity analysis requires a complete assignment",
        ));
    }
    snapshot.validate_hint(values)
}

fn evaluated_intervals(
    snapshot: &ConstraintProgrammingSnapshot,
    values: &BTreeMap<StableVariableId, i64>,
) -> Result<BTreeMap<crate::model::constraint_programming::IntervalVariableId, IntervalValue>> {
    snapshot
        .intervals
        .iter()
        .map(|interval| {
            interval
                .interval
                .evaluate(values)
                .map(|value| (interval.interval.id.clone(), value))
        })
        .try_fold(BTreeMap::new(), |mut result, item| {
            let (id, value) = item?;
            if let Some(value) = value {
                result.insert(id, value);
            }
            Ok::<_, crate::error::CoreError>(result)
        })
}

fn build_report(
    config: &ActivityConfig,
    constraints: Vec<ConstraintActivity>,
    variable_bounds: Vec<VariableBoundActivity>,
) -> Result<ConstraintActivityReport> {
    let summary = summarize_constraints(&constraints);
    let variable_bound_summary =
        variable_bounds
            .iter()
            .fold(ActivitySummary::default(), |mut summary, activity| {
                summary.add(activity.status);
                summary
            });
    let mut grouped = BTreeMap::<Option<String>, ActivitySummary>::new();
    for activity in &constraints {
        grouped
            .entry(activity.group.clone())
            .or_default()
            .add(activity.status);
    }
    let groups = grouped
        .into_iter()
        .map(|(group, summary)| ActivityGroupSummary { group, summary })
        .collect();
    let report = ConstraintActivityReport {
        schema_version: ACTIVITY_REPORT_SCHEMA_VERSION.to_owned(),
        constraints,
        variable_bounds,
        summary,
        variable_bound_summary,
        groups,
        tolerance: config.tolerance,
        nearly_active_tolerance: config.nearly_active_tolerance,
    };
    report.validate()?;
    Ok(report)
}

fn summarize_constraints(constraints: &[ConstraintActivity]) -> ActivitySummary {
    constraints
        .iter()
        .fold(ActivitySummary::default(), |mut summary, activity| {
            summary.add(activity.status);
            summary
        })
}

fn validate_finite(value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid_activity("activity value must be finite"))
    }
}

fn validate_optional_finite(value: Option<f64>) -> Result<()> {
    if let Some(value) = value {
        validate_finite(value)?;
    }
    Ok(())
}

fn invalid_activity(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ObjectiveCategory;
    use crate::model::constraint_programming::{
        ConstraintDefinition, IntegerDomain, IntegerExpression, IntegerVariable,
    };
    use crate::model::intermediate::{BasicLinearTriadModel, SparseVector};
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableType};

    fn linear_model() -> LinearTriadModel {
        let mut basic = BasicLinearTriadModel::new("activity");
        let x = ContinuousVariableItem::auto("x");
        let y = ContinuousVariableItem::auto("y");
        basic.add_variable_with_bounds(
            Token::from_generic(x, 0),
            0.0,
            10.0,
            VariableType::Continuous,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(y, 1),
            0.0,
            10.0,
            VariableType::Continuous,
        );
        let mut active = SparseVector::new();
        active.add(0, 1.0);
        active.add(1, 1.0);
        basic.add_constraint_with_metadata(
            active,
            10.0,
            "capacity".to_owned(),
            Some(7),
            false,
            0,
            None,
            None,
        );
        let mut inactive = SparseVector::new();
        inactive.add(0, 1.0);
        basic.add_constraint_with_metadata(
            inactive,
            100.0,
            "wide".to_owned(),
            Some(7),
            false,
            0,
            None,
            None,
        );
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0, 1.0], ObjectiveCategory::Maximum);
        model
    }

    #[test]
    fn linear_activity_reuses_audit_residual_and_groups_constraints() {
        let report = ConstraintActivityAnalyzer::default()
            .analyze_linear(&linear_model(), &[5.0, 5.0])
            .expect("activity report");
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.active, 1);
        assert_eq!(report.summary.inactive, 1);
        assert_eq!(report.constraints[0].slack, Some(0.0));
        assert_eq!(report.constraints[1].slack, Some(95.0));
        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.groups[0].group.as_deref(), Some("7"));
        assert_eq!(report.groups[0].summary.total, 2);
        assert!(report.validate().is_ok());
    }

    #[test]
    fn tolerance_distinguishes_active_nearly_active_and_violation() {
        let config = ActivityConfig::new(1e-6, 1e-3);
        assert_eq!(config.classify(1e-7, 1.0), ActivityStatus::Active);
        assert_eq!(config.classify(1e-4, 1.0), ActivityStatus::NearlyActive);
        assert_eq!(config.classify(-2e-6, 1.0), ActivityStatus::Violated);
        assert_eq!(config.classify(2e-2, 1.0), ActivityStatus::Inactive);
    }

    #[test]
    fn variable_bounds_are_reported_per_side() {
        let report = ConstraintActivityAnalyzer::default()
            .analyze_linear(&linear_model(), &[0.0, 5.0])
            .expect("activity report");
        assert_eq!(report.variable_bounds.len(), 4);
        assert!(report.variable_bounds.iter().any(|activity| {
            activity.side == BoundSide::Lower
                && activity.variable_id.0.contains("x")
                && activity.status == ActivityStatus::Active
        }));
        assert!(report.variable_bounds.iter().all(|activity| {
            matches!(activity.evidence, ActivityEvidence::VariableBound { .. })
        }));
    }

    #[test]
    fn cp_global_constraint_has_semantic_status_without_fake_slack() {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("cp-activity");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 2).expect("domain"))
            .expect("x");
        model
            .register_variable(y.clone(), IntegerDomain::range(0, 2).expect("domain"))
            .expect("y");
        // `ForbiddenAssignments` 没有无歧义的标量"边界距离"：满足时无法说"离违反还有多远"。
        // 因此它必须保持 `SatisfiedWithoutSlackMetric` 且不带任何 slack——这是计划 2.2 的红线。
        //
        // `ForbiddenAssignments` has no unambiguous scalar boundary distance: when satisfied there is
        // no meaningful "how far from violation". It must therefore keep
        // `SatisfiedWithoutSlackMetric` with no slack, which is plan 2.2's rule.
        model
            .add_constraint(ConstraintDefinition::new(
                "forbidden",
                ConstraintProgrammingConstraint::ForbiddenAssignments {
                    expressions: vec![
                        IntegerExpression::variable(x.clone()),
                        IntegerExpression::variable(y.clone()),
                    ],
                    tuples: vec![vec![0, 0]],
                },
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let values = BTreeMap::from([
            (StableVariableId::from("x"), 1),
            (StableVariableId::from("y"), 1),
        ]);
        let report = ConstraintActivityAnalyzer::default()
            .analyze_cp(&snapshot, &values)
            .expect("activity report");
        let activity = &report.constraints[0];
        assert_eq!(activity.status, ActivityStatus::SatisfiedWithoutSlackMetric);
        assert_eq!(activity.slack, None);
        assert_eq!(activity.normalized_slack, None);
        assert_eq!(activity.satisfied, Some(true));
        assert!(!activity.evidence.has_slack_metric());
        assert_eq!(
            activity.evidence,
            ActivityEvidence::Semantic {
                satisfied: true,
                margin: None,
            }
        );
    }

    #[test]
    fn cp_all_different_reports_a_natural_margin() {
        // `AllDifferent` 有明确语义的自然 margin：最小两两间距减一。该测试覆盖贴边、宽松与违反
        // 三种情形，并确认 margin 确实进入了公开证据（`has_slack_metric` 为真）。
        //
        // `AllDifferent` has an unambiguous natural margin: smallest pairwise gap minus one. This
        // covers the boundary, slack, and violated cases and confirms the margin reaches the public
        // evidence (`has_slack_metric` is true).
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("cp-margin");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 5).expect("domain"))
            .expect("x");
        model
            .register_variable(y.clone(), IntegerDomain::range(0, 5).expect("domain"))
            .expect("y");
        model
            .add_constraint(ConstraintDefinition::new(
                "different",
                ConstraintProgrammingConstraint::AllDifferent {
                    expressions: vec![
                        IntegerExpression::variable(x.clone()),
                        IntegerExpression::variable(y.clone()),
                    ],
                },
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let analyzer = ConstraintActivityAnalyzer::default();

        let activity_for = |left: i64, right: i64| {
            let values = BTreeMap::from([
                (StableVariableId::from("x"), left),
                (StableVariableId::from("y"), right),
            ]);
            analyzer
                .analyze_cp(&snapshot, &values)
                .expect("activity report")
                .constraints[0]
                .clone()
        };

        // 相邻取值：间距 1 → margin 0 → 已贴边。
        // Adjacent values: gap 1 gives margin 0, i.e. exactly at the boundary.
        let boundary = activity_for(0, 1);
        assert_eq!(boundary.status, ActivityStatus::Active);
        assert_eq!(boundary.slack, Some(0.0));
        assert_eq!(boundary.satisfied, Some(true));
        assert!(boundary.evidence.has_slack_metric());
        assert_eq!(
            boundary.evidence,
            ActivityEvidence::Semantic {
                satisfied: true,
                margin: Some(0.0),
            }
        );

        // 间距充足：margin 为正 → 不贴边。
        // A wide gap gives a positive margin, so the constraint is not near its boundary.
        let wide = activity_for(0, 5);
        assert_eq!(wide.status, ActivityStatus::Inactive);
        assert_eq!(wide.slack, Some(4.0));
        assert_eq!(wide.satisfied, Some(true));

        // 取值重复：语义违反且 margin 为负，二者必须一致。
        // A duplicate violates the semantics and gives a negative margin; the two must agree.
        let violated = activity_for(2, 2);
        assert_eq!(violated.status, ActivityStatus::Violated);
        assert_eq!(violated.slack, Some(-1.0));
        assert_eq!(violated.satisfied, Some(false));
    }

    #[test]
    fn cp_integer_comparison_has_natural_margin() {
        let x = IntegerVariable::new("x");
        let mut model = crate::model::constraint_programming::ConstraintProgrammingModel::new(
            "cp-integer-activity",
        );
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("x");
        model
            .add_constraint(ConstraintDefinition::new(
                "limit",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x),
                    IntegerRelation::LessOrEqual,
                    5,
                ),
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let values = BTreeMap::from([(StableVariableId::from("x"), 4)]);
        let report = ConstraintActivityAnalyzer::default()
            .analyze_cp(&snapshot, &values)
            .expect("activity report");
        assert_eq!(report.constraints[0].slack, Some(1.0));
        assert_eq!(report.constraints[0].status, ActivityStatus::Inactive);
        assert!(matches!(
            report.constraints[0].evidence,
            ActivityEvidence::IntegerComparison { .. }
        ));
    }

    #[test]
    fn cp_all_different_margin_handles_i64_extremes() {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model = crate::model::constraint_programming::ConstraintProgrammingModel::new(
            "cp-extreme-margin",
        );
        let extreme_domain = IntegerDomain::values([i64::MIN, i64::MAX]).expect("domain");
        model
            .register_variable(x.clone(), extreme_domain.clone())
            .expect("x");
        model
            .register_variable(y.clone(), extreme_domain)
            .expect("y");
        model
            .add_constraint(ConstraintDefinition::new(
                "different",
                ConstraintProgrammingConstraint::AllDifferent {
                    expressions: vec![
                        IntegerExpression::variable(x.clone()),
                        IntegerExpression::variable(y.clone()),
                    ],
                },
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let values = BTreeMap::from([
            (StableVariableId::from("x"), i64::MIN),
            (StableVariableId::from("y"), i64::MAX),
        ]);
        let report = ConstraintActivityAnalyzer::default()
            .analyze_cp(&snapshot, &values)
            .expect("activity report");
        let expected = (i128::from(i64::MAX) - i128::from(i64::MIN) - 1) as f64;
        assert_eq!(report.constraints[0].slack, Some(expected));
        assert_eq!(report.constraints[0].status, ActivityStatus::Inactive);
        assert_eq!(report.constraints[0].satisfied, Some(true));
    }
}
