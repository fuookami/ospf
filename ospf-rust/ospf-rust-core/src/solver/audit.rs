//! 求解模型身份与解审计 / Solver-model identity and solution audit.
//!
//! 本模块在 solver 列/行索引与报告级稳定身份之间建立确定性映射，并在报告边界重新求值。
//! This module creates deterministic solver-index mappings and re-evaluates solutions at the report boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::error::{CoreError, Result, SolverError};
use crate::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, QuadraticTetradModel};

use super::report::{
    ConstraintEvaluation, InfeasibilityEvidence, InfeasibilityEvidenceSource,
    InfeasibilityMinimality, ProblemStatus, ProofCompleteness, ProofReliability, SolveDiagnostics,
    SolveIssue, SolveReport, StableVariableId, VariableBoundEvaluation,
};

/// 线性模型的稳定 solver 映射 / Stable solver mapping for a linear model.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearModelMapping {
    /// 按 solver 列顺序排列的稳定变量 ID / Stable variable IDs in solver-column order.
    pub variables_by_column: Vec<StableVariableId>,
    /// 稳定变量 ID 到 solver 列的反向映射 / Reverse mapping from stable variable ID to solver column.
    pub columns_by_variable: BTreeMap<StableVariableId, usize>,
    /// 按 solver 行顺序排列的稳定约束 ID / Stable constraint IDs in solver-row order.
    pub constraints_by_row: Vec<String>,
    /// 稳定约束 ID 到 solver 行的反向映射 / Reverse mapping from stable constraint ID to solver row.
    pub rows_by_constraint: BTreeMap<String, usize>,
    /// 按二次约束顺序排列的稳定 ID / Stable IDs in quadratic-constraint order.
    pub quadratic_constraints_by_row: Vec<String>,
    /// 二次约束稳定 ID 到模型索引的反向映射 / Reverse mapping for quadratic constraints.
    pub rows_by_quadratic_constraint: BTreeMap<String, usize>,
    /// 稳定目标 ID / Stable objective ID.
    pub objective_id: String,
}

/// 校验 native backend 使用的线性模型结构 / Validate linear model structure before native backend use.
///
/// 该检查在任何 solver 行列映射前执行，避免损坏的稀疏索引落入 backend binding 的直接索引。
/// This check runs before any solver row/column mapping so malformed sparse indices cannot reach
/// direct indexing in a backend binding.
pub fn validate_linear_model_for_backend(model: &LinearTriadModel) -> Result<()> {
    validate_linear_parts_for_backend(&model.basic, &model.c, "linear")
}

/// 校验 native backend 使用的二次模型结构 / Validate quadratic model structure before native backend use.
///
/// 除线性部分外，还校验二次目标矩阵和二次约束中的变量索引。
/// In addition to the linear part, this validates quadratic objective and constraint variable indices.
pub fn validate_quadratic_model_for_backend(model: &QuadraticTetradModel) -> Result<()> {
    validate_linear_parts_for_backend(&model.basic.linear, &model.c, "quadratic")?;

    let variable_count = model.num_variables();
    if !model.Q.rows.is_empty() && model.Q.rows.len() != variable_count {
        return Err(audit_error(&format!(
            "quadratic objective row count {} does not match variable count {}",
            model.Q.rows.len(),
            variable_count
        )));
    }
    for (row_index, row) in model.Q.rows.iter().enumerate() {
        validate_sparse_row(
            row,
            variable_count,
            &format!("quadratic objective row {}", row_index),
        )?;
    }

    for (constraint_index, constraint) in model.quadratic_constraints.iter().enumerate() {
        validate_finite_value(
            *constraint.polynomial.constant(),
            &format!("quadratic constraint {} constant", constraint_index),
        )?;
        validate_finite_value(
            constraint.rhs,
            &format!("quadratic constraint {} right-hand side", constraint_index),
        )?;
        for monomial in constraint.polynomial.monomials() {
            let first = monomial.var_index1();
            if first >= variable_count {
                return Err(audit_error(&format!(
                    "quadratic constraint {} references invalid variable index {}",
                    constraint_index, first
                )));
            }
            if let Some(second) = monomial.var_index2()
                && second >= variable_count
            {
                return Err(audit_error(&format!(
                    "quadratic constraint {} references invalid variable index {}",
                    constraint_index, second
                )));
            }
            validate_finite_value(
                *monomial.coefficient(),
                &format!("quadratic constraint {} coefficient", constraint_index),
            )?;
        }
    }
    Ok(())
}

