//! Core-backed extension solver adapters.
//! Core 驱动的扩展求解器适配器。
//!
//! 本模块将 `ospf-rust-core` 的通用求解器接口适配到 framework 层的
//! `ColumnGenerationSolver` 和 `LinearBendersDecompositionSolver`。
//! This module adapts generic `ospf-rust-core` solvers to framework-level
//! `ColumnGenerationSolver` and `LinearBendersDecompositionSolver`.

use std::collections::HashMap;
use std::sync::Arc;
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::model::intermediate::{

    LPExportableModel, LinearTriadModel, QuadraticTetradModel, SparseVector,
};
use ospf_rust_core::model::mechanism::{ConstraintRelation, LinearInequality, MechanismModel};
use ospf_rust_core::solver::{Solver, SolverExt, SolverOutput};
use ospf_rust_core::variable::VariableId;

use super::CutSense;
use super::column_generation_solver::{
    ColumnGenerationSolver, FeasibleSolution, FeasibleSolutionV, LPResult, LPResultV,
    LinearDualSolution, RegistrationStatus, RegistrationStatusCallback, SolvingStatus,
    SolvingStatusCallback,
};
use super::linear_benders_decomposition_solver::{
    LinearBendersDecompositionSolver, LinearCut, LinearFeasibleResult, LinearInfeasibleResult,
    LinearSubResult, LinearSubResultV, QuadraticSubResultV,
};
use super::quadratic_benders_decomposition_solver::{
    QuadraticBendersDecompositionSolver, QuadraticCut, QuadraticFeasibleResult,
    QuadraticInfeasibleResult, QuadraticSubResult,
};

/// Benders 割生成上下文 / Benders cut generation context
#[derive(Debug, Clone)]
pub(crate) struct BendersCutContext {
    /// 割模板机理模型 / Cut-template mechanism model
    pub mechanism_model: MechanismModel<f64>,
    /// 最优性割目标变量 / Optimality-cut objective variable
    pub objective_variable: Option<VariableId>,
    /// 固定变量 ID 映射顺序 / Fixed-variable ID order
    pub fixed_variable_ids: Vec<VariableId>,
}

/// 二次子问题 cut 能力探针 / Quadratic sub-problem cut capability probe
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuadraticCutCapabilityProbe {
    /// 是否为真实二次子问题（非线性退化）/ Whether this is a true quadratic sub-problem (not linear-degenerate)
    pub true_quadratic_subproblem: bool,
    /// 是否配置 cut 上下文 / Whether cut context is configured
    pub has_cut_context: bool,
    /// 是否配置最优性割目标变量 / Whether optimal-cut objective variable is configured
    pub has_objective_variable: bool,
    /// 是否可用对偶乘子 / Whether dual multipliers are available
    pub has_dual_solution: bool,
    /// 是否可用 Farkas 乘子 / Whether Farkas multipliers are available
    pub has_farkas_solution: bool,
}

impl QuadraticCutCapabilityProbe {
    /// 是否可生成最优性割 / Whether optimal cuts can be generated
    pub(crate) fn can_generate_optimal_cut(&self) -> bool {
        self.has_cut_context
            && self.has_objective_variable
            && (!self.true_quadratic_subproblem || self.has_dual_solution)
    }

    /// 是否可生成可行性割 / Whether feasibility cuts can be generated
    pub(crate) fn can_generate_feasibility_cut(&self) -> bool {
        self.has_cut_context && (!self.true_quadratic_subproblem || self.has_farkas_solution)
    }
}

fn count_non_zeros(model: &LinearTriadModel) -> usize {
    model.A.rows.iter().map(|row| row.entries.len()).sum()
}

fn emit_registration_status(
    solver_name: &str,
    model: &LinearTriadModel,
    callback: Option<RegistrationStatusCallback>,
) -> Result<()> {
    if let Some(callback) = callback {
        callback(&RegistrationStatus::new(
            solver_name.to_string(),
            model.num_variables(),
            model.num_constraints(),
            count_non_zeros(model),
        ))?;
    }
    Ok(())
}

fn map_solving_status(
    status: &ospf_rust_core::solver::SolvingStatus,
    solver_index: usize,
) -> SolvingStatus {
    SolvingStatus {
        solver: status.solver.clone(),
        solver_index,
        obj: status.objective_value.unwrap_or(0.0),
        lower_bound: status.best_bound,
        upper_bound: status.objective_value,
        gap: status.mip_gap,
        elapsed: status.solve_time,
        node_count: status.node_count,
    }
}

fn build_core_solving_callback(
    callback: Option<SolvingStatusCallback>,
    solver_index: usize,
) -> Option<ospf_rust_core::solver::SolvingStatusCallback> {
    callback.map(|callback| {
        Arc::new(move |status: &ospf_rust_core::solver::SolvingStatus| {
            callback(&map_solving_status(status, solver_index))
        }) as ospf_rust_core::solver::SolvingStatusCallback
    })
}

fn sanitize_file_stem(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "solve".to_string()
    } else {
        sanitized
    }
}

fn export_lp_model_if_requested<M>(
    model: &M,
    solver_name: &str,
    stage: &str,
    options: &super::FrameworkSolveOptions,
) -> Result<()>
where
    M: LPExportableModel,
{
    if !options.to_log_model {
        return Ok(());
    }
    let solve_name = options.name.as_deref().unwrap_or("solve");
    let file_stem = sanitize_file_stem(solve_name);
    let file_name = format!("{}_{}_{}.lp", file_stem, solver_name, stage);
    model.write_lp(&file_name).map_err(|e| {
        CoreError::Solver(SolverError::SolveFailed(format!(
            "failed to export LP model `{}`: {}",
            file_name, e
        )))
    })?;
    Ok(())
}

pub(crate) fn solver_output_to_feasible(output: &SolverOutput) -> Result<FeasibleSolution> {
    if matches!(
        output.status,
        ospf_rust_core::solver::SolverStatus::TimeLimit
    ) && output.solution.is_none()
    {
        return Err(CoreError::Solver(SolverError::SolveFailed(
            "solver terminated by time limit without feasible solution".into(),
        )));
    }
    if output.status.is_feasible() {
        return FeasibleSolution::try_from_output(
            output,
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );
    }

    let err = match output.status {
        ospf_rust_core::solver::SolverStatus::Infeasible => SolverError::Infeasible,
        ospf_rust_core::solver::SolverStatus::InfeasibleOrUnbounded => {
            SolverError::SolveFailed("problem is infeasible or unbounded".into())
        }
        ospf_rust_core::solver::SolverStatus::Unbounded => SolverError::Unbounded,
        ospf_rust_core::solver::SolverStatus::TimeLimit => SolverError::SolveFailed(
            "solver terminated by time limit without feasible solution".into(),
        ),
        _ => SolverError::SolveFailed(format!(
            "solver returned non-feasible status {:?}",
            output.status
        )),
    };
    Err(CoreError::Solver(err))
}

pub(crate) fn solution_vector_from_output(
    output: &SolverOutput,
    context: &str,
) -> Result<Vec<f64>> {
    if !output.status.is_feasible() {
        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "{} did not produce feasible output: {:?}",
            context, output.status
        ))));
    }
    FeasibleSolution::try_feasible_typed_output_from_output(
        output,
        ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
    )
    .map(|typed_output| typed_output.solution)
    .map_err(|err| match err {
        CoreError::Solver(SolverError::NoSolution) => CoreError::Solver(SolverError::SolveFailed(
            format!("{} missing solution vector", context),
        )),
        other => other,
    })
}

pub(crate) fn resolve_lp_dual_solution<S>(
    solver: &S,
    lp_model: &LinearTriadModel,
    output: &SolverOutput,
) -> Result<Vec<f64>>
where
    S: Solver + Send + Sync,
{
    if let Some(dual) = output.dual_solution.clone() {
        if lp_dual_row_objective_matches(lp_model, output, &dual) {
            return Ok(dual[..lp_model.num_constraints()].to_vec());
        }
    }

    resolve_lp_dual_solution_by_model(solver, lp_model)
}

