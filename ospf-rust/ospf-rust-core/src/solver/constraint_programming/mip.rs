//! CP 到线性 MIP 的严格有限降维 / Strict finite CP-to-linear-MIP lowering.
//!
//! 本模块只接受可以证明与原始 `i64` CP 语义等价的有限 formulation。所有整数常量在
//! 进入既有 `f64` 线性模型前都必须可无损表示；未实现的 global constraint 显式返回
//! `Unsupported`，不会被静默忽略。/ This module accepts only finite formulations that are
//! provably equivalent to the source `i64` CP semantics. Every integer constant must be exactly
//! representable before crossing the existing `f64` linear-model boundary. Unsupported global
//! constraints return `Unsupported` instead of being silently ignored.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use super::validate_conflict_assumption_ids;
use crate::error::{CoreError, SolverError};
use crate::model::ObjectiveCategory;
use crate::model::constraint_programming::{
    BooleanLiteral, ConstraintProgrammingConstraint, ConstraintProgrammingSnapshot, IntegerDomain,
    IntegerExpression, IntegerRelation, IntegerVariable, IntervalDuration, IntervalVariableId,
};
use crate::model::intermediate::{LinearTriadModel, SparseVector};
use crate::solver::{
    AuditFingerprint, ConstraintProgrammingAssumption, ConstraintProgrammingSession,
    ConstraintProgrammingSolveOptions, ConstraintProgrammingSolver, ConstraintProgrammingSupport,
    ConstraintProgrammingSupportReport, LinearSolver, ProblemStatus, SolveDiagnostics,
    SolveFingerprints, SolveOptions, SolveProof, SolveReport, SolveSolution, SolveStage,
    SolveStatistics, SolveWarning, SolverCapability, SolverDescriptor, SolverInfo,
    SolverProvenance, StableConstraintId, StableVariableId, TerminationReason,
};
use crate::token::Token;
use crate::variable::{
    Binary, Integer, VariableId, VariableItem, VariableRange, VariableType, new_group_id,
};

type Result<T, E = CoreError> = std::result::Result<T, E>;

/// 严格 MIP 降维错误 / Strict MIP-lowering failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MipLoweringError {
    /// 当前 formulation 尚未实现 / The formulation is not implemented.
    Unsupported {
        /// 关联的源约束 / Related source constraint.
        constraint: Option<StableConstraintId>,
        /// 稳定能力或 formulation 名称 / Stable capability or formulation name.
        feature: String,
    },
    /// 整数到 backend 数值的转换不精确 / Integer-to-backend conversion is inexact.
    Numeric {
        /// 数值字段 / Numeric field.
        field: String,
        /// 原始值 / Original value.
        value: String,
    },
    /// 生成模型超出显式预算 / Generated model exceeds an explicit budget.
    BudgetExceeded {
        /// 预算资源 / Budgeted resource.
        resource: String,
        /// 预算上限 / Budget limit.
        limit: usize,
        /// 实际需求 / Actual requirement.
        actual: usize,
    },
    /// CP snapshot 或 formulation 结构无效 / Invalid CP snapshot or formulation structure.
    Invalid {
        /// 失败说明 / Failure description.
        message: String,
    },
}

impl Display for MipLoweringError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported {
                constraint,
                feature,
            } => {
                if let Some(constraint) = constraint {
                    write!(
                        formatter,
                        "unsupported CP MIP formulation {feature} at {constraint}"
                    )
                } else {
                    write!(formatter, "unsupported CP MIP formulation {feature}")
                }
            }
            Self::Numeric { field, value } => {
                write!(
                    formatter,
                    "exact CP MIP numeric conversion failed for {field}: {value}"
                )
            }
            Self::BudgetExceeded {
                resource,
                limit,
                actual,
            } => write!(
                formatter,
                "CP MIP lowering budget {resource} exceeded: {actual} > {limit}"
            ),
            Self::Invalid { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for MipLoweringError {}

/// 严格 CP MIP 降维参数 / Strict CP MIP-lowering options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MipLoweringOptions {
    /// 生成模型允许的总变量数 / Maximum total generated variables.
    pub max_variables: usize,
    /// 生成模型允许的总线性行数 / Maximum total generated rows.
    pub max_constraints: usize,
    /// 允许的辅助变量数 / Maximum auxiliary variables.
    pub max_auxiliary_variables: usize,
    /// 单个 table 允许的元组数 / Maximum tuples in one table.
    pub max_table_tuples: usize,
    /// table 允许的最大列数 / Maximum table width.
    pub max_table_width: usize,
    /// 允许的绝对 Big-M 上限 / Maximum absolute Big-M value.
    pub max_big_m: i64,
}

impl Default for MipLoweringOptions {
    fn default() -> Self {
        Self {
            max_variables: 100_000,
            max_constraints: 300_000,
            max_auxiliary_variables: 100_000,
            max_table_tuples: 10_000,
            max_table_width: 256,
            max_big_m: 9_007_199_254_740_992,
        }
    }
}

/// 生成 MIP 行的来源 / Origin of a generated MIP row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MipRowOrigin {
    /// 源 CP 约束 / Source CP constraint.
    Constraint(StableConstraintId),
    /// 区间定义行 / Interval-definition row.
    Interval(IntervalVariableId),
}

/// 生成 MIP 变量的来源 / Origin of a generated MIP variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MipVariableOrigin {
    /// 源 CP 变量 / Source CP variable.
    Source(StableVariableId),
    /// 辅助变量 / Auxiliary variable.
    Auxiliary {
        /// 辅助稳定 ID / Auxiliary stable ID.
        stable_id: StableVariableId,
        /// 生成原因 / Generation reason.
        reason: String,
    },
}

/// 降维统计 / Lowering statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MipLoweringStatistics {
    /// 源变量数 / Source variable count.
    pub source_variables: usize,
    /// 辅助变量数 / Auxiliary variable count.
    pub auxiliary_variables: usize,
    /// 源约束数 / Source constraint count.
    pub source_constraints: usize,
    /// 生成线性行数 / Generated linear-row count.
    pub generated_constraints: usize,
    /// 生成 Boolean 辅助变量数 / Generated Boolean auxiliary count.
    pub boolean_auxiliaries: usize,
    /// table 元组总数 / Total table tuple count.
    pub table_tuples: usize,
}

/// CP snapshot 的线性 MIP 降维结果 / Linear-MIP lowering result for a CP snapshot.
#[derive(Debug, Clone)]
pub struct MipLoweringResult {
    /// 生成的线性模型 / Generated linear model.
    pub model: LinearTriadModel,
    /// 源稳定变量到 solver 索引的映射 / Source stable-variable to solver-index map.
    pub source_variables: BTreeMap<StableVariableId, usize>,
    /// 辅助稳定变量到 solver 索引的映射 / Auxiliary stable-variable to solver-index map.
    pub auxiliary_variables: BTreeMap<StableVariableId, usize>,
    /// 按线性行索引排列的来源 / Origins indexed by linear-row index.
    pub row_origins: Vec<MipRowOrigin>,
    /// 按 solver 变量索引排列的来源 / Origins indexed by solver-variable index.
    pub variable_origins: Vec<MipVariableOrigin>,
    /// 降维统计 / Lowering statistics.
    pub statistics: MipLoweringStatistics,
    /// 源 CP snapshot 指纹 / Source CP snapshot fingerprint.
    pub source_fingerprint: AuditFingerprint,
    /// 未进入线性目标的 CP 常数项 / CP objective constant omitted from the linear objective.
    pub objective_constant: i64,
}

impl MipLoweringResult {
    /// 将 solver 向量投影回源 CP 赋值并复核 / Project and verify a solver vector as a source CP assignment.
    pub fn project_solution(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        values: &[f64],
    ) -> Result<BTreeMap<StableVariableId, i64>, MipLoweringError> {
        if snapshot.fingerprint != self.source_fingerprint {
            return Err(MipLoweringError::Invalid {
                message: "CP snapshot fingerprint does not match the lowering result".to_owned(),
            });
        }
        if values.len() != self.model.num_variables() {
            return Err(MipLoweringError::Invalid {
                message: format!(
                    "MIP solution dimension {} does not match generated variable count {}",
                    values.len(),
                    self.model.num_variables()
                ),
            });
        }
        self.validate_mip_solution(values)?;
        let mut assignment = BTreeMap::new();
        for variable in &snapshot.variables {
            let index = self
                .source_variables
                .get(&variable.variable.stable_id)
                .ok_or_else(|| MipLoweringError::Invalid {
                    message: format!(
                        "source variable {} is missing from the lowering map",
                        variable.variable.stable_id
                    ),
                })?;
            let value = values[*index];
            let integer = exact_f64_integer(value, variable.variable.stable_id.0.as_str())?;
            if !variable.domain.contains(integer) {
                return Err(MipLoweringError::Invalid {
                    message: format!(
                        "projected value {}={} is outside the source domain",
                        variable.variable.stable_id, integer
                    ),
                });
            }
            assignment.insert(variable.variable.stable_id.clone(), integer);
        }
        snapshot
            .validate_assignment(&assignment)
            .map_err(|error| MipLoweringError::Invalid {
                message: format!("projected MIP solution failed CP verification: {error}"),
            })?;
        Ok(assignment)
    }

    fn validate_mip_solution(&self, values: &[f64]) -> Result<(), MipLoweringError> {
        let integer_values = values
            .iter()
            .enumerate()
            .map(|(index, value)| exact_f64_integer(*value, &format!("mip.solution[{index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        for (index, value) in integer_values.iter().enumerate() {
            let lower =
                exact_f64_integer(self.model.basic.lb[index], &format!("mip.lower[{index}]"))?;
            let upper =
                exact_f64_integer(self.model.basic.ub[index], &format!("mip.upper[{index}]"))?;
            if *value < lower || *value > upper {
                return Err(MipLoweringError::Invalid {
                    message: format!(
                        "MIP solution value at index {index} is outside [{lower}, {upper}]"
                    ),
                });
            }
            if self.model.basic.var_types[index] == crate::variable::VariableType::Binary
                && !matches!(*value, 0 | 1)
            {
                return Err(MipLoweringError::Invalid {
                    message: format!("MIP binary solution at index {index} is {value}"),
                });
            }
        }
        for (row_index, row) in self.model.basic.A.rows.iter().enumerate() {
            let lhs = row
                .entries
                .iter()
                .try_fold(0_i128, |sum, (index, coefficient)| {
                    let coefficient = exact_f64_integer(
                        *coefficient,
                        &format!("mip.row[{row_index}].coefficient"),
                    )?;
                    let coefficient = i128::from(coefficient);
                    sum.checked_add(
                        coefficient
                            .checked_mul(i128::from(integer_values[*index]))
                            .ok_or_else(|| numeric("MIP row evaluation overflow", coefficient))?,
                    )
                    .ok_or_else(|| numeric("MIP row evaluation overflow", sum))
                })?;
            let rhs = exact_f64_integer(
                self.model.basic.b[row_index],
                &format!("mip.row[{row_index}].rhs"),
            )?;
            if lhs > i128::from(rhs) {
                return Err(MipLoweringError::Invalid {
                    message: format!(
                        "MIP solution violates generated row {row_index}: {lhs} > {rhs}"
                    ),
                });
            }
        }
        Ok(())
    }

    /// 计算源 CP 目标值 / Evaluate the source CP objective value.
    pub fn objective_value(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        values: &[f64],
    ) -> Result<Option<i64>, MipLoweringError> {
        let assignment = self.project_solution(snapshot, values)?;
        snapshot
            .objective_value(&assignment)
            .map_err(|error| MipLoweringError::Invalid {
                message: format!("source CP objective verification failed: {error}"),
            })
    }
}

/// 对 CP snapshot 执行严格默认降维 / Lower a CP snapshot with default strict options.
pub fn lower_constraint_programming(
    snapshot: &ConstraintProgrammingSnapshot,
) -> Result<MipLoweringResult, MipLoweringError> {
    lower_constraint_programming_with_options(snapshot, MipLoweringOptions::default())
}

/// 按显式预算执行严格降维 / Lower a CP snapshot under explicit budgets.
pub fn lower_constraint_programming_with_options(
    snapshot: &ConstraintProgrammingSnapshot,
    options: MipLoweringOptions,
) -> Result<MipLoweringResult, MipLoweringError> {
    snapshot
        .validate_identity()
        .map_err(|error| MipLoweringError::Invalid {
            message: format!("invalid CP snapshot: {error}"),
        })?;
    if options.max_big_m < 0 {
        return Err(MipLoweringError::Invalid {
            message: "MIP lowering max_big_m must not be negative".to_owned(),
        });
    }
    let mut context = LoweringContext::new(snapshot, options);
    context.register_source_variables()?;
    context.register_intervals()?;
    for constraint in &snapshot.constraints {
        let origin = MipRowOrigin::Constraint(constraint.id.clone());
        let truth = context.compile_constraint(&constraint.constraint, &origin)?;
        context.enforce_truth(&truth, &origin)?;
    }
    context.compile_objective()?;
    context.finish()
}

const MAX_EXACT_F64_INTEGER: i128 = 1_i128 << 53;

fn exact_i128(value: i128, field: impl Into<String>) -> Result<f64, MipLoweringError> {
    if value.unsigned_abs() > MAX_EXACT_F64_INTEGER as u128 {
        return Err(MipLoweringError::Numeric {
            field: field.into(),
            value: value.to_string(),
        });
    }
    Ok(value as f64)
}

fn exact_i64(value: i128, field: impl Into<String>) -> Result<i64, MipLoweringError> {
    i64::try_from(value).map_err(|_| MipLoweringError::Numeric {
        field: field.into(),
        value: value.to_string(),
    })
}

fn exact_f64_integer(value: f64, field: &str) -> Result<i64, MipLoweringError> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(MipLoweringError::Numeric {
            field: field.to_owned(),
            value: value.to_string(),
        });
    }
    let integer = value as i128;
    if integer as f64 != value {
        return Err(MipLoweringError::Numeric {
            field: field.to_owned(),
            value: value.to_string(),
        });
    }
    i64::try_from(integer).map_err(|_| MipLoweringError::Numeric {
        field: field.to_owned(),
        value: value.to_string(),
    })
}

#[derive(Debug, Clone, Default)]
struct LinearForm {
    constant: i128,
    terms: BTreeMap<usize, i128>,
    lower: i128,
    upper: i128,
}

