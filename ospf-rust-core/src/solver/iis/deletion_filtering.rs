//! 删除过滤算法
//! Deletion Filtering Algorithm

use std::collections::BTreeSet;
use std::time::Instant;
use crate::error::Result;
#[cfg(not(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12")))]
use crate::error::{CoreError, SolverNotFoundError};
use crate::model::intermediate::{BasicLinearTriadModel, LinearTriadModel};
use crate::solver::SolverOutput;
#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
use crate::solver::solvers::{GurobiSolver, gurobi::GurobiConfig};
use super::{ConstraintSource, IISConfig, LinearIISModel, LinearTriadModelIISSource};

#[derive(Debug, Clone, Default)]
pub(crate) struct ActiveSources {
    constraints: BTreeSet<usize>,
    lower_bounds: BTreeSet<usize>,
    upper_bounds: BTreeSet<usize>,
}

impl ActiveSources {
    pub(crate) fn from_model<M>(model: &M, include_bounds: bool) -> Self
    where
        M: LinearTriadModelIISSource + ?Sized,
    {
        let model = model.as_basic_linear_triad_model();
        let mut active = Self::default();
        for constraint_index in 0..model.num_constraints() {
            active.constraints.insert(constraint_index);
        }

        if include_bounds {
            for (var_index, lb) in model.lb.iter().copied().enumerate() {
                if lb.is_finite() {
                    active.lower_bounds.insert(var_index);
                }
            }
            for (var_index, ub) in model.ub.iter().copied().enumerate() {
                if ub.is_finite() {
                    active.upper_bounds.insert(var_index);
                }
            }
        }

        active
    }

    pub(crate) fn ordered_sources(&self) -> Vec<ConstraintSource> {
        let mut sources = Vec::with_capacity(
            self.constraints.len() + self.lower_bounds.len() + self.upper_bounds.len(),
        );
        sources.extend(
            self.constraints
                .iter()
                .copied()
                .map(ConstraintSource::Constraint),
        );
        sources.extend(
            self.lower_bounds
                .iter()
                .copied()
                .map(ConstraintSource::LowerBound),
        );
        sources.extend(
            self.upper_bounds
                .iter()
                .copied()
                .map(ConstraintSource::UpperBound),
        );
        sources
    }

    pub(crate) fn insert(&mut self, source: ConstraintSource) {
        match source {
            ConstraintSource::Constraint(index) => {
                self.constraints.insert(index);
            }
            ConstraintSource::LowerBound(index) => {
                self.lower_bounds.insert(index);
            }
            ConstraintSource::UpperBound(index) => {
                self.upper_bounds.insert(index);
            }
        }
    }

    pub(crate) fn remove(&mut self, source: ConstraintSource) {
        match source {
            ConstraintSource::Constraint(index) => {
                self.constraints.remove(&index);
            }
            ConstraintSource::LowerBound(index) => {
                self.lower_bounds.remove(&index);
            }
            ConstraintSource::UpperBound(index) => {
                self.upper_bounds.remove(&index);
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.constraints.is_empty() && self.lower_bounds.is_empty() && self.upper_bounds.is_empty()
    }

    pub(crate) fn to_iis_model(
        &self,
        num_constraints: usize,
        num_variables: usize,
    ) -> LinearIISModel {
        let mut iis = LinearIISModel::new(num_constraints, num_variables);
        for index in &self.constraints {
            iis.add_constraint(*index);
        }
        for index in &self.lower_bounds {
            iis.add_lower_bound(*index);
        }
        for index in &self.upper_bounds {
            iis.add_upper_bound(*index);
        }
        iis
    }
}

pub(crate) fn solve_linear_model(model: &LinearTriadModel) -> Result<SolverOutput> {
    #[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
    {
        let solver = GurobiSolver::with_config(GurobiConfig::new().with_output(false));
        return solver.solve_linear(model);
    }

    #[cfg(not(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12")))]
    {
        let _ = model;
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "IIS computation requires one of features: gurobi10, gurobi11, gurobi12",
        )))
    }
}

fn normalized_tolerance(tolerance: f64) -> f64 {
    if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        1e-9
    }
}

