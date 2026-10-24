use ospf_rust_core::solver::{SolveFingerprints, SolveProof, SolveSolution};
use ospf_rust_core::variable::{Binary, Continuous, VariableRange};
use super::*;

fn source(objective_recourse: bool, fixed_y: bool) -> (MetaModel, VariableId, VariableId) {
    let mut model = MetaModel::new("full_source");
    let x = model.register_auto_variable::<Binary>("decision").unwrap();
    let y = model.register_auto_variable_with_range::<Continuous>("resource", VariableRange::bounded(0.0, if fixed_y { 0.0 } else { 2.0 })).unwrap();
    model.add_ge_constraint(&[(x, 1.0), (y, 1.0)], 1.0, "coupling").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(0.5, x), LinearMonomial::new(if objective_recourse { 1.0 } else { 0.0 }, y)], 7.0), "business"));
    let ids = (model.tokens()[x].id(), model.tokens()[y].id());
    (model, ids.0, ids.1)
}

fn report(model: &LinearTriadModel, values: Vec<f64>, dual: Option<Vec<f64>>) -> Result<SolveReport<f64>> {
    let mut solution = SolveSolution::vector(values);
    solution.objective_value = Some(model.c.iter().zip(&solution.values).map(|(coefficient, value)| coefficient * value).sum());
    solution.dual_solution = dual;
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() })
        .model_mapping(Some(linear_model_mapping(model)?)).build()
}

#[test]
fn default_partition_keeps_all_integer_and_objective_variables_and_rejects_invalid_stages() {
    let (mut model, x, y) = source(true, false);
    let helper = model.register_auto_variable::<Binary>("expanded_helper").unwrap();
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    assert!(partition.master_variables().contains(&x));
    assert!(partition.master_variables().contains(&y));
    assert!(partition.master_variables().contains(&model.tokens()[helper].id()));
    assert!(partition.recourse_variables().is_empty());
    assert!(LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages: BTreeMap::from([(x, LinearBendersStage::ContinuousRecourse)]) }).is_err());
    assert!(LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages: BTreeMap::from([(new_standalone_id(), LinearBendersStage::Master)]) }).is_err());
    model.add_le_constraint(&[(helper, 1.0)], 0.0, "new_business_rule").unwrap();
    assert!(FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |_, _, _| panic!("stale partition")).is_err());
}

#[test]
fn bound_aware_farkas_cut_excludes_anchor_and_complete_source_solution_is_verified() {
    let (model, x, y) = source(false, true);
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    let mut master_round = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
        if model.has_integer_variables() {
            let value = if master_round == 0 { 0.0 } else { 1.0 };
            master_round += 1;
            return report(model, vec![value], None);
        }
        assert_eq!(model.num_constraints(), 3);
        assert_eq!(model.lb[0], f64::NEG_INFINITY);
        assert_eq!(model.ub[0], f64::INFINITY);
        if model.b[0] < -0.5 {
            let mut proof = SolveProof::infeasibility();
            proof.evidence = Some(vec![1.0, 0.0, 1.0]);
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed).proof(proof)
                .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() })
                .model_mapping(Some(linear_model_mapping(model)?)).build()
        } else { report(model, vec![0.0], Some(vec![0.0; 3])) }
    }).unwrap();
    assert_eq!(result.stop, FixedRecourseStop::Converged);
    assert_eq!(result.feasibility_cuts, 1);
    assert_eq!(result.values[&x], 1.0);
    assert_eq!(result.values[&y], 0.0);
    assert_eq!(result.objective, Some(7.5));
}