impl LinearForm {
    fn constant(value: i128) -> Self {
        Self {
            constant: value,
            lower: value,
            upper: value,
            ..Self::default()
        }
    }

    fn variable(index: usize, lower: i128, upper: i128) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(index, 1);
        Self {
            constant: 0,
            terms,
            lower,
            upper,
        }
    }

    fn add(&self, other: &Self) -> Result<Self, MipLoweringError> {
        let mut result = Self {
            constant: self
                .constant
                .checked_add(other.constant)
                .ok_or_else(|| numeric("linear form constant overflow", 0))?,
            terms: self.terms.clone(),
            lower: self
                .lower
                .checked_add(other.lower)
                .ok_or_else(|| numeric("linear form lower bound overflow", 0))?,
            upper: self
                .upper
                .checked_add(other.upper)
                .ok_or_else(|| numeric("linear form upper bound overflow", 0))?,
        };
        for (index, coefficient) in &other.terms {
            let entry = result.terms.entry(*index).or_insert(0);
            *entry = entry
                .checked_add(*coefficient)
                .ok_or_else(|| numeric("linear form coefficient overflow", *coefficient))?;
            if *entry == 0 {
                result.terms.remove(index);
            }
        }
        Ok(result)
    }

    fn negate(&self) -> Result<Self, MipLoweringError> {
        let mut terms = BTreeMap::new();
        for (index, coefficient) in &self.terms {
            terms.insert(
                *index,
                coefficient
                    .checked_neg()
                    .ok_or_else(|| numeric("linear form coefficient overflow", *coefficient))?,
            );
        }
        Ok(Self {
            constant: self
                .constant
                .checked_neg()
                .ok_or_else(|| numeric("linear form constant overflow", self.constant))?,
            terms,
            lower: self
                .upper
                .checked_neg()
                .ok_or_else(|| numeric("linear form lower bound overflow", self.upper))?,
            upper: self
                .lower
                .checked_neg()
                .ok_or_else(|| numeric("linear form upper bound overflow", self.lower))?,
        })
    }

    fn scale(&self, factor: i128) -> Result<Self, MipLoweringError> {
        let mut terms = BTreeMap::new();
        for (index, coefficient) in &self.terms {
            let value = coefficient
                .checked_mul(factor)
                .ok_or_else(|| numeric("linear form coefficient overflow", *coefficient))?;
            if value != 0 {
                terms.insert(*index, value);
            }
        }
        let (lower, upper) = if factor >= 0 {
            (
                self.lower
                    .checked_mul(factor)
                    .ok_or_else(|| numeric("linear form lower bound overflow", self.lower))?,
                self.upper
                    .checked_mul(factor)
                    .ok_or_else(|| numeric("linear form upper bound overflow", self.upper))?,
            )
        } else {
            (
                self.upper
                    .checked_mul(factor)
                    .ok_or_else(|| numeric("linear form lower bound overflow", self.upper))?,
                self.lower
                    .checked_mul(factor)
                    .ok_or_else(|| numeric("linear form upper bound overflow", self.lower))?,
            )
        };
        Ok(Self {
            constant: self
                .constant
                .checked_mul(factor)
                .ok_or_else(|| numeric("linear form constant overflow", self.constant))?,
            terms,
            lower,
            upper,
        })
    }
}

fn numeric(message: &str, value: i128) -> MipLoweringError {
    MipLoweringError::Numeric {
        field: message.to_owned(),
        value: value.to_string(),
    }
}

#[derive(Debug, Clone)]
struct BoolForm {
    form: LinearForm,
}

impl BoolForm {
    fn constant(value: bool) -> Self {
        Self {
            form: LinearForm::constant(i128::from(value)),
        }
    }

    fn negate(&self) -> Result<Self, MipLoweringError> {
        Ok(Self {
            form: LinearForm::constant(1).add(&self.form.negate()?)?,
        })
    }
}

struct LoweringContext<'a> {
    snapshot: &'a ConstraintProgrammingSnapshot,
    options: MipLoweringOptions,
    model: LinearTriadModel,
    source_variables: BTreeMap<StableVariableId, usize>,
    auxiliary_variables: BTreeMap<StableVariableId, usize>,
    variable_origins: Vec<MipVariableOrigin>,
    row_origins: Vec<MipRowOrigin>,
    used_stable_ids: BTreeSet<StableVariableId>,
    statistics: MipLoweringStatistics,
    objective_constant: i64,
    auxiliary_sequence: usize,
}

impl<'a> LoweringContext<'a> {
    fn new(snapshot: &'a ConstraintProgrammingSnapshot, options: MipLoweringOptions) -> Self {
        Self {
            snapshot,
            options,
            model: LinearTriadModel::new(&snapshot.name),
            source_variables: BTreeMap::new(),
            auxiliary_variables: BTreeMap::new(),
            variable_origins: Vec::new(),
            row_origins: Vec::new(),
            used_stable_ids: snapshot
                .variables
                .iter()
                .map(|variable| variable.variable.stable_id.clone())
                .collect(),
            statistics: MipLoweringStatistics {
                source_constraints: snapshot.constraints.len(),
                ..MipLoweringStatistics::default()
            },
            objective_constant: 0,
            auxiliary_sequence: 0,
        }
    }

    fn register_source_variables(&mut self) -> Result<(), MipLoweringError> {
        for variable in &self.snapshot.variables {
            let (lower, upper) = domain_bounds(&variable.domain)?;
            let lower_f64 = exact_i128(lower, format!("{}.lower", variable.variable.stable_id))?;
            let upper_f64 = exact_i128(upper, format!("{}.upper", variable.variable.stable_id))?;
            let index = self.add_model_variable(
                VariableItem::<Integer>::with_range(
                    VariableId::standalone(new_group_id()),
                    &format!("cp::{}", variable.variable.stable_id),
                    VariableRange::bounded(lower_f64, upper_f64),
                ),
                VariableType::Integer,
                MipVariableOrigin::Source(variable.variable.stable_id.clone()),
            )?;
            self.source_variables
                .insert(variable.variable.stable_id.clone(), index);
            self.statistics.source_variables = self.statistics.source_variables.saturating_add(1);
            if let IntegerDomain::Values(values) = &variable.domain {
                self.lower_sparse_domain(&variable.variable.stable_id, index, values)?;
            }
        }
        Ok(())
    }

    fn register_intervals(&mut self) -> Result<(), MipLoweringError> {
        for interval in &self.snapshot.intervals {
            let origin = MipRowOrigin::Interval(interval.interval.id.clone());
            let start = self.expression_form(&interval.interval.start)?;
            let end = self.expression_form(&interval.interval.end)?;
            let duration = match &interval.interval.duration {
                IntervalDuration::Fixed(value) => LinearForm::constant(i128::from(*value)),
                IntervalDuration::Variable(variable) => self.variable_form(&variable.stable_id)?,
            };
            let relation = end.add(&start.negate()?)?.add(&duration.negate()?)?;
            let truth = self.relation_truth(&relation, IntegerRelation::Equal, 0, &origin)?;
            if let Some(presence) = &interval.interval.presence {
                let enforcement = self.literal_form(presence)?;
                let implication = self.bool_or(&[enforcement.negate()?, truth], &origin)?;
                self.enforce_truth(&implication, &origin)?;
            } else {
                self.enforce_truth(&truth, &origin)?;
            }
        }
        Ok(())
    }

    fn lower_sparse_domain(
        &mut self,
        source_id: &StableVariableId,
        source_index: usize,
        values: &[i64],
    ) -> Result<(), MipLoweringError> {
        let mut selectors = Vec::with_capacity(values.len());
        for (position, value) in values.iter().copied().enumerate() {
            let selector = self.add_aux_binary(
                "sparse-domain",
                source_id,
                position,
                "sparse domain selector",
            )?;
            selectors.push(selector);
            exact_i128(i128::from(value), format!("{source_id}.sparse[{position}]"))?;
        }
        let mut selector_sum = LinearForm::constant(0);
        let mut value_sum = LinearForm::constant(0);
        for (selector, value) in selectors.iter().zip(values) {
            selector_sum = selector_sum.add(&selector.form)?;
            value_sum = value_sum.add(&selector.form.scale(i128::from(*value))?)?;
        }
        self.add_eq(
            &selector_sum,
            1,
            &MipRowOrigin::Constraint(StableConstraintId(format!("__sparse_domain__{source_id}"))),
        )?;
        let source = self.variable_form_by_index(source_index)?;
        self.add_eq(
            &source.add(&value_sum.negate()?)?,
            0,
            &MipRowOrigin::Constraint(StableConstraintId(format!(
                "__sparse_domain_value__{source_id}"
            ))),
        )?;
        Ok(())
    }

    fn add_model_variable<T>(
        &mut self,
        variable: VariableItem<T>,
        _variable_type: VariableType,
        origin: MipVariableOrigin,
    ) -> Result<usize, MipLoweringError>
    where
        T: crate::variable::VariableTypeTrait<Value = f64>,
    {
        let next = self.model.num_variables().saturating_add(1);
        if next > self.options.max_variables {
            return Err(MipLoweringError::BudgetExceeded {
                resource: "variables".to_owned(),
                limit: self.options.max_variables,
                actual: next,
            });
        }
        let index = self.model.num_variables();
        self.model
            .basic
            .add_variable(Token::from_generic(variable, index));
        self.variable_origins.push(origin);
        Ok(index)
    }

    fn add_aux_binary(
        &mut self,
        kind: &str,
        source: &StableVariableId,
        ordinal: usize,
        reason: &str,
    ) -> Result<BoolForm, MipLoweringError> {
        let stable_id = StableVariableId(format!(
            "__ospf_cp_aux__{kind}__{}__{source}__{ordinal}",
            source.0.len()
        ));
        if !self.used_stable_ids.insert(stable_id.clone()) {
            return Err(MipLoweringError::Invalid {
                message: format!("auxiliary stable ID collides with an existing ID: {stable_id}"),
            });
        }
        let index = self.add_model_variable(
            VariableItem::<Binary>::with_range(
                VariableId::standalone(new_group_id()),
                &format!("cp::aux::{stable_id}"),
                VariableRange::bounded(0.0, 1.0),
            ),
            VariableType::Binary,
            MipVariableOrigin::Auxiliary {
                stable_id: stable_id.clone(),
                reason: reason.to_owned(),
            },
        )?;
        self.auxiliary_variables.insert(stable_id, index);
        self.statistics.auxiliary_variables = self.statistics.auxiliary_variables.saturating_add(1);
        self.statistics.boolean_auxiliaries = self.statistics.boolean_auxiliaries.saturating_add(1);
        if self.statistics.auxiliary_variables > self.options.max_auxiliary_variables {
            return Err(MipLoweringError::BudgetExceeded {
                resource: "auxiliary_variables".to_owned(),
                limit: self.options.max_auxiliary_variables,
                actual: self.statistics.auxiliary_variables,
            });
        }
        Ok(BoolForm {
            form: LinearForm::variable(index, 0, 1),
        })
    }

    fn variable_form(&self, id: &StableVariableId) -> Result<LinearForm, MipLoweringError> {
        let index =
            self.source_variables
                .get(id)
                .copied()
                .ok_or_else(|| MipLoweringError::Invalid {
                    message: format!("unknown CP variable {id}"),
                })?;
        self.variable_form_by_index(index)
    }

    fn variable_form_by_index(&self, index: usize) -> Result<LinearForm, MipLoweringError> {
        let origin = self
            .variable_origins
            .get(index)
            .ok_or_else(|| MipLoweringError::Invalid {
                message: format!("missing generated variable origin for index {index}"),
            })?;
        let stable_id = match origin {
            MipVariableOrigin::Source(id) => id,
            MipVariableOrigin::Auxiliary { stable_id, .. } => stable_id,
        };
        let (lower, upper) = if let Some(variable) = self.snapshot.variable(stable_id) {
            domain_bounds(&variable.domain)?
        } else {
            (0, 1)
        };
        Ok(LinearForm::variable(index, lower, upper))
    }

    fn expression_form(
        &self,
        expression: &IntegerExpression,
    ) -> Result<LinearForm, MipLoweringError> {
        exact_i128(i128::from(expression.constant), "expression.constant")?;
        let mut form = LinearForm::constant(i128::from(expression.constant));
        for term in &expression.terms {
            exact_i128(
                i128::from(term.coefficient),
                format!("expression.{}.coefficient", term.variable.stable_id),
            )?;
            let variable = self.variable_form(&term.variable.stable_id)?;
            form = form.add(&variable.scale(i128::from(term.coefficient))?)?;
        }
        Ok(form)
    }

    fn literal_form(&self, literal: &BooleanLiteral) -> Result<BoolForm, MipLoweringError> {
        let form = self.variable_form(&literal.variable.stable_id)?;
        Ok(BoolForm {
            form: if literal.negated {
                LinearForm::constant(1).add(&form.negate()?)?
            } else {
                form
            },
        })
    }

    fn add_row_le(
        &mut self,
        form: &LinearForm,
        rhs: i128,
        origin: &MipRowOrigin,
    ) -> Result<(), MipLoweringError> {
        let next = self.model.num_constraints().saturating_add(1);
        if next > self.options.max_constraints {
            return Err(MipLoweringError::BudgetExceeded {
                resource: "constraints".to_owned(),
                limit: self.options.max_constraints,
                actual: next,
            });
        }
        let adjusted_rhs = rhs
            .checked_sub(form.constant)
            .ok_or_else(|| numeric("constraint right-hand side overflow", rhs))?;
        let mut row = SparseVector::new();
        for (index, coefficient) in &form.terms {
            if *coefficient != 0 {
                row.add(
                    *index,
                    exact_i128(*coefficient, format!("row[{index}].coefficient"))?,
                );
            }
        }
        let rhs = exact_i128(adjusted_rhs, "constraint.right_hand_side")?;
        let row_index = self.model.basic.add_constraint_with_metadata(
            row,
            rhs,
            format!("cp::row::{}", self.row_origins.len()),
            None,
            false,
            0,
            Some(format!("{origin:?}")),
            None,
        );
        debug_assert_eq!(row_index, self.row_origins.len());
        self.row_origins.push(origin.clone());
        self.statistics.generated_constraints =
            self.statistics.generated_constraints.saturating_add(1);
        Ok(())
    }