fn validate_linear_parts_for_backend(
    model: &BasicLinearTriadModel,
    objective: &[f64],
    model_kind: &str,
) -> Result<()> {
    let variable_count = model.variables.len();
    let constraint_count = model.A.rows.len();

    for (name, actual, expected) in [
        ("lower bounds", model.lb.len(), variable_count),
        ("upper bounds", model.ub.len(), variable_count),
        ("variable types", model.var_types.len(), variable_count),
        ("objective coefficients", objective.len(), variable_count),
        (
            "constraint right-hand sides",
            model.b.len(),
            constraint_count,
        ),
    ] {
        if actual != expected {
            return Err(audit_error(&format!(
                "{} {} has length {}, expected {}",
                model_kind, name, actual, expected
            )));
        }
    }

    // 反向有限界表示合法的数学不可行实例，必须交给 backend 形成 Infeasible report。
    // A reversed finite bound pair is a valid mathematically infeasible instance and must reach
    // the backend so it can produce an Infeasible report.
    for variable in 0..variable_count {
        finite_bound(model.lb[variable], true)?;
        finite_bound(model.ub[variable], false)?;
    }

    for (constraint, rhs) in model.b.iter().copied().enumerate() {
        validate_finite_value(
            rhs,
            &format!("{} constraint {} right-hand side", model_kind, constraint),
        )?;
    }
    for (variable, coefficient) in objective.iter().copied().enumerate() {
        validate_finite_value(
            coefficient,
            &format!("{} objective coefficient {}", model_kind, variable),
        )?;
    }
    for (row_index, row) in model.A.rows.iter().enumerate() {
        validate_sparse_row(
            row,
            variable_count,
            &format!("{} constraint row {}", model_kind, row_index),
        )?;
    }
    Ok(())
}

fn validate_sparse_row(
    row: &crate::model::intermediate::SparseVector<f64>,
    variable_count: usize,
    context: &str,
) -> Result<()> {
    for &(column, value) in &row.entries {
        if column >= variable_count {
            return Err(audit_error(&format!(
                "{} references invalid variable index {}",
                context, column
            )));
        }
        validate_finite_value(value, &format!("{} coefficient", context))?;
    }
    Ok(())
}

fn validate_finite_value(value: f64, context: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CoreError::Solver(SolverError::NonFinite(format!(
            "{} contains {}",
            context, value
        ))))
    }
}

impl LinearModelMapping {
    /// 校验双向映射的一致性 / Validate bidirectional mapping consistency.
    pub fn validate(&self) -> Result<()> {
        if self.variables_by_column.len() != self.columns_by_variable.len()
            || self.constraints_by_row.len() != self.rows_by_constraint.len()
            || self.quadratic_constraints_by_row.len() != self.rows_by_quadratic_constraint.len()
            || self.objective_id.is_empty()
        {
            return Err(audit_error(
                "solver model mapping dimensions are inconsistent",
            ));
        }
        for (column, variable_id) in self.variables_by_column.iter().enumerate() {
            if self.columns_by_variable.get(variable_id) != Some(&column) {
                return Err(audit_error("variable solver mapping is not bijective"));
            }
        }
        for (row, constraint_id) in self.constraints_by_row.iter().enumerate() {
            if self.rows_by_constraint.get(constraint_id) != Some(&row) {
                return Err(audit_error("constraint solver mapping is not bijective"));
            }
        }
        for (row, constraint_id) in self.quadratic_constraints_by_row.iter().enumerate() {
            if self.rows_by_quadratic_constraint.get(constraint_id) != Some(&row) {
                return Err(audit_error(
                    "quadratic constraint solver mapping is not bijective",
                ));
            }
        }
        Ok(())
    }

