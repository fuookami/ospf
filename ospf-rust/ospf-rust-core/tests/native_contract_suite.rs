//! Gurobi/SCIP 共享原生求解合同夹具 / Shared native solver contract fixtures for Gurobi/SCIP.

#![allow(dead_code)]

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};
use ospf_rust_core::solver::{
    LinearSolver, ProblemStatus, QuadraticSolver, linear_model_fingerprint,
    quadratic_model_fingerprint, require_infeasibility_certificate_for_model,
    require_optimal_lp_certificate_for_linear_model,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn optimal_model() -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new("shared_native_contract_optimal");
    let variable = ContinuousVariableItem::create(VariableId::standalone(3601), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        0.0,
        4.0,
        VariableType::Continuous,
    );
    let mut row = SparseVector::new();
    row.add(0, 1.0);
    basic.add_constraint(row, 3.0);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
    model
}

fn infeasible_model() -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new("shared_native_contract_infeasible");
    let variable = ContinuousVariableItem::create(VariableId::standalone(3602), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );
    let mut upper = SparseVector::new();
    upper.add(0, 1.0);
    basic.add_constraint(upper, 0.0);
    let mut lower = SparseVector::new();
    lower.add(0, -1.0);
    basic.add_constraint(lower, -1.0);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![0.0], ObjectiveCategory::Minimum);
    model
}

fn quadratic_model() -> QuadraticTetradModel {
    let mut basic = BasicLinearTriadModel::new("shared_native_contract_quadratic");
    let variable = ContinuousVariableItem::create(VariableId::standalone(3603), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        -2.0,
        2.0,
        VariableType::Continuous,
    );
    let mut linear = LinearTriadModel::from_basic(basic);
    linear.set_objective(vec![0.0], ObjectiveCategory::Minimum);
    let mut model =
        QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::from_linear(linear.basic));
    let mut objective = SparseMatrix::new();
    let mut row = SparseVector::new();
    row.add(0, 1.0);
    objective.add_row(row);
    model.set_objective(vec![0.0], objective, ObjectiveCategory::Minimum);
    model
}

fn assert_native_linear_contract<S>(solver: &S)
where
    S: LinearSolver + QuadraticSolver,
{
    let model = optimal_model();
    let model_fingerprint = linear_model_fingerprint(&model).expect("model fingerprint");
    let report = solver
        .solve_linear_report(&model)
        .expect("native optimal report should be returned");

    assert!(report.is_optimal());
    assert!(report.statistics.solution_count.unwrap_or_default() >= 1);
    assert!(
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.dual_solution.as_ref())
            .is_some()
    );
    assert_eq!(report.fingerprints.model.as_ref(), Some(&model_fingerprint));
    assert!(report.fingerprints.configuration.is_some());
    assert!(report.fingerprints.solver.is_some());
    assert!(!report.provenance.backend_name.is_empty());
    assert!(report.provenance.backend_version.is_some());
    require_optimal_lp_certificate_for_linear_model(&report, &model)
        .expect("native optimal LP report should pass the shared certificate gate");

    let infeasible = infeasible_model();
    let infeasible_fingerprint =
        linear_model_fingerprint(&infeasible).expect("infeasible model fingerprint");
    let infeasible_report = solver
        .solve_linear_report(&infeasible)
        .expect("native infeasible report should be returned");
    assert_eq!(infeasible_report.problem_status, ProblemStatus::Infeasible);
    assert!(!infeasible_report.has_incumbent());
    assert_eq!(
        infeasible_report.fingerprints.model.as_ref(),
        Some(&infeasible_fingerprint)
    );
    require_infeasibility_certificate_for_model(&infeasible_report, &infeasible_fingerprint)
        .expect("native infeasible report should pass the shared certificate gate");

    let quadratic = quadratic_model();
    let quadratic_fingerprint =
        quadratic_model_fingerprint(&quadratic).expect("quadratic model fingerprint");
    let quadratic_report = solver
        .solve_quadratic_report(&quadratic)
        .expect("native quadratic report should be returned");
    assert!(quadratic_report.is_optimal());
    assert!(
        quadratic_report
            .statistics
            .solution_count
            .unwrap_or_default()
            >= 1
    );
    assert_eq!(
        quadratic_report.fingerprints.model.as_ref(),
        Some(&quadratic_fingerprint)
    );
    assert!(quadratic_report.fingerprints.configuration.is_some());
    assert!(quadratic_report.fingerprints.solver.is_some());
}