    fn add_eq(
        &mut self,
        form: &LinearForm,
        rhs: i128,
        origin: &MipRowOrigin,
    ) -> Result<(), MipLoweringError> {
        self.add_row_le(form, rhs, origin)?;
        self.add_row_le(&form.negate()?, -rhs, origin)
    }

    fn enforce_truth(
        &mut self,
        truth: &BoolForm,
        origin: &MipRowOrigin,
    ) -> Result<(), MipLoweringError> {
        if truth.form.terms.is_empty() {
            if truth.form.constant == 1 {
                return Ok(());
            }
            if truth.form.constant == 0 {
                return self.add_row_le(&LinearForm::constant(0), -1, origin);
            }
            return Err(MipLoweringError::Invalid {
                message: format!(
                    "Boolean formulation has non-Boolean constant {}",
                    truth.form.constant
                ),
            });
        }
        self.add_eq(&truth.form, 1, origin)
    }

    fn relation_truth(
        &mut self,
        form: &LinearForm,
        relation: IntegerRelation,
        rhs: i128,
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        match relation {
            IntegerRelation::LessOrEqual => {
                if form.upper <= rhs {
                    return Ok(BoolForm::constant(true));
                }
                if form.lower > rhs {
                    return Ok(BoolForm::constant(false));
                }
                let truth = self.add_aux_binary(
                    "relation",
                    &StableVariableId::from(origin_key(origin)),
                    self.auxiliary_sequence,
                    "relation truth",
                )?;
                self.auxiliary_sequence = self.auxiliary_sequence.saturating_add(1);
                let upper_m = form
                    .upper
                    .checked_sub(rhs)
                    .ok_or_else(|| numeric("upper Big-M overflow", form.upper))?;
                let lower_m = rhs
                    .checked_add(1)
                    .and_then(|value| value.checked_sub(form.lower))
                    .ok_or_else(|| numeric("lower Big-M overflow", rhs))?;
                self.check_big_m(upper_m)?;
                self.check_big_m(lower_m)?;
                let first = form.add(&truth.form.scale(upper_m)?)?;
                self.add_row_le(
                    &first,
                    rhs.checked_add(upper_m)
                        .ok_or_else(|| numeric("constraint right-hand side overflow", rhs))?,
                    origin,
                )?;
                let second = form.negate()?.add(&truth.form.scale(-lower_m)?)?;
                self.add_row_le(
                    &second,
                    (-rhs)
                        .checked_sub(1)
                        .ok_or_else(|| numeric("constraint right-hand side overflow", rhs))?,
                    origin,
                )?;
                Ok(truth)
            }
            IntegerRelation::GreaterOrEqual => {
                self.relation_truth(&form.negate()?, IntegerRelation::LessOrEqual, -rhs, origin)
            }
            IntegerRelation::Equal => {
                let lower = self.relation_truth(form, IntegerRelation::LessOrEqual, rhs, origin)?;
                let upper =
                    self.relation_truth(form, IntegerRelation::GreaterOrEqual, rhs, origin)?;
                self.bool_and(&[lower, upper], origin)
            }
            IntegerRelation::NotEqual => {
                let lower =
                    self.relation_truth(form, IntegerRelation::LessOrEqual, rhs - 1, origin)?;
                let upper =
                    self.relation_truth(form, IntegerRelation::GreaterOrEqual, rhs + 1, origin)?;
                self.bool_or(&[lower, upper], origin)
            }
        }
    }

    fn check_big_m(&self, value: i128) -> Result<(), MipLoweringError> {
        if value < 0 || value > i128::from(self.options.max_big_m) {
            return Err(MipLoweringError::BudgetExceeded {
                resource: "big_m".to_owned(),
                limit: self.options.max_big_m as usize,
                actual: usize::try_from(value.max(0)).unwrap_or(usize::MAX),
            });
        }
        exact_i128(value, "big_m")?;
        Ok(())
    }

    fn bool_and(
        &mut self,
        values: &[BoolForm],
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        if values.is_empty() {
            return Ok(BoolForm::constant(true));
        }
        let result = self.add_aux_binary(
            "and",
            &StableVariableId::from(origin_key(origin)),
            self.auxiliary_sequence,
            "AND truth",
        )?;
        self.auxiliary_sequence = self.auxiliary_sequence.saturating_add(1);
        let mut sum = LinearForm::constant(0);
        for value in values {
            sum = sum.add(&value.form)?;
            self.add_row_le(&result.form.add(&value.form.negate()?)?, 0, origin)?;
        }
        let bound = i128::try_from(values.len().saturating_sub(1))
            .map_err(|_| numeric("AND arity overflow", values.len() as i128))?;
        self.add_row_le(&sum.add(&result.form.negate()?)?, bound, origin)?;
        Ok(result)
    }

    fn bool_or(
        &mut self,
        values: &[BoolForm],
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        if values.is_empty() {
            return Ok(BoolForm::constant(false));
        }
        let result = self.add_aux_binary(
            "or",
            &StableVariableId::from(origin_key(origin)),
            self.auxiliary_sequence,
            "OR truth",
        )?;
        self.auxiliary_sequence = self.auxiliary_sequence.saturating_add(1);
        let mut sum = LinearForm::constant(0);
        for value in values {
            sum = sum.add(&value.form)?;
            self.add_row_le(&value.form.add(&result.form.negate()?)?, 0, origin)?;
        }
        self.add_row_le(&result.form.add(&sum.negate()?)?, 0, origin)?;
        Ok(result)
    }

    fn bool_xor(
        &mut self,
        values: &[BoolForm],
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        let Some(first) = values.first() else {
            return Ok(BoolForm::constant(false));
        };
        let mut current = first.clone();
        for value in values.iter().skip(1) {
            let result = self.add_aux_binary(
                "xor",
                &StableVariableId::from(origin_key(origin)),
                self.auxiliary_sequence,
                "XOR truth",
            )?;
            self.auxiliary_sequence = self.auxiliary_sequence.saturating_add(1);
            self.add_row_le(
                &result
                    .form
                    .add(&current.form.negate()?)?
                    .add(&value.form.negate()?)?,
                0,
                origin,
            )?;
            self.add_row_le(
                &result.form.add(&current.form)?.add(&value.form)?,
                2,
                origin,
            )?;
            self.add_row_le(
                &current
                    .form
                    .add(&value.form.negate()?)?
                    .add(&result.form.negate()?)?,
                0,
                origin,
            )?;
            self.add_row_le(
                &value
                    .form
                    .add(&current.form.negate()?)?
                    .add(&result.form.negate()?)?,
                0,
                origin,
            )?;
            current = result;
        }
        Ok(current)
    }

    fn compile_constraint(
        &mut self,
        constraint: &ConstraintProgrammingConstraint,
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        match constraint {
            ConstraintProgrammingConstraint::Integer {
                expression,
                relation,
                rhs,
            } => self.relation_truth(
                &self.expression_form(expression)?,
                *relation,
                i128::from(*rhs),
                origin,
            ),
            ConstraintProgrammingConstraint::Boolean { literal } => self.literal_form(literal),
            ConstraintProgrammingConstraint::And { literals } => {
                let values = literals
                    .iter()
                    .map(|literal| self.literal_form(literal))
                    .collect::<Result<Vec<_>, _>>()?;
                self.bool_and(&values, origin)
            }
            ConstraintProgrammingConstraint::Or { literals } => {
                let values = literals
                    .iter()
                    .map(|literal| self.literal_form(literal))
                    .collect::<Result<Vec<_>, _>>()?;
                self.bool_or(&values, origin)
            }
            ConstraintProgrammingConstraint::Xor { literals } => {
                let values = literals
                    .iter()
                    .map(|literal| self.literal_form(literal))
                    .collect::<Result<Vec<_>, _>>()?;
                self.bool_xor(&values, origin)
            }
            ConstraintProgrammingConstraint::AtMostOne { literals } => {
                let values = literals
                    .iter()
                    .map(|literal| self.literal_form(literal))
                    .collect::<Result<Vec<_>, _>>()?;
                if values.is_empty() {
                    return Ok(BoolForm::constant(true));
                }
                let mut sum = LinearForm::constant(0);
                for value in values {
                    sum = sum.add(&value.form)?;
                }
                self.relation_truth(&sum, IntegerRelation::LessOrEqual, 1, origin)
            }
            ConstraintProgrammingConstraint::ExactlyOne { literals } => {
                let values = literals
                    .iter()
                    .map(|literal| self.literal_form(literal))
                    .collect::<Result<Vec<_>, _>>()?;
                if values.is_empty() {
                    return Ok(BoolForm::constant(false));
                }
                let mut sum = LinearForm::constant(0);
                for value in values {
                    sum = sum.add(&value.form)?;
                }
                self.relation_truth(&sum, IntegerRelation::Equal, 1, origin)
            }
            ConstraintProgrammingConstraint::Implication {
                enforcement,
                constraint,
            } => {
                let enforcement = self.literal_form(enforcement)?;
                let consequent = self.compile_constraint(constraint, origin)?;
                self.bool_or(&[enforcement.negate()?, consequent], origin)
            }
            ConstraintProgrammingConstraint::Reification {
                literal,
                constraint,
                direction,
            } => {
                let literal = self.literal_form(literal)?;
                let truth = self.compile_constraint(constraint, origin)?;
                match direction {
                    crate::model::constraint_programming::ReificationDirection::Equivalent => {
                        let left = self.bool_or(&[literal.negate()?, truth.clone()], origin)?;
                        let right = self.bool_or(&[truth.negate()?, literal], origin)?;
                        self.bool_and(&[left, right], origin)
                    }
                    crate::model::constraint_programming::ReificationDirection::ImpliedByLiteral => {
                        self.bool_or(&[literal.negate()?, truth], origin)
                    }
                    crate::model::constraint_programming::ReificationDirection::ImpliesLiteral => {
                        self.bool_or(&[truth.negate()?, literal], origin)
                    }
                }
            }
            ConstraintProgrammingConstraint::AllDifferent { expressions } => {
                let mut values = Vec::new();
                for (left_index, left) in expressions.iter().enumerate() {
                    let left = self.expression_form(left)?;
                    for right in expressions.iter().skip(left_index + 1) {
                        let right = self.expression_form(right)?;
                        let difference = left.add(&right.negate()?)?;
                        values.push(self.relation_truth(
                            &difference,
                            IntegerRelation::NotEqual,
                            0,
                            origin,
                        )?);
                    }
                }
                self.bool_and(&values, origin)
            }
            ConstraintProgrammingConstraint::Element {
                index,
                values,
                target,
            } => self.compile_element(index, values, target, origin),
            ConstraintProgrammingConstraint::AllowedAssignments {
                expressions,
                tuples,
            } => self.compile_allowed_table(expressions, tuples, origin),
            ConstraintProgrammingConstraint::ForbiddenAssignments {
                expressions,
                tuples,
            } => self.compile_forbidden_table(expressions, tuples, origin),
            ConstraintProgrammingConstraint::NoOverlap { intervals } => {
                self.compile_no_overlap(intervals, origin)
            }
            ConstraintProgrammingConstraint::Cumulative { .. } => {
                self.unsupported(origin, "cumulative")
            }
            ConstraintProgrammingConstraint::Circuit { .. } => self.unsupported(origin, "circuit"),
            ConstraintProgrammingConstraint::Automaton { .. } => {
                self.unsupported(origin, "automaton")
            }
            ConstraintProgrammingConstraint::Reservoir { .. } => {
                self.unsupported(origin, "reservoir")
            }
        }
    }

    fn compile_element(
        &mut self,
        index: &IntegerExpression,
        values: &[i64],
        target: &IntegerExpression,
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        self.check_table(values.len(), 1)?;
        if values.is_empty() {
            return Err(MipLoweringError::Invalid {
                message: "Element table must not be empty".to_owned(),
            });
        }
        let index = self.expression_form(index)?;
        let target = self.expression_form(target)?;
        let mut selectors = Vec::with_capacity(values.len());
        for (position, value) in values.iter().copied().enumerate() {
            selectors.push(self.relation_truth(
                &index,
                IntegerRelation::Equal,
                position as i128,
                origin,
            )?);
            exact_i128(i128::from(value), format!("Element.values[{position}]"))?;
        }
        let mut selector_sum = LinearForm::constant(0);
        let mut target_sum = LinearForm::constant(0);
        for (selector, value) in selectors.iter().zip(values) {
            selector_sum = selector_sum.add(&selector.form)?;
            target_sum = target_sum.add(&selector.form.scale(i128::from(*value))?)?;
        }
        self.add_eq(&selector_sum, 1, origin)?;
        self.add_eq(&target.add(&target_sum.negate()?)?, 0, origin)?;
        Ok(BoolForm::constant(true))
    }

    fn compile_allowed_table(
        &mut self,
        expressions: &[IntegerExpression],
        tuples: &[Vec<i64>],
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        self.check_table(tuples.len(), expressions.len())?;
        if tuples.is_empty() {
            return Ok(BoolForm::constant(false));
        }
        let expressions = expressions
            .iter()
            .map(|expression| self.expression_form(expression))
            .collect::<Result<Vec<_>, _>>()?;
        let mut selectors = Vec::with_capacity(tuples.len());
        for (tuple_index, tuple) in tuples.iter().enumerate() {
            if tuple.len() != expressions.len() {
                return Err(MipLoweringError::Invalid {
                    message: "allowed table tuple width does not match expressions".to_owned(),
                });
            }
            let selector = self.add_aux_binary(
                "allowed-table",
                &StableVariableId::from(origin_key(origin)),
                tuple_index,
                "allowed-table tuple selector",
            )?;
            for (expression, value) in expressions.iter().zip(tuple) {
                let equality = self.relation_truth(
                    expression,
                    IntegerRelation::Equal,
                    i128::from(*value),
                    origin,
                )?;
                self.add_row_le(&selector.form.add(&equality.form.negate()?)?, 0, origin)?;
                exact_i128(i128::from(*value), format!("allowed_table[{tuple_index}]"))?;
            }
            selectors.push(selector);
        }
        let mut sum = LinearForm::constant(0);
        for selector in selectors {
            sum = sum.add(&selector.form)?;
        }
        self.add_eq(&sum, 1, origin)?;
        Ok(BoolForm::constant(true))
    }