    /// 根据变量稳定 ID 查询 solver 列 / Find a solver column by stable variable ID.
    pub fn solver_column(&self, variable_id: &StableVariableId) -> Option<usize> {
        self.columns_by_variable.get(variable_id).copied()
    }

    /// 根据 solver 列查询稳定变量 ID / Find a stable variable ID by solver column.
    pub fn stable_variable(&self, column: usize) -> Option<&StableVariableId> {
        self.variables_by_column.get(column)
    }

    /// 根据约束稳定 ID 查询 solver 行 / Find a solver row by stable constraint ID.
    pub fn solver_row(&self, constraint_id: &str) -> Option<usize> {
        self.rows_by_constraint.get(constraint_id).copied()
    }

    /// 根据 solver 行查询稳定约束 ID / Find a stable constraint ID by solver row.
    pub fn stable_constraint(&self, row: usize) -> Option<&str> {
        self.constraints_by_row.get(row).map(String::as_str)
    }
}

/// 为线性模型生成确定性双向映射 / Build a deterministic bidirectional mapping for a linear model.
pub fn linear_model_mapping(model: &LinearTriadModel) -> Result<LinearModelMapping> {
    validate_linear_model_for_backend(model)?;

    let variables_by_column = model
        .basic
        .variables
        .iter()
        .enumerate()
        .map(|(column, token)| {
            StableVariableId(stable_element_id(
                "linear-variable",
                &[&model.basic.name, &column.to_string(), token.name()],
            ))
        })
        .collect::<Vec<_>>();
    let columns_by_variable = variables_by_column
        .iter()
        .cloned()
        .enumerate()
        .map(|(column, variable_id)| (variable_id, column))
        .collect::<BTreeMap<_, _>>();

    let constraints_by_row = (0..model.num_constraints())
        .map(|row| {
            let name = model
                .basic
                .constraint_names
                .get(row)
                .map(String::as_str)
                .unwrap_or("");
            let source = model
                .basic
                .constraint_source_symbol_ids
                .get(row)
                .and_then(|source| *source)
                .map(|source| source.to_string())
                .unwrap_or_default();
            stable_element_id(
                "linear-constraint",
                &[&model.basic.name, &row.to_string(), name, &source],
            )
        })
        .collect::<Vec<_>>();
    let rows_by_constraint = constraints_by_row
        .iter()
        .cloned()
        .enumerate()
        .map(|(row, constraint_id)| (constraint_id, row))
        .collect::<BTreeMap<_, _>>();

    let mapping = LinearModelMapping {
        variables_by_column,
        columns_by_variable,
        constraints_by_row,
        rows_by_constraint,
        quadratic_constraints_by_row: Vec::new(),
        rows_by_quadratic_constraint: BTreeMap::new(),
        objective_id: stable_element_id(
            "linear-objective",
            &[
                &model.basic.name,
                &format!("{:?}", model.objective_category),
            ],
        ),
    };
    mapping.validate()?;
    Ok(mapping)
}

/// 为二次模型生成稳定元素映射 / Build a stable element mapping for a quadratic model.
pub fn quadratic_model_mapping(model: &QuadraticTetradModel) -> Result<LinearModelMapping> {
    validate_quadratic_model_for_backend(model)?;
    let linear = LinearTriadModel {
        basic: model.basic.linear.clone(),
        c: model.c.clone(),
        objective_category: model.objective_category,
    };
    let mut mapping = linear_model_mapping(&linear)?;
    mapping.objective_id = stable_element_id(
        "quadratic-objective",
        &[
            &model.basic.linear.name,
            &format!("{:?}", model.objective_category),
        ],
    );
    mapping.quadratic_constraints_by_row = (0..model.num_quadratic_constraints())
        .map(|row| {
            let name = model
                .quadratic_constraint_names
                .get(row)
                .map(String::as_str)
                .unwrap_or("");
            let source = model
                .quadratic_constraint_source_symbol_ids
                .get(row)
                .and_then(|source| *source)
                .map(|source| source.to_string())
                .unwrap_or_default();
            stable_element_id(
                "quadratic-constraint",
                &[&model.basic.linear.name, &row.to_string(), name, &source],
            )
        })
        .collect();
    mapping.rows_by_quadratic_constraint = mapping
        .quadratic_constraints_by_row
        .iter()
        .cloned()
        .enumerate()
        .map(|(row, constraint_id)| (constraint_id, row))
        .collect();
    mapping.validate()?;
    Ok(mapping)
}

