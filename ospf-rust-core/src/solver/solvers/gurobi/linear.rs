//! Gurobi 线性求解路径
//! Gurobi linear solve pipeline

use super::config::GurobiStage;
use super::solver::GurobiSolver;
use crate::MechanismModel;
use crate::error::{CoreError, Result, SolverError, SolverModelingError};
use crate::intermediate::{FallbackReason, NativeLoweringReport, NativeWriteOutcome};
use crate::model::intermediate::LinearTriadModel;
use crate::solver::{SolveHandle, SolveOptions, SolverOutput};
use crate::variable::{VariableId, VariableType};
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
    let (output, _) = solve_linear_internal(solver, model, 1, false, None, None, None)?;
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
        None,
    )?;
    Ok(output)
}

pub(super) fn solve_linear_with_solution_pool(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
    solve_linear_internal(solver, model, solution_amount.max(1), true, None, None, None)
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
        None,
    )
}

/// 创建一个求解器列 / Create one solver column
///
/// 原生 lowering 路径与普通路径共用本函数，保证两条路径的列类型、边界与初始值处理完全一致。
///
/// The native-lowering path and the ordinary path share this function, so both handle column type,
/// bounds and initial value identically.
fn add_linear_column(
    grb_model: &mut grb::Model,
    name: &str,
    lb: f64,
    ub: f64,
    var_type: VariableType,
    obj: f64,
    start: Option<f64>,
) -> Result<grb::Var> {
    use grb::prelude::*;

    let vtype = match var_type {
        VariableType::Binary => Binary,
        VariableType::Integer
        | VariableType::Ternary
        | VariableType::BalancedTernary
        | VariableType::UInteger => Integer,
        _ => Continuous,
    };

    let var = add_var!(grb_model, vtype, name: name, obj: obj, bounds: lb..ub)
        .map_err(|e| CoreError::solver_modeling(format!("Gurobi add_var error: {}", e)))?;
    if let Some(initial_result) = start {
        grb_model
            .set_obj_attr(attr::Start, &var, initial_result)
            .map_err(|e| CoreError::solver_modeling(format!("Gurobi set Start error: {}", e)))?;
    }
    Ok(var)
}