    fn compile_forbidden_table(
        &mut self,
        expressions: &[IntegerExpression],
        tuples: &[Vec<i64>],
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        self.check_table(tuples.len(), expressions.len())?;
        if tuples.is_empty() {
            return Ok(BoolForm::constant(true));
        }
        let expressions = expressions
            .iter()
            .map(|expression| self.expression_form(expression))
            .collect::<Result<Vec<_>, _>>()?;
        let mut allowed = Vec::with_capacity(tuples.len());
        for (tuple_index, tuple) in tuples.iter().enumerate() {
            if tuple.len() != expressions.len() {
                return Err(MipLoweringError::Invalid {
                    message: "forbidden table tuple width does not match expressions".to_owned(),
                });
            }
            let mut equalities = Vec::with_capacity(tuple.len());
            for (expression, value) in expressions.iter().zip(tuple) {
                equalities.push(self.relation_truth(
                    expression,
                    IntegerRelation::Equal,
                    i128::from(*value),
                    origin,
                )?);
                exact_i128(
                    i128::from(*value),
                    format!("forbidden_table[{tuple_index}]"),
                )?;
            }
            let match_tuple = self.bool_and(&equalities, origin)?;
            allowed.push(match_tuple.negate()?);
        }
        self.bool_and(&allowed, origin)
    }

    fn compile_no_overlap(
        &mut self,
        intervals: &[IntervalVariableId],
        origin: &MipRowOrigin,
    ) -> Result<BoolForm, MipLoweringError> {
        let mut ids = BTreeSet::new();
        for interval in intervals {
            if !ids.insert(interval.clone()) {
                return Err(MipLoweringError::Invalid {
                    message: format!("NoOverlap contains duplicate interval {interval}"),
                });
            }
        }
        let mut pairwise = Vec::new();
        for (left_index, left_id) in intervals.iter().enumerate() {
            let left = self.interval(left_id)?.clone();
            let left_start = self.expression_form(&left.start)?;
            let left_end = self.expression_form(&left.end)?;
            let left_presence = left
                .presence
                .as_ref()
                .map(|literal| self.literal_form(literal))
                .transpose()?;
            for right_id in intervals.iter().skip(left_index + 1) {
                let right = self.interval(right_id)?.clone();
                let right_start = self.expression_form(&right.start)?;
                let right_end = self.expression_form(&right.end)?;
                let right_presence = right
                    .presence
                    .as_ref()
                    .map(|literal| self.literal_form(literal))
                    .transpose()?;
                let first = self.relation_truth(
                    &left_end.add(&right_start.negate()?)?,
                    IntegerRelation::LessOrEqual,
                    0,
                    origin,
                )?;
                let second = self.relation_truth(
                    &right_end.add(&left_start.negate()?)?,
                    IntegerRelation::LessOrEqual,
                    0,
                    origin,
                )?;
                let order = self.bool_or(&[first, second], origin)?;
                let condition = match (&left_presence, &right_presence) {
                    (Some(left_presence), Some(right_presence)) => self.bool_or(
                        &[left_presence.negate()?, right_presence.negate()?, order],
                        origin,
                    )?,
                    (Some(left_presence), None) => {
                        self.bool_or(&[left_presence.negate()?, order], origin)?
                    }
                    (None, Some(right_presence)) => {
                        self.bool_or(&[right_presence.negate()?, order], origin)?
                    }
                    (None, None) => order,
                };
                pairwise.push(condition);
            }
        }
        self.bool_and(&pairwise, origin)
    }

    fn interval(
        &self,
        id: &IntervalVariableId,
    ) -> Result<&crate::model::constraint_programming::IntervalVariable, MipLoweringError> {
        self.snapshot
            .intervals
            .iter()
            .find(|interval| interval.interval.id == *id)
            .map(|interval| &interval.interval)
            .ok_or_else(|| MipLoweringError::Invalid {
                message: format!("unknown interval {id}"),
            })
    }

    fn check_table(&self, tuple_count: usize, width: usize) -> Result<(), MipLoweringError> {
        if tuple_count > self.options.max_table_tuples {
            return Err(MipLoweringError::BudgetExceeded {
                resource: "table_tuples".to_owned(),
                limit: self.options.max_table_tuples,
                actual: tuple_count,
            });
        }
        if width > self.options.max_table_width {
            return Err(MipLoweringError::BudgetExceeded {
                resource: "table_width".to_owned(),
                limit: self.options.max_table_width,
                actual: width,
            });
        }
        Ok(())
    }

    fn unsupported<T>(&self, origin: &MipRowOrigin, feature: &str) -> Result<T, MipLoweringError> {
        Err(MipLoweringError::Unsupported {
            constraint: match origin {
                MipRowOrigin::Constraint(id) => Some(id.clone()),
                MipRowOrigin::Interval(_) => None,
            },
            feature: feature.to_owned(),
        })
    }

    fn compile_objective(&mut self) -> Result<(), MipLoweringError> {
        let Some(objective) = &self.snapshot.objective else {
            self.model.set_objective(
                vec![0.0; self.model.num_variables()],
                ObjectiveCategory::Minimum,
            );
            return Ok(());
        };
        let form = self.expression_form(&objective.expression)?;
        let mut coefficients = vec![0.0; self.model.num_variables()];
        for (index, coefficient) in form.terms {
            coefficients[index] = exact_i128(coefficient, format!("objective[{index}]"))?;
        }
        exact_i128(form.constant, "objective.constant")?;
        self.objective_constant = exact_i64(form.constant, "objective.constant")?;
        self.model.set_objective(coefficients, objective.category);
        Ok(())
    }

    fn finish(self) -> Result<MipLoweringResult, MipLoweringError> {
        Ok(MipLoweringResult {
            model: self.model,
            source_variables: self.source_variables,
            auxiliary_variables: self.auxiliary_variables,
            row_origins: self.row_origins,
            variable_origins: self.variable_origins,
            statistics: self.statistics,
            source_fingerprint: self.snapshot.fingerprint.clone(),
            objective_constant: self.objective_constant,
        })
    }
}

fn domain_bounds(domain: &IntegerDomain) -> Result<(i128, i128), MipLoweringError> {
    match domain {
        IntegerDomain::Range { lower, upper } => Ok((i128::from(*lower), i128::from(*upper))),
        IntegerDomain::Values(values) => {
            let lower = values
                .first()
                .copied()
                .ok_or_else(|| MipLoweringError::Invalid {
                    message: "sparse domain must not be empty".to_owned(),
                })?;
            let upper = values
                .last()
                .copied()
                .ok_or_else(|| MipLoweringError::Invalid {
                    message: "sparse domain must not be empty".to_owned(),
                })?;
            Ok((i128::from(lower), i128::from(upper)))
        }
    }
}

fn origin_key(origin: &MipRowOrigin) -> String {
    match origin {
        MipRowOrigin::Constraint(id) => format!("constraint-{}", id.0),
        MipRowOrigin::Interval(id) => format!("interval-{id}"),
    }
}

/// 使用既有线性 solver 的 CP 求解器 / CP solver backed by an existing linear solver.
///
/// 该 wrapper 只把严格降维结果交给 `LinearSolver`，并在返回前重新验证源 CP snapshot。
/// 它不把 backend 的浮点解直接冒充为 CP 解，也不把未传递的 limit、hint 静默丢弃。/
/// This wrapper submits only a strict lowering to `LinearSolver` and re-verifies the source CP
/// snapshot before returning. It never treats a backend floating-point vector as a CP solution
/// without projection, and it never silently drops options that cannot be forwarded.
pub struct MipBackedConstraintProgrammingSolver<S> {
    backend: Arc<S>,
    name: String,
    lowering_options: MipLoweringOptions,
}

impl<S> Clone for MipBackedConstraintProgrammingSolver<S> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            name: self.name.clone(),
            lowering_options: self.lowering_options,
        }
    }
}

impl<S: LinearSolver> MipBackedConstraintProgrammingSolver<S> {
    /// 创建 MIP-backed CP solver / Create a MIP-backed CP solver.
    pub fn new(backend: S) -> Self {
        let name = format!("cp-mip-backed::{}", backend.name());
        Self {
            backend: Arc::new(backend),
            name,
            lowering_options: MipLoweringOptions::default(),
        }
    }

    /// 设置严格降维预算 / Set strict lowering budgets.
    pub fn with_lowering_options(mut self, options: MipLoweringOptions) -> Self {
        self.lowering_options = options;
        self
    }

    /// 获取底层线性 solver / Return the wrapped linear solver.
    pub fn backend(&self) -> &S {
        &self.backend
    }

    fn validate_options(&self, options: &ConstraintProgrammingSolveOptions<'_>) -> Result<()> {
        options.validate()
    }

    fn solve_snapshot(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        self.validate_options(options)?;
        snapshot.validate_identity()?;
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            return cancelled_cp_report(&self.name, handle);
        }
        if let Some(termination_reason) = immediate_limit_reason(options) {
            let report = immediate_limit_report(&self.name, snapshot, options, termination_reason)?;
            if let Some(handle) = options.cancellation_handle {
                handle.mark_completed();
            }
            return Ok(report);
        }
        // 完整 hint 必须先通过原 CP snapshot 校验，再投影到 MIP。
        // A complete hint must pass source CP validation before it is projected to MIP.
        super::validate_complete_hint(snapshot, options.solution_hint, &[])?;
        let mut lowered =
            lower_constraint_programming_with_options(snapshot, self.lowering_options)
                .map_err(lowering_error)?;
        let hinted_source_variables =
            apply_solution_hint(snapshot, &mut lowered, options.solution_hint)?;
        let backend_options = SolveOptions::new()
            .with_time_limit(options.time_limit)
            .with_node_limit(options.node_limit)
            .with_solution_limit(options.solution_limit)
            .with_cancellation_handle(options.cancellation_handle)
            .with_progress_reporter(options.progress_reporter);
        let backend_report = match self
            .backend
            .solve_linear_report_with_options(&lowered.model, &backend_options)
        {
            Ok(report) => report,
            Err(error) => {
                if let Some(handle) = options.cancellation_handle {
                    handle.mark_completed();
                }
                return Err(error);
            }
        };
        if let Some(handle) = options.cancellation_handle {
            handle.mark_completed();
        }
        let mut report = map_mip_report(self, snapshot, &lowered, backend_report)?;
        if hinted_source_variables > 0 {
            report.diagnostics.extensions.insert(
                "cp.mip.hint.sourceVariables".to_owned(),
                hinted_source_variables.to_string(),
            );
            report.warnings.push(SolveWarning::new(
                "CPMipPartialWarmStart",
                "validated CP hint was projected to source variables; generated auxiliaries remain unspecified",
            ));
        }
        Ok(report)
    }
}

fn immediate_limit_reason(
    options: &ConstraintProgrammingSolveOptions<'_>,
) -> Option<TerminationReason> {
    if options.time_limit.is_some_and(|limit| limit.is_zero()) {
        Some(TerminationReason::TimeLimit)
    } else if options.node_limit == Some(0) {
        Some(TerminationReason::NodeLimit)
    } else if options.solution_limit == Some(0) {
        Some(TerminationReason::SolutionLimit)
    } else {
        None
    }
}

fn immediate_limit_report(
    solver_name: &str,
    snapshot: &ConstraintProgrammingSnapshot,
    options: &ConstraintProgrammingSolveOptions<'_>,
    termination_reason: TerminationReason,
) -> Result<SolveReport<i64>> {
    if let Some(handle) = options.cancellation_handle
        && handle.is_cancelled()
    {
        return cancelled_cp_report(solver_name, handle);
    }

    super::emit_progress(options.progress_reporter, SolveStage::Solving, false, None)?;
    if let Some(handle) = options.cancellation_handle
        && handle.is_cancelled()
    {
        return cancelled_cp_report(solver_name, handle);
    }

    let complete_hint = super::validate_complete_hint(snapshot, options.solution_hint, &[])?;

    let best_bound = super::objective_bound(snapshot)?;
    let best_bound_value = best_bound.and_then(super::f64_snapshot);
    let solution = complete_hint
        .map(|assignment| {
            let objective = snapshot.objective_value(&assignment)?;
            let objective_value = objective.and_then(super::f64_snapshot);
            let values = snapshot
                .variables
                .iter()
                .map(|variable| assignment[&variable.variable.stable_id])
                .collect();
            Ok::<SolveSolution<i64>, CoreError>(SolveSolution {
                value: None,
                values,
                stable_values: assignment,
                objective,
                objective_value,
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
        })
        .transpose()?;
    let (absolute_gap, relative_gap) = solution
        .as_ref()
        .and_then(|solution| solution.objective_value)
        .zip(best_bound_value)
        .map(|(objective, bound)| {
            let absolute = (objective - bound).abs();
            (Some(absolute), Some(absolute / objective.abs().max(1.0)))
        })
        .unwrap_or((None, None));
    let problem_status = if solution.is_some() {
        ProblemStatus::Feasible
    } else {
        ProblemStatus::Unknown
    };
    let mut diagnostics = SolveDiagnostics::default();
    diagnostics.extensions.insert(
        "cp.mip.immediateLimit".to_owned(),
        format!("{termination_reason:?}"),
    );
    let mut builder = SolveReport::builder(problem_status, termination_reason)
        .diagnostics(diagnostics)
        .provenance(SolverProvenance {
            solver_id: solver_name.to_owned(),
            backend_name: solver_name.to_owned(),
            deterministic: Some(true),
            ..SolverProvenance::default()
        })
        .fingerprints(SolveFingerprints {
            model: Some(snapshot.fingerprint.clone()),
            ..SolveFingerprints::default()
        })
        .statistics(SolveStatistics {
            nodes: Some(0),
            best_bound,
            best_bound_value,
            absolute_gap,
            relative_gap,
            solution_count: Some(usize::from(solution.is_some())),
            ..SolveStatistics::default()
        });
    if let Some(solution) = solution {
        builder = builder.solution(solution);
    }
    let report = builder.build()?;
    super::emit_progress(
        options.progress_reporter,
        SolveStage::Completed,
        true,
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective),
    )?;
    Ok(report)
}

impl<S: LinearSolver> SolverInfo for MipBackedConstraintProgrammingSolver<S> {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        let mut capabilities = self.backend.capabilities();
        if !capabilities.contains(&SolverCapability::ConstraintProgramming) {
            capabilities.push(SolverCapability::ConstraintProgramming);
        }
        capabilities
    }

    fn descriptor(&self) -> SolverDescriptor {
        let mut descriptor = self.backend.descriptor();
        descriptor.solver_id = self.name.clone();
        descriptor.display_name = format!("MIP-backed CP ({})", self.backend.name());
        descriptor.capabilities.levels.insert(
            "constraint_programming".to_owned(),
            crate::solver::CapabilitySupport::Conditional,
        );
        descriptor
            .warnings
            .push("only the exact finite lowering subset is supported".to_owned());
        descriptor
    }
}