#[test]
fn invalid_and_missing_numeric_evidence_are_not_convergence_or_infeasibility() {
    let (model, _, y) = source(true, false);
    let partition = LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages: BTreeMap::from([(y, LinearBendersStage::ContinuousRecourse)]) }).unwrap();
    for (dual, expected) in [(None, FixedRecourseStop::CertificateMissing), (Some(vec![0.0; 3]), FixedRecourseStop::CertificateInvalid)] {
        let result = FixedRecourseAlgorithm::solve(&model, &partition, FixedRecourseConfig { recover_missing_certificates: false, ..Default::default() }, &SolveHandle::new(), |model, _, _| {
            if model.has_integer_variables() { report(model, vec![0.0, 0.0], None) }
            else { report(model, vec![1.0], dual.clone()) }
        }).unwrap();
        assert_eq!(result.stop, expected);
        assert_eq!(result.optimality_cuts, 0);
        assert_eq!(result.objective, Some(8.0));
    }
}

#[test]
fn recovery_backend_failure_keeps_the_previously_validated_source_incumbent() {
    let (model, _, y) = source(true, false);
    let partition = LinearBendersPartition::build(&model, LinearBendersPartitionOptions {
        stages: BTreeMap::from([(y, LinearBendersStage::ContinuousRecourse)]) }).unwrap();
    let mut calls = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
        calls += 1;
        match calls {
            1 => report(model, vec![0.0, 0.0], None),
            2 => report(model, vec![1.0], None),
            _ => Err(CoreError::NotImplemented("certificate recovery failed after source incumbent".into())),
        }
    }).unwrap();
    assert_eq!(calls, 3);
    assert_eq!(result.objective, Some(8.0));
    assert!(result.backend_error.is_some());
    assert_eq!(result.stop, FixedRecourseStop::BackendFailure(TerminationReason::BackendFailure));
    assert!(!result.values.is_empty());
}

#[test]
fn cancellation_zero_budget_and_unbounded_theta_preflight_never_launch_backend() {
    let (mut model, _, y) = source(true, false);
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    let handle = SolveHandle::new();
    let result = FixedRecourseAlgorithm::solve(&model, &partition, FixedRecourseConfig { time_limit: Duration::ZERO, ..Default::default() }, &handle, |_, _, _| panic!("zero budget")).unwrap();
    assert_eq!(result.stop, FixedRecourseStop::TimeLimit);
    handle.cancel("test");
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &handle, |_, _, _| panic!("cancelled")).unwrap();
    assert_eq!(result.stop, FixedRecourseStop::Cancelled);
    let free = model.register_auto_variable::<Continuous>("free_objective").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(1.0, free)], 0.0), "unbounded_box"));
    assert!(LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages: BTreeMap::from([(model.tokens()[free].id(), LinearBendersStage::ContinuousRecourse), (y, LinearBendersStage::ContinuousRecourse)]) }).is_err());
}

#[test]
fn terminal_master_with_no_recourse_retains_complete_validated_incumbent() {
    let (model, _, _) = source(true, false);
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    for reason in [TerminationReason::TimeLimit, TerminationReason::Cancelled] {
        let mut calls = 0;
        let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
            calls += 1;
            let mut result = report(model, vec![1.0, 0.0], None)?;
            result.proof = None;
            result.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
            result.termination_reason = reason;
            Ok(result)
        }).unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.objective, Some(7.5));
        assert_eq!(result.values.len(), 2);
        assert_eq!(result.stop, if reason == TerminationReason::TimeLimit { FixedRecourseStop::TimeLimit } else { FixedRecourseStop::Cancelled });
    }
}

#[test]
fn terminal_subproblem_retains_merged_source_incumbent_without_generating_cuts() {
    let (model, _, y) = source(true, false);
    let partition = LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages: BTreeMap::from([(y, LinearBendersStage::ContinuousRecourse)]) }).unwrap();
    let mut calls = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
        calls += 1;
        if model.has_integer_variables() { return report(model, vec![0.0, 0.0], None); }
        let mut result = report(model, vec![1.0], None)?;
        result.proof = None;
        result.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
        result.termination_reason = TerminationReason::TimeLimit;
        Ok(result)
    }).unwrap();
    assert_eq!(calls, 2);
    assert_eq!(result.stop, FixedRecourseStop::TimeLimit);
    assert_eq!(result.objective, Some(8.0));
    assert_eq!(result.values.len(), 2);
    assert_eq!(result.optimality_cuts, 0);
}