/// 求解一个线性三角模型 / Solve one linear triad model
///
/// `prepared` 允许调用方（原生 lowering 路径）提供**已经建好列、并且可能已经写入原生约束**的 SDK
/// 模型与列变量；此时本函数跳过环境/模型/变量创建，只做参数设置、行装载与求解，从而让原生写入
/// 发生在「建列」与「装行」之间。
///
/// `prepared` lets a caller (the native-lowering path) supply an SDK model whose columns already exist
/// and which may already carry native constraints; the function then skips environment, model and
/// variable creation and only applies parameters, loads rows and solves. That is what lets a native
/// write happen between column creation and row loading.
fn solve_linear_internal(
    solver: &GurobiSolver,
    model: &LinearTriadModel,
    solution_amount: usize,
    collect_solution_pool: bool,
    cancellation_handle: Option<&SolveHandle>,
    per_solve_options: Option<&SolveOptions<'_>>,
    prepared: Option<(grb::Model, Vec<grb::Var>)>,
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

    // 原生 lowering 路径提供已建列的模型；其余路径在此创建环境与模型。
    // The native-lowering path supplies a model whose columns already exist; every other path
    // creates the environment and model here.
    let uses_prepared_model = prepared.is_some();
    let (mut grb_model, mut grb_vars) = match prepared {
        Some((prepared_model, prepared_vars)) => (prepared_model, prepared_vars),
        None => {
            // 创建环境
            let env = solver.create_env()?;

            // 创建模型
            let created = Model::with_env("model", &env).map_err(|e| {
                CoreError::SolverModeling(SolverModelingError::new(format!(
                    "Gurobi model error: {}",
                    e
                )))
            })?;
            (created, Vec::new())
        }
    };

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

    // 添加变量；原生 lowering 路径已在阶段一建好列，此处跳过。
    // Add variables; the native-lowering path already created its columns in phase one, so this is
    // skipped there.
    if !uses_prepared_model {
        grb_vars.reserve(model.num_variables());
        for (i, token) in model.variables.iter().enumerate() {
            let lb = model.lb[i];
            let ub = model.ub[i];
            let model_var_type = model
                .var_types
                .get(i)
                .copied()
                .unwrap_or_else(|| token.variable.var_type());
            let raw_obj = model.c.get(i).copied().unwrap_or(0.0);
            let obj = if raw_obj.abs() > zero_tolerance {
                raw_obj
            } else {
                0.0
            };

            let var = add_linear_column(
                &mut grb_model,
                &token.variable.name(),
                lb,
                ub,
                model_var_type,
                obj,
                token.get_result(),
            )?;
            grb_vars.push(var);
        }
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

/// 两阶段求解：先建列，再做原生 lowering，最后装行并求解
/// Two-phase solve: create columns, lower natively, then load rows and solve
///
/// 这是 P3 接线的入口：机制模型携带的延迟结构在**建列之后、装行之前**交给原生 writer。
/// 原生写入成功的结构由模型层从待展开列表丢弃（否则会与原生行重复），其余结构一次性物化为通用
/// fallback，因此任一 writer 失败或没有 writer 认领时，模型仍然得到与 EAGER 逐列等价的完整行；
/// writer 报错时错误向上传播，调用方对整模型回退。
///
/// This is the P3 wiring entry point: the deferred structures carried by the mechanism model are
/// handed to the native writers after the columns exist and before the rows are loaded. Natively
/// written structures are dropped from the pending list by the model layer (otherwise they would
/// duplicate the native rows) while the rest are materialized as the generic fallback in one step,
/// so the model still receives rows that are column-identical to eager expansion whenever a writer
/// fails or claims nothing; a writer error propagates so the caller can fall back for the whole
/// model.
/// 按列视图建立与三角模型逐列一致的 SDK 列 / Build SDK columns that match the column view column for column.
///
/// 阶段一与「原生写入失败后的整模型回退」共用本函数，因此两条路径建出的列完全一致：列顺序与边界
/// 规则都来自同一个 `linear_column_view()`。
///
/// Phase one and the whole-model fallback after a native write failure share this function, so both
/// paths build identical columns: the order and bound rules come from the same `linear_column_view()`.
fn build_sdk_columns(
    solver: &GurobiSolver,
    mechanism: &MechanismModel<f64>,
) -> Result<(
    grb::Model,
    std::collections::HashMap<VariableId, grb::Var>,
    Vec<grb::Var>,
)> {
    let env = solver.create_env()?;
    let mut grb_model = grb::Model::with_env("model", &env).map_err(|e| {
        CoreError::SolverModeling(SolverModelingError::new(format!(
            "Gurobi model error: {}",
            e
        )))
    })?;
    let columns_view = mechanism.linear_column_view();
    let mut columns = std::collections::HashMap::with_capacity(columns_view.len());
    let mut grb_vars = Vec::with_capacity(columns_view.len());
    for column in &columns_view {
        let var = add_linear_column(
            &mut grb_model,
            &column.name,
            column.lb,
            column.ub,
            column.var_type,
            0.0,
            column.start,
        )?;
        columns.insert(column.id.clone(), var);
        grb_vars.push(var);
    }
    Ok((grb_model, columns, grb_vars))
}

/// 用默认 writer 集合求解（P3 接线入口）/ Solve with the default writer set (the P3 wiring entry).
pub(super) fn solve_linear_with_native_lowering(
    solver: &GurobiSolver,
    mechanism: MechanismModel<f64>,
    per_solve_options: Option<&SolveOptions<'_>>,
) -> Result<(SolverOutput, NativeLoweringReport)> {
    use crate::model::intermediate::NativeFunctionWriterRegistry;

    use super::native::{
        GurobiAbsWriter, GurobiBinaryzationWriter, GurobiExtremumWriter, GurobiIfInWriter,
        GurobiImplyWriter, GurobiIndicatorWriter, GurobiLogicalWriter, GurobiNativeContainer,
        GurobiPwlWriter,
    };

    let mut registry: NativeFunctionWriterRegistry<GurobiNativeContainer, f64> =
        NativeFunctionWriterRegistry::new();
    registry.register(Box::new(GurobiAbsWriter::new()));
    registry.register(Box::new(GurobiExtremumWriter::maximum()));
    registry.register(Box::new(GurobiExtremumWriter::minimum()));
    // 分段线性形状（Sin/Cos/Sigmoid）的原生 writer；范围证明在 writer 内完成。
    // Native writer for the piecewise-linear shapes (Sin/Cos/Sigmoid); its range proof happens inside
    // the writer.
    registry.register(Box::new(GurobiPwlWriter::new()));
    // 关系指示（条件形状）的原生 writer；Big-M 冗余证明在 writer 内完成。
    // Native writer for the relation indicator (condition shape); its big-M redundancy proof happens
    // inside the writer.
    registry.register(Box::new(GurobiIndicatorWriter::new()));
    // IF-IN（离散值集合判定）的原生 writer；逐候选值的 Big-M 冗余证明在 writer 内完成。
    // Native writer for IF-IN (discrete set membership); its per-candidate big-M redundancy proof
    // happens inside the writer.
    registry.register(Box::new(GurobiIfInWriter::new()));
    // AND/OR（紧凑二值 hull）的原生 writer；hull 不依赖 Big-M，因此无需冗余证明。
    // Native writer for AND/OR (the compact binary hull); the hull does not depend on the big-M, so no
    // redundancy proof is needed.
    registry.register(Box::new(GurobiLogicalWriter::conjunction()));
    registry.register(Box::new(GurobiLogicalWriter::disjunction()));
    // 二值化（阈值/Big-M 两种机制等价变体）的原生 writer；Big-M 冗余证明在 writer 内完成。
    // Native writer for binaryzation (both mechanism-equivalent variants, Threshold and Big-M); its big-M
    // redundancy proof happens inside the writer.
    registry.register(Box::new(GurobiBinaryzationWriter::new()));
    // 蕴含的原生 writer：两个内部关系指示器各自写两条指示约束（松弛行按各自冻结的 M 证明），再加 3 条
    // 耦合指示约束；耦合等价依赖 r/p/c 的二元类型，二元校验在 writer 内完成。
    // Native writer for implication: each internal relation indicator writes two indicator constraints (its
    // relaxed rows proven with its own frozen Big-M) plus three coupling indicator constraints; the coupling
    // equivalence relies on r/p/c being binary, and the writer performs that check.
    registry.register(Box::new(GurobiImplyWriter::new()));
    solve_linear_with_native_writers(solver, mechanism, per_solve_options, registry)
}

/// 用给定 writer 集合求解两阶段流程 / Run the two-phase flow with the given writer set.
///
/// 之所以把 registry 作为参数暴露，是因为「原生写入失败 → 整模型回退」这条路径只有注入一个必然
/// 失败的 writer 才能被端到端验证；公开入口始终使用上面的默认集合。
///
/// The registry is a parameter because the "native write failure ⇒ whole-model fallback" path can only
/// be verified end to end by injecting a writer that must fail; the public entry point above always
/// uses the default set.
#[doc(hidden)]
pub fn solve_linear_with_native_writers(
    solver: &GurobiSolver,
    mut mechanism: MechanismModel<f64>,
    per_solve_options: Option<&SolveOptions<'_>>,
    registry: crate::model::intermediate::NativeFunctionWriterRegistry<
        super::native::GurobiNativeContainer,
        f64,
    >,
) -> Result<(SolverOutput, NativeLoweringReport)> {
    // 阶段一：按列视图建列。列视图的顺序与边界规则和随后转换得到的三角模型逐列一致，因此这里
    // 建立的 SDK 列就是最终模型的列。
    // Phase one: create columns from the column view. Its order and bound rules match the triad
    // produced afterwards column for column, so the SDK columns created here are the final ones.
    let (mut grb_model, columns, grb_vars) = build_sdk_columns(solver, &mechanism)?;

    // 阶段二：原生 lowering。registry 按注册顺序决定每个结构的去向，容器按值持有模型。
    // Phase two: native lowering. The registry decides each structure's destination in registration
    // order while the container owns the model by value.
    let report = {
        let mut container =
            super::native::GurobiNativeContainer::new(grb_model, columns, grb_vars.clone());
        match mechanism.lower_deferred_functions(&registry, &mut container) {
            Ok(report) => {
                let (model, _columns) = container.into_parts();
                grb_model = model;
                report
            }
            Err(error) => {
                // 原生写入失败 → **整模型回退**（失败原子性）。此时机制模型尚未被改动
                // （`apply_native_lowering` 只在全部 writer 成功后执行），但 SDK 模型里可能已经
                // 写入了一部分一般约束，且无法逐条回滚；因此丢弃这个 SDK 模型，用同一份机制模型
                // 走通用展开路径**重建**并求解。
                //
                // A native write failure triggers a **whole-model fallback** (failure atomicity). The
                // mechanism model is still untouched here (`apply_native_lowering` runs only after
                // every writer succeeded), but the SDK model may already carry some general
                // constraints and they cannot be rolled back one by one; the SDK model is therefore
                // discarded and rebuilt from the same mechanism model along the generic path.
                drop(container);
                let pending = mechanism.as_basic().deferred_functions().len();
                let mut outcomes = Vec::with_capacity(pending);
                for _ in 0..pending {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::WriterFailed(
                        error.to_string(),
                    )));
                }
                let report = NativeLoweringReport {
                    outcomes,
                    materialized_fallbacks: pending,
                    native_writes: 0,
                };
                return solve_linear_without_native_lowering(
                    solver,
                    mechanism,
                    per_solve_options,
                    report,
                );
            }
        }
    };

    // 阶段三：转换为三角模型（原生结构已丢弃、其余已物化），再装行、设参数并求解。
    // Phase three: convert to the triad (native structures are gone, the rest are materialized),
    // then load rows, apply parameters and solve.
    let model = mechanism.into_linear_triad_model();
    let cancellation_handle = per_solve_options.and_then(|options| options.cancellation_handle);
    let (output, _) = solve_linear_internal(
        solver,
        &model,
        1,
        false,
        cancellation_handle,
        per_solve_options,
        Some((grb_model, grb_vars)),
    )?;
    Ok((output, report))
}