impl<S: LinearSolver + 'static> ConstraintProgrammingSolver
    for MipBackedConstraintProgrammingSolver<S>
{
    fn analyze_support(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> ConstraintProgrammingSupportReport {
        let mut report = ConstraintProgrammingSupportReport {
            constraints: BTreeMap::new(),
            satisfaction: false,
            integer_objective: false,
            sparse_domain: false,
            one_shot: false,
            rebuild_session: false,
            incremental_session: false,
            assumptions: false,
            cancellation: true,
            solution_hint: false,
            verified_conflict_seed: false,
            irreducible_conflict: false,
            progress: true,
            deterministic: false,
            solution_pool: false,
            notes: Vec::new(),
        };
        let mut all_constraints_supported = true;
        for constraint in &snapshot.constraints {
            let mut constraint_snapshot = snapshot.clone();
            constraint_snapshot.constraints = vec![constraint.clone()];
            constraint_snapshot.fingerprint = constraint_snapshot.compute_fingerprint();
            match lower_constraint_programming_with_options(
                &constraint_snapshot,
                self.lowering_options,
            ) {
                Ok(lowered) => {
                    report.constraints.insert(
                        constraint.id.clone(),
                        ConstraintProgrammingSupport::ExactLowering,
                    );
                    report.notes.push(format!(
                        "constraint {} has an exact finite lowering ({} variables, {} rows)",
                        constraint.id,
                        lowered.model.num_variables(),
                        lowered.model.num_constraints()
                    ));
                }
                Err(error) => {
                    all_constraints_supported = false;
                    report.constraints.insert(
                        constraint.id.clone(),
                        ConstraintProgrammingSupport::Unsupported,
                    );
                    report.notes.push(format!(
                        "constraint {} is unavailable to exact finite lowering: {error}",
                        constraint.id
                    ));
                }
            }
        }
        if all_constraints_supported {
            match lower_constraint_programming_with_options(snapshot, self.lowering_options) {
                Ok(lowered) => {
                    report.satisfaction = true;
                    report.integer_objective = true;
                    report.sparse_domain = true;
                    report.one_shot = true;
                    report.rebuild_session = true;
                    report.assumptions = true;
                    report.solution_hint = true;
                    report.verified_conflict_seed = true;
                    report.irreducible_conflict = true;
                    report.notes.push(format!(
                        "full exact finite MIP lowering generated {} variables and {} rows",
                        lowered.model.num_variables(),
                        lowered.model.num_constraints()
                    ));
                }
                Err(error) => {
                    report.notes.push(format!(
                        "full CP snapshot lowering is unavailable even though individual constraints passed: {error}"
                    ));
                }
            }
        } else {
            report.notes.push(
                "overall CP solving capabilities remain disabled until every constraint lowers exactly"
                    .to_owned(),
            );
        }
        report
    }

    fn solve_constraint_programming(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        self.solve_snapshot(snapshot, options)
    }

    fn create_session(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> Result<Box<dyn ConstraintProgrammingSession>> {
        snapshot.validate_identity()?;
        Ok(Box::new(MipBackedConstraintProgrammingSession {
            solver: self.clone(),
            snapshot: snapshot.clone(),
        }))
    }
}

fn apply_solution_hint(
    snapshot: &ConstraintProgrammingSnapshot,
    lowering: &mut MipLoweringResult,
    hint: Option<&BTreeMap<StableVariableId, i64>>,
) -> Result<usize> {
    let Some(hint) = hint else {
        return Ok(0);
    };
    let mut applied = 0usize;
    for (stable_id, value) in hint {
        let variable = snapshot.variable(stable_id).ok_or_else(|| {
            CoreError::Solver(SolverError::ContractViolation(format!(
                "CP solution hint references unknown variable {}",
                stable_id.0
            )))
        })?;
        if !variable.domain.contains(*value) {
            return Err(CoreError::Solver(SolverError::ContractViolation(format!(
                "CP solution hint value {}={} is outside the source domain",
                stable_id.0, value
            ))));
        }
        let index = lowering
            .source_variables
            .get(stable_id)
            .copied()
            .ok_or_else(|| {
                CoreError::Solver(SolverError::ContractViolation(format!(
                    "CP solution hint variable {} is missing from the lowering map",
                    stable_id.0
                )))
            })?;
        let value = exact_i128(i128::from(*value), format!("hint[{}]", stable_id.0))
            .map_err(lowering_error)?;
        lowering.model.basic.variables[index].set_result(value);
        applied = applied.saturating_add(1);
    }
    Ok(applied)
}

struct MipBackedConstraintProgrammingSession<S> {
    solver: MipBackedConstraintProgrammingSolver<S>,
    snapshot: ConstraintProgrammingSnapshot,
}

impl<S: LinearSolver> ConstraintProgrammingSession for MipBackedConstraintProgrammingSession<S> {
    fn solve_with_assumptions(
        &mut self,
        assumptions: &[ConstraintProgrammingAssumption],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        let snapshot = snapshot_with_assumptions(&self.snapshot, assumptions)?;
        self.solver.solve_snapshot(&snapshot, options)
    }
}

pub(crate) fn snapshot_with_assumptions(
    snapshot: &ConstraintProgrammingSnapshot,
    assumptions: &[ConstraintProgrammingAssumption],
) -> Result<ConstraintProgrammingSnapshot> {
    let mut result = snapshot.clone();
    let existing = result
        .constraints
        .iter()
        .map(|constraint| constraint.id.clone())
        .collect::<BTreeSet<_>>();
    validate_conflict_assumption_ids(assumptions)?;
    for assumption in assumptions {
        let id = StableConstraintId(format!("__ospf_cp_assumption__{}", assumption.stable_id()));
        if existing.contains(&id) {
            return Err(CoreError::Solver(SolverError::ContractViolation(format!(
                "assumption stable ID collides with source constraint: {id}"
            ))));
        }
        let constraint = match assumption {
            ConstraintProgrammingAssumption::Literal(literal) => {
                let variable = validate_assumption_variable(snapshot, &literal.variable)?;
                if !snapshot
                    .variable(&literal.variable.stable_id)
                    .is_some_and(|entry| entry.domain.is_boolean())
                {
                    return Err(CoreError::Solver(SolverError::InvalidInput(
                        "Boolean assumption references a non-Boolean variable".to_owned(),
                    )));
                }
                ConstraintProgrammingConstraint::Boolean {
                    literal: BooleanLiteral {
                        variable,
                        negated: literal.negated,
                    },
                }
            }
            ConstraintProgrammingAssumption::Equal(variable, value) => {
                validate_assumption_value(snapshot, variable, *value)?;
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(validate_assumption_variable(snapshot, variable)?),
                    IntegerRelation::Equal,
                    *value,
                )
            }
            ConstraintProgrammingAssumption::LowerBound(variable, value) => {
                validate_assumption_variable(snapshot, variable)?;
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(validate_assumption_variable(snapshot, variable)?),
                    IntegerRelation::GreaterOrEqual,
                    *value,
                )
            }
            ConstraintProgrammingAssumption::UpperBound(variable, value) => {
                validate_assumption_variable(snapshot, variable)?;
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(validate_assumption_variable(snapshot, variable)?),
                    IntegerRelation::LessOrEqual,
                    *value,
                )
            }
        };
        result
            .constraints
            .push(crate::model::constraint_programming::ConstraintSnapshot {
                id,
                name: format!("assumption-{}", assumption.stable_id()),
                group: Some("assumption".to_owned()),
                origin: Some("constraint-programming-session".to_owned()),
                constraint,
            });
    }
    result
        .constraints
        .sort_by(|left, right| left.id.cmp(&right.id));
    result.fingerprint = result.compute_fingerprint();
    result.validate_identity()?;
    Ok(result)
}

fn validate_assumption_variable(
    snapshot: &ConstraintProgrammingSnapshot,
    variable: &IntegerVariable,
) -> Result<IntegerVariable> {
    let entry = snapshot.variable(&variable.stable_id).ok_or_else(|| {
        CoreError::Solver(SolverError::InvalidInput(format!(
            "assumption references unknown variable {}",
            variable.stable_id
        )))
    })?;
    Ok(entry.variable.clone())
}

fn validate_assumption_value(
    snapshot: &ConstraintProgrammingSnapshot,
    variable: &IntegerVariable,
    value: i64,
) -> Result<()> {
    validate_assumption_variable(snapshot, variable)?;
    if !snapshot
        .variable(&variable.stable_id)
        .is_some_and(|entry| entry.domain.contains(value))
    {
        return Err(CoreError::Solver(SolverError::InvalidInput(format!(
            "assumption value {}={} is outside the source domain",
            variable.stable_id, value
        ))));
    }
    Ok(())
}

fn lowering_error(error: MipLoweringError) -> CoreError {
    match error {
        MipLoweringError::Unsupported { .. } => {
            CoreError::Solver(SolverError::UnsupportedValueType(error.to_string()))
        }
        MipLoweringError::Numeric { .. } | MipLoweringError::BudgetExceeded { .. } => {
            CoreError::Solver(SolverError::NumericalError(error.to_string()))
        }
        MipLoweringError::Invalid { .. } => {
            CoreError::Solver(SolverError::ContractViolation(error.to_string()))
        }
    }
}

fn cancelled_cp_report(
    name: &str,
    handle: &crate::solver::SolveHandle,
) -> Result<SolveReport<i64>> {
    let cancellation = handle.cancellation().ok_or_else(|| {
        CoreError::Solver(SolverError::ContractViolation(
            "cancelled CP report has no cancellation record".to_owned(),
        ))
    })?;
    let mut diagnostics = SolveDiagnostics::default();
    diagnostics.extensions.insert(
        "cancellation.origin".to_owned(),
        cancellation.origin.to_string(),
    );
    diagnostics.extensions.insert(
        "cancellation.requestedAtEpochMs".to_owned(),
        cancellation.requested_at_epoch_ms.to_string(),
    );
    SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Cancelled)
        .diagnostics(diagnostics)
        .provenance(SolverProvenance {
            solver_id: name.to_owned(),
            backend_name: name.to_owned(),
            ..SolverProvenance::default()
        })
        .build()
}

fn exact_report_i64(value: f64, field: impl Into<String>) -> Result<i64> {
    exact_f64_integer(value, &field.into()).map_err(lowering_error)
}

fn approximately_equal(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance
}

fn rounded_report_i64(
    value: f64,
    category: ObjectiveCategory,
    objective_constant: i64,
    field: impl Into<String>,
) -> Result<i64> {
    if !value.is_finite() {
        return Err(lowering_error(MipLoweringError::Numeric {
            field: field.into(),
            value: value.to_string(),
        }));
    }
    let integer_bound = if category.is_minimum() {
        value.ceil()
    } else {
        value.floor()
    };
    let integer_bound =
        exact_f64_integer(integer_bound, "MIP best bound").map_err(lowering_error)?;
    i64::try_from(i128::from(integer_bound) + i128::from(objective_constant)).map_err(|_| {
        lowering_error(MipLoweringError::Numeric {
            field: "CP best bound with objective constant".to_owned(),
            value: format!("{integer_bound} + {objective_constant}"),
        })
    })
}

fn linear_objective_value(model: &LinearTriadModel, values: &[f64]) -> Result<f64> {
    if values.len() != model.num_variables() || model.c.len() != model.num_variables() {
        return Err(CoreError::Solver(SolverError::ContractViolation(
            "MIP solution dimension does not match the generated objective".to_owned(),
        )));
    }
    let objective = model
        .c
        .iter()
        .zip(values)
        .fold(0.0_f64, |sum, (coefficient, value)| {
            sum + coefficient * value
        });
    if !objective.is_finite() {
        return Err(CoreError::Solver(SolverError::NumericalError(
            "MIP objective evaluation is non-finite".to_owned(),
        )));
    }
    Ok(objective)
}

fn validate_mip_objective_and_bound(
    model: &LinearTriadModel,
    report: &SolveReport<f64>,
) -> Result<()> {
    let best_bound = report_best_bound(report)?;
    let Some(solution) = report.solution.as_ref() else {
        return Ok(());
    };
    let expected_objective = linear_objective_value(model, &solution.values)?;
    let reported_objective = match (solution.objective, solution.objective_value) {
        (Some(objective), Some(objective_value)) => {
            if !objective.is_finite()
                || !objective_value.is_finite()
                || !approximately_equal(objective, objective_value, 1e-9)
            {
                return Err(CoreError::Solver(SolverError::ContractViolation(
                    "MIP incumbent objective fields are inconsistent".to_owned(),
                )));
            }
            objective_value
        }
        (Some(objective), None) => objective,
        (None, Some(objective_value)) => objective_value,
        (None, None) => {
            return Err(CoreError::Solver(SolverError::ContractViolation(
                "MIP incumbent is missing its objective value".to_owned(),
            )));
        }
    };
    if !approximately_equal(expected_objective, reported_objective, 1e-9) {
        return Err(CoreError::Solver(SolverError::ContractViolation(
            "MIP incumbent objective does not match the generated linear objective".to_owned(),
        )));
    }
    if let Some(bound) = best_bound {
        let valid_direction = if model.objective_category.is_minimum() {
            bound <= reported_objective + 1e-9
        } else {
            bound + 1e-9 >= reported_objective
        };
        if !valid_direction {
            return Err(CoreError::Solver(SolverError::ContractViolation(
                "MIP best bound has the wrong direction for the generated objective".to_owned(),
            )));
        }
    }
    Ok(())
}

fn report_best_bound(report: &SolveReport<f64>) -> Result<Option<f64>> {
    match (
        report.statistics.best_bound,
        report.statistics.best_bound_value,
    ) {
        (Some(typed), Some(value)) => {
            if !typed.is_finite() || !value.is_finite() {
                return Err(CoreError::Solver(SolverError::NumericalError(
                    "MIP best bound is non-finite".to_owned(),
                )));
            }
            if !approximately_equal(typed, value, 1e-9) {
                return Err(CoreError::Solver(SolverError::ContractViolation(
                    "MIP typed and floating-point best bounds are inconsistent".to_owned(),
                )));
            }
            Ok(Some(value))
        }
        (Some(value), None) | (None, Some(value)) => {
            if !value.is_finite() {
                return Err(CoreError::Solver(SolverError::NumericalError(
                    "MIP best bound is non-finite".to_owned(),
                )));
            }
            Ok(Some(value))
        }
        (None, None) => Ok(None),
    }
}

