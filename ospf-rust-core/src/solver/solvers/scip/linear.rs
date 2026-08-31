use super::*;

#[cfg(feature = "scip")]
impl SCIPSolver {
    pub(super) fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        let start_time = Instant::now();

        let mut scip = self.create_problem(model.objective_category)?;

        let mut scip_vars = Vec::with_capacity(model.num_variables());
        for (i, token) in model.variables.iter().enumerate() {
            let lb = model.lb[i];
            let ub = model.ub[i];
            let model_var_type = model
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

        let mut linear_constraints: Vec<Constraint> = Vec::with_capacity(model.num_constraints());
        for i in 0..model.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            if let Some(row) = model.A.get_row(i) {
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
                model.b[i],
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