fn lp_dual_row_objective_matches(
    lp_model: &LinearTriadModel,
    output: &SolverOutput,
    dual: &[f64],
) -> bool {
    let expected_rows = lp_model.num_constraints();
    if dual.len() < expected_rows {
        return false;
    }
    let Some(primal_objective) = output.objective_value else {
        return true;
    };
    if !primal_objective.is_finite() {
        return true;
    }

    let dual_objective = lp_model
        .basic
        .b
        .iter()
        .take(expected_rows)
        .zip(dual.iter())
        .map(|(rhs, dual_value)| rhs * dual_value)
        .sum::<f64>();
    if !dual_objective.is_finite() {
        return false;
    }

    let scale = primal_objective.abs().max(dual_objective.abs()).max(1.0);
    (primal_objective - dual_objective).abs() <= 1e-6 * scale
}

fn resolve_lp_dual_solution_by_model<S>(solver: &S, lp_model: &LinearTriadModel) -> Result<Vec<f64>>
where
    S: Solver + Send + Sync,
{
    // 回退：显式构造并求解对偶模型，取前 m 个行乘子。
    // Fallback: explicitly solve dual model and take first m row multipliers.
    let dual_model = lp_model.to_dual();
    let dual_output = solver.solve_linear(&dual_model)?;
    let dual_solution = solution_vector_from_output(&dual_output, "dual model")?;
    let expected_rows = lp_model.num_constraints();
    if dual_solution.len() < expected_rows {
        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "dual solution length {} is smaller than expected row count {}",
            dual_solution.len(),
            expected_rows
        ))));
    }
    Ok(dual_solution[..expected_rows].to_vec())
}

pub(crate) fn resolve_farkas_dual_solution<S>(
    solver: &S,
    lp_model: &LinearTriadModel,
    output: &SolverOutput,
) -> Result<Vec<f64>>
where
    S: Solver + Send + Sync,
{
    if let Some(farkas_solution) = output.dual_solution.clone() {
        let expected_rows = lp_model.num_constraints();
        if farkas_solution.len() >= expected_rows {
            return Ok(farkas_solution[..expected_rows].to_vec());
        }
    }
    resolve_farkas_dual_solution_by_model(solver, lp_model)
}

fn resolve_farkas_dual_solution_by_model<S>(
    solver: &S,
    lp_model: &LinearTriadModel,
) -> Result<Vec<f64>>
where
    S: Solver + Send + Sync,
{
    // 回退：显式构造并求解 Farkas 对偶模型。
    // Fallback: explicitly solve the Farkas dual model.
    let farkas_model = lp_model.to_farkas_dual();
    let farkas_output = solver.solve_linear(&farkas_model)?;
    let farkas_solution = solution_vector_from_output(&farkas_output, "farkas dual model")?;
    let expected_rows = lp_model.num_constraints();
    if farkas_solution.len() < expected_rows {
        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "farkas dual solution length {} is smaller than expected row count {}",
            farkas_solution.len(),
            expected_rows
        ))));
    }
    Ok(farkas_solution[..expected_rows].to_vec())
}

fn append_linear_cut_row(
    model: &mut LinearTriadModel,
    name: &str,
    coefficients: &[(usize, f64)],
    rhs: f64,
    negate: bool,
) -> Result<()> {
    let mut row = SparseVector::new();
    for (var_index, coefficient) in coefficients {
        if *var_index >= model.num_variables() {
            return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                "cut `{}` references invalid variable index {} (variables={})",
                name,
                var_index,
                model.num_variables()
            ))));
        }
        let mapped = if negate { -*coefficient } else { *coefficient };
        if mapped.abs() > f64::EPSILON {
            row.add(*var_index, mapped);
        }
    }
    let rhs = if negate { -rhs } else { rhs };
    model.basic.add_constraint_with_metadata(
        row,
        rhs,
        name.to_string(),
        None,
        false,
        0,
        None,
        None,
    );
    Ok(())
}

pub(crate) fn apply_linear_cuts(
    model: &LinearTriadModel,
    cuts: &[LinearCut],
) -> Result<LinearTriadModel> {
    if cuts.is_empty() {
        return Ok(model.clone());
    }

    let mut augmented = model.clone();
    for (index, cut) in cuts.iter().enumerate() {
        let base_name = if cut.name.is_empty() {
            format!("benders_cut_{}", index)
        } else {
            cut.name.clone()
        };
        match cut.sense {
            CutSense::LessOrEqual => append_linear_cut_row(
                &mut augmented,
                &base_name,
                &cut.coefficients,
                cut.rhs,
                false,
            )?,
            CutSense::GreaterOrEqual => {
                append_linear_cut_row(&mut augmented, &base_name, &cut.coefficients, cut.rhs, true)?
            }
            CutSense::Equal => {
                append_linear_cut_row(
                    &mut augmented,
                    &format!("{}_eq_le", base_name),
                    &cut.coefficients,
                    cut.rhs,
                    false,
                )?;
                append_linear_cut_row(
                    &mut augmented,
                    &format!("{}_eq_ge", base_name),
                    &cut.coefficients,
                    cut.rhs,
                    true,
                )?;
            }
        }
    }
    Ok(augmented)
}

fn map_cut_sense_to_relation(sense: CutSense) -> ConstraintRelation {
    match sense {
        CutSense::LessOrEqual => ConstraintRelation::LessEqual,
        CutSense::GreaterOrEqual => ConstraintRelation::GreaterEqual,
        CutSense::Equal => ConstraintRelation::Equal,
    }
}

fn validate_cut_variable_indexes(
    dimension: usize,
    coefficients: &[(usize, f64)],
    name: &str,
) -> Result<()> {
    if let Some((invalid_index, _)) = coefficients.iter().find(|(index, _)| *index >= dimension) {
        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "cut `{}` references invalid variable index {} (variables={})",
            name, invalid_index, dimension
        ))));
    }
    Ok(())
}

fn is_pure_linear_quadratic_model(model: &QuadraticTetradModel) -> bool {
    let has_quadratic_objective = model.Q.rows.iter().any(|row| {
        row.entries
            .iter()
            .any(|(_, value)| value.abs() > f64::EPSILON)
    });
    !has_quadratic_objective && model.quadratic_constraints.is_empty()
}

fn quadratic_model_to_linear_surrogate(model: &QuadraticTetradModel) -> Option<LinearTriadModel> {
    if !is_pure_linear_quadratic_model(model) {
        return None;
    }
    Some(LinearTriadModel {
        basic: model.basic.linear.clone(),
        c: model.c.clone(),
        objective_category: model.objective_category,
    })
}

pub(crate) fn apply_cuts_to_quadratic_model(
    model: &QuadraticTetradModel,
    linear_cuts: &[LinearCut],
    quadratic_cuts: &[QuadraticCut],
) -> Result<QuadraticTetradModel> {
    let mut augmented = model.clone();
    if !linear_cuts.is_empty() {
        let linear_view = LinearTriadModel {
            basic: augmented.basic.linear.clone(),
            c: augmented.c.clone(),
            objective_category: augmented.objective_category,
        };
        let linear_with_cuts = apply_linear_cuts(&linear_view, linear_cuts)?;
        augmented.basic.linear = linear_with_cuts.basic;
        augmented.c = linear_with_cuts.c;
        augmented.objective_category = linear_with_cuts.objective_category;
    }

    for (index, cut) in quadratic_cuts.iter().enumerate() {
        let base_name = if cut.linear.name.is_empty() {
            format!("quadratic_cut_{}", index)
        } else {
            cut.linear.name.clone()
        };
        validate_cut_variable_indexes(
            augmented.num_variables(),
            &cut.linear.coefficients,
            &base_name,
        )?;
        if let Some(((invalid_i, invalid_j), _)) = cut
            .quadratic_coefficients
            .iter()
            .find(|((i, j), _)| *i >= augmented.num_variables() || *j >= augmented.num_variables())
        {
            return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                "quadratic cut `{}` references invalid variable pair ({}, {}) with variables={}",
                base_name,
                invalid_i,
                invalid_j,
                augmented.num_variables()
            ))));
        }

        let mut monomials = Vec::new();
        for (var_index, coefficient) in &cut.linear.coefficients {
            if coefficient.abs() > f64::EPSILON {
                monomials.push(QuadraticMonomial::new_linear(*coefficient, *var_index));
            }
        }
        for ((var_index1, var_index2), coefficient) in &cut.quadratic_coefficients {
            if coefficient.abs() > f64::EPSILON {
                monomials.push(QuadraticMonomial::new_quadratic(
                    *coefficient,
                    *var_index1,
                    *var_index2,
                ));
            }
        }

        let inequality = ospf_rust_core::model::mechanism::QuadraticInequality::new(
            Quadratic::new(monomials, 0.0),
            map_cut_sense_to_relation(cut.linear.sense),
            cut.linear.rhs,
        );
        augmented.add_quadratic_constraint_with_metadata(
            inequality, base_name, None, false, 0, None, None,
        );
    }

    Ok(augmented)
}