#[test]
fn total_deadline_expiring_on_return_still_retains_the_complete_incumbent() {
    let (model, _, _) = source(true, false);
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    let mut calls = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, FixedRecourseConfig { time_limit: Duration::from_millis(250), ..Default::default() },
        &SolveHandle::new(), |model, _, _| {
            calls += 1;
            std::thread::sleep(Duration::from_millis(300));
            report(model, vec![1.0, 0.0], None)
        }).unwrap();
    assert_eq!(calls, 1);
    assert_eq!(result.stop, FixedRecourseStop::TimeLimit);
    assert_eq!(result.objective, Some(7.5));
    assert_eq!(result.values.len(), 2);
}

#[test]
fn cuts_accumulate_exactly_compensate_bounded_rounding_and_refuse_unbounded_rounding() {
    for bounded in [true, false] {
        let mut model = MetaModel::new("rounded_cut");
        let x = if bounded { model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(-1e12, 1e12)).unwrap() }
            else { model.register_auto_variable::<Continuous>("x").unwrap() };
        let y = model.register_auto_variable::<Continuous>("y").unwrap();
        for (index, (a, w)) in [(1.0, 1.0), (-2.0_f64.powi(-54), 1.0), (0.0, -2.0)].into_iter().enumerate() {
            model.add_le_constraint(&[(x, a), (y, w)], 0.0, &format!("row_{index}")).unwrap();
        }
        model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
        let variable = model.tokens()[x].id();
        let partition = LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages: BTreeMap::from([(variable, LinearBendersStage::Master)]) }).unwrap();
        let cut = partition.cut(&[1.0; 3], false);
        if bounded {
            let (coefficients, rhs) = cut.unwrap();
            assert_eq!(coefficients[&variable], 1.0);
            assert!(rhs >= 2.0_f64.powi(-54) * 1e12);
        } else { assert_eq!(cut.unwrap_err().solver_error_class(), ospf_rust_core::error::SolverErrorClass::Unsupported); }
    }
}

#[test]
fn cancelling_master_activity_cannot_return_an_infeasible_source_incumbent() {
    let mut model = MetaModel::new("exact_anchor_rhs");
    let xs = (0..3).map(|index| model.register_auto_variable_with_range::<Binary>(
        &format!("x_{index}"), VariableRange::bounded(1.0, 1.0)).unwrap()).collect::<Vec<_>>();
    let y = model.register_auto_variable_with_range::<Continuous>("y", VariableRange::bounded(0.0, 0.0)).unwrap();
    model.add_le_constraint(&[(xs[0], 1e16), (xs[1], 1.0), (xs[2], -1e16), (y, 1.0)], 0.0, "cancelling").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
    let snapshot = model.linear_model_snapshot().unwrap();
    let invalid_values = snapshot.variables.iter().enumerate().map(|(index, state)| (state.variable, if index < 3 { 1.0 } else { 0.0 })).collect();
    assert!(snapshot.validate_solution(&invalid_values, &1e-7).is_err());
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    let mut calls = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
        calls += 1;
        if model.has_integer_variables() { return report(model, vec![1.0; 3], None); }
        assert_eq!(model.b[0], -1.0);
        // 故意返回旧浮点算法曾接受的错误报告。 / Deliberately return the invalid report accepted by the old floating accumulation.
        report(model, vec![0.0], None)
    });
    assert_eq!(calls, 2);
    assert!(result.is_err());
}