/// 对线性 incumbent 执行约束和变量界求值 / Evaluate linear constraints and variable bounds for an incumbent.
pub fn evaluate_linear_solution(
    model: &LinearTriadModel,
    values: &[f64],
    tolerance: f64,
) -> Result<SolveDiagnostics<f64>> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(audit_error(
            "solution-evaluation tolerance must be finite and non-negative",
        ));
    }
    if values.len() != model.num_variables() {
        return Err(audit_error(&format!(
            "solution length {} does not match variable count {}",
            values.len(),
            model.num_variables()
        )));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(audit_error("solution contains a non-finite value"));
    }

    let mapping = linear_model_mapping(model)?;
    let mut diagnostics = SolveDiagnostics::default();
    diagnostics
        .extensions
        .insert("evaluation.tolerance".to_owned(), tolerance.to_string());

    for (column, value) in values.iter().copied().enumerate() {
        let lower = finite_bound(model.basic.lb[column], true)?;
        let upper = finite_bound(model.basic.ub[column], false)?;
        let satisfied = lower.is_none_or(|bound| value + tolerance >= bound)
            && upper.is_none_or(|bound| value - tolerance <= bound);
        diagnostics
            .variable_bound_evaluations
            .push(VariableBoundEvaluation {
                variable_id: mapping.variables_by_column[column].clone(),
                value,
                lower,
                upper,
                satisfied,
            });
        if !satisfied {
            diagnostics.issues.push(SolveIssue::new(
                "VariableBoundViolation",
                format!("solver column {} is outside its declared bounds", column),
            ));
        }
    }

    for (row_index, row) in model.basic.A.rows.iter().enumerate() {
        let rhs = *model
            .basic
            .b
            .get(row_index)
            .ok_or_else(|| audit_error("constraint row is missing a right-hand side"))?;
        if !rhs.is_finite() {
            return Err(audit_error("constraint right-hand side is non-finite"));
        }
        let mut lhs = 0.0;
        for (column, coefficient) in &row.entries {
            let value = values
                .get(*column)
                .ok_or_else(|| audit_error("constraint references an invalid solver column"))?;
            if !coefficient.is_finite() {
                return Err(audit_error("constraint coefficient is non-finite"));
            }
            lhs += coefficient * value;
        }
        if !lhs.is_finite() {
            return Err(audit_error(
                "constraint evaluation overflowed to a non-finite value",
            ));
        }
        let residual = rhs - lhs;
        let violation = (lhs - rhs - tolerance).max(0.0);
        let satisfied = residual + tolerance >= 0.0;
        diagnostics
            .constraint_evaluations
            .push(ConstraintEvaluation {
                constraint_id: mapping.constraints_by_row[row_index].clone(),
                lhs,
                rhs,
                residual,
                violation,
                tolerance,
                satisfied,
            });
        if !satisfied {
            diagnostics.issues.push(SolveIssue::new(
                "ConstraintViolation",
                format!("solver row {} violates its less-equal relation", row_index),
            ));
        }
    }

    Ok(diagnostics)
}

/// 将模型映射附加到报告 / Attach a model mapping to a report.
pub fn attach_linear_model_mapping(
    mut report: SolveReport<f64>,
    model: &LinearTriadModel,
) -> Result<SolveReport<f64>> {
    let mapping = linear_model_mapping(model)?;
    report.model_mapping = Some(mapping.clone());
    attach_farkas_evidence(&mut report, &mapping);
    report.validate()?;
    Ok(report)
}

