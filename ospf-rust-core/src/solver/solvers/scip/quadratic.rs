use super::*;

#[cfg(feature = "scip")]
impl SCIPSolver {
    pub(super) fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        let start_time = Instant::now();

        let mut scip = self.create_problem(model.objective_category)?;

        let mut scip_vars = Vec::with_capacity(model.num_variables());
        for (i, token) in model.linear.variables.iter().enumerate() {
            let lb = model.linear.lb[i];
            let ub = model.linear.ub[i];
            let model_var_type = model
                .linear
                .var_types
                .get(i)
                .copied()
                .unwrap_or_else(|| token.variable.var_type());
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

        let has_quadratic_objective = model.Q.rows.iter().any(|row| {
            row.entries
                .iter()
                .any(|(_, value)| value.abs() > f64::EPSILON)
        });
        if has_quadratic_objective {
            let objective_var = scip.add_var(
                -f64::INFINITY,
                f64::INFINITY,
                1.0,
                "__scip_quadratic_objective",
                VarType::Continuous,
            );
            let mut linear_coefs = vec![-1.0];
            let linear_vars = vec![&objective_var];
            let mut quad_vars_1 = Vec::new();
            let mut quad_vars_2 = Vec::new();
            let mut quad_coefs = Vec::new();

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
                    lin_vars.push(&scip_vars[var_index1]);
                    lin_coefs.push(coefficient);
                }
            }

            let shifted_rhs = constraint.rhs - *constraint.polynomial.constant();
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

        let mut linear_constraints: Vec<Constraint> =
            Vec::with_capacity(model.linear.num_constraints());
        for i in 0..model.linear.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            if let Some(row) = model.linear.A.get_row(i) {
                for &(j, val) in row.entries.iter() {
                    if val != 0.0 {
                        vars.push(&scip_vars[j]);
                        values.push(val);
                    }
                }
            }

            let constraint = scip.add_cons(
                vars,
                &values,
                -f64::INFINITY,
                model.linear.b[i],
                &format!("c{}", i),
            );
            linear_constraints.push(constraint);
        }

        self.emit_stage_status(SCIPStage::AfterModeling, None, start_time.elapsed(), None)?;
        self.install_telemetry_handler(&mut scip, model.objective_category);
        self.emit_stage_status(SCIPStage::Configuration, None, start_time.elapsed(), None)?;

        let solved = scip.solve();
        let solution = solved.best_sol();
        let solver_status = Self::refine_status_with_solution(
            Self::convert_status(solved.status()),
            solution.is_some(),
        );

        let mut output = SolverOutput::new(solver_status);
        output.node_count = Some(solved.n_nodes());
        output.iterations = Some(solved.n_lp_iterations());

        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        if let Some(solution) = solution {
            let objective_value = solution.obj_val();
            output.objective_value = Some(objective_value);
            output.solution = Some(scip_vars.iter().map(|var| solution.val(var)).collect());
            if let Some(best_bound) = output.best_bound {
                output.mip_gap = Self::relative_gap(objective_value, best_bound);
            }
        }

        if matches!(solver_status, SolverStatus::Optimal) {
            let dual_solution = Self::collect_dual_solution(&linear_constraints);
            if !dual_solution.is_empty() {
                output.dual_solution = Some(dual_solution);
            }
        }

        if solver_status.is_infeasible() {
            let farkas_solution = Self::collect_farkas_solution(&linear_constraints);
            if !farkas_solution.is_empty() {
                output.dual_solution = Some(farkas_solution);
            }
        }

        output.solve_time = start_time.elapsed();
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