#[test]
fn cancelling_recourse_objective_preserves_the_true_theta_lower_bound_and_incumbent() {
    let mut model = MetaModel::new("exact_theta_bound");
    let mut stages = BTreeMap::new();
    let mut terms = Vec::new();
    for (index, coefficient) in [1e16, -1.0, -1e16].into_iter().enumerate() {
        let variable = model.register_auto_variable_with_range::<Continuous>(&format!("y_{index}"), VariableRange::bounded(1.0, 1.0)).unwrap();
        stages.insert(model.tokens()[variable].id(), LinearBendersStage::ContinuousRecourse);
        terms.push(LinearMonomial::new(coefficient, variable));
    }
    model.add_sub_objective(SubObjective::minimize(Linear::new(terms, 0.0), "cancelling_objective"));
    let partition = LinearBendersPartition::build(&model, LinearBendersPartitionOptions { stages }).unwrap();
    assert_eq!(partition.theta_lower, -1.0);
    let mut calls = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
        calls += 1;
        if model.num_variables() == 1 {
            assert_eq!(model.lb[0], -1.0);
            return report(model, vec![-1.0], None);
        }
        let mut result = report(model, vec![1.0; 3], Some(vec![-1e16, 0.0, 0.0, -1.0, 0.0, -1e16]))?;
        result.solution.as_mut().unwrap().objective_value = Some(-1.0);
        Ok(result)
    }).unwrap();
    assert_eq!(calls, 2);
    assert_eq!(result.stop, FixedRecourseStop::Converged);
    assert_eq!(result.lower_bound, Some(-1.0));
    assert_eq!(result.objective, Some(-1.0));
}

#[test]
fn unrepresentable_anchor_rhs_stops_before_solving_a_different_subproblem() {
    let between = exact(1e16).unwrap() - exact(1.0).unwrap();
    assert_eq!(rounded(&between, false).unwrap(), 1e16_f64.next_down());
    assert_eq!(rounded(&between, true).unwrap(), 1e16);
    let mut model = MetaModel::new("unrepresentable_rhs");
    let x = model.register_auto_variable::<Binary>("x").unwrap();
    let y = model.register_auto_variable::<Continuous>("y").unwrap();
    model.add_le_constraint(&[(x, -1.0), (y, 1.0)], 1e16, "rhs").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::constant(0.0), "zero"));
    let partition = LinearBendersPartition::build(&model, Default::default()).unwrap();
    let mut calls = 0;
    let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, _, _| {
        calls += 1;
        report(model, vec![1.0], None)
    }).unwrap();
    assert_eq!(calls, 1);
    assert_eq!(result.stop, FixedRecourseStop::Unsupported);
    assert!(result.values.is_empty());
}

#[cfg(any(feature = "scip", feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
mod native {
    use ospf_rust_core::solver::{LinearSolver, SolveOptions};
    use super::*;

    fn verify(solver: &dyn LinearSolver) {
        for (optimal, fixed) in [(false, true), (true, false)] {
            let (model, _, y) = source(optimal, fixed);
            let options = if optimal { LinearBendersPartitionOptions { stages: BTreeMap::from([(y, LinearBendersStage::ContinuousRecourse)]) } } else { Default::default() };
            let partition = LinearBendersPartition::build(&model, options).unwrap();
            let result = FixedRecourseAlgorithm::solve(&model, &partition, Default::default(), &SolveHandle::new(), |model, remaining, handle|
                solver.solve_linear_report_with_options(model, &SolveOptions { time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default() })).unwrap();
            assert_eq!(result.stop, FixedRecourseStop::Converged);
            assert_eq!(result.objective, Some(7.5));
            if optimal { assert!(result.optimality_cuts > 0); } else { assert!(result.feasibility_cuts > 0); }
        }
    }

    #[cfg(feature = "scip")]
    #[test]
    fn scip_fixed_recourse_optimal_dual_and_farkas_are_independent_gates() {
        use ospf_rust_core::solver::solvers::scip::{SCIPConfig, SCIPSolver};
        verify(&SCIPSolver::with_config(SCIPConfig::new().with_lp_subproblem_defaults().with_output(false)));
    }

    #[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
    #[test]
    fn gurobi_fixed_recourse_optimal_dual_and_farkas_are_independent_gates() {
        use ospf_rust_core::solver::solvers::gurobi::{GurobiConfig, GurobiSolver};
        verify(&GurobiSolver::with_config(GurobiConfig::new().with_output(false)));
    }
}