fn fixed_variables_by_id_for_quadratic(
    model: &QuadraticTetradModel,
    master_solution: &[f64],
    preferred_ids: &[VariableId],
) -> HashMap<VariableId, f64> {
    let mut fixed = HashMap::new();
    if !preferred_ids.is_empty() {
        for (var_id, value) in preferred_ids
            .iter()
            .copied()
            .zip(master_solution.iter().copied())
        {
            fixed.insert(var_id, value);
        }
        return fixed;
    }

    for (token, value) in model
        .basic
        .linear
        .variables
        .iter()
        .zip(master_solution.iter().copied())
    {
        fixed.insert(token.id(), value);
    }
    fixed
}

pub(crate) fn fixed_variables_by_id(
    model: &LinearTriadModel,
    master_solution: &[f64],
    preferred_ids: &[VariableId],
) -> HashMap<VariableId, f64> {
    let mut fixed = HashMap::new();
    if !preferred_ids.is_empty() {
        for (var_id, value) in preferred_ids
            .iter()
            .copied()
            .zip(master_solution.iter().copied())
        {
            fixed.insert(var_id, value);
        }
        return fixed;
    }

    for (token, value) in model.variables.iter().zip(master_solution.iter().copied()) {
        fixed.insert(token.id(), value);
    }
    fixed
}

fn fix_linear_subproblem_variables(
    model: &mut LinearTriadModel,
    master_solution: &[f64],
    preferred_ids: &[VariableId],
) {
    if preferred_ids.is_empty() {
        for (index, value) in master_solution.iter().copied().enumerate() {
            if index >= model.num_variables() {
                break;
            }
            model.lb[index] = value;
            model.ub[index] = value;
        }
        return;
    }

    for (variable_id, value) in preferred_ids
        .iter()
        .copied()
        .zip(master_solution.iter().copied())
    {
        if let Some(index) = model.find_variable_index(variable_id) {
            model.lb[index] = value;
            model.ub[index] = value;
        }
    }
}

fn fix_quadratic_subproblem_variables(
    model: &mut QuadraticTetradModel,
    master_solution: &[f64],
    preferred_ids: &[VariableId],
) {
    if preferred_ids.is_empty() {
        for (index, value) in master_solution.iter().copied().enumerate() {
            if index >= model.num_variables() {
                break;
            }
            model.basic.linear.lb[index] = value;
            model.basic.linear.ub[index] = value;
        }
        return;
    }

    for (variable_id, value) in preferred_ids
        .iter()
        .copied()
        .zip(master_solution.iter().copied())
    {
        if let Some(index) = model.basic.linear.find_variable_index(variable_id) {
            model.basic.linear.lb[index] = value;
            model.basic.linear.ub[index] = value;
        }
    }
}

pub(crate) fn linear_inequality_to_cut(
    inequality: &LinearInequality<f64>,
    name: String,
) -> LinearCut {
    let coefficients = inequality
        .polynomial
        .monomials()
        .iter()
        .map(|monomial| (monomial.var_index(), *monomial.coefficient()))
        .collect();
    let rhs = inequality.rhs - *inequality.polynomial.constant_term();
    let sense = match inequality.relation {
        ConstraintRelation::LessEqual => CutSense::LessOrEqual,
        ConstraintRelation::Equal => CutSense::Equal,
        ConstraintRelation::GreaterEqual => CutSense::GreaterOrEqual,
    };
    LinearCut::new(name, coefficients, rhs, sense)
}

fn promote_linear_cuts_to_quadratic(
    cuts: &[LinearCut],
    fallback_prefix: &str,
) -> Vec<QuadraticCut> {
    cuts.iter()
        .enumerate()
        .map(|(index, cut)| {
            let name = if cut.name.is_empty() {
                format!("{}_{}", fallback_prefix, index)
            } else {
                cut.name.clone()
            };
            QuadraticCut::new(
                name,
                cut.coefficients.clone(),
                Vec::new(),
                cut.rhs,
                cut.sense,
            )
        })
        .collect()
}

/// 基于 core `Solver` 的列生成适配器 / Column-generation adapter backed by core `Solver`
#[derive(Debug)]
pub(crate) struct CoreColumnGenerationAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    name: String,
    solver: S,
}

impl<S> CoreColumnGenerationAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    pub(crate) fn new(name: impl Into<String>, solver: S) -> Self {
        Self {
            name: name.into(),
            solver,
        }
    }

    pub(crate) fn solver(&self) -> &S {
        &self.solver
    }

    pub(crate) fn solver_mut(&mut self) -> &mut S {
        &mut self.solver
    }

    fn solve_milp_impl(
        &self,
        model: &LinearTriadModel,
        options: &super::FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        emit_registration_status(
            &self.name,
            model,
            options.registration_status_callback.clone(),
        )?;
        export_lp_model_if_requested(model, &self.name, "milp", options)?;
        let core_callback = build_core_solving_callback(options.solving_status_callback.clone(), 0);
        let core_options = options.to_core_solve_options(core_callback.as_ref());
        let output = self
            .solver
            .solve_linear_with_options(model, &core_options)?;
        solver_output_to_feasible(&output)
    }

    fn solve_milp_typed_impl<V>(
        &self,
        model: &LinearTriadModel,
        options: &super::FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        emit_registration_status(
            &self.name,
            model,
            options.registration_status_callback.clone(),
        )?;
        export_lp_model_if_requested(model, &self.name, "milp", options)?;
        let core_callback = build_core_solving_callback(options.solving_status_callback.clone(), 0);
        let core_options = options.to_core_solve_options(core_callback.as_ref());
        let output = self
            .solver
            .solve_linear_with_options(model, &core_options)?;
        let typed_output = output.try_into_feasible_typed::<V>(options.value_conversion_policy)?;
        FeasibleSolutionV::try_from_typed_output(typed_output)
    }

    fn solve_lp_impl(
        &self,
        model: &LinearTriadModel,
        options: &super::FrameworkSolveOptions,
    ) -> Result<LPResult> {
        emit_registration_status(
            &self.name,
            model,
            options.registration_status_callback.clone(),
        )?;
        let mut lp_model = model.clone();
        lp_model.linear_relax();
        export_lp_model_if_requested(&lp_model, &self.name, "lp_relaxed", options)?;

        let core_callback = build_core_solving_callback(options.solving_status_callback.clone(), 0);
        let core_options = options.to_core_solve_options(core_callback.as_ref());
        let output = self
            .solver
            .solve_linear_with_options(&lp_model, &core_options)?;
        let feasible = solver_output_to_feasible(&output)?;
        let dual = resolve_lp_dual_solution(&self.solver, &lp_model, &output)?;

        Ok(LPResult::new(
            feasible,
            LinearDualSolution::new(dual, Vec::new()),
        ))
    }

    fn solve_lp_typed_impl<V>(
        &self,
        model: &LinearTriadModel,
        options: &super::FrameworkSolveOptions,
    ) -> Result<LPResultV<V>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        emit_registration_status(
            &self.name,
            model,
            options.registration_status_callback.clone(),
        )?;
        let mut lp_model = model.clone();
        lp_model.linear_relax();
        export_lp_model_if_requested(&lp_model, &self.name, "lp_relaxed", options)?;

        let core_callback = build_core_solving_callback(options.solving_status_callback.clone(), 0);
        let core_options = options.to_core_solve_options(core_callback.as_ref());
        let output = self
            .solver
            .solve_linear_with_options(&lp_model, &core_options)?;
        let typed_output = output
            .clone()
            .try_into_feasible_typed::<V>(options.value_conversion_policy)?;
        let feasible = FeasibleSolutionV::try_from_typed_output(typed_output)?;
        let dual = resolve_lp_dual_solution(&self.solver, &lp_model, &output)?;
        let dual_solution = LinearDualSolution::new(dual, Vec::new())
            .try_into_typed::<V>(options.value_conversion_policy)?;

        Ok(LPResultV {
            result: feasible,
            dual_solution,
        })
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<S> ColumnGenerationSolver for CoreColumnGenerationAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_milp_with_options(
        &self,
        model: &LinearTriadModel,
        options: super::FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_milp_impl(model, &options)
    }

    async fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: super::FrameworkSolveOptions,
    ) -> Result<LPResult> {
        self.solve_lp_impl(model, &options)
    }

    fn solve_lp_typed_with_options<'a, V>(
        &'a self,
        model: &'a LinearTriadModel,
        options: super::FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LPResultV<V>>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue,
    {
        Box::pin(async move { self.solve_lp_typed_impl(model, &options) })
    }

    fn solve_typed_with_options<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
        options: super::FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let mechanism_model = match meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let triad_model = match mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let options = if options.name.is_some() {
            options
        } else {
            options.with_name(self.name())
        };
        Box::pin(async move { self.solve_milp_typed_impl(&triad_model, &options) })
    }
}