/// 将二次模型映射附加到报告 / Attach a quadratic-model mapping to a report.
pub fn attach_quadratic_model_mapping(
    mut report: SolveReport<f64>,
    model: &QuadraticTetradModel,
) -> Result<SolveReport<f64>> {
    let mapping = quadratic_model_mapping(model)?;
    report.model_mapping = Some(mapping.clone());
    attach_farkas_evidence(&mut report, &mapping);
    report.validate()?;
    Ok(report)
}

/// 将线性解审计附加到报告，并拒绝非法 incumbent / Attach linear-solution audit and reject an invalid incumbent.
pub fn attach_linear_solution_audit(
    mut report: SolveReport<f64>,
    model: &LinearTriadModel,
) -> Result<SolveReport<f64>> {
    let mapping = linear_model_mapping(model)?;
    report.model_mapping = Some(mapping.clone());
    attach_farkas_evidence(&mut report, &mapping);
    if let Some(solution) = report.solution.as_ref()
        && !solution.values.is_empty()
    {
        let diagnostics = evaluate_linear_solution(model, &solution.values, 1e-7)?;
        if diagnostics
            .constraint_evaluations
            .iter()
            .any(|evaluation| !evaluation.satisfied)
            || diagnostics
                .variable_bound_evaluations
                .iter()
                .any(|evaluation| !evaluation.satisfied)
        {
            return Err(audit_error(
                "solver incumbent failed independent model evaluation",
            ));
        }
        report.diagnostics.constraint_evaluations = diagnostics.constraint_evaluations;
        report.diagnostics.variable_bound_evaluations = diagnostics.variable_bound_evaluations;
        report.diagnostics.extensions.extend(diagnostics.extensions);
    }
    report.validate()?;
    Ok(report)
}

fn attach_farkas_evidence(report: &mut SolveReport<f64>, mapping: &LinearModelMapping) {
    if report.problem_status != ProblemStatus::Infeasible {
        return;
    }
    if report
        .diagnostics
        .infeasibility_evidence
        .as_ref()
        .is_some_and(InfeasibilityEvidence::is_authoritative)
    {
        return;
    }
    let Some(evidence) = report
        .proof
        .as_ref()
        .and_then(|proof| proof.evidence.as_ref())
    else {
        return;
    };
    if evidence.is_empty() {
        return;
    }
    let constraint_ids = mapping
        .constraints_by_row
        .iter()
        .take(evidence.len())
        .cloned()
        .collect();
    report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
        source: InfeasibilityEvidenceSource::Farkas,
        reliability: ProofReliability::Reliable,
        completeness: if evidence.len() >= mapping.constraints_by_row.len() {
            ProofCompleteness::Complete
        } else {
            ProofCompleteness::Partial
        },
        constraint_ids,
        members: BTreeSet::new(),
        minimality: InfeasibilityMinimality::NotChecked,
        computation_time: Duration::ZERO,
        unavailable_reason: None,
    });
}

fn finite_bound(value: f64, lower: bool) -> Result<Option<f64>> {
    if value.is_nan() {
        return Err(audit_error("variable bounds must not contain NaN"));
    }
    if value.is_infinite() {
        if (lower && value.is_sign_negative()) || (!lower && value.is_sign_positive()) {
            return Ok(None);
        }
        return Err(audit_error(
            "variable bound has the wrong infinity direction",
        ));
    }
    Ok(Some(value))
}

/// 生成跨模型和远程协议复用的确定性元素 ID / Build a deterministic element ID shared by model and remote protocols.
pub fn stable_element_id(kind: &str, parts: &[&str]) -> String {
    let mut value = String::from(kind);
    for part in parts {
        value.push(':');
        value.push_str(&part.len().to_string());
        value.push('#');
        value.push_str(part);
    }
    value
}

