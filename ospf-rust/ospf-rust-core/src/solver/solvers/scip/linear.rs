//! SCIP 线性求解实现
//! SCIP linear solve implementation
//!
//! 将 `LinearTriadModel` 映射为 SCIP 问题实例并求解，
//! 包括变量创建、约束添加、遥测安装及结果提取。
//! Maps a `LinearTriadModel` into a SCIP problem instance and solves it,
//! including variable creation, constraint addition, telemetry installation, and result extraction.

use super::*;

#[cfg(feature = "scip")]
impl SCIPSolver {
    /// 求解线性三元组模型，返回求解器输出
    /// Solve a linear triad model and return the solver output
    ///
    /// 流程：
    /// 1. 创建 SCIP 问题并设置目标方向
    /// 2. 为模型中的每个变量创建 SCIP 变量（含边界、类型、目标系数）
    /// 3. 从稀疏矩阵 A 和右端向量 b 构建线性约束
    /// 4. 安装遥测处理器并执行求解
    /// 5. 提取原始解、对偶解、最优边界及 MIP 间隙
    ///
    /// Workflow:
    /// 1. Create a SCIP problem and set the objective direction
    /// 2. Create a SCIP variable for each model variable (with bounds, type, objective coefficient)
    /// 3. Build linear constraints from sparse matrix A and RHS vector b
    /// 4. Install the telemetry handler and solve
    /// 5. Extract the primal solution, dual solution, best bound, and MIP gap
    pub(super) fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        crate::solver::audit::validate_linear_model_for_backend(model)?;
        let start_time = Instant::now();

        // 创建 SCIP 问题实例，根据目标类别设置最大化/最小化
        // Create a SCIP problem instance; set maximize/minimize based on objective category
        let mut scip = self.create_problem(model.objective_category)?;

        // 添加变量：遍历模型变量，设置下界、上界、变量类型和目标系数
        // Add variables: iterate model variables, setting lower/upper bounds, variable type, and objective coefficient
        let mut scip_vars = Vec::with_capacity(model.num_variables());
        for (i, token) in model.variables.iter().enumerate() {
            let lb = model.lb[i];
            let ub = model.ub[i];
            // 优先使用模型显式指定的变量类型，否则从令牌推断
            // Prefer the explicitly specified variable type; fall back to the token's inferred type
            let model_var_type = model
                .var_types
                .get(i)
                .copied()
                .unwrap_or_else(|| token.variable.var_type());
            // 将模型变量类型映射为 SCIP VarType
            // Map model variable type to SCIP VarType
            let vtype = match model_var_type {
                VariableType::Binary => VarType::Binary,
                VariableType::Integer
                | VariableType::Ternary
                | VariableType::BalancedTernary
                | VariableType::UInteger => VarType::Integer,
                _ => VarType::Continuous,
            };

            let obj = model.c.get(i).copied().unwrap_or(0.0);
            let var = scip.add_var(lb, ub, obj, &token.variable.name(), vtype);
            scip_vars.push(var);
        }

        // 添加线性约束：Ax <= b
        // Add linear constraints: Ax <= b
        let mut linear_constraints: Vec<Constraint> = Vec::with_capacity(model.num_constraints());
        for i in 0..model.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            // 从稀疏矩阵提取约束行的非零元素
            // Extract non-zero entries from the sparse matrix row
            if let Some(row) = model.A.get_row(i) {
                for &(j, val) in row.entries.iter() {
                    if val != 0.0 {
                        vars.push(&scip_vars[j]);
                        values.push(val);
                    }
                }
            }

            // 左端为 -∞，右端为 b[i]，即约束形式为 Ax <= b
            // LHS is -∞, RHS is b[i], i.e. constraint form is Ax <= b
            let constraint = scip.add_cons(
                vars,
                &values,
                -f64::INFINITY,
                model.b[i],
                &format!("c{}", i),
            );
            scip.set_cons_removable(&constraint, false);
            linear_constraints.push(constraint);
        }

        // 发出建模完成阶段状态
        // Emit after-modeling stage status
        self.emit_stage_status(SCIPStage::AfterModeling, None, start_time.elapsed(), None)?;
        // 安装遥测事件处理器
        // Install the telemetry event handler
        self.install_telemetry_handler(&mut scip, model.objective_category);
        // 发出配置完成阶段状态
        // Emit configuration stage status
        self.emit_stage_status(SCIPStage::Configuration, None, start_time.elapsed(), None)?;

        // 执行求解
        // Execute the solve
        let solved = scip.solve();
        let solution = solved.best_sol();
        // 将 SCIP 状态转换为框架内求解器状态，并根据是否有解进行修正
        // Convert SCIP status to framework solver status, refined by solution availability
        let solver_status = Self::refine_status_with_solution(
            Self::convert_status(solved.status()),
            solution.is_some(),
        );

        // 构建求解器输出
        // Build the solver output
        let mut output = SolverOutput::new(solver_status);
        output.node_count = Some(solved.n_nodes());
        output.iterations = Some(solved.n_lp_iterations());
        output.solution_count = Some(solved.n_sols());

        // 提取最优边界（若有限）
        // Extract the best bound (if finite)
        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        // 提取原始解和目标值
        // Extract the primal solution and objective value
        if solver_status.is_feasible()
            && let Some(solution) = solution
        {
            let objective_value = solution.obj_val();
            output.objective_value = Some(objective_value);
            output.solution = Some(scip_vars.iter().map(|var| solution.val(var)).collect());
            // 计算相对 MIP 间隙
            // Compute the relative MIP gap
            if let Some(best_bound) = output.best_bound {
                output.mip_gap = Self::relative_gap(objective_value, best_bound);
            }
        }

        // 在最优解时提取线性对偶乘子（尽力获取）
        // Extract linear dual multipliers when optimal (best effort)
        if matches!(solver_status, SolverStatus::Optimal) {
            let dual_solution = Self::collect_dual_solution(
                &linear_constraints,
                model.objective_category,
            );
            if let Some(dual_solution) = dual_solution
                && !dual_solution.is_empty()
            {
                output.dual_solution = Some(dual_solution);
            }
        }

        if solver_status.is_infeasible() {
            // 读取线性 LP 的 Farkas 乘子；失败时保留不可行报告并交给 generic fallback。
            // Read linear LP Farkas multipliers; preserve the infeasible report and use the generic fallback when unavailable.
            if let Some(farkas_solution) = Self::collect_farkas_solution(&linear_constraints)
                && !farkas_solution.is_empty()
            {
                output.dual_solution = Some(farkas_solution);
            }
        }

        output.solve_time = start_time.elapsed();
        // 根据求解结果发出最终阶段状态
        // Emit the final stage status based on the solve result
        self.emit_stage_status(
            if output.status.is_feasible() {
                SCIPStage::AnalyzingSolution
            } else {
                SCIPStage::AfterFailure
            },
            Some(output.status),
            output.solve_time,
            Some(&output),
        )?;
        Ok(output)
    }
}