fn map_mip_report<S: LinearSolver>(
    _solver: &MipBackedConstraintProgrammingSolver<S>,
    snapshot: &ConstraintProgrammingSnapshot,
    lowering: &MipLoweringResult,
    report: SolveReport<f64>,
) -> Result<SolveReport<i64>> {
    report.validate()?;
    validate_mip_objective_and_bound(&lowering.model, &report)?;
    let backend_solution = report.solution.as_ref();
    let solution = backend_solution
        .map(|solution| {
            let assignment = lowering
                .project_solution(snapshot, &solution.values)
                .map_err(lowering_error)?;
            let source_values = snapshot
                .variables
                .iter()
                .map(|variable| assignment[&variable.variable.stable_id])
                .collect::<Vec<_>>();
            let objective = snapshot.objective_value(&assignment).map_err(|error| {
                CoreError::Solver(SolverError::ContractViolation(format!(
                    "CP objective verification failed: {error}"
                )))
            })?;
            let pool = solution
                .pool
                .iter()
                .map(|values| {
                    let assignment = lowering
                        .project_solution(snapshot, values)
                        .map_err(lowering_error)?;
                    Ok::<Vec<i64>, CoreError>(
                        snapshot
                            .variables
                            .iter()
                            .map(|variable| assignment[&variable.variable.stable_id])
                            .collect(),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let objective_value = objective.and_then(super::f64_snapshot);
            Ok::<SolveSolution<i64>, CoreError>(SolveSolution {
                value: None,
                values: source_values,
                stable_values: assignment,
                objective,
                objective_value,
                dual_solution: None,
                quadratic_dual_solution: None,
                pool,
            })
        })
        .transpose()?;

    let source_objective = solution
        .as_ref()
        .and_then(|solution| solution.objective_value);
    let raw_best_bound = report_best_bound(&report)?;
    let best_bound_value = raw_best_bound.map(|value| value + lowering.objective_constant as f64);
    if best_bound_value.is_some_and(|value| !value.is_finite()) {
        return Err(CoreError::Solver(SolverError::NumericalError(
            "CP best bound with objective constant is non-finite".to_owned(),
        )));
    }
    let absolute_gap = source_objective
        .zip(best_bound_value)
        .map(|(objective, bound)| (objective - bound).abs());
    let relative_gap =
        absolute_gap.map(|gap| gap / source_objective.unwrap_or_default().abs().max(1.0));
    let best_bound = raw_best_bound
        .map(|value| {
            rounded_report_i64(
                value,
                lowering.model.objective_category,
                lowering.objective_constant,
                "best_bound",
            )
        })
        .transpose()?;
    let mut diagnostics = SolveDiagnostics::default();
    diagnostics.infeasibility_evidence = report.diagnostics.infeasibility_evidence.clone();
    diagnostics.warnings = report.diagnostics.warnings.clone();
    diagnostics.issues = report.diagnostics.issues.clone();
    diagnostics.extensions = report.diagnostics.extensions.clone();
    diagnostics.extensions.insert(
        "cp.mip.sourceFingerprint".to_owned(),
        snapshot.fingerprint.value.clone(),
    );
    diagnostics.extensions.insert(
        "cp.mip.lowering.auxiliaryVariables".to_owned(),
        lowering.statistics.auxiliary_variables.to_string(),
    );
    diagnostics.extensions.insert(
        "cp.mip.lowering.generatedConstraints".to_owned(),
        lowering.statistics.generated_constraints.to_string(),
    );
    let mut warnings = report.warnings.clone();
    warnings.push(SolveWarning::new(
        "CPMipExactLowering",
        "the incumbent was projected and independently verified against the source CP snapshot",
    ));
    let mut provenance = report.provenance.clone();
    provenance
        .effective_configuration
        .insert("cp.lowering".to_owned(), "exact-finite-linear".to_owned());
    let statistics = SolveStatistics {
        solve_time: report.statistics.solve_time,
        iterations: report.statistics.iterations,
        nodes: report.statistics.nodes,
        best_bound,
        best_bound_value,
        absolute_gap,
        relative_gap,
        solution_count: report.statistics.solution_count,
        extensions: report.statistics.extensions.clone(),
    };
    let mut builder = SolveReport::builder(report.problem_status, report.termination_reason)
        .statistics(statistics)
        .diagnostics(diagnostics)
        .provenance(provenance)
        .fingerprints(SolveFingerprints {
            model: Some(snapshot.fingerprint.clone()),
            configuration: report.fingerprints.configuration.clone(),
            solver: report.fingerprints.solver.clone(),
        })
        .trace(report.trace.clone());
    if let Some(solution) = solution {
        builder = builder.solution(solution);
    }
    if let Some(proof) = report.proof {
        let evidence = proof
            .evidence
            .map(|values| {
                values
                    .into_iter()
                    .map(|value| exact_report_i64(value, "proof.evidence"))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?;
        builder = builder.proof(SolveProof {
            kind: proof.kind,
            status: proof.status,
            reliability: proof.reliability,
            completeness: proof.completeness,
            reference: proof.reference,
            evidence,
        });
    }
    for warning in warnings {
        builder = builder.warning(warning);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        AutomatonTransition, ConstraintDefinition, ConstraintProgrammingConstraint,
        ConstraintProgrammingModel, IntegerObjective, IntegerTerm, IntervalDuration,
        IntervalVariable, ReificationDirection, ReservoirEvent,
    };
    use crate::solver::{
        InfeasibilityEvidence, InfeasibilityEvidenceMember, InfeasibilityEvidenceSource,
        ProofCompleteness, ProofReliability, SolutionPresence, SolverOutput, StableVariableId,
    };

    #[derive(Debug, Clone, Copy)]
    struct ExhaustiveLinearSolver;

    impl SolverInfo for ExhaustiveLinearSolver {
        fn name(&self) -> &str {
            "test-exhaustive-linear"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    impl LinearSolver for ExhaustiveLinearSolver {
        fn solve_linear(&self, model: &LinearTriadModel) -> crate::error::Result<SolverOutput> {
            let solution =
                find_feasible_vector(model).ok_or(CoreError::Solver(SolverError::NoSolution))?;
            let objective = model
                .c
                .iter()
                .zip(solution.iter())
                .map(|(coefficient, value)| coefficient * value)
                .sum();
            Ok(SolverOutput::optimal(objective, solution))
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct ContractLinearSolver {
        objective_delta: f64,
        best_bound: Option<f64>,
        typed_best_bound: Option<f64>,
    }

    #[derive(Debug, Clone, Copy)]
    struct NeverCalledLinearSolver;

    #[derive(Debug, Clone, Copy)]
    struct FarkasLinearSolver;

    impl SolverInfo for FarkasLinearSolver {
        fn name(&self) -> &str {
            "test-farkas-linear"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    impl LinearSolver for FarkasLinearSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> crate::error::Result<SolverOutput> {
            Err(CoreError::Solver(SolverError::NoSolution))
        }

        fn solve_linear_report(
            &self,
            _model: &LinearTriadModel,
        ) -> crate::error::Result<SolveReport<f64>> {
            let diagnostics = SolveDiagnostics {
                infeasibility_evidence: Some(InfeasibilityEvidence {
                    source: InfeasibilityEvidenceSource::Farkas,
                    reliability: ProofReliability::Exact,
                    completeness: ProofCompleteness::Complete,
                    constraint_ids: std::collections::BTreeSet::from(["farkas/row".to_owned()]),
                    members: std::collections::BTreeSet::from([
                        InfeasibilityEvidenceMember::Constraint("farkas/row".to_owned()),
                    ]),
                    minimality: crate::solver::InfeasibilityMinimality::NotChecked,
                    computation_time: std::time::Duration::ZERO,
                    unavailable_reason: None,
                }),
                ..Default::default()
            };
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .diagnostics(diagnostics)
                .proof(SolveProof::infeasibility())
                .build()
        }
    }

    impl SolverInfo for NeverCalledLinearSolver {
        fn name(&self) -> &str {
            "test-never-called-linear"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    impl LinearSolver for NeverCalledLinearSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> crate::error::Result<SolverOutput> {
            Err(CoreError::Solver(SolverError::ContractViolation(
                "the CP backend must not be called for an immediate limit".to_owned(),
            )))
        }

        fn solve_linear_report_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: &SolveOptions<'_>,
        ) -> crate::error::Result<SolveReport<f64>> {
            Err(CoreError::Solver(SolverError::ContractViolation(
                "the CP backend must not be called for an immediate limit".to_owned(),
            )))
        }
    }

    impl SolverInfo for ContractLinearSolver {
        fn name(&self) -> &str {
            "test-contract-linear"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    impl LinearSolver for ContractLinearSolver {
        fn solve_linear(&self, model: &LinearTriadModel) -> crate::error::Result<SolverOutput> {
            let solution =
                find_feasible_vector(model).ok_or(CoreError::Solver(SolverError::NoSolution))?;
            let objective = model
                .c
                .iter()
                .zip(solution.iter())
                .map(|(coefficient, value)| coefficient * value)
                .sum();
            Ok(SolverOutput::optimal(objective, solution))
        }

        fn solve_linear_report(
            &self,
            model: &LinearTriadModel,
        ) -> crate::error::Result<SolveReport<f64>> {
            let solution =
                find_feasible_vector(model).ok_or(CoreError::Solver(SolverError::NoSolution))?;
            let objective = model
                .c
                .iter()
                .zip(solution.iter())
                .map(|(coefficient, value)| coefficient * value)
                .sum::<f64>();
            let reported_objective = objective + self.objective_delta;
            let solution = SolveSolution {
                objective: Some(reported_objective),
                objective_value: Some(reported_objective),
                ..SolveSolution::vector(solution)
            };
            SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(solution)
                .statistics(SolveStatistics {
                    best_bound: self.typed_best_bound,
                    best_bound_value: self.best_bound,
                    ..SolveStatistics::default()
                })
                .build()
        }
    }

    fn variable(
        model: &mut ConstraintProgrammingModel,
        id: &str,
        domain: IntegerDomain,
    ) -> crate::model::constraint_programming::IntegerVariable {
        let variable = crate::model::constraint_programming::IntegerVariable::new(id);
        model
            .register_variable(variable.clone(), domain)
            .expect("variable");
        variable
    }

    fn expression(
        variable: &crate::model::constraint_programming::IntegerVariable,
    ) -> IntegerExpression {
        IntegerExpression::variable(variable.clone())
    }

    fn snapshot_with(
        constraint: ConstraintProgrammingConstraint,
        variables: impl IntoIterator<
            Item = (
                crate::model::constraint_programming::IntegerVariable,
                IntegerDomain,
            ),
        >,
    ) -> ConstraintProgrammingSnapshot {
        snapshot_with_id("c", constraint, variables)
    }

    fn snapshot_with_id(
        id: &str,
        constraint: ConstraintProgrammingConstraint,
        variables: impl IntoIterator<
            Item = (
                crate::model::constraint_programming::IntegerVariable,
                IntegerDomain,
            ),
        >,
    ) -> ConstraintProgrammingSnapshot {
        let mut model = ConstraintProgrammingModel::new("lowering-test");
        for (variable, domain) in variables {
            model.register_variable(variable, domain).expect("variable");
        }
        model
            .add_constraint(ConstraintDefinition::new(id, constraint))
            .expect("constraint");
        model.freeze().expect("snapshot")
    }

    fn assert_lowering_matches_source(snapshot: &ConstraintProgrammingSnapshot) {
        let lowering = lower_constraint_programming(snapshot).expect("exact lowering");
        let mut assignment = BTreeMap::new();
        let mut checked = 0usize;
        enumerate_source_assignments(snapshot, 0, &mut assignment, &mut |assignment| {
            let source_feasible = snapshot.validate_assignment(assignment).is_ok();
            let mip_feasible = has_mip_extension(&lowering, assignment);
            assert_eq!(
                mip_feasible, source_feasible,
                "lowering changed the feasible set for assignment {assignment:?}"
            );
            checked = checked.saturating_add(1);
        });
        assert!(
            checked > 0,
            "the differential oracle must inspect assignments"
        );
    }

    fn enumerate_source_assignments<F>(
        snapshot: &ConstraintProgrammingSnapshot,
        index: usize,
        assignment: &mut BTreeMap<StableVariableId, i64>,
        callback: &mut F,
    ) where
        F: FnMut(&BTreeMap<StableVariableId, i64>),
    {
        if index == snapshot.variables.len() {
            callback(assignment);
            return;
        }
        let variable = &snapshot.variables[index];
        let values = variable
            .domain
            .enumerate(16)
            .expect("small differential-oracle domain");
        for value in values {
            assignment.insert(variable.variable.stable_id.clone(), value);
            enumerate_source_assignments(snapshot, index + 1, assignment, callback);
        }
        assignment.remove(&variable.variable.stable_id);
    }

    fn has_mip_extension(
        lowering: &MipLoweringResult,
        assignment: &BTreeMap<StableVariableId, i64>,
    ) -> bool {
        let mut values = vec![0.0; lowering.model.num_variables()];
        for (stable_id, value) in assignment {
            let Some(index) = lowering.source_variables.get(stable_id) else {
                return false;
            };
            values[*index] = *value as f64;
        }
        has_mip_extension_at(lowering, &mut values, 0)
    }

    fn has_mip_extension_at(
        lowering: &MipLoweringResult,
        values: &mut [f64],
        index: usize,
    ) -> bool {
        if index == values.len() {
            if lowering
                .model
                .basic
                .lb
                .iter()
                .zip(&lowering.model.basic.ub)
                .zip(values.iter())
                .any(|((lower, upper), value)| *value < *lower - 1e-9 || *value > *upper + 1e-9)
            {
                return false;
            }
            return lowering
                .model
                .basic
                .A
                .rows
                .iter()
                .enumerate()
                .all(|(row_index, row)| {
                    let lhs = row
                        .entries
                        .iter()
                        .map(|(variable, coefficient)| values[*variable] * coefficient)
                        .sum::<f64>();
                    lhs <= lowering.model.basic.b[row_index] + 1e-9
                });
        }
        if lowering
            .source_variables
            .values()
            .any(|source_index| *source_index == index)
        {
            return has_mip_extension_at(lowering, values, index + 1);
        }
        for candidate in [0.0, 1.0] {
            values[index] = candidate;
            if has_mip_extension_at(lowering, values, index + 1) {
                return true;
            }
        }
        false
    }

    fn objective_snapshot(
        category: ObjectiveCategory,
        constant: i64,
        fixed_value: Option<i64>,
    ) -> ConstraintProgrammingSnapshot {
        let mut model = ConstraintProgrammingModel::new("objective-contract");
        let x = variable(&mut model, "x", IntegerDomain::boolean());
        if let Some(fixed_value) = fixed_value {
            model
                .add_constraint(ConstraintDefinition::new(
                    "fix-x",
                    ConstraintProgrammingConstraint::integer(
                        expression(&x),
                        IntegerRelation::Equal,
                        fixed_value,
                    ),
                ))
                .expect("fixed objective variable");
        }
        model.set_objective(IntegerObjective {
            category,
            expression: IntegerExpression::linear(
                constant,
                [IntegerTerm {
                    variable: x,
                    coefficient: 1,
                }],
            )
            .expect("objective expression"),
        });
        model.freeze().expect("objective snapshot")
    }

    #[test]
    fn lower_integer_and_boolean_logic_with_stable_origins() {
        let mut model = ConstraintProgrammingModel::new("logic");
        let x = variable(&mut model, "x", IntegerDomain::boolean());
        let y = variable(&mut model, "y", IntegerDomain::boolean());
        model
            .add_constraint(ConstraintDefinition::new(
                "c",
                ConstraintProgrammingConstraint::Implication {
                    enforcement: BooleanLiteral::positive(x.clone()),
                    constraint: Box::new(ConstraintProgrammingConstraint::Reification {
                        literal: BooleanLiteral::positive(y.clone()),
                        constraint: Box::new(ConstraintProgrammingConstraint::integer(
                            expression(&x),
                            IntegerRelation::Equal,
                            1,
                        )),
                        direction: ReificationDirection::Equivalent,
                    }),
                },
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let result = lower_constraint_programming(&snapshot).expect("lowering");
        assert_eq!(result.source_variables.len(), 2);
        assert!(
            result
                .row_origins
                .iter()
                .any(|origin| matches!(origin, MipRowOrigin::Constraint(id) if id.0 == "c"))
        );
        assert!(result.statistics.auxiliary_variables > 0);
        assert_eq!(result.source_fingerprint, snapshot.fingerprint);
        assert_lowering_matches_source(&snapshot);
    }

    #[test]
    fn lowering_preserves_boolean_connectives_reification_directions_and_relations() {
        let x = crate::model::constraint_programming::IntegerVariable::new("x");
        let y = crate::model::constraint_programming::IntegerVariable::new("y");
        let binary_domains = [
            (x.clone(), IntegerDomain::boolean()),
            (y.clone(), IntegerDomain::boolean()),
        ];
        let snapshots = vec![
            snapshot_with(
                ConstraintProgrammingConstraint::And {
                    literals: vec![
                        BooleanLiteral::positive(x.clone()),
                        BooleanLiteral::negative(y.clone()),
                    ],
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::Or {
                    literals: vec![
                        BooleanLiteral::negative(x.clone()),
                        BooleanLiteral::positive(y.clone()),
                    ],
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::Xor {
                    literals: vec![
                        BooleanLiteral::positive(x.clone()),
                        BooleanLiteral::positive(y.clone()),
                    ],
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::Implication {
                    enforcement: BooleanLiteral::positive(x.clone()),
                    constraint: Box::new(ConstraintProgrammingConstraint::integer(
                        expression(&y),
                        IntegerRelation::GreaterOrEqual,
                        1,
                    )),
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::Reification {
                    literal: BooleanLiteral::positive(y.clone()),
                    constraint: Box::new(ConstraintProgrammingConstraint::integer(
                        expression(&x),
                        IntegerRelation::GreaterOrEqual,
                        1,
                    )),
                    direction: ReificationDirection::Equivalent,
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::Reification {
                    literal: BooleanLiteral::negative(y.clone()),
                    constraint: Box::new(ConstraintProgrammingConstraint::integer(
                        expression(&x),
                        IntegerRelation::GreaterOrEqual,
                        1,
                    )),
                    direction: ReificationDirection::ImpliedByLiteral,
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::Reification {
                    literal: BooleanLiteral::positive(y.clone()),
                    constraint: Box::new(ConstraintProgrammingConstraint::integer(
                        expression(&x),
                        IntegerRelation::GreaterOrEqual,
                        1,
                    )),
                    direction: ReificationDirection::ImpliesLiteral,
                },
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::integer(
                    expression(&x),
                    IntegerRelation::NotEqual,
                    1,
                ),
                binary_domains.clone(),
            ),
            snapshot_with(
                ConstraintProgrammingConstraint::integer(
                    expression(&x),
                    IntegerRelation::GreaterOrEqual,
                    1,
                ),
                binary_domains,
            ),
        ];
        for snapshot in snapshots {
            assert_lowering_matches_source(&snapshot);
        }
    }

    #[test]
    fn lowering_preserves_at_most_one_and_exactly_one_for_all_boolean_assignments() {
        let x = crate::model::constraint_programming::IntegerVariable::new("at-most-one/x");
        let y = crate::model::constraint_programming::IntegerVariable::new("at-most-one/y");
        let domains = [
            (x.clone(), IntegerDomain::boolean()),
            (y.clone(), IntegerDomain::boolean()),
        ];
        for constraint in [
            ConstraintProgrammingConstraint::at_most_one([
                BooleanLiteral::positive(x.clone()),
                BooleanLiteral::negative(y.clone()),
            ]),
            ConstraintProgrammingConstraint::exactly_one([
                BooleanLiteral::positive(x.clone()),
                BooleanLiteral::negative(y.clone()),
            ]),
        ] {
            let snapshot = snapshot_with(constraint.clone(), domains.clone());
            let _lowered = lower_constraint_programming(&snapshot).expect("global lowering");
            assert_lowering_matches_source(&snapshot);
        }
    }

    #[test]
    fn lower_sparse_domain_and_element_without_widening_the_domain() {
        let mut model = ConstraintProgrammingModel::new("element");
        let index = variable(
            &mut model,
            "index",
            IntegerDomain::values([0, 2]).expect("domain"),
        );
        let target = variable(
            &mut model,
            "target",
            IntegerDomain::values([10, 30]).expect("domain"),
        );
        model
            .add_constraint(ConstraintDefinition::new(
                "element",
                ConstraintProgrammingConstraint::Element {
                    index: expression(&index),
                    values: vec![10, 20, 30],
                    target: expression(&target),
                },
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let result = lower_constraint_programming(&snapshot).expect("lowering");
        assert_eq!(result.source_variables.len(), 2);
        assert!(result.statistics.auxiliary_variables >= 5);
        assert!(result.model.num_constraints() > snapshot.constraints.len());
        assert_lowering_matches_source(&snapshot);
    }

    #[test]
    fn lowering_preserves_all_different_and_assignment_tables() {
        let x = crate::model::constraint_programming::IntegerVariable::new("x");
        let y = crate::model::constraint_programming::IntegerVariable::new("y");
        let all_different = snapshot_with(
            ConstraintProgrammingConstraint::AllDifferent {
                expressions: vec![expression(&x), expression(&y)],
            },
            [
                (x.clone(), IntegerDomain::range(0, 1).expect("domain")),
                (y.clone(), IntegerDomain::range(0, 1).expect("domain")),
            ],
        );
        assert_lowering_matches_source(&all_different);

        let allowed = snapshot_with(
            ConstraintProgrammingConstraint::AllowedAssignments {
                expressions: vec![expression(&x), expression(&y)],
                tuples: vec![vec![0, 0], vec![1, 1]],
            },
            [
                (x.clone(), IntegerDomain::boolean()),
                (y.clone(), IntegerDomain::boolean()),
            ],
        );
        assert_lowering_matches_source(&allowed);

        let forbidden = snapshot_with(
            ConstraintProgrammingConstraint::ForbiddenAssignments {
                expressions: vec![expression(&x), expression(&y)],
                tuples: vec![vec![0, 0], vec![1, 1]],
            },
            [(x, IntegerDomain::boolean()), (y, IntegerDomain::boolean())],
        );
        assert_lowering_matches_source(&forbidden);
    }

    #[test]
    fn lowering_preserves_mandatory_and_optional_variable_duration_no_overlap() {
        let mut model = ConstraintProgrammingModel::new("no-overlap-oracle");
        let presence = variable(&mut model, "presence", IntegerDomain::boolean());
        let first_start = variable(
            &mut model,
            "first-start",
            IntegerDomain::range(0, 1).expect("domain"),
        );
        let duration = variable(
            &mut model,
            "duration",
            IntegerDomain::range(1, 2).expect("domain"),
        );
        let first_end = variable(
            &mut model,
            "first-end",
            IntegerDomain::range(1, 3).expect("domain"),
        );
        let second_start = variable(
            &mut model,
            "second-start",
            IntegerDomain::range(0, 1).expect("domain"),
        );
        let second_end = variable(
            &mut model,
            "second-end",
            IntegerDomain::range(1, 2).expect("domain"),
        );
        model
            .register_interval(
                IntervalVariable::new(
                    "first",
                    expression(&first_start),
                    IntervalDuration::Variable(duration),
                    expression(&first_end),
                    Some(BooleanLiteral::positive(presence)),
                )
                .expect("first interval"),
            )
            .expect("first interval registration");
        model
            .register_interval(
                IntervalVariable::new(
                    "second",
                    expression(&second_start),
                    IntervalDuration::Fixed(1),
                    expression(&second_end),
                    None,
                )
                .expect("second interval"),
            )
            .expect("second interval registration");
        model
            .add_constraint(ConstraintDefinition::new(
                "no-overlap",
                ConstraintProgrammingConstraint::NoOverlap {
                    intervals: vec!["first".into(), "second".into()],
                },
            ))
            .expect("NoOverlap");
        let snapshot = model.freeze().expect("snapshot");
        assert_lowering_matches_source(&snapshot);
    }

    #[test]
    fn table_empty_semantics_are_preserved() {
        let x = crate::model::constraint_programming::IntegerVariable::new("x");
        let allowed = snapshot_with(
            ConstraintProgrammingConstraint::AllowedAssignments {
                expressions: vec![expression(&x)],
                tuples: Vec::new(),
            },
            [(x.clone(), IntegerDomain::range(0, 1).expect("domain"))],
        );
        let forbidden = snapshot_with(
            ConstraintProgrammingConstraint::ForbiddenAssignments {
                expressions: vec![expression(&x)],
                tuples: Vec::new(),
            },
            [(x, IntegerDomain::range(0, 1).expect("domain"))],
        );
        let allowed_result = lower_constraint_programming(&allowed).expect("allowed lowering");
        let forbidden_result =
            lower_constraint_programming(&forbidden).expect("forbidden lowering");
        assert_eq!(allowed_result.model.num_constraints(), 1);
        assert_eq!(forbidden_result.model.num_constraints(), 0);
    }

    #[test]
    fn unsupported_global_constraints_are_structured() {
        let cases = vec![
            (
                "cumulative",
                ConstraintProgrammingConstraint::Cumulative {
                    tasks: Vec::new(),
                    capacity: 1,
                },
                "cumulative",
            ),
            (
                "circuit",
                ConstraintProgrammingConstraint::Circuit {
                    successors: vec![IntegerExpression::constant(0)],
                },
                "circuit",
            ),
            (
                "automaton",
                ConstraintProgrammingConstraint::Automaton {
                    expressions: vec![IntegerExpression::constant(0)],
                    initial_state: 0,
                    final_states: std::collections::BTreeSet::from([1]),
                    transitions: vec![AutomatonTransition {
                        from_state: 0,
                        value: 0,
                        to_state: 1,
                    }],
                },
                "automaton",
            ),
            (
                "reservoir",
                ConstraintProgrammingConstraint::Reservoir {
                    events: vec![ReservoirEvent {
                        time: IntegerExpression::constant(0),
                        level_change: IntegerExpression::constant(0),
                    }],
                    initial_level: 0,
                    minimum_level: 0,
                    maximum_level: 1,
                },
                "reservoir",
            ),
        ];
        for (id, constraint, feature) in cases {
            let snapshot = snapshot_with_id(id, constraint, std::iter::empty());
            let error = lower_constraint_programming(&snapshot).expect_err("unsupported");
            assert!(matches!(
                error,
                MipLoweringError::Unsupported { constraint: Some(actual_id), feature: actual_feature }
                    if actual_id == StableConstraintId::from(id) && actual_feature == feature
            ));
        }
    }

    #[test]
    fn support_analysis_marks_unsupported_global_constraints_explicitly() {
        let x = crate::model::constraint_programming::IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("support-analysis");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 0).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "cumulative",
                ConstraintProgrammingConstraint::Cumulative {
                    tasks: Vec::new(),
                    capacity: 0,
                },
            ))
            .expect("cumulative");
        model
            .add_constraint(ConstraintDefinition::new(
                "circuit",
                ConstraintProgrammingConstraint::Circuit {
                    successors: vec![expression(&x)],
                },
            ))
            .expect("circuit");
        model
            .add_constraint(ConstraintDefinition::new(
                "automaton",
                ConstraintProgrammingConstraint::Automaton {
                    expressions: vec![expression(&x)],
                    initial_state: 0,
                    final_states: std::collections::BTreeSet::from([1]),
                    transitions: vec![AutomatonTransition {
                        from_state: 0,
                        value: 0,
                        to_state: 1,
                    }],
                },
            ))
            .expect("automaton");
        model
            .add_constraint(ConstraintDefinition::new(
                "reservoir",
                ConstraintProgrammingConstraint::Reservoir {
                    events: vec![ReservoirEvent {
                        time: IntegerExpression::constant(0),
                        level_change: IntegerExpression::constant(0),
                    }],
                    initial_level: 0,
                    minimum_level: 0,
                    maximum_level: 0,
                },
            ))
            .expect("reservoir");
        let snapshot = model.freeze().expect("snapshot");
        let solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: None,
            typed_best_bound: None,
        });
        let report = solver.analyze_support(&snapshot);
        for id in ["cumulative", "circuit", "automaton", "reservoir"] {
            assert_eq!(
                report.constraints.get(&StableConstraintId::from(id)),
                Some(&ConstraintProgrammingSupport::Unsupported)
            );
        }
        assert!(!report.satisfaction);
        assert!(!report.integer_objective);
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("overall CP solving capabilities remain disabled"))
        );
    }

    #[test]
    fn exact_numeric_gate_rejects_large_integer_constants() {
        let x = crate::model::constraint_programming::IntegerVariable::new("x");
        let snapshot = snapshot_with(
            ConstraintProgrammingConstraint::integer(
                IntegerExpression::linear(
                    0,
                    [IntegerTerm {
                        variable: x.clone(),
                        coefficient: 9_007_199_254_740_993,
                    }],
                )
                .expect("expression"),
                IntegerRelation::GreaterOrEqual,
                0,
            ),
            [(x, IntegerDomain::boolean())],
        );
        let error = lower_constraint_programming(&snapshot).expect_err("numeric gate");
        assert!(matches!(error, MipLoweringError::Numeric { .. }));
    }

    #[test]
    fn exact_numeric_gate_rejects_large_objective_constants() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 9_007_199_254_740_993, None);
        let error = lower_constraint_programming(&snapshot).expect_err("numeric gate");
        assert!(matches!(error, MipLoweringError::Numeric { .. }));
    }

    #[test]
    fn mip_lowering_projects_validated_source_hint_to_a_partial_start() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, None);
        let mut lowering = lower_constraint_programming(&snapshot).expect("lowering");
        let hint = BTreeMap::from([(StableVariableId::from("x"), 1_i64)]);
        let applied =
            apply_solution_hint(&snapshot, &mut lowering, Some(&hint)).expect("hint projection");
        assert_eq!(applied, 1);
        let index = lowering.source_variables[&StableVariableId::from("x")];
        assert_eq!(
            lowering.model.basic.variables[index].get_result(),
            Some(1.0)
        );
        assert!(
            lowering
                .model
                .basic
                .variables
                .iter()
                .skip(lowering.source_variables.len())
                .all(|variable| variable.get_result().is_none())
        );
    }

    #[test]
    fn mip_backed_solver_accepts_a_validated_solution_hint() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, Some(1));
        let hint = BTreeMap::from([(StableVariableId::from("x"), 1_i64)]);
        let options = ConstraintProgrammingSolveOptions::builder()
            .solution_hint(Some(&hint))
            .finish();
        let solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: None,
            typed_best_bound: None,
        });
        let report = solver
            .solve_constraint_programming(&snapshot, &options)
            .expect("solve with hint");
        assert_eq!(
            report
                .diagnostics
                .extensions
                .get("cp.mip.hint.sourceVariables")
                .map(String::as_str),
            Some("1")
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.code == "CPMipPartialWarmStart")
        );
    }

    #[test]
    fn mip_backed_solver_rejects_a_complete_hint_before_calling_the_backend() {
        let x = crate::model::constraint_programming::IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("invalid-hint");
        model
            .register_variable(x.clone(), IntegerDomain::boolean())
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "x-one",
                ConstraintProgrammingConstraint::integer(
                    expression(&x),
                    crate::model::constraint_programming::IntegerRelation::Equal,
                    1,
                ),
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let hint = BTreeMap::from([(StableVariableId::from("x"), 0_i64)]);
        let solver = MipBackedConstraintProgrammingSolver::new(NeverCalledLinearSolver);
        let error = solver
            .solve_constraint_programming(
                &snapshot,
                &ConstraintProgrammingSolveOptions::builder()
                    .solution_hint(Some(&hint))
                    .finish(),
            )
            .expect_err("invalid complete hint must be rejected before backend invocation");
        assert!(error.to_string().contains("complete CP solution hint"));
    }

    #[test]
    fn mip_backed_immediate_limits_do_not_call_backend_and_keep_a_complete_hint() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, Some(1));
        let hint = BTreeMap::from([(StableVariableId::from("x"), 1_i64)]);
        let solver = MipBackedConstraintProgrammingSolver::new(NeverCalledLinearSolver);

        for options in [
            ConstraintProgrammingSolveOptions::builder()
                .time_limit(Some(std::time::Duration::ZERO))
                .solution_hint(Some(&hint))
                .finish(),
            ConstraintProgrammingSolveOptions::builder()
                .node_limit(Some(0))
                .solution_hint(Some(&hint))
                .finish(),
            ConstraintProgrammingSolveOptions::builder()
                .solution_limit(Some(0))
                .solution_hint(Some(&hint))
                .finish(),
        ] {
            let report = solver
                .solve_constraint_programming(&snapshot, &options)
                .expect("immediate limit report");
            assert_eq!(report.problem_status, ProblemStatus::Feasible);
            assert_eq!(report.termination_reason, options_limit_reason(&options));
            assert_eq!(report.solution_presence, SolutionPresence::Incumbent);
            assert_eq!(report.statistics.solution_count, Some(1));
            assert_eq!(report.statistics.best_bound, Some(0));
            assert!(report.statistics.absolute_gap.is_some());
            assert!(report.statistics.relative_gap.is_some());
            assert!(report.proof.is_none());
        }
    }

    #[test]
    fn mip_backed_pre_cancel_wins_over_an_immediate_limit() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, Some(1));
        let handle = crate::solver::SolveHandle::new();
        assert!(handle.cancel(crate::solver::CancellationOrigin::User));
        let options = ConstraintProgrammingSolveOptions::builder()
            .time_limit(Some(std::time::Duration::ZERO))
            .cancellation_handle(Some(&handle))
            .finish();
        let report = MipBackedConstraintProgrammingSolver::new(NeverCalledLinearSolver)
            .solve_constraint_programming(&snapshot, &options)
            .expect("cancelled report");
        assert_eq!(report.problem_status, ProblemStatus::Unknown);
        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert_eq!(report.solution, None);
        assert_eq!(report.proof, None);
        assert_eq!(
            report.diagnostics.extensions.get("cancellation.origin"),
            Some(&crate::solver::CancellationOrigin::User.to_string())
        );
    }

    fn options_limit_reason(options: &ConstraintProgrammingSolveOptions<'_>) -> TerminationReason {
        if options.time_limit.is_some_and(|limit| limit.is_zero()) {
            TerminationReason::TimeLimit
        } else if options.node_limit == Some(0) {
            TerminationReason::NodeLimit
        } else {
            TerminationReason::SolutionLimit
        }
    }

    #[test]
    fn mip_report_restores_minimum_objective_constant_and_ceils_bound() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 7, Some(1));
        let solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: Some(0.4),
            typed_best_bound: Some(0.4),
        });
        let report = solver
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("solve");
        assert_eq!(report.solution.as_ref().and_then(|s| s.objective), Some(8));
        assert_eq!(report.statistics.best_bound, Some(8));
        assert_eq!(report.statistics.best_bound_value, Some(7.4));
    }

    #[test]
    fn mip_report_restores_maximum_objective_constant_and_floors_bound() {
        let snapshot = objective_snapshot(ObjectiveCategory::Maximum, 7, Some(0));
        let solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: Some(0.6),
            typed_best_bound: Some(0.6),
        });
        let report = solver
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("solve");
        assert_eq!(report.solution.as_ref().and_then(|s| s.objective), Some(7));
        assert_eq!(report.statistics.best_bound, Some(7));
        assert_eq!(report.statistics.best_bound_value, Some(7.6));
    }

    #[test]
    fn mip_report_rejects_backend_objective_mismatch() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, Some(1));
        let solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 1.0,
            best_bound: None,
            typed_best_bound: None,
        });
        let error = solver
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect_err("objective mismatch");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::ContractViolation(message))
                if message.contains("objective does not match")
        ));
    }

    #[test]
    fn mip_report_rejects_bound_direction_mismatch() {
        let minimum_snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, Some(1));
        let minimum_solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: Some(1.5),
            typed_best_bound: Some(1.5),
        });
        let error = minimum_solver
            .solve_constraint_programming(
                &minimum_snapshot,
                &ConstraintProgrammingSolveOptions::new(),
            )
            .expect_err("minimum bound direction");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::ContractViolation(message))
                if message.contains("wrong direction")
        ));

        let maximum_snapshot = objective_snapshot(ObjectiveCategory::Maximum, 0, Some(0));
        let maximum_solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: Some(-0.5),
            typed_best_bound: Some(-0.5),
        });
        let error = maximum_solver
            .solve_constraint_programming(
                &maximum_snapshot,
                &ConstraintProgrammingSolveOptions::new(),
            )
            .expect_err("maximum bound direction");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::ContractViolation(message))
                if message.contains("wrong direction")
        ));
    }

    #[test]
    fn mip_report_rejects_inconsistent_typed_and_floating_bounds() {
        let snapshot = objective_snapshot(ObjectiveCategory::Minimum, 0, Some(1));
        let solver = MipBackedConstraintProgrammingSolver::new(ContractLinearSolver {
            objective_delta: 0.0,
            best_bound: Some(0.5),
            typed_best_bound: Some(0.25),
        });
        let error = solver
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect_err("inconsistent bounds");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::ContractViolation(message))
                if message.contains("typed and floating-point best bounds")
        ));
    }

    #[test]
    fn projected_solution_is_rechecked_against_the_source_snapshot() {
        let mut model = ConstraintProgrammingModel::new("projection");
        let x = variable(&mut model, "x", IntegerDomain::boolean());
        model
            .add_constraint(ConstraintDefinition::new(
                "one",
                ConstraintProgrammingConstraint::integer(expression(&x), IntegerRelation::Equal, 1),
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let result = lower_constraint_programming(&snapshot).expect("lowering");
        let source_index = result.source_variables[&StableVariableId::from("x")];
        let mut values = find_feasible_vector(&result.model).expect("MIP feasible vector");
        values[source_index] = 0.0;
        assert!(matches!(
            result.project_solution(&snapshot, &values),
            Err(MipLoweringError::Invalid { .. })
        ));
        values = find_feasible_vector(&result.model).expect("MIP feasible vector");
        assert!(matches!(
            result.project_solution(&snapshot, &{
                let mut invalid = values.clone();
                invalid[source_index] = 0.0;
                invalid
            }),
            Err(MipLoweringError::Invalid { .. })
        ));
        let assignment = result
            .project_solution(&snapshot, &values)
            .expect("verified assignment");
        assert_eq!(assignment[&StableVariableId::from("x")], 1);
    }

    #[test]
    fn mip_backed_solver_returns_a_verified_i64_report() {
        let mut model = ConstraintProgrammingModel::new("mip-backed");
        let x = variable(&mut model, "x", IntegerDomain::boolean());
        model
            .add_constraint(ConstraintDefinition::new(
                "one",
                ConstraintProgrammingConstraint::integer(expression(&x), IntegerRelation::Equal, 1),
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let solver = MipBackedConstraintProgrammingSolver::new(ExhaustiveLinearSolver);
        let report = solver
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("solve");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.solution_presence, SolutionPresence::Incumbent);
        assert!(!report.is_optimal());
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.stable_values.get(&StableVariableId::from("x")))
                .copied(),
            Some(1)
        );
        assert_eq!(report.fingerprints.model, Some(snapshot.fingerprint));
        report.validate().expect("report is valid");
    }

    #[test]
    fn mip_backed_session_rebuilds_the_snapshot_for_assumptions() {
        let mut model = ConstraintProgrammingModel::new("mip-session");
        let x = variable(&mut model, "x", IntegerDomain::boolean());
        let snapshot = model.freeze().expect("snapshot");
        let solver = MipBackedConstraintProgrammingSolver::new(ExhaustiveLinearSolver);
        let mut session = solver.create_session(&snapshot).expect("session");
        let report = session
            .solve_with_assumptions(
                &[ConstraintProgrammingAssumption::Equal(x, 0)],
                &ConstraintProgrammingSolveOptions::new(),
            )
            .expect("solve");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.stable_values.get(&StableVariableId::from("x")))
                .copied(),
            Some(0)
        );
        assert_ne!(report.fingerprints.model, Some(snapshot.fingerprint));
    }

    #[test]
    fn cp_conflict_orchestration_preserves_backend_farkas_evidence() {
        let x = IntegerVariable::new("farkas/x");
        let mut model = ConstraintProgrammingModel::new("farkas-priority");
        model
            .register_variable(x.clone(), IntegerDomain::boolean())
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "force-zero",
                ConstraintProgrammingConstraint::integer(expression(&x), IntegerRelation::Equal, 0),
            ))
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let solver = MipBackedConstraintProgrammingSolver::new(FarkasLinearSolver);
        let report = solver
            .solve_constraint_programming_with_conflict(
                &snapshot,
                &ConstraintProgrammingSolveOptions::builder()
                    .request_conflict(true)
                    .finish(),
            )
            .expect("CP conflict orchestration");
        let evidence = report
            .diagnostics
            .infeasibility_evidence
            .as_ref()
            .expect("Farkas evidence");
        assert_eq!(evidence.source, InfeasibilityEvidenceSource::Farkas);
        assert_eq!(evidence.reliability, ProofReliability::Exact);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .all(|issue| { issue.code != "CpConflictEvidenceReplaced" })
        );
        report.validate().expect("Farkas CP report should be valid");
    }

    fn find_feasible_vector(model: &LinearTriadModel) -> Option<Vec<f64>> {
        fn search(model: &LinearTriadModel, index: usize, values: &mut [f64]) -> bool {
            if index == model.num_variables() {
                return model
                    .basic
                    .A
                    .rows
                    .iter()
                    .enumerate()
                    .all(|(row_index, row)| {
                        let lhs = row
                            .entries
                            .iter()
                            .map(|(variable, coefficient)| values[*variable] * coefficient)
                            .sum::<f64>();
                        lhs <= model.basic.b[row_index] + f64::EPSILON
                    });
            }
            let lower = model.basic.lb[index] as i64;
            let upper = model.basic.ub[index] as i64;
            for value in lower..=upper {
                values[index] = value as f64;
                if search(model, index + 1, values) {
                    return true;
                }
            }
            false
        }

        let mut values = vec![0.0; model.num_variables()];
        search(model, 0, &mut values).then_some(values)
    }
}
