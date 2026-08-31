//! Gurobi 线性求解路径
//! Gurobi linear solve pipeline

use std::time::Instant;
#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
use grb::expr::LinExpr;
use crate::error::{CoreError, Result, SolverError, SolverModelingError};
use crate::model::intermediate::LinearTriadModel;
use crate::solver::SolverOutput;
use crate::variable::VariableType;
use super::config::GurobiStage;
use super::solver::GurobiSolver;

pub(super) fn solve_linear(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
) -> Result<SolverOutput> {
    let (output, _) = solve_linear_internal(solver, model, 1, false)?;
    Ok(output)
}

pub(super) fn solve_linear_with_solution_pool(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
    solve_linear_internal(solver, model, solution_amount.max(1), true)
}

fn solve_linear_internal(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
    collect_solution_pool: bool,
) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
    let start_time = Instant::now();
    let mut numeric_coefficients = Vec::with_capacity(model.c.len() + model.A.rows.len() * 4);
    numeric_coefficients.extend(model.c.iter().copied());
    for row in &model.A.rows {
        for &(_, value) in &row.entries {
            numeric_coefficients.push(value);
        }
    }
    let numeric_settings = solver.resolve_numeric_settings(&numeric_coefficients)?;
    let zero_tolerance = numeric_settings.zero_tolerance;

    use grb::prelude::*;

    // 创建环境
    let env = solver.create_env()?;

    // 创建模型
    let mut grb_model = Model::with_env("model", &env).map_err(|e| {
        CoreError::SolverModeling(SolverModelingError::new(format!(
            "Gurobi model error: {}",
            e
        )))
    })?;

    // 模型级补充参数（部分版本 crate 未显式暴露，采用名称方式最佳努力设置）
    // Additional model-level parameters set on a best-effort basis by name.
    if let Some(node_limit) = solver.config().node_limit {
        if let Ok(parameter) = grb::parameter::Parameter::new("NodeLimit") {
            let _ = grb_model.set_param(&parameter, node_limit);
        }
    }
    if let Some(mem_limit) = solver.config().mem_limit {
        if let Ok(parameter) = grb::parameter::Parameter::new("MemLimit") {
            let _ = grb_model.set_param(&parameter, mem_limit);
        }
    }
    if let Some(numeric_focus) = numeric_settings.numeric_focus {
        if let Ok(parameter) = grb::parameter::Parameter::new("NumericFocus") {
            let _ = grb_model.set_param(&parameter, numeric_focus);
        }
    }
    if let Some(scale_flag) = numeric_settings.scale_flag {
        if let Ok(parameter) = grb::parameter::Parameter::new("ScaleFlag") {
            let _ = grb_model.set_param(&parameter, scale_flag);
        }
    }

    if collect_solution_pool && solution_amount > 1 {
        if let Ok(parameter) = grb::parameter::Parameter::new("PoolGap") {
            let _ = grb_model.set_param(&parameter, 1.0);
        }
        if let Ok(parameter) = grb::parameter::Parameter::new("PoolSearchMode") {
            let _ = grb_model.set_param(&parameter, 2);
        }
        if let Ok(parameter) = grb::parameter::Parameter::new("PoolSolutions") {
            let _ = grb_model.set_param(&parameter, solution_amount as i32);
        }
    }

    // 添加变量
    let mut grb_vars = Vec::with_capacity(model.num_variables());
    for (i, token) in model.variables.iter().enumerate() {
        let lb = model.lb[i];
        let ub = model.ub[i];
        let model_var_type = model
            .var_types
            .get(i)
            .copied()
            .unwrap_or_else(|| token.variable.var_type());
        let vtype = match model_var_type {
            VariableType::Binary => Binary,
            VariableType::Integer
            | VariableType::Ternary
            | VariableType::BalancedTernary
            | VariableType::UInteger => Integer,
            _ => Continuous,
        };
        let raw_obj = model.c.get(i).copied().unwrap_or(0.0);
        let obj = if raw_obj.abs() > zero_tolerance {
            raw_obj
        } else {
            0.0
        };

        let var =
            add_var!(grb_model, vtype, name: &token.variable.name(), obj: obj, bounds: lb..ub)
                .map_err(|e| {
                    CoreError::Solver(SolverError::SolveFailed(format!(
                        "Gurobi add_var error: {}",
                        e
                    )))
                })?;
        if let Some(initial_result) = token.get_result() {
            grb_model
                .set_obj_attr(attr::Start, &var, initial_result)
                .map_err(|e| {
                    CoreError::Solver(SolverError::SolveFailed(format!(
                        "Gurobi set Start error: {}",
                        e
                    )))
                })?;
        }

        grb_vars.push(var);
    }

    // 添加约束: Ax <= b
    for i in 0..model.num_constraints() {
        let mut expr = LinExpr::new();

        // 从稀疏矩阵提取约束行
        if let Some(row) = model.A.get_row(i) {
            for &(j, val) in row.entries.iter() {
                if val.abs() > zero_tolerance {
                    expr.add_term(val, grb_vars[j]);
                }
            }
        }

        grb_model
            .add_constr(&format!("c{}", i), c!(expr <= model.b[i]))
            .map_err(|e| {
                CoreError::Solver(SolverError::SolveFailed(format!(
                    "Gurobi add_constr error: {}",
                    e
                )))
            })?;
    }

    // 设置目标方向
    let sense = match model.objective_category {
        crate::model::object::ObjectiveCategory::Minimum => Minimize,
        crate::model::object::ObjectiveCategory::Maximum => Maximize,
    };

    // 构建目标表达式
    let mut obj_expr = LinExpr::new();
    for (i, &coeff) in model.c.iter().enumerate() {
        if coeff.abs() > zero_tolerance {
            obj_expr.add_term(coeff, grb_vars[i]);
        }
    }
    grb_model.set_objective(obj_expr, sense).map_err(|e| {
        CoreError::Solver(SolverError::SolveFailed(format!(
            "Gurobi set_objective error: {}",
            e
        )))
    })?;

    solver.emit_stage_status(GurobiStage::AfterModeling, None, start_time.elapsed(), None)?;
    solver.emit_stage_status(GurobiStage::Configuration, None, start_time.elapsed(), None)?;

    // 优化
    solver.optimize_model(&mut grb_model, model.objective_category, Some(&grb_vars))?;

    // 获取结果；当状态为 InfOrUnbd 时，关闭 DualReductions 再优化一次以区分 Infeasible/Unbounded
    let mut status = grb_model.status().map_err(|e| {
        CoreError::Solver(SolverError::SolveFailed(format!(
            "Gurobi get status error: {}",
            e
        )))
    })?;
    if status == Status::InfOrUnbd {
        grb_model.set_param(param::DualReductions, 0).map_err(|e| {
            CoreError::Solver(SolverError::SolveFailed(format!(
                "Gurobi set DualReductions error: {}",
                e
            )))
        })?;
        solver.optimize_model(&mut grb_model, model.objective_category, Some(&grb_vars))?;
        status = grb_model.status().map_err(|e| {
            CoreError::Solver(SolverError::SolveFailed(format!(
                "Gurobi get status error: {}",
                e
            )))
        })?;
    }
    let mapped_status = GurobiSolver::convert_status(status);
    let has_solution = matches!(mapped_status, crate::solver::SolverStatus::Optimal)
        || grb_model.get_attr(attr::SolCount).unwrap_or(0) > 0;
    let solver_status = GurobiSolver::refine_status_with_solution(mapped_status, has_solution);

    let mut output = SolverOutput::new(solver_status);

    if solver_status.is_feasible() && has_solution {
        // 获取目标值
        if let Ok(obj) = grb_model.get_attr(attr::ObjVal) {
            output.objective_value = Some(obj);
        }

        // 获取解
        let vars = grb_model.get_vars().map_err(|e| {
            CoreError::Solver(SolverError::SolveFailed(format!(
                "Gurobi get_vars error: {}",
                e
            )))
        })?;
        let mut solution = Vec::with_capacity(vars.len());
        for var in vars {
            let val = grb_model.get_obj_attr(attr::X, &var).unwrap_or(0.0);
            solution.push(val);
        }
        output.solution = Some(solution);

        // 获取对偶解（仅 LP）
        if status == Status::Optimal {
            let constrs = grb_model.get_constrs().map_err(|e| {
                CoreError::Solver(SolverError::SolveFailed(format!(
                    "Gurobi get_constrs error: {}",
                    e
                )))
            })?;
            let mut dual = Vec::with_capacity(constrs.len());
            for constr in constrs {
                let pi = grb_model.get_obj_attr(attr::Pi, &constr).unwrap_or(0.0);
                dual.push(pi);
            }
            if !dual.is_empty() {
                output.dual_solution = Some(dual);
            }
        }

        // 获取迭代次数
        if let Ok(iter) = grb_model.get_attr(attr::IterCount) {
            output.iterations = Some(iter as usize);
        }

        // 获取节点数（MIP）
        if let Ok(nodes) = grb_model.get_attr(attr::NodeCount) {
            output.node_count = Some(nodes as usize);
        }

        // 获取 MIP Gap
        if let Ok(gap) = grb_model.get_attr(attr::MIPGap) {
            output.mip_gap = Some(gap);
        }

        // 获取最优下界
        if let Ok(bound) = grb_model.get_attr(attr::ObjBound) {
            output.best_bound = Some(bound);
        }
    }

    let mut solutions = Vec::new();
    if collect_solution_pool && solution_amount > 1 && solver_status.is_feasible() && has_solution {
        if let Some(primary) = output.solution.clone() {
            solutions.push(primary);
        }
        let available = grb_model.get_attr(attr::SolCount).unwrap_or(0).max(0) as usize;
        let expected = available.min(solution_amount);
        if expected > 1 {
            for solution_index in 0..expected {
                if let Ok(parameter) = grb::parameter::Parameter::new("SolutionNumber") {
                    let _ = grb_model.set_param(&parameter, solution_index as i32);
                }
                let candidate: Vec<f64> = grb_vars
                    .iter()
                    .map(|var| grb_model.get_obj_attr(attr::Xn, var).unwrap_or(0.0))
                    .collect();
                if !candidate.is_empty()
                    && !solutions.iter().any(|existing| {
                        existing.len() == candidate.len()
                            && existing
                                .iter()
                                .zip(candidate.iter())
                                .all(|(left, right)| (*left - *right).abs() <= f64::EPSILON)
                    })
                {
                    solutions.push(candidate);
                }
            }
        }
    }

    output.solve_time = start_time.elapsed();
    if solver_status.is_feasible() {
        solver.emit_stage_status(
            GurobiStage::AnalyzingSolution,
            Some(solver_status),
            output.solve_time,
            Some(&output),
        )?;
    } else {
        solver.emit_stage_status(
            GurobiStage::AfterFailure,
            Some(solver_status),
            output.solve_time,
            Some(&output),
        )?;
    }
    Ok((output, solutions))
}
