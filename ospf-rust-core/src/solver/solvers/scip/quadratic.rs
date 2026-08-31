//! SCIP 二次求解实现
//! SCIP quadratic solve implementation
//!
//! 将 `QuadraticTetradModel` 映射为 SCIP 问题实例并求解，
//! 包括二次目标处理、二次约束构建、线性约束添加及结果提取。
//! Maps a `QuadraticTetradModel` into a SCIP problem instance and solves it,
//! including quadratic objective handling, quadratic constraint construction,
//! linear constraint addition, and result extraction.

use super::*;

#[cfg(feature = "scip")]
impl SCIPSolver {
    /// 求解二次四元组模型，返回求解器输出
    /// Solve a quadratic tetrad model and return the solver output
    ///
    /// 流程：
    /// 1. 创建 SCIP 问题并设置目标方向
    /// 2. 为模型中的每个变量创建 SCIP 变量
    /// 3. 若存在二次目标项，引入辅助目标变量并添加二次约束
    /// 4. 构建二次约束（从多项式单项式分离线性/二次项）
    /// 5. 构建线性约束
    /// 6. 安装遥测处理器并执行求解
    /// 7. 提取原始解、对偶解（含 Farkas 证明）、最优边界及 MIP 间隙
    ///
    /// Workflow:
    /// 1. Create a SCIP problem and set the objective direction
    /// 2. Create a SCIP variable for each model variable
    /// 3. If quadratic objective terms exist, introduce an auxiliary objective variable and add a quadratic constraint
    /// 4. Build quadratic constraints (separating linear/quadratic terms from polynomial monomials)
    /// 5. Build linear constraints
    /// 6. Install the telemetry handler and solve
    /// 7. Extract the primal solution, dual solution (including Farkas proof), best bound, and MIP gap
    pub(super) fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        let start_time = Instant::now();

        // 创建 SCIP 问题实例，根据目标类别设置最大化/最小化
        // Create a SCIP problem instance; set maximize/minimize based on objective category
        let mut scip = self.create_problem(model.objective_category)?;