#[cfg(not(feature = "async"))]
impl<S> ColumnGenerationSolver for CoreColumnGenerationAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_milp_with_options(
        &self,
        model: &LinearTriadModel,
        options: super::FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_milp_impl(model, &options)
    }

    fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: super::FrameworkSolveOptions,
    ) -> Result<LPResult> {
        self.solve_lp_impl(model, &options)
    }

    fn solve_lp_typed_with_options<V>(
        &self,
        model: &LinearTriadModel,
        options: super::FrameworkSolveOptions,
    ) -> Result<LPResultV<V>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_lp_typed_impl(model, &options)
    }

    fn solve_typed_with_options<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
        options: super::FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let mechanism_model = meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
        )?;
        let triad_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let options = if options.name.is_some() {
            options
        } else {
            options.with_name(self.name())
        };
        self.solve_milp_typed_impl(&triad_model, &options)
    }
}

/// 基于 core `Solver` 的线性 Benders 适配器 / Linear Benders adapter backed by core `Solver`
#[derive(Debug)]
pub(crate) struct CoreLinearBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    name: String,
    solver: S,
    cut_context: Option<BendersCutContext>,
}

impl<S> CoreLinearBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    pub(crate) fn new(name: impl Into<String>, solver: S) -> Self {
        Self {
            name: name.into(),
            solver,
            cut_context: None,
        }
    }

    pub(crate) fn with_cut_context(mut self, context: BendersCutContext) -> Self {
        self.cut_context = Some(context);
        self
    }

    pub(crate) fn solver(&self) -> &S {
        &self.solver
    }

    pub(crate) fn solver_mut(&mut self) -> &mut S {
        &mut self.solver
    }

    fn solve_master_impl(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolverOutput> {
        let augmented_model = apply_linear_cuts(model, cuts)?;
        self.solver.solve_linear(&augmented_model)
    }

    fn build_subproblem_model(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> LinearTriadModel {
        let mut fixed_model = model.clone();
        let fixed_variable_ids = self
            .cut_context
            .as_ref()
            .map(|context| context.fixed_variable_ids.as_slice())
            .unwrap_or_default();
        fix_linear_subproblem_variables(&mut fixed_model, master_solution, fixed_variable_ids);
        fixed_model.linear_relax();
        fixed_model
    }

    fn solve_sub_impl(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult> {
        let fixed_subproblem = self.build_subproblem_model(model, master_solution);
        let sub_output = self.solver.solve_linear(&fixed_subproblem)?;

        if sub_output.status.is_feasible() {
            let feasible = solver_output_to_feasible(&sub_output)?;
            let dual_solution =
                resolve_lp_dual_solution(&self.solver, &fixed_subproblem, &sub_output)?;
            let mut result = LinearFeasibleResult::new(
                feasible,
                LinearDualSolution::new(dual_solution.clone(), Vec::new()),
            );

            if let Some(context) = &self.cut_context {
                if let Some(objective_variable) = context.objective_variable {
                    let fixed_by_id =
                        fixed_variables_by_id(model, master_solution, &context.fixed_variable_ids);
                    let cuts = context
                        .mechanism_model
                        .generate_optimal_cuts_from_dual_solution(
                            objective_variable,
                            &fixed_by_id,
                            &dual_solution,
                        )?;
                    let mapped = cuts
                        .iter()
                        .enumerate()
                        .map(|(index, cut)| {
                            linear_inequality_to_cut(cut, format!("optimal_cut_{}", index))
                        })
                        .collect();
                    result = result.with_cuts(mapped);
                }
            }

            return Ok(LinearSubResult::Feasible(result));
        }

        if sub_output.status.is_infeasible() {
            let farkas_solution =
                resolve_farkas_dual_solution(&self.solver, &fixed_subproblem, &sub_output)?;
            let mut result = LinearInfeasibleResult::new(LinearDualSolution::new(
                farkas_solution.clone(),
                Vec::new(),
            ));

            if let Some(context) = &self.cut_context {
                let fixed_by_id =
                    fixed_variables_by_id(model, master_solution, &context.fixed_variable_ids);
                let cuts = context
                    .mechanism_model
                    .generate_feasibility_cuts_from_farkas_solution(
                        &fixed_by_id,
                        &farkas_solution,
                    )?;
                let mapped = cuts
                    .iter()
                    .enumerate()
                    .map(|(index, cut)| {
                        linear_inequality_to_cut(cut, format!("feasibility_cut_{}", index))
                    })
                    .collect();
                result = result.with_cuts(mapped);
            }

            return Ok(LinearSubResult::Infeasible(result));
        }

        if sub_output.status.is_unbounded() {
            return Err(CoreError::Solver(SolverError::Unbounded));
        }

        Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "sub-problem solve failed with status {:?}",
            sub_output.status
        ))))
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<S> LinearBendersDecompositionSolver for CoreLinearBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_master(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolverOutput> {
        self.solve_master_impl(model, cuts)
    }

    async fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult> {
        self.solve_sub_impl(model, master_solution)
    }

    fn solve_sub_typed<'a, V>(
        &'a self,
        model: &'a LinearTriadModel,
        master_solution: &'a [f64],
        policy: ospf_rust_core::solver::SolveValueConversionPolicy,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LinearSubResultV<V>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        Box::pin(async move {
            self.solve_sub_impl(model, master_solution)?
                .try_into_typed(policy)
        })
    }
}

#[cfg(not(feature = "async"))]
impl<S> LinearBendersDecompositionSolver for CoreLinearBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_master(&self, model: &LinearTriadModel, cuts: &[LinearCut]) -> Result<SolverOutput> {
        self.solve_master_impl(model, cuts)
    }

    fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult> {
        self.solve_sub_impl(model, master_solution)
    }

    fn solve_sub_typed<V>(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
        policy: ospf_rust_core::solver::SolveValueConversionPolicy,
    ) -> Result<LinearSubResultV<V>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_sub_impl(model, master_solution)?
            .try_into_typed(policy)
    }
}

/// 基于 core `Solver` 的二次 Benders 适配器 / Quadratic Benders adapter backed by core `Solver`
#[derive(Debug)]
pub(crate) struct CoreQuadraticBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    name: String,
    solver: S,
    cut_context: Option<BendersCutContext>,
}