fn is_effectively_zero(value: f64, tolerance: f64) -> bool {
    value.abs() <= tolerance
}

fn quick_feasibility_probe<M>(model: &M, tolerance: f64) -> Option<bool>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    for var_index in 0..model.num_variables() {
        let lower = model
            .lb
            .get(var_index)
            .copied()
            .unwrap_or(f64::NEG_INFINITY);
        let upper = model.ub.get(var_index).copied().unwrap_or(f64::INFINITY);
        if lower.is_finite() && upper.is_finite() && lower > upper + tolerance {
            return Some(false);
        }
    }

    if model.num_constraints() == 0 {
        return Some(true);
    }

    let mut all_rows_zero = true;
    for row_index in 0..model.num_constraints() {
        let rhs = model.b.get(row_index).copied().unwrap_or(0.0);
        let row_has_nonzero = model
            .A
            .get_row(row_index)
            .map(|row| {
                row.entries
                    .iter()
                    .any(|(_, value)| !is_effectively_zero(*value, tolerance))
            })
            .unwrap_or(false);
        if !row_has_nonzero {
            if rhs < -tolerance {
                return Some(false);
            }
            continue;
        }
        all_rows_zero = false;
    }

    if all_rows_zero { Some(true) } else { None }
}

fn quick_active_feasibility_probe<M>(
    model: &M,
    active: &ActiveSources,
    tolerance: f64,
) -> Option<bool>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    for var_index in 0..model.num_variables() {
        let lower = if active.lower_bounds.contains(&var_index) {
            model
                .lb
                .get(var_index)
                .copied()
                .unwrap_or(f64::NEG_INFINITY)
        } else {
            f64::NEG_INFINITY
        };
        let upper = if active.upper_bounds.contains(&var_index) {
            model.ub.get(var_index).copied().unwrap_or(f64::INFINITY)
        } else {
            f64::INFINITY
        };
        if lower.is_finite() && upper.is_finite() && lower > upper + tolerance {
            return Some(false);
        }
    }

    if active.constraints.is_empty() {
        return Some(true);
    }

    let mut all_rows_zero = true;
    for constraint_index in &active.constraints {
        let rhs = model.b.get(*constraint_index).copied().unwrap_or(0.0);
        let row_has_nonzero = model
            .A
            .get_row(*constraint_index)
            .map(|row| {
                row.entries
                    .iter()
                    .any(|(_, value)| !is_effectively_zero(*value, tolerance))
            })
            .unwrap_or(false);
        if !row_has_nonzero {
            if rhs < -tolerance {
                return Some(false);
            }
            continue;
        }
        all_rows_zero = false;
    }

    if all_rows_zero { Some(true) } else { None }
}

pub(crate) fn is_basic_model_feasible_with_tolerance<M>(model: &M, tolerance: f64) -> Result<bool>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let tolerance = normalized_tolerance(tolerance);
    if let Some(feasible) = quick_feasibility_probe(model, tolerance) {
        return Ok(feasible);
    }

    let linear_model = LinearTriadModel::from_basic(model.as_basic_linear_triad_model().clone());
    let output = solve_linear_model(&linear_model)?;
    Ok(output.status.is_feasible())
}

