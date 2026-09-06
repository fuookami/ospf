//! Gurobi 线性求解路径
//! Gurobi linear solve pipeline

use super::config::GurobiStage;
use super::solver::GurobiSolver;
use crate::error::{CoreError, Result, SolverError, SolverModelingError};
use crate::model::intermediate::LinearTriadModel;
use crate::solver::{SolveHandle, SolveOptions, SolverOutput};
use crate::variable::VariableType;
#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
use grb::expr::LinExpr;
use std::time::Instant;

fn set_model_int_param(model: &mut grb::Model, name: &str, value: i32) -> Result<()> {
    let parameter =
        grb::parameter::Parameter::new(name).map_err(GurobiSolver::environment_error)?;
    model
        .set_param(&parameter, value)
        .map_err(GurobiSolver::environment_error)
}

fn set_model_real_param(model: &mut grb::Model, name: &str, value: f64) -> Result<()> {
    let parameter =
        grb::parameter::Parameter::new(name).map_err(GurobiSolver::environment_error)?;
    model
        .set_param(&parameter, value)
        .map_err(GurobiSolver::environment_error)
}

pub(super) fn solve_linear(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
) -> Result<SolverOutput> {
    let (output, _) = solve_linear_internal(solver, model, 1, false, None, None)?;
    Ok(output)
}

pub(super) fn solve_linear_with_options(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    options: &SolveOptions<'_>,
) -> Result<SolverOutput> {
    let (output, _) = solve_linear_internal(
        solver,
        model,
        1,
        false,
        options.cancellation_handle,
        Some(options),
    )?;
    Ok(output)
}

pub(super) fn solve_linear_with_solution_pool(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
    solve_linear_internal(solver, model, solution_amount.max(1), true, None, None)
}

pub(super) fn solve_linear_with_solution_pool_with_options(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
    options: &SolveOptions<'_>,
) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
    solve_linear_internal(
        solver,
        model,
        solution_amount.max(1),
        true,
        options.cancellation_handle,
        Some(options),
    )
}