impl<S> CoreQuadraticBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    pub(crate) fn new(name: impl Into<String>, solver: S) -> Self {
        Self {
            name: name.into(),
            solver,
            cut_context: None,
        }
    }

    pub(crate) fn with_cut_context(mut self, context: BendersCutContext) -> Self {
        self.cut_context = Some(context);
        self
    }

    pub(crate) fn solver(&self) -> &S {
        &self.solver
    }

    pub(crate) fn solver_mut(&mut self) -> &mut S {
        &mut self.solver
    }

    fn solve_master_quadratic_impl(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput> {
        let augmented_model = apply_cuts_to_quadratic_model(model, linear_cuts, quadratic_cuts)?;
        self.solver.solve_quadratic(&augmented_model)
    }

    fn build_subproblem_model(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> QuadraticTetradModel {
        let mut fixed_model = model.clone();
        let fixed_variable_ids = self
            .cut_context
            .as_ref()
            .map(|context| context.fixed_variable_ids.as_slice())
            .unwrap_or_default();
        fix_quadratic_subproblem_variables(&mut fixed_model, master_solution, fixed_variable_ids);
        fixed_model.linear_relax();
        fixed_model
    }

    fn probe_quadratic_cut_capability(
        &self,
        model: &QuadraticTetradModel,
        has_dual_solution: bool,
        has_farkas_solution: bool,
    ) -> QuadraticCutCapabilityProbe {
        let has_cut_context = self.cut_context.is_some();
        let has_objective_variable = self
            .cut_context
            .as_ref()
            .and_then(|context| context.objective_variable)
            .is_some();
        QuadraticCutCapabilityProbe {
            true_quadratic_subproblem: !is_pure_linear_quadratic_model(model),
            has_cut_context,
            has_objective_variable,
            has_dual_solution,
            has_farkas_solution,
        }
    }

    fn solve_sub_quadratic_impl(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult> {
        let fixed_subproblem = self.build_subproblem_model(model, master_solution);
        let sub_output = self.solver.solve_quadratic(&fixed_subproblem)?;

        if sub_output.status.is_feasible() {
            let feasible = solver_output_to_feasible(&sub_output)?;
            let mut dual_solution = sub_output.dual_solution.clone().unwrap_or_default();
            if dual_solution.is_empty() {
                if let Some(linear_surrogate) =
                    quadratic_model_to_linear_surrogate(&fixed_subproblem)
                {
                    let linear_output = self.solver.solve_linear(&linear_surrogate)?;
                    dual_solution =
                        resolve_lp_dual_solution(&self.solver, &linear_surrogate, &linear_output)?;
                }
            }

            let cut_capability = self.probe_quadratic_cut_capability(
                &fixed_subproblem,
                !dual_solution.is_empty(),
                false,
            );
            if cut_capability.has_cut_context
                && cut_capability.has_objective_variable
                && !cut_capability.can_generate_optimal_cut()
            {
                log::warn!(
                    "quadratic sub-problem cannot generate optimal cuts in this iteration: {:?}",
                    cut_capability
                );
            }

            let mut linear_result = LinearFeasibleResult::new(
                feasible,
                LinearDualSolution::new(dual_solution.clone(), Vec::new()),
            );
            let mut generated_linear_cuts = Vec::new();
            if let Some(context) = &self.cut_context {
                if let Some(objective_variable) = context.objective_variable {
                    if !dual_solution.is_empty() {
                        let fixed_by_id = fixed_variables_by_id_for_quadratic(
                            model,
                            master_solution,
                            &context.fixed_variable_ids,
                        );
                        let cuts = context
                            .mechanism_model
                            .generate_optimal_cuts_from_dual_solution(
                                objective_variable,
                                &fixed_by_id,
                                &dual_solution,
                            )?;
                        let mapped = cuts
                            .iter()
                            .enumerate()
                            .map(|(index, cut)| {
                                linear_inequality_to_cut(cut, format!("optimal_cut_{}", index))
                            })
                            .collect();
                        generated_linear_cuts = mapped;
                    }
                }
            }
            let mut quadratic_cuts = Vec::new();
            if !generated_linear_cuts.is_empty() {
                if cut_capability.true_quadratic_subproblem {
                    quadratic_cuts = promote_linear_cuts_to_quadratic(
                        &generated_linear_cuts,
                        "optimal_quadratic_cut",
                    );
                } else {
                    linear_result = linear_result.with_cuts(generated_linear_cuts);
                }
            }

            let mut quadratic_result = QuadraticFeasibleResult::new(linear_result);
            if !quadratic_cuts.is_empty() {
                quadratic_result = quadratic_result.with_quadratic_cuts(quadratic_cuts);
            }
            return Ok(QuadraticSubResult::Feasible(quadratic_result));
        }

        if sub_output.status.is_infeasible() {
            let mut farkas_solution = sub_output.dual_solution.clone().unwrap_or_default();
            if farkas_solution.is_empty() {
                if let Some(linear_surrogate) =
                    quadratic_model_to_linear_surrogate(&fixed_subproblem)
                {
                    farkas_solution =
                        resolve_farkas_dual_solution(&self.solver, &linear_surrogate, &sub_output)?;
                }
            }

            let cut_capability = self.probe_quadratic_cut_capability(
                &fixed_subproblem,
                false,
                !farkas_solution.is_empty(),
            );
            if cut_capability.has_cut_context && !cut_capability.can_generate_feasibility_cut() {
                log::warn!(
                    "quadratic sub-problem cannot generate feasibility cuts in this iteration: {:?}",
                    cut_capability
                );
            }

            let mut linear_result = LinearInfeasibleResult::new(LinearDualSolution::new(
                farkas_solution.clone(),
                Vec::new(),
            ));
            let mut generated_linear_cuts = Vec::new();
            if let Some(context) = &self.cut_context {
                if !farkas_solution.is_empty() {
                    let fixed_by_id = fixed_variables_by_id_for_quadratic(
                        model,
                        master_solution,
                        &context.fixed_variable_ids,
                    );
                    let cuts = context
                        .mechanism_model
                        .generate_feasibility_cuts_from_farkas_solution(
                            &fixed_by_id,
                            &farkas_solution,
                        )?;
                    let mapped = cuts
                        .iter()
                        .enumerate()
                        .map(|(index, cut)| {
                            linear_inequality_to_cut(cut, format!("feasibility_cut_{}", index))
                        })
                        .collect();
                    generated_linear_cuts = mapped;
                }
            }
            let mut quadratic_cuts = Vec::new();
            if !generated_linear_cuts.is_empty() {
                if cut_capability.true_quadratic_subproblem {
                    quadratic_cuts = promote_linear_cuts_to_quadratic(
                        &generated_linear_cuts,
                        "feasibility_quadratic_cut",
                    );
                } else {
                    linear_result = linear_result.with_cuts(generated_linear_cuts);
                }
            }

            let mut quadratic_result = QuadraticInfeasibleResult::new(linear_result);
            if !quadratic_cuts.is_empty() {
                quadratic_result = quadratic_result.with_quadratic_cuts(quadratic_cuts);
            }
            return Ok(QuadraticSubResult::Infeasible(quadratic_result));
        }

        if sub_output.status.is_unbounded() {
            return Err(CoreError::Solver(SolverError::Unbounded));
        }

        Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "quadratic sub-problem solve failed with status {:?}",
            sub_output.status
        ))))
    }

    fn solve_master_linear_impl(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolverOutput> {
        let augmented_model = apply_linear_cuts(model, cuts)?;
        self.solver.solve_linear(&augmented_model)
    }

    fn solve_sub_linear_impl(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult> {
        let mut fixed_subproblem = model.clone();
        let fixed_variable_ids = self
            .cut_context
            .as_ref()
            .map(|context| context.fixed_variable_ids.as_slice())
            .unwrap_or_default();
        fix_linear_subproblem_variables(
            &mut fixed_subproblem,
            master_solution,
            fixed_variable_ids,
        );
        fixed_subproblem.linear_relax();

        let sub_output = self.solver.solve_linear(&fixed_subproblem)?;
        if sub_output.status.is_feasible() {
            let feasible = solver_output_to_feasible(&sub_output)?;
            let dual_solution =
                resolve_lp_dual_solution(&self.solver, &fixed_subproblem, &sub_output)?;
            let mut result = LinearFeasibleResult::new(
                feasible,
                LinearDualSolution::new(dual_solution.clone(), Vec::new()),
            );
            if let Some(context) = &self.cut_context {
                if let Some(objective_variable) = context.objective_variable {
                    let fixed_by_id =
                        fixed_variables_by_id(model, master_solution, &context.fixed_variable_ids);
                    let cuts = context
                        .mechanism_model
                        .generate_optimal_cuts_from_dual_solution(
                            objective_variable,
                            &fixed_by_id,
                            &dual_solution,
                        )?;
                    let mapped = cuts
                        .iter()
                        .enumerate()
                        .map(|(index, cut)| {
                            linear_inequality_to_cut(cut, format!("optimal_cut_{}", index))
                        })
                        .collect();
                    result = result.with_cuts(mapped);
                }
            }
            return Ok(LinearSubResult::Feasible(result));
        }

        if sub_output.status.is_infeasible() {
            let farkas_solution =
                resolve_farkas_dual_solution(&self.solver, &fixed_subproblem, &sub_output)?;
            let mut result = LinearInfeasibleResult::new(LinearDualSolution::new(
                farkas_solution.clone(),
                Vec::new(),
            ));

            if let Some(context) = &self.cut_context {
                let fixed_by_id =
                    fixed_variables_by_id(model, master_solution, &context.fixed_variable_ids);
                let cuts = context
                    .mechanism_model
                    .generate_feasibility_cuts_from_farkas_solution(
                        &fixed_by_id,
                        &farkas_solution,
                    )?;
                let mapped = cuts
                    .iter()
                    .enumerate()
                    .map(|(index, cut)| {
                        linear_inequality_to_cut(cut, format!("feasibility_cut_{}", index))
                    })
                    .collect();
                result = result.with_cuts(mapped);
            }
            return Ok(LinearSubResult::Infeasible(result));
        }

        if sub_output.status.is_unbounded() {
            return Err(CoreError::Solver(SolverError::Unbounded));
        }

        Err(CoreError::Solver(SolverError::SolveFailed(format!(
            "sub-problem solve failed with status {:?}",
            sub_output.status
        ))))
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<S> LinearBendersDecompositionSolver for CoreQuadraticBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_master(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolverOutput> {
        self.solve_master_linear_impl(model, cuts)
    }

    async fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult> {
        self.solve_sub_linear_impl(model, master_solution)
    }

    fn solve_sub_typed<'a, V>(
        &'a self,
        model: &'a LinearTriadModel,
        master_solution: &'a [f64],
        policy: ospf_rust_core::solver::SolveValueConversionPolicy,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LinearSubResultV<V>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        Box::pin(async move {
            self.solve_sub_linear_impl(model, master_solution)?
                .try_into_typed(policy)
        })
    }
}

#[cfg(not(feature = "async"))]
impl<S> LinearBendersDecompositionSolver for CoreQuadraticBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_master(&self, model: &LinearTriadModel, cuts: &[LinearCut]) -> Result<SolverOutput> {
        self.solve_master_linear_impl(model, cuts)
    }

    fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult> {
        self.solve_sub_linear_impl(model, master_solution)
    }

    fn solve_sub_typed<V>(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
        policy: ospf_rust_core::solver::SolveValueConversionPolicy,
    ) -> Result<LinearSubResultV<V>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_sub_linear_impl(model, master_solution)?
            .try_into_typed(policy)
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<S> QuadraticBendersDecompositionSolver for CoreQuadraticBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    async fn solve_master_quadratic(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput> {
        self.solve_master_quadratic_impl(model, linear_cuts, quadratic_cuts)
    }

    async fn solve_sub_quadratic(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult> {
        self.solve_sub_quadratic_impl(model, master_solution)
    }

    fn solve_sub_quadratic_typed<'a, V>(
        &'a self,
        model: &'a QuadraticTetradModel,
        master_solution: &'a [f64],
        policy: ospf_rust_core::solver::SolveValueConversionPolicy,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<QuadraticSubResultV<V>>> + Send + 'a>,
    >
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        Box::pin(async move {
            self.solve_sub_quadratic_impl(model, master_solution)?
                .try_into_typed(policy)
        })
    }
}

#[cfg(not(feature = "async"))]
impl<S> QuadraticBendersDecompositionSolver for CoreQuadraticBendersAdapter<S>
where
    S: Solver + SolverExt + Send + Sync,
{
    fn solve_master_quadratic(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput> {
        self.solve_master_quadratic_impl(model, linear_cuts, quadratic_cuts)
    }

    fn solve_sub_quadratic(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult> {
        self.solve_sub_quadratic_impl(model, master_solution)
    }

    fn solve_sub_quadratic_typed<V>(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
        policy: ospf_rust_core::solver::SolveValueConversionPolicy,
    ) -> Result<QuadraticSubResultV<V>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_sub_quadratic_impl(model, master_solution)?
            .try_into_typed(policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use ospf_rust_core::model::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, SparseMatrix, SparseVector,
    };
    use ospf_rust_core::model::{
        BasicMechanismModel, Linear, LinearConstraint, LinearInequality, LinearMonomial,
        ObjectiveCategory,
    };
    use ospf_rust_core::solver::{
        LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverStatus,
    };
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::{ContinuousVariableItem, UContinuousVariableItem};

    #[derive(Debug)]
    struct MockCoreColumnGenerationSolver;

    #[test]
    fn linear_subproblem_fixes_only_declared_shared_variables_by_id() {
        let local = ContinuousVariableItem::auto("local");
        let shared = ContinuousVariableItem::auto("shared");
        let shared_id = shared.id();
        let mut basic = BasicLinearTriadModel::new("linear_fixed_variables");
        basic.add_variable(Token::from_generic(local, 0));
        basic.add_variable(Token::from_generic(shared, 1));
        let mut model = LinearTriadModel::from_basic(basic);

        fix_linear_subproblem_variables(&mut model, &[3.0, 99.0], &[shared_id]);

        assert!(model.lb[0].is_infinite() && model.lb[0].is_sign_negative());
        assert!(model.ub[0].is_infinite() && model.ub[0].is_sign_positive());
        assert_eq!(model.lb[1], 3.0);
        assert_eq!(model.ub[1], 3.0);
    }

    #[test]
    fn quadratic_subproblem_fixes_only_declared_shared_variables_by_id() {
        let local = ContinuousVariableItem::auto("local");
        let shared = ContinuousVariableItem::auto("shared");
        let shared_id = shared.id();
        let mut linear = BasicLinearTriadModel::new("quadratic_fixed_variables");
        linear.add_variable(Token::from_generic(local, 0));
        linear.add_variable(Token::from_generic(shared, 1));
        let basic = BasicQuadraticTetradModel::from_linear(linear);
        let mut model = QuadraticTetradModel::from_basic(basic);

        fix_quadratic_subproblem_variables(&mut model, &[4.0, 99.0], &[shared_id]);

        assert!(
            model.basic.linear.lb[0].is_infinite()
                && model.basic.linear.lb[0].is_sign_negative()
        );
        assert!(
            model.basic.linear.ub[0].is_infinite()
                && model.basic.linear.ub[0].is_sign_positive()
        );
        assert_eq!(model.basic.linear.lb[1], 4.0);
        assert_eq!(model.basic.linear.ub[1], 4.0);
    }

    #[test]
    fn solution_vector_from_output_returns_solution_for_feasible_output() {
        let output = SolverOutput::optimal(2.0, vec![1.0, 3.0]);
        let solution =
            solution_vector_from_output(&output, "core_extensions_helper").expect("must map");
        assert_eq!(solution, vec![1.0, 3.0]);
    }

    #[test]
    fn solution_vector_from_output_reports_missing_solution_context() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_objective(2.0);
        let err = solution_vector_from_output(&output, "core_extensions_helper")
            .expect_err("missing solution vector should fail");
        let message = err.to_string();
        assert!(message.contains("core_extensions_helper missing solution vector"));
    }

    #[test]
    fn solver_output_to_feasible_maps_feasible_output() {
        let output = SolverOutput::optimal(2.0, vec![1.0, 3.0]);
        let feasible =
            solver_output_to_feasible(&output).expect("feasible solver output should map");
        assert_eq!(feasible.obj, 2.0);
        assert_eq!(feasible.solution, vec![1.0, 3.0]);
    }

    #[test]
    fn solver_output_to_feasible_reports_missing_objective_for_feasible_status() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_solution(vec![1.0]);
        let err = solver_output_to_feasible(&output)
            .expect_err("feasible status without objective should fail");
        let message = err.to_string();
        assert!(message.contains("missing objective value"));
    }

    #[test]
    fn solver_output_to_feasible_reports_missing_solution_for_feasible_status() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_objective(1.0);
        let err = solver_output_to_feasible(&output)
            .expect_err("feasible status without solution should fail");
        let message = err.to_string();
        assert!(message.contains("missing solution vector"));
    }

    #[test]
    fn solver_output_to_feasible_maps_infeasible_status_to_infeasible_error() {
        let output = SolverOutput::new(SolverStatus::Infeasible);
        let err = solver_output_to_feasible(&output)
            .expect_err("infeasible status should map to infeasible error");
        assert!(matches!(err, CoreError::Solver(SolverError::Infeasible)));
    }

    #[test]
    fn solver_output_to_feasible_maps_unbounded_status_to_unbounded_error() {
        let output = SolverOutput::new(SolverStatus::Unbounded);
        let err = solver_output_to_feasible(&output)
            .expect_err("unbounded status should map to unbounded error");
        assert!(matches!(err, CoreError::Solver(SolverError::Unbounded)));
    }

    #[test]
    fn solver_output_to_feasible_preserves_time_limit_diagnostic_message() {
        let output = SolverOutput::new(SolverStatus::TimeLimit);
        let err = solver_output_to_feasible(&output)
            .expect_err("time limit without feasible solution should fail");
        let message = err.to_string();
        assert!(message.contains("time limit"));
    }

    impl SolverInfo for MockCoreColumnGenerationSolver {
        fn name(&self) -> &str {
            "mock_core_column_generation_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for MockCoreColumnGenerationSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![2.0]))
        }
    }

    impl QuadraticSolver for MockCoreColumnGenerationSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![2.0]))
        }
    }

    fn build_single_row_lp_model() -> LinearTriadModel {
        let mut basic = BasicLinearTriadModel::new("single_row_lp");
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            ospf_rust_core::variable::VariableType::UContinuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        basic.add_constraint(row, 2.0);
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
        model
    }

    #[derive(Debug)]
    struct DualFallbackSolver;

    impl SolverInfo for DualFallbackSolver {
        fn name(&self) -> &str {
            "dual_fallback_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for DualFallbackSolver {
        fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
            if model.basic.name.ends_with("_farkas_dual") {
                return Ok(SolverOutput::optimal(0.0, vec![0.5]));
            }
            if model.basic.name.ends_with("_dual") {
                return Ok(SolverOutput::optimal(2.0, vec![1.0]));
            }
            Ok(SolverOutput::optimal(2.0, vec![2.0]))
        }
    }

    impl QuadraticSolver for DualFallbackSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![]))
        }
    }

    #[test]
    fn resolve_lp_dual_solution_keeps_consistent_native_dual() {
        let model = build_single_row_lp_model();
        let mut output = SolverOutput::optimal(2.0, vec![2.0]);
        output.dual_solution = Some(vec![1.0]);

        let dual = resolve_lp_dual_solution(&DualFallbackSolver, &model, &output)
            .expect("consistent native dual should be accepted");
        assert_eq!(dual, vec![1.0]);
    }

    #[test]
    fn resolve_lp_dual_solution_truncates_extra_native_entries() {
        let model = build_single_row_lp_model();
        let mut output = SolverOutput::optimal(2.0, vec![2.0]);
        output.dual_solution = Some(vec![1.0, 999.0]);

        let dual = resolve_lp_dual_solution(&DualFallbackSolver, &model, &output)
            .expect("consistent native dual should be normalized to row multipliers");
        assert_eq!(dual, vec![1.0]);
    }

    #[test]
    fn resolve_lp_dual_solution_falls_back_when_native_dual_objective_mismatches() {
        let model = build_single_row_lp_model();
        let mut output = SolverOutput::optimal(2.0, vec![2.0]);
        output.dual_solution = Some(vec![0.0]);

        let dual = resolve_lp_dual_solution(&DualFallbackSolver, &model, &output)
            .expect("inconsistent native dual should fall back to explicit dual model");
        assert_eq!(dual, vec![1.0]);
    }

    #[test]
    fn resolve_farkas_dual_solution_keeps_solver_provided_certificate() {
        let model = build_single_row_lp_model();
        let mut output = SolverOutput::new(SolverStatus::Infeasible);
        output.dual_solution = Some(vec![3.0]);

        let farkas = resolve_farkas_dual_solution(&DualFallbackSolver, &model, &output)
            .expect("solver-provided Farkas certificate should be accepted");
        assert_eq!(farkas, vec![3.0]);
    }

    #[test]
    fn resolve_farkas_dual_solution_truncates_extra_native_entries() {
        let model = build_single_row_lp_model();
        let mut output = SolverOutput::new(SolverStatus::Infeasible);
        output.dual_solution = Some(vec![3.0, 4.0]);

        let farkas = resolve_farkas_dual_solution(&DualFallbackSolver, &model, &output)
            .expect("solver-provided Farkas certificate should be normalized to row multipliers");
        assert_eq!(farkas, vec![3.0]);
    }

    #[test]
    fn resolve_farkas_dual_solution_falls_back_to_explicit_model_when_missing() {
        let model = build_single_row_lp_model();
        let output = SolverOutput::new(SolverStatus::Infeasible);

        let farkas = resolve_farkas_dual_solution(&DualFallbackSolver, &model, &output)
            .expect("missing Farkas certificate should fall back to explicit model");
        assert_eq!(farkas, vec![0.5]);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn core_column_generation_adapter_typed_meta_shortcut_returns_typed_output() {
        let mut meta_model = ospf_rust_core::model::MetaModel::<f64>::new("core_cg_typed_meta");
        let x = ContinuousVariableItem::auto("core_cg_typed_meta_x");
        let x_index = meta_model
            .register_variable(x)
            .expect("register variable should succeed");
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "core_cg_typed_meta_c",
            )
            .expect("add constraint should succeed");

        let adapter =
            CoreColumnGenerationAdapter::new("mock_core_cg", MockCoreColumnGenerationSolver);
        let output = adapter
            .solve_typed_with_options(
                &meta_model,
                crate::solver::FrameworkSolveOptions::new().with_value_conversion_policy(
                    ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
                ),
            )
            .expect("typed solve should succeed");
        assert!((output.obj - 1.0).abs() <= f64::EPSILON);
        assert_eq!(output.solution, vec![2.0]);
    }

    #[derive(Debug)]
    struct MockQuadraticDualSolver {
        quadratic_output: SolverOutput,
        linear_call_counter: Arc<AtomicUsize>,
    }

    impl SolverInfo for MockQuadraticDualSolver {
        fn name(&self) -> &str {
            "mock_quadratic_dual_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for MockQuadraticDualSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            self.linear_call_counter.fetch_add(1, Ordering::SeqCst);
            Err(CoreError::Solver(SolverError::SolveFailed(
                "linear fallback should not be called in this test".into(),
            )))
        }
    }

    impl QuadraticSolver for MockQuadraticDualSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(self.quadratic_output.clone())
        }
    }

    fn build_quadratic_subproblem_model() -> QuadraticTetradModel {
        let x = ContinuousVariableItem::auto("sub_x");
        let mut basic = BasicQuadraticTetradModel::new("quadratic_subproblem");
        basic.linear.add_variable(Token::from_generic(x, 0));
        let mut model = QuadraticTetradModel::from_basic(basic);

        let mut q = SparseMatrix::new();
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        q.add_row(row);
        model.set_objective(vec![0.0], q, ObjectiveCategory::Minimum);
        model
    }

    fn build_cut_context() -> BendersCutContext {
        let x = ContinuousVariableItem::auto("x");
        let y = ContinuousVariableItem::auto("y");
        let theta = ContinuousVariableItem::auto("theta");
        let x_id = x.id();
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("quadratic_cut_context");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));

        BendersCutContext {
            mechanism_model: MechanismModel::from_basic(basic),
            objective_variable: Some(theta_id),
            fixed_variable_ids: vec![x_id],
        }
    }

    #[test]
    fn to_log_model_exports_lp_file_for_core_adapter_path() {
        let model = LinearTriadModel::new("lp_export_smoke");
        let unique = format!(
            "lp_export_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        );
        let options = crate::solver::FrameworkSolveOptions::new()
            .with_name(unique.clone())
            .with_log_model(true);
        export_lp_model_if_requested(&model, "mock_solver", "milp", &options)
            .expect("lp export should succeed when to_log_model is enabled");

        let file_name = format!(
            "{}_{}_{}.lp",
            sanitize_file_stem(&unique),
            "mock_solver",
            "milp"
        );
        assert!(
            Path::new(&file_name).exists(),
            "expected exported LP file `{}`",
            file_name
        );
        let _ = fs::remove_file(&file_name);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn quadratic_feasible_uses_solver_dual_solution_without_linear_fallback() {
        let linear_calls = Arc::new(AtomicUsize::new(0));
        let solver = MockQuadraticDualSolver {
            quadratic_output: SolverOutput::optimal(3.0, vec![2.0]).with_dual(vec![-1.0]),
            linear_call_counter: linear_calls.clone(),
        };
        let adapter =
            CoreQuadraticBendersAdapter::new("mock", solver).with_cut_context(build_cut_context());
        let model = build_quadratic_subproblem_model();

        let result = adapter
            .solve_sub_quadratic(&model, &[2.0])
            .expect("quadratic sub-problem should succeed with solver-provided dual");

        match result {
            QuadraticSubResult::Feasible(feasible) => {
                assert_eq!(feasible.linear.dual_solution.constraints, vec![-1.0]);
                assert!(feasible.linear.cuts.as_ref().is_none_or(Vec::is_empty));
                assert!(
                    feasible
                        .quadratic_cuts
                        .as_ref()
                        .is_some_and(|cuts| !cuts.is_empty())
                );
                assert!(feasible.quadratic_cuts.as_ref().is_some_and(|cuts| {
                    cuts.iter().all(|cut| cut.quadratic_coefficients.is_empty())
                }));
            }
            _ => panic!("expected feasible quadratic sub-result"),
        }
        assert_eq!(linear_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn quadratic_feasible_uses_solver_dual_solution_without_linear_fallback() {
        let linear_calls = Arc::new(AtomicUsize::new(0));
        let solver = MockQuadraticDualSolver {
            quadratic_output: SolverOutput::optimal(3.0, vec![2.0]).with_dual(vec![-1.0]),
            linear_call_counter: linear_calls.clone(),
        };
        let adapter =
            CoreQuadraticBendersAdapter::new("mock", solver).with_cut_context(build_cut_context());
        let model = build_quadratic_subproblem_model();

        let result = adapter
            .solve_sub_quadratic(&model, &[2.0])
            .await
            .expect("quadratic sub-problem should succeed with solver-provided dual");

        match result {
            QuadraticSubResult::Feasible(feasible) => {
                assert_eq!(feasible.linear.dual_solution.constraints, vec![-1.0]);
                assert!(feasible.linear.cuts.as_ref().is_none_or(Vec::is_empty));
                assert!(
                    feasible
                        .quadratic_cuts
                        .as_ref()
                        .is_some_and(|cuts| !cuts.is_empty())
                );
                assert!(feasible.quadratic_cuts.as_ref().is_some_and(|cuts| {
                    cuts.iter().all(|cut| cut.quadratic_coefficients.is_empty())
                }));
            }
            _ => panic!("expected feasible quadratic sub-result"),
        }
        assert_eq!(linear_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn quadratic_infeasible_uses_solver_farkas_solution_without_linear_fallback() {
        let linear_calls = Arc::new(AtomicUsize::new(0));
        let solver = MockQuadraticDualSolver {
            quadratic_output: SolverOutput::new(SolverStatus::Infeasible).with_dual(vec![1.0]),
            linear_call_counter: linear_calls.clone(),
        };
        let adapter =
            CoreQuadraticBendersAdapter::new("mock", solver).with_cut_context(build_cut_context());
        let model = build_quadratic_subproblem_model();

        let result = adapter.solve_sub_quadratic(&model, &[2.0]).expect(
            "quadratic infeasible sub-problem should succeed with solver-provided farkas dual",
        );

        match result {
            QuadraticSubResult::Infeasible(infeasible) => {
                assert_eq!(
                    infeasible.linear.farkas_dual_solution.constraints,
                    vec![1.0]
                );
                assert!(infeasible.linear.cuts.as_ref().is_none_or(Vec::is_empty));
                assert!(
                    infeasible
                        .quadratic_cuts
                        .as_ref()
                        .is_some_and(|cuts| !cuts.is_empty())
                );
                assert!(infeasible.quadratic_cuts.as_ref().is_some_and(|cuts| {
                    cuts.iter().all(|cut| cut.quadratic_coefficients.is_empty())
                }));
            }
            _ => panic!("expected infeasible quadratic sub-result"),
        }
        assert_eq!(linear_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn quadratic_feasible_without_dual_on_true_quadratic_subproblem_returns_result_with_empty_cuts()
    {
        let linear_calls = Arc::new(AtomicUsize::new(0));
        let solver = MockQuadraticDualSolver {
            quadratic_output: SolverOutput::optimal(3.0, vec![2.0]),
            linear_call_counter: linear_calls.clone(),
        };
        let adapter =
            CoreQuadraticBendersAdapter::new("mock", solver).with_cut_context(build_cut_context());
        let model = build_quadratic_subproblem_model();

        let result = adapter.solve_sub_quadratic(&model, &[2.0]).expect(
            "quadratic sub-problem should still return feasible result without dual multipliers",
        );

        match result {
            QuadraticSubResult::Feasible(feasible) => {
                assert!(feasible.linear.dual_solution.constraints.is_empty());
                assert!(feasible.linear.cuts.as_ref().is_none_or(Vec::is_empty));
                assert!(feasible.quadratic_cuts.as_ref().is_none_or(Vec::is_empty));
            }
            _ => panic!("expected feasible quadratic sub-result"),
        }
        assert_eq!(linear_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn quadratic_infeasible_without_farkas_on_true_quadratic_subproblem_returns_result_with_empty_cuts()
     {
        let linear_calls = Arc::new(AtomicUsize::new(0));
        let solver = MockQuadraticDualSolver {
            quadratic_output: SolverOutput::new(SolverStatus::Infeasible),
            linear_call_counter: linear_calls.clone(),
        };
        let adapter =
            CoreQuadraticBendersAdapter::new("mock", solver).with_cut_context(build_cut_context());
        let model = build_quadratic_subproblem_model();

        let result = adapter
            .solve_sub_quadratic(&model, &[2.0])
            .expect("quadratic sub-problem should still return infeasible result without farkas multipliers");

        match result {
            QuadraticSubResult::Infeasible(infeasible) => {
                assert!(
                    infeasible
                        .linear
                        .farkas_dual_solution
                        .constraints
                        .is_empty()
                );
                assert!(infeasible.linear.cuts.as_ref().is_none_or(Vec::is_empty));
                assert!(infeasible.quadratic_cuts.as_ref().is_none_or(Vec::is_empty));
            }
            _ => panic!("expected infeasible quadratic sub-result"),
        }
        assert_eq!(linear_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn quadratic_infeasible_uses_solver_farkas_solution_without_linear_fallback() {
        let linear_calls = Arc::new(AtomicUsize::new(0));
        let solver = MockQuadraticDualSolver {
            quadratic_output: SolverOutput::new(SolverStatus::Infeasible).with_dual(vec![1.0]),
            linear_call_counter: linear_calls.clone(),
        };
        let adapter =
            CoreQuadraticBendersAdapter::new("mock", solver).with_cut_context(build_cut_context());
        let model = build_quadratic_subproblem_model();

        let result = adapter.solve_sub_quadratic(&model, &[2.0]).await.expect(
            "quadratic infeasible sub-problem should succeed with solver-provided farkas dual",
        );

        match result {
            QuadraticSubResult::Infeasible(infeasible) => {
                assert_eq!(
                    infeasible.linear.farkas_dual_solution.constraints,
                    vec![1.0]
                );
                assert!(infeasible.linear.cuts.as_ref().is_none_or(Vec::is_empty));
                assert!(
                    infeasible
                        .quadratic_cuts
                        .as_ref()
                        .is_some_and(|cuts| !cuts.is_empty())
                );
                assert!(infeasible.quadratic_cuts.as_ref().is_some_and(|cuts| {
                    cuts.iter().all(|cut| cut.quadratic_coefficients.is_empty())
                }));
            }
            _ => panic!("expected infeasible quadratic sub-result"),
        }
        assert_eq!(linear_calls.load(Ordering::SeqCst), 0);
    }
}