fn audit_error(message: &str) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(format!(
        "invalid solver model audit: {}",
        message
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SolverErrorClass;
    use crate::model::ObjectiveCategory;
    use crate::model::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, QuadraticTetradModel, SparseMatrix,
        SparseVector,
    };

    fn model() -> LinearTriadModel {
        let mut basic = BasicLinearTriadModel::new("audit");
        let x = crate::variable::ContinuousVariableItem::auto("same");
        let y = crate::variable::ContinuousVariableItem::auto("same");
        basic.add_variable_with_bounds(
            crate::token::Token::from_generic(x, 0),
            0.0,
            2.0,
            crate::variable::VariableType::Continuous,
        );
        basic.add_variable_with_bounds(
            crate::token::Token::from_generic(y, 1),
            0.0,
            3.0,
            crate::variable::VariableType::Continuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        row.add(1, 2.0);
        basic.add_constraint_with_metadata(row, 8.0, "same".to_owned(), None, false, 0, None, None);
        LinearTriadModel::from_basic(basic)
    }

    #[test]
    fn duplicate_names_still_have_unique_bidirectional_ids() {
        let mapping = linear_model_mapping(&model()).unwrap();
        assert_eq!(mapping.variables_by_column.len(), 2);
        assert_ne!(
            mapping.variables_by_column[0],
            mapping.variables_by_column[1]
        );
        assert_eq!(
            mapping.solver_column(&mapping.variables_by_column[1]),
            Some(1)
        );
        assert_eq!(
            mapping.stable_variable(0),
            Some(&mapping.variables_by_column[0])
        );
        assert_eq!(mapping.solver_row(&mapping.constraints_by_row[0]), Some(0));
    }

    #[test]
    fn evaluation_reports_slack_and_bound_violation_without_false_satisfaction() {
        let model = model();
        let diagnostics = evaluate_linear_solution(&model, &[1.0, 2.0], 1e-7).unwrap();
        assert_eq!(diagnostics.constraint_evaluations[0].lhs, 5.0);
        assert_eq!(diagnostics.constraint_evaluations[0].residual, 3.0);
        assert!(diagnostics.constraint_evaluations[0].satisfied);

        let diagnostics = evaluate_linear_solution(&model, &[2.5, 2.0], 1e-7).unwrap();
        assert!(!diagnostics.variable_bound_evaluations[0].satisfied);
        assert!(!diagnostics.issues.is_empty());
    }

    #[test]
    fn backend_preflight_rejects_invalid_sparse_column_as_contract_error() {
        let mut model = model();
        model.basic.A.rows[0].add(2, 1.0);

        let error = validate_linear_model_for_backend(&model).expect_err("invalid column");
        assert_eq!(
            error.solver_error_class(),
            SolverErrorClass::InternalContract
        );
        assert!(error.to_string().contains("invalid variable index 2"));
    }

    #[test]
    fn backend_preflight_rejects_dimension_and_non_finite_bound_mismatches() {
        let mut objective_model = model();
        objective_model.c.push(1.0);
        let error =
            validate_linear_model_for_backend(&objective_model).expect_err("objective dimension");
        assert!(error.to_string().contains("objective coefficients"));

        let mut bound_model = model();
        bound_model.basic.lb[0] = f64::NAN;
        let error = validate_linear_model_for_backend(&bound_model).expect_err("NaN bound");
        assert!(
            error
                .to_string()
                .contains("variable bounds must not contain NaN")
        );

        let mut infeasible_bound_model = model();
        infeasible_bound_model.basic.lb[0] = 4.0;
        assert!(validate_linear_model_for_backend(&infeasible_bound_model).is_ok());

        let mut rhs_model = model();
        rhs_model.basic.b.clear();
        let error = validate_linear_model_for_backend(&rhs_model).expect_err("rhs dimension");
        assert!(error.to_string().contains("right-hand sides"));
    }

    #[test]
    fn quadratic_backend_preflight_rejects_invalid_quadratic_column() {
        let linear = model();
        let mut quadratic =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::from_linear(linear.basic));
        let mut objective = SparseMatrix::new();
        objective.add_row(SparseVector::new());
        objective.add_row(SparseVector::new());
        objective.rows[0].add(2, 1.0);
        quadratic.set_objective(vec![0.0, 0.0], objective, ObjectiveCategory::Minimum);

        let error =
            validate_quadratic_model_for_backend(&quadratic).expect_err("invalid quadratic column");
        assert_eq!(
            error.solver_error_class(),
            SolverErrorClass::InternalContract
        );
        assert!(error.to_string().contains("invalid variable index 2"));
    }
}
