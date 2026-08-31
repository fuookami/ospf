//! 弹性过滤算法
//! Elastic Filtering Algorithm

use super::deletion_filtering::{
    ActiveSources, compute_iis_deletion_with_active, is_active_model_feasible_with_tolerance,
    is_basic_model_feasible_with_tolerance, solve_linear_model,
};
use super::{ConstraintSource, IISConfig, LinearIISModel, LinearTriadModelIISSource};
use crate::error::Result;
use crate::model::ObjectiveCategory;
use crate::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use crate::token::Token;
use crate::variable::{UContinuousVariableItem, VariableId};

fn next_group_id<M>(model: &M) -> usize
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    model
        .variables
        .iter()
        .map(|token| token.id().group_id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn add_slack_variable(model: &mut BasicLinearTriadModel, next_id: &mut usize, name: &str) -> usize {
    let var = UContinuousVariableItem::create(VariableId::standalone(*next_id), name);
    *next_id = next_id.saturating_add(1);
    let token = Token::from_generic(var, model.num_variables());
    model.add_variable(token)
}

fn build_elastic_relaxation<M>(
    model: &M,
    config: &IISConfig,
) -> (LinearTriadModel, Vec<ConstraintSource>, usize)
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    let mut relaxed = BasicLinearTriadModel::new(&format!("{}_iis_elastic", model.name));
    let original_variable_count = model.num_variables();

    for (var_index, token) in model.variables.iter().cloned().enumerate() {
        let lb = if config.include_bounds {
            f64::NEG_INFINITY
        } else {
            model
                .lb
                .get(var_index)
                .copied()
                .unwrap_or(f64::NEG_INFINITY)
        };
        let ub = if config.include_bounds {
            f64::INFINITY
        } else {
            model.ub.get(var_index).copied().unwrap_or(f64::INFINITY)
        };
        let var_type = model
            .var_types
            .get(var_index)
            .copied()
            .unwrap_or_else(|| token.var_type());
        relaxed.add_variable_with_bounds(token, lb, ub, var_type);
    }

    let mut slack_sources = Vec::new();
    let mut current_group_id = next_group_id(model);

    for constraint_index in 0..model.num_constraints() {
        if let Some(row) = model.A.get_row(constraint_index) {
            let slack_index = add_slack_variable(
                &mut relaxed,
                &mut current_group_id,
                &format!("elastic_c{}", constraint_index),
            );
            let mut relaxed_row = row.clone();
            relaxed_row.add(slack_index, -1.0);
            let rhs = model.b.get(constraint_index).copied().unwrap_or(0.0);
            relaxed.add_constraint(relaxed_row, rhs);
            slack_sources.push(ConstraintSource::Constraint(constraint_index));
        }
    }

    if config.include_bounds {
        for (var_index, lb) in model.lb.iter().copied().enumerate() {
            if !lb.is_finite() {
                continue;
            }
            let slack_index = add_slack_variable(
                &mut relaxed,
                &mut current_group_id,
                &format!("elastic_lb{}", var_index),
            );
            let mut row = SparseVector::new();
            row.add(var_index, -1.0);
            row.add(slack_index, -1.0);
            relaxed.add_constraint(row, -lb);
            slack_sources.push(ConstraintSource::LowerBound(var_index));
        }

        for (var_index, ub) in model.ub.iter().copied().enumerate() {
            if !ub.is_finite() {
                continue;
            }
            let slack_index = add_slack_variable(
                &mut relaxed,
                &mut current_group_id,
                &format!("elastic_ub{}", var_index),
            );
            let mut row = SparseVector::new();
            row.add(var_index, 1.0);
            row.add(slack_index, -1.0);
            relaxed.add_constraint(row, ub);
            slack_sources.push(ConstraintSource::UpperBound(var_index));
        }
    }

    let mut relaxed_model = LinearTriadModel::from_basic(relaxed);
    let mut objective = vec![0.0; relaxed_model.num_variables()];
    let penalty = if config.elastic_penalty > 0.0 {
        config.elastic_penalty
    } else {
        1.0
    };
    for slack_offset in 0..slack_sources.len() {
        let slack_var_index = original_variable_count + slack_offset;
        if slack_var_index < objective.len() {
            objective[slack_var_index] = penalty;
        }
    }
    relaxed_model.set_objective(objective, ObjectiveCategory::Minimum);

    (relaxed_model, slack_sources, original_variable_count)
}

/// 使用弹性过滤算法计算 IIS / Compute IIS using elastic filtering algorithm
///
/// 弹性过滤通过为每个约束添加松弛变量（弹性变量），然后最小化松弛变量的和来识别 IIS。
/// Elastic filtering adds slack variables (elastic variables) to each constraint,
/// then minimizes the sum of slack variables to identify IIS.
///
/// # 参数 / Parameters
/// - `model`: 线性三角模型 / Linear triad model
/// - `config`: IIS 配置 / IIS configuration
///
/// # 返回 / Returns
/// IIS 模型 / IIS model
pub fn compute_iis_elastic<M>(model: &M, config: &IISConfig) -> Result<LinearIISModel>
where
    M: LinearTriadModelIISSource + ?Sized,
{
    let model = model.as_basic_linear_triad_model();
    let start = std::time::Instant::now();

    if config.verbose {
        println!("Starting elastic filtering IIS computation");
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

    let (relaxed_model, slack_sources, slack_start_index) = build_elastic_relaxation(model, config);
    let relaxed_output = solve_linear_model(&relaxed_model)?;

    let mut active = ActiveSources::default();
    if relaxed_output.status.is_feasible() {
        if let Some(solution) = &relaxed_output.solution {
            for (slack_offset, source) in slack_sources.iter().enumerate() {
                let slack_index = slack_start_index + slack_offset;
                let slack_value = solution.get(slack_index).copied().unwrap_or(0.0);
                if slack_value > config.tolerance {
                    active.insert(*source);
                }
            }
        }
    }

    if active.is_empty()
        || is_active_model_feasible_with_tolerance(model, &active, config.tolerance)?
    {
        active = ActiveSources::from_model(model, config.include_bounds);
    }

    let mut iis = compute_iis_deletion_with_active(model, config, active)?;
    iis.set_computation_time(start.elapsed());

    if config.verbose {
        println!("IIS computation completed in {:?}", iis.computation_time);
        println!("IIS contains {} elements", iis.len());
    }

    Ok(iis)
}