        // 添加变量：遍历线性子模型的变量，设置下界、上界、变量类型和目标系数
        // Add variables: iterate the linear sub-model's variables, setting bounds, type, and objective coefficient
        let mut scip_vars = Vec::with_capacity(model.linear.variables.len());
        for (i, token) in model.linear.variables.iter().enumerate() {
            let lb = model.linear.lb[i];
            let ub = model.linear.ub[i];
            // 优先使用模型显式指定的变量类型，否则从令牌推断
            // Prefer the explicitly specified variable type; fall back to the token's inferred type
            let model_var_type = model
                .linear
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

        // 检查是否存在非零二次目标项
        // Check whether non-zero quadratic objective terms exist
        let has_quadratic_objective = model.Q.rows.iter().any(|row| {
            row.entries
                .iter()
                .any(|(_, value)| value.abs() > f64::EPSILON)
        });
        if has_quadratic_objective {
            // 引入辅助连续变量作为二次目标代理，目标系数设为 1.0
            // Introduce an auxiliary continuous variable as the quadratic objective proxy with objective coefficient 1.0
            let objective_var = scip.add_var(
                -f64::INFINITY,
                f64::INFINITY,
                1.0,
                "__scip_quadratic_objective",
                VarType::Continuous,
            );
            // 约束形式：obj_var - Σ qᵢⱼ xᵢxⱼ <= 0（最小化）或 obj_var - Σ qᵢⱼ xᵢxⱼ >= 0（最大化）
            // Constraint form: obj_var - Σ qᵢⱼ xᵢxⱼ <= 0 (minimize) or obj_var - Σ qᵢⱼ xᵢxⱼ >= 0 (maximize)
            let mut linear_coefs = vec![-1.0];
            let linear_vars = vec![&objective_var];
            let mut quad_vars_1 = Vec::new();
            let mut quad_vars_2 = Vec::new();
            let mut quad_coefs = Vec::new();

            // 从二次目标矩阵 Q 提取非零项
            // Extract non-zero entries from the quadratic objective matrix Q
            for (i, row) in model.Q.rows.iter().enumerate() {
                if i >= scip_vars.len() {
                    return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                        "quadratic objective row index {} out of bounds for {} variables",
                        i,
                        scip_vars.len()
                    ))));
                }
                for &(j, qval) in row.entries.iter() {
                    if qval.abs() <= f64::EPSILON {
                        continue;
                    }
                    if j >= scip_vars.len() {
                        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                            "quadratic objective column index {} out of bounds for {} variables",
                            j,
                            scip_vars.len()
                        ))));
                    }
                    quad_vars_1.push(&scip_vars[i]);
                    quad_vars_2.push(&scip_vars[j]);
                    quad_coefs.push(qval);
                }
            }

            // 根据目标类别确定约束方向
            // Determine constraint direction based on objective category
            let (lhs, rhs) = match model.objective_category {
                ObjectiveCategory::Minimum => (-f64::INFINITY, 0.0),
                ObjectiveCategory::Maximum => (0.0, f64::INFINITY),
            };
            scip.add_cons_quadratic(
                linear_vars,
                linear_coefs.as_mut_slice(),
                quad_vars_1,
                quad_vars_2,
                quad_coefs.as_mut_slice(),
                lhs,
                rhs,
                "__scip_quadratic_objective_cons",
            );
        }

        // 构建二次约束：从多项式单项式分离线性项和二次项
        // Build quadratic constraints: separate linear and quadratic terms from polynomial monomials
        for (i, constraint) in model.quadratic_constraints.iter().enumerate() {
            let mut lin_vars = Vec::new();
            let mut lin_coefs = Vec::new();
            let mut quad_vars_1 = Vec::new();
            let mut quad_vars_2 = Vec::new();
            let mut quad_coefs = Vec::new();

            for monomial in constraint.polynomial.monomials() {
                let var_index1 = monomial.var_index1();
                if var_index1 >= scip_vars.len() {
                    return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                        "quadratic constraint {} references invalid variable index {}",
                        i, var_index1
                    ))));
                }
                let coefficient = *monomial.coefficient();
                if coefficient.abs() <= f64::EPSILON {
                    continue;
                }
                if let Some(var_index2) = monomial.var_index2() {
                    // 二次项：c * xᵢ * xⱼ
                    // Quadratic term: c * xᵢ * xⱼ
                    if var_index2 >= scip_vars.len() {
                        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                            "quadratic constraint {} references invalid variable index {}",
                            i, var_index2
                        ))));
                    }
                    quad_vars_1.push(&scip_vars[var_index1]);
                    quad_vars_2.push(&scip_vars[var_index2]);
                    quad_coefs.push(coefficient);
                } else {
                    // 线性项：c * xᵢ
                    // Linear term: c * xᵢ
                    lin_vars.push(&scip_vars[var_index1]);
                    lin_coefs.push(coefficient);
                }
            }

            // 右端减去常数项，将约束移项为标准形式
            // Subtract the constant term from RHS to bring the constraint into standard form
            let shifted_rhs = constraint.rhs - *constraint.polynomial.constant();
            // 根据约束关系确定左端和右端
            // Determine LHS and RHS based on constraint relation
            let (lhs, rhs) = match constraint.relation {
                ConstraintRelation::LessEqual => (-f64::INFINITY, shifted_rhs),
                ConstraintRelation::Equal => (shifted_rhs, shifted_rhs),
                ConstraintRelation::GreaterEqual => (shifted_rhs, f64::INFINITY),
            };
            let constraint_name = model
                .quadratic_constraint_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("qc{}", i));
            scip.add_cons_quadratic(
                lin_vars,
                lin_coefs.as_mut_slice(),
                quad_vars_1,
                quad_vars_2,
                quad_coefs.as_mut_slice(),
                lhs,
                rhs,
                &constraint_name,
            );
        }

        // 构建线性约束：Ax <= b
        // Build linear constraints: Ax <= b
        let mut linear_constraints: Vec<Constraint> =
            Vec::with_capacity(model.linear.num_constraints());
        for i in 0..model.linear.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            // 从稀疏矩阵提取约束行的非零元素
            // Extract non-zero entries from the sparse matrix row
            if let Some(row) = model.linear.A.get_row(i) {
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
                model.linear.b[i],
                &format!("c{}", i),
            );
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

        // 提取最优边界（若有限）
        // Extract the best bound (if finite)
        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        // 提取原始解和目标值
        // Extract the primal solution and objective value
        if let Some(solution) = solution {
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
            let dual_solution = Self::collect_dual_solution(&linear_constraints);
            if !dual_solution.is_empty() {
                output.dual_solution = Some(dual_solution);
            }
        }

        // 在不可行时提取 Farkas 证明作为对偶信息
        // Extract Farkas proof as dual information when infeasible
        if solver_status.is_infeasible() {
            let farkas_solution = Self::collect_farkas_solution(&linear_constraints);
            if !farkas_solution.is_empty() {
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