fn assert_native_replay_contract<S>(solver: &S)
where
    S: LinearSolver,
{
    let model = optimal_model();
    let first = solver
        .solve_linear_report(&model)
        .expect("first native replay solve should return a report");
    let second = solver
        .solve_linear_report(&model)
        .expect("second native replay solve should return a report");

    // Runtime counters and wall-clock duration are intentionally excluded from replay identity.
    // 数学结论、指纹和证书来源必须稳定；耗时、节点数等运行时计数不参与重放比较。
    assert_eq!(first.problem_status, second.problem_status);
    assert_eq!(first.termination_reason, second.termination_reason);
    assert_eq!(first.proof, second.proof);
    assert_eq!(first.fingerprints, second.fingerprints);
    assert_eq!(first.provenance, second.provenance);
    assert_optional_close(
        first.statistics.best_bound_value,
        second.statistics.best_bound_value,
        1e-7,
        "best bound",
    );
    assert_optional_close(
        first.statistics.absolute_gap,
        second.statistics.absolute_gap,
        1e-7,
        "absolute gap",
    );
    assert_optional_close(
        first.statistics.relative_gap,
        second.statistics.relative_gap,
        1e-7,
        "relative gap",
    );
    let first_solution = first
        .solution
        .as_ref()
        .expect("first replay report should carry an incumbent");
    let second_solution = second
        .solution
        .as_ref()
        .expect("second replay report should carry an incumbent");
    assert_close_vectors(
        &first_solution.values,
        &second_solution.values,
        1e-7,
        "solution",
    );
    assert_eq!(
        first.diagnostics.constraint_evaluations.len(),
        second.diagnostics.constraint_evaluations.len()
    );
    for (first_evaluation, second_evaluation) in first
        .diagnostics
        .constraint_evaluations
        .iter()
        .zip(&second.diagnostics.constraint_evaluations)
    {
        assert_eq!(
            first_evaluation.constraint_id,
            second_evaluation.constraint_id
        );
        assert_eq!(first_evaluation.satisfied, second_evaluation.satisfied);
        assert_close(
            first_evaluation.residual,
            second_evaluation.residual,
            1e-7,
            "constraint residual",
        );
        assert_close(
            first_evaluation.violation,
            second_evaluation.violation,
            1e-7,
            "constraint violation",
        );
    }
    assert_eq!(
        first
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value),
        second
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value)
    );
    let objective = first
        .solution
        .as_ref()
        .and_then(|solution| solution.objective_value)
        .expect("replay fixture should have an incumbent objective");
    assert!((objective - 3.0).abs() <= 1e-7);
    require_optimal_lp_certificate_for_linear_model(&first, &model)
        .expect("first replay report should retain its optimality certificate");
    require_optimal_lp_certificate_for_linear_model(&second, &model)
        .expect("second replay report should retain its optimality certificate");
}

fn assert_close(left: f64, right: f64, tolerance: f64, field: &str) {
    assert!(
        (left - right).abs() <= tolerance,
        "{} differs: left={}, right={}, tolerance={}",
        field,
        left,
        right,
        tolerance
    );
}

fn assert_optional_close(left: Option<f64>, right: Option<f64>, tolerance: f64, field: &str) {
    match (left, right) {
        (Some(left), Some(right)) => assert_close(left, right, tolerance, field),
        (None, None) => {}
        (left, right) => panic!(
            "{} presence differs: left={:?}, right={:?}",
            field, left, right
        ),
    }
}

fn assert_close_vectors(left: &[f64], right: &[f64], tolerance: f64, field: &str) {
    assert_eq!(left.len(), right.len(), "{} length differs", field);
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        assert_close(*left, *right, tolerance, &format!("{}[{}]", field, index));
    }
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
#[test]
fn gurobi_uses_the_shared_native_contract_suite() {
    use ospf_rust_core::solver::solvers::GurobiSolver;
    use ospf_rust_core::solvers::gurobi::GurobiConfig;

    let solver = GurobiSolver::with_config(GurobiConfig::new().with_output(false));
    assert_native_linear_contract(&solver);
    assert_native_replay_contract(&solver);
}

#[cfg(feature = "scip")]
#[test]
fn scip_uses_the_shared_native_contract_suite() {
    use ospf_rust_core::solver::solvers::SCIPSolver;
    use ospf_rust_core::solvers::scip::SCIPConfig;

    let solver = SCIPSolver::with_config(
        SCIPConfig::recommended_lp_subproblem_defaults().with_output(false),
    );
    assert_native_linear_contract(&solver);
    assert_native_replay_contract(&solver);
}

#[cfg(not(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
)))]
#[test]
#[ignore = "native contract is unsupported without a Gurobi or SCIP feature"]
fn native_contract_requires_explicit_backend_feature() {
    panic!("native contract is unsupported without a Gurobi or SCIP feature");
}
