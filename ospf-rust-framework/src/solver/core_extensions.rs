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

use super::benders_decomposition::{
    CutSense, LinearBendersDecompositionSolver, LinearCut, LinearFeasibleResult,
    LinearInfeasibleResult, LinearSubResult, QuadraticBendersDecompositionSolver, QuadraticCut,
    QuadraticFeasibleResult, QuadraticInfeasibleResult, QuadraticSubResult,
};
use super::column_generation::{
    ColumnGenerationSolver, FeasibleSolution, LPResult, LinearDualSolution, RegistrationStatus,
    RegistrationStatusCallback, SolvingStatus, SolvingStatusCallback,
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
    options: &super::SolveOptions,
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
    if let Some(feasible) = FeasibleSolution::from_output(output) {
        return Ok(feasible);
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
    output.solution.clone().ok_or_else(|| {
        CoreError::Solver(SolverError::SolveFailed(format!(
            "{} missing solution vector",
            context
        )))
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
        return Ok(dual);
    }

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
        options: &super::SolveOptions,
    ) -> Result<FeasibleSolution> {
        emit_registration_status(
            &self.name,
            model,
            options.registration_status_callback.clone(),
        )?;
        export_lp_model_if_requested(model, &self.name, "milp", options)?;
        let core_callback = build_core_solving_callback(options.solving_status_callback.clone(), 0);
        let options = ospf_rust_core::solver::SolveOptions::new()
            .with_value_conversion_policy(options.value_conversion_policy)
            .with_solving_callback(core_callback.as_ref());
        let output = self.solver.solve_linear_with_options(model, &options)?;
        solver_output_to_feasible(&output)
    }

    fn solve_lp_impl(
        &self,
        model: &LinearTriadModel,
        options: &super::SolveOptions,
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
        let options = ospf_rust_core::solver::SolveOptions::new()
            .with_value_conversion_policy(options.value_conversion_policy)
            .with_solving_callback(core_callback.as_ref());
        let output = self.solver.solve_linear_with_options(&lp_model, &options)?;
        let feasible = solver_output_to_feasible(&output)?;
        let dual = resolve_lp_dual_solution(&self.solver, &lp_model, &output)?;

        Ok(LPResult::new(
            feasible,
            LinearDualSolution::new(dual, Vec::new()),
        ))
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
        options: super::SolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_milp_impl(model, &options)
    }

    async fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: super::SolveOptions,
    ) -> Result<LPResult> {
        self.solve_lp_impl(model, &options)
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
        options: super::SolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_milp_impl(model, &options)
    }

    fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: super::SolveOptions,
    ) -> Result<LPResult> {
        self.solve_lp_impl(model, &options)
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
        for (index, value) in master_solution.iter().copied().enumerate() {
            if index >= fixed_model.num_variables() {
                break;
            }
            fixed_model.lb[index] = value;
            fixed_model.ub[index] = value;
        }
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
            let farkas_model = fixed_subproblem.to_farkas_dual();
            let farkas_output = self.solver.solve_linear(&farkas_model)?;
            let farkas_solution = solution_vector_from_output(&farkas_output, "farkas dual model")?;
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
        for (index, value) in master_solution.iter().copied().enumerate() {
            if index >= fixed_model.num_variables() {
                break;
            }
            fixed_model.basic.linear.lb[index] = value;
            fixed_model.basic.linear.ub[index] = value;
        }
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
                    let farkas_model = linear_surrogate.to_farkas_dual();
                    let farkas_output = self.solver.solve_linear(&farkas_model)?;
                    farkas_solution =
                        solution_vector_from_output(&farkas_output, "quadratic farkas dual model")?;
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
        for (index, value) in master_solution.iter().copied().enumerate() {
            if index >= fixed_subproblem.num_variables() {
                break;
            }
            fixed_subproblem.lb[index] = value;
            fixed_subproblem.ub[index] = value;
        }
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
            let farkas_model = fixed_subproblem.to_farkas_dual();
            let farkas_output = self.solver.solve_linear(&farkas_model)?;
            let farkas_solution = solution_vector_from_output(&farkas_output, "farkas dual model")?;
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
        BasicQuadraticTetradModel, SparseMatrix, SparseVector,
    };
    use ospf_rust_core::model::{
        BasicMechanismModel, Linear, LinearConstraint, LinearInequality, LinearMonomial,
        ObjectiveCategory,
    };
    use ospf_rust_core::solver::{
        LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverStatus,
    };
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::ContinuousVariableItem;

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
        let options = crate::solver::SolveOptions::new()
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