/// 完全跳过原生 lowering 的求解 / Solve while skipping native lowering entirely.
///
/// 用于原生写入失败后的整模型回退：全新 SDK 环境与模型（绝不复用可能已有部分原生约束的模型），
/// 转换阶段会把全部延迟结构物化为通用展开，因此语义与 EAGER 一致。
///
/// Used for the whole-model fallback after a native write failure: a fresh SDK environment and model
/// (never the model that may already carry some native constraints), and the conversion step
/// materializes every deferred structure generically, so the semantics are those of eager expansion.
fn solve_linear_without_native_lowering(
    solver: &GurobiSolver,
    mut mechanism: MechanismModel<f64>,
    per_solve_options: Option<&SolveOptions<'_>>,
    report: NativeLoweringReport,
) -> Result<(SolverOutput, NativeLoweringReport)> {
    let (grb_model, _columns, grb_vars) = build_sdk_columns(solver, &mechanism)?;
    let model = mechanism.into_linear_triad_model();
    let cancellation_handle = per_solve_options.and_then(|options| options.cancellation_handle);
    let (output, _) = solve_linear_internal(
        solver,
        &model,
        1,
        false,
        cancellation_handle,
        per_solve_options,
        Some((grb_model, grb_vars)),
    )?;
    Ok((output, report))
}