fn solve_linear_internal(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
    collect_solution_pool: bool,
    cancellation_handle: Option<&SolveHandle>,
    per_solve_options: Option<&SolveOptions<'_>>,
) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
    crate::solver::audit::validate_linear_model_for_backend(model)?;
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

    // 模型级补充参数；配置错误必须显式返回 / Additional model-level parameters; configuration errors are surfaced.
    if let Some(node_limit) = solver.config().node_limit {
        set_model_real_param(&mut grb_model, "NodeLimit", node_limit as f64)?;
    }
    if let Some(solution_limit) = solver.config().solution_limit {
        set_model_int_param(&mut grb_model, "SolutionLimit", solution_limit)?;
    }
    if let Some(options) = per_solve_options {
        if let Some(time_limit) = options.time_limit {
            set_model_real_param(&mut grb_model, "TimeLimit", time_limit.as_secs_f64())?;
        }
        if let Some(node_limit) = options.node_limit {
            let node_limit = node_limit as f64;
            if !node_limit.is_finite() {
                return Err(CoreError::Solver(SolverError::InvalidInput(
                    "Gurobi per-solve node limit is outside the native range".to_owned(),
                )));
            }
            set_model_real_param(&mut grb_model, "NodeLimit", node_limit)?;
        }
        if let Some(solution_limit) = options.solution_limit {
            let solution_limit = i32::try_from(solution_limit).map_err(|_| {
                CoreError::Solver(SolverError::InvalidInput(
                    "Gurobi per-solve solution limit exceeds the native int range".to_owned(),
                ))
            })?;
            set_model_int_param(&mut grb_model, "SolutionLimit", solution_limit)?;
        }
    }
    if let Some(mem_limit) = solver.config().mem_limit {
        set_model_real_param(&mut grb_model, "MemLimit", mem_limit)?;
    }
    if let Some(numeric_focus) = numeric_settings.numeric_focus {
        set_model_int_param(&mut grb_model, "NumericFocus", numeric_focus)?;
    }
    if let Some(scale_flag) = numeric_settings.scale_flag {
        set_model_int_param(&mut grb_model, "ScaleFlag", scale_flag)?;
    }

    if collect_solution_pool && solution_amount > 1 {
        set_model_real_param(&mut grb_model, "PoolGap", 1.0)?;
        set_model_int_param(&mut grb_model, "PoolSearchMode", 2)?;
        set_model_int_param(&mut grb_model, "PoolSolutions", solution_amount as i32)?;
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
                .map_err(|e| CoreError::solver_modeling(format!("Gurobi add_var error: {}", e)))?;
        if let Some(initial_result) = token.get_result() {
            grb_model
                .set_obj_attr(attr::Start, &var, initial_result)
                .map_err(|e| {
                    CoreError::solver_modeling(format!("Gurobi set Start error: {}", e))
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
            .map_err(|e| CoreError::solver_modeling(format!("Gurobi add_constr error: {}", e)))?;
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
    grb_model
        .set_objective(obj_expr, sense)
        .map_err(|e| CoreError::solver_modeling(format!("Gurobi set_objective error: {}", e)))?;

    solver.emit_stage_status(GurobiStage::AfterModeling, None, start_time.elapsed(), None)?;
    solver.emit_stage_status(GurobiStage::Configuration, None, start_time.elapsed(), None)?;

    // 优化
    solver.optimize_model(
        &mut grb_model,
        model.objective_category,
        Some(&grb_vars),
        cancellation_handle,
    )?;

    // 获取结果；当状态为 InfOrUnbd 时，关闭 DualReductions 再优化一次以区分 Infeasible/Unbounded
    let mut status = grb_model
        .status()
        .map_err(|e| CoreError::solver_backend(format!("Gurobi get status error: {}", e)))?;
    if status == Status::InfOrUnbd {
        grb_model.set_param(param::DualReductions, 0).map_err(|e| {
            CoreError::solver_backend(format!("Gurobi set DualReductions error: {}", e))
        })?;
        solver.optimize_model(
            &mut grb_model,
            model.objective_category,
            Some(&grb_vars),
            cancellation_handle,
        )?;
        status = grb_model
            .status()
            .map_err(|e| CoreError::solver_backend(format!("Gurobi get status error: {}", e)))?;
    }
    let mapped_status = GurobiSolver::convert_status(status);
    let solution_count = grb_model
        .get_attr(attr::SolCount)
        .map_err(|e| CoreError::solver_backend(format!("Gurobi get SolCount error: {}", e)))?;
    // SolCount 是 incumbent 的原生事实来源；无解的 Optimal 不是可行结果，不能读取 ObjVal/X。
    // SolCount is the native source of truth for an incumbent; an Optimal status without a
    // solution is not feasible and must not make ObjVal/X appear available.
    let has_solution = solution_count > 0;
    let mut solver_status = GurobiSolver::refine_status_with_solution(mapped_status, has_solution);

    let mut output = SolverOutput::new(solver_status);
    output.solution_count = Some(solution_count.max(0) as usize);

    // These are solver facts, not incumbent facts. Keep them for limit and
    // infeasible reports as well; reading them only inside the incumbent
    // branch made a limit without a solution look as if no work occurred.
    if let Ok(iter) = grb_model.get_attr(attr::IterCount)
        && iter.is_finite()
        && iter >= 0.0
    {
        output.iterations = Some(iter as usize);
    }
    if let Ok(nodes) = grb_model.get_attr(attr::NodeCount)
        && nodes.is_finite()
        && nodes >= 0.0
    {
        output.node_count = Some(nodes as usize);
    }
    if let Ok(bound) = grb_model.get_attr(attr::ObjBound)
        && bound.is_finite()
    {
        output.best_bound = Some(bound);
    }

    if solver_status.is_feasible() && has_solution {
        // 获取目标值
        let obj = grb_model
            .get_attr(attr::ObjVal)
            .map_err(|e| CoreError::solver_backend(format!("Gurobi get ObjVal error: {}", e)))?;
        output.objective_value = Some(obj);

        // 获取解
        let vars = grb_model
            .get_vars()
            .map_err(|e| CoreError::solver_backend(format!("Gurobi get_vars error: {}", e)))?;
        let mut solution = Vec::with_capacity(vars.len());
        for var in vars {
            let val = grb_model
                .get_obj_attr(attr::X, &var)
                .map_err(|e| CoreError::solver_backend(format!("Gurobi get X error: {}", e)))?;
            solution.push(val);
        }
        output.solution = Some(solution);

        // 获取对偶解（仅 LP）
        if status == Status::Optimal && !model.var_types.iter().any(VariableType::is_integer) {
            let constrs = grb_model.get_constrs().map_err(|e| {
                CoreError::solver_backend(format!("Gurobi get_constrs error: {}", e))
            })?;
            let mut dual = Vec::with_capacity(constrs.len());
            for constr in constrs {
                let pi = grb_model.get_obj_attr(attr::Pi, &constr).map_err(|e| {
                    CoreError::solver_backend(format!("Gurobi get Pi error: {}", e))
                })?;
                dual.push(pi);
            }
            output.dual_solution = Some(dual);
        }

        // 按统一报告合同由 incumbent 和 best bound 重算 gap。
        // Recompute the gap from the incumbent and best bound for the unified report contract.
        output.mip_gap = output
            .objective_value
            .zip(output.best_bound)
            .map(|(objective, bound)| (objective - bound).abs() / objective.abs().max(1.0));

        solver_status = GurobiSolver::refine_status_with_mip_gap(
            solver_status,
            has_solution,
            model.var_types.iter().any(VariableType::is_integer),
            solver.config().mip_gap,
            output.mip_gap,
        );
        output.status = solver_status;
    }

    if solver_status.is_infeasible() && !model.var_types.iter().any(VariableType::is_integer) {
        // 读取 LP 不可行证书；读取失败时保留原始不可行终态，由上层 generic IIS/Farkas fallback 处理。
        // Read the LP infeasibility certificate; keep the original terminal when unavailable so the upper layer can use a generic fallback.
        if let Ok(constrs) = grb_model.get_constrs() {
            let mut farkas = Vec::with_capacity(constrs.len());
            let mut all_available = true;
            for constr in constrs {
                match grb_model.get_obj_attr(attr::FarkasDual, &constr) {
                    Ok(value) => farkas.push(value),
                    Err(_) => {
                        all_available = false;
                        break;
                    }
                }
            }
            if all_available && !farkas.is_empty() {
                output.dual_solution = Some(farkas);
            }
        }
    }

    let mut solutions = Vec::new();
    if collect_solution_pool && solution_amount > 1 && solver_status.is_feasible() && has_solution {
        if let Some(primary) = output.solution.clone() {
            solutions.push(primary);
        }
        let expected = (solution_count.max(0) as usize).min(solution_amount);
        if expected > 1 {
            for solution_index in 0..expected {
                let parameter = grb::parameter::Parameter::new("SolutionNumber").map_err(|e| {
                    CoreError::solver_backend(format!(
                        "Gurobi create SolutionNumber parameter error: {}",
                        e
                    ))
                })?;
                grb_model
                    .set_param(&parameter, solution_index as i32)
                    .map_err(|e| {
                        CoreError::solver_backend(format!("Gurobi set SolutionNumber error: {}", e))
                    })?;
                let candidate: Vec<f64> = grb_vars
                    .iter()
                    .map(|var| {
                        grb_model.get_obj_attr(attr::Xn, var).map_err(|e| {
                            CoreError::solver_backend(format!("Gurobi get Xn error: {}", e))
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
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