pub(crate) fn build_submodel_from_active<M>(
    model: &M,
    active: &ActiveSources,
) -> BasicLinearTriadModel
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    let mut submodel = BasicLinearTriadModel::new(&format!("{}_iis_active", model.name));

    for (var_index, token) in model.variables.iter().cloned().enumerate() {
        let mut lb = model
            .lb
            .get(var_index)
            .copied()
            .unwrap_or(f64::NEG_INFINITY);
        let mut ub = model.ub.get(var_index).copied().unwrap_or(f64::INFINITY);

        if !active.lower_bounds.contains(&var_index) {
            lb = f64::NEG_INFINITY;
        }
        if !active.upper_bounds.contains(&var_index) {
            ub = f64::INFINITY;
        }

        let var_type = model
            .var_types
            .get(var_index)
            .copied()
            .unwrap_or_else(|| token.var_type());
        submodel.add_variable_with_bounds(token, lb, ub, var_type);
    }

    for constraint_index in &active.constraints {
        if let Some(row) = model.A.get_row(*constraint_index) {
            let rhs = model.b.get(*constraint_index).copied().unwrap_or(0.0);
            let name = model
                .constraint_names
                .get(*constraint_index)
                .cloned()
                .unwrap_or_else(|| format!("c{}", constraint_index));
            let group_id = model
                .constraint_group_ids
                .get(*constraint_index)
                .copied()
                .unwrap_or(None);
            let lazy = model
                .constraint_lazy_flags
                .get(*constraint_index)
                .copied()
                .unwrap_or(false);
            let priority = model
                .constraint_priorities
                .get(*constraint_index)
                .copied()
                .unwrap_or(0);
            let args = model
                .constraint_args
                .get(*constraint_index)
                .cloned()
                .unwrap_or(None);
            let source_symbol_id = model
                .constraint_source_symbol_ids
                .get(*constraint_index)
                .copied()
                .unwrap_or(None);

            submodel.add_constraint_with_metadata(
                row.clone(),
                rhs,
                name,
                group_id,
                lazy,
                priority,
                args,
                source_symbol_id,
            );
        }
    }

    submodel
}

pub(crate) fn is_active_model_feasible_with_tolerance<M>(
    model: &M,
    active: &ActiveSources,
    tolerance: f64,
) -> Result<bool>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let tolerance = normalized_tolerance(tolerance);
    if let Some(feasible) = quick_active_feasibility_probe(model, active, tolerance) {
        return Ok(feasible);
    }

    let submodel = build_submodel_from_active(model, active);
    is_basic_model_feasible_with_tolerance(&submodel, tolerance)
}

pub(crate) fn compute_iis_deletion_with_active<M>(
    model: &M,
    config: &IISConfig,
    mut active: ActiveSources,
) -> Result<LinearIISModel>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    let start = Instant::now();

    if config.verbose {
        println!("Starting deletion filtering IIS computation");
        println!(
            "Model has {} variables and {} constraints",
            model.num_variables(),
            model.num_constraints()
        );
    }

    if is_basic_model_feasible_with_tolerance(model, config.tolerance)? {
        let mut iis = LinearIISModel::new(model.num_constraints(), model.num_variables());
        iis.set_computation_time(start.elapsed());
        return Ok(iis);
    }

    let mut attempts = 0usize;
    loop {
        if attempts >= config.max_iterations {
            break;
        }
        if let Some(limit) = config.time_limit {
            if start.elapsed() >= limit {
                break;
            }
        }

        let mut removed_any = false;
        let sources = active.ordered_sources();
        if sources.is_empty() {
            break;
        }

        for source in sources {
            if attempts >= config.max_iterations {
                break;
            }
            if let Some(limit) = config.time_limit {
                if start.elapsed() >= limit {
                    break;
                }
            }

            active.remove(source);
            let feasible =
                is_active_model_feasible_with_tolerance(model, &active, config.tolerance)?;
            attempts += 1;

            if feasible {
                active.insert(source);
            } else {
                removed_any = true;
            }
        }

        if !removed_any {
            break;
        }
    }

    let mut iis = active.to_iis_model(model.num_constraints(), model.num_variables());
    iis.set_computation_time(start.elapsed());

    if config.verbose {
        println!("IIS computation completed in {:?}", iis.computation_time);
        println!("IIS contains {} elements", iis.len());
        if attempts >= config.max_iterations {
            println!("Reached maximum iteration limit: {}", config.max_iterations);
        }
    }

    Ok(iis)
}

/// 使用删除过滤算法计算 IIS / Compute IIS using deletion filtering algorithm
///
/// 删除过滤通过逐个删除约束，检查模型是否变为可行来识别 IIS。
/// Deletion filtering identifies IIS by removing constraints one by one
/// and checking if the model becomes feasible.
///
/// # 参数 / Parameters
/// - `model`: 线性三角模型 / Linear triad model
/// - `config`: IIS 配置 / IIS configuration
///
/// # 返回 / Returns
/// IIS 模型 / IIS model
pub fn compute_iis_deletion<M>(model: &M, config: &IISConfig) -> Result<LinearIISModel>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let active = ActiveSources::from_model(model, config.include_bounds);
    compute_iis_deletion_with_active(model, config, active)
}
