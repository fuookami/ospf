#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseVector,
};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solver::{
    linear_model_fingerprint, require_optimal_lp_certificate_for_linear_model,
    require_optimal_lp_certificate_for_model,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{
    ContinuousVariableItem, UContinuousVariableItem, VariableId, VariableType,
};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "expected {}, got {}",
        expected,
        actual
    );
}

fn sparse_row(entries: &[(usize, f64)]) -> SparseVector<f64> {
    let mut row = SparseVector::new();
    for (index, value) in entries {
        row.add(*index, *value);
    }
    row
}

#[test]
fn gurobi_solves_primal_and_dual_with_equal_objectives() {
    let mut basic = BasicLinearTriadModel::new("lp_primal");
    let x1 = UContinuousVariableItem::create(VariableId::standalone(3001), "x1");
    let x2 = UContinuousVariableItem::create(VariableId::standalone(3002), "x2");
    basic.add_variable(Token::from_generic(x1, 0));
    basic.add_variable(Token::from_generic(x2, 1));

    // x1 + x2 <= 4
    // x1 <= 2
    // x2 <= 3
    basic.add_constraint(sparse_row(&[(0, 1.0), (1, 1.0)]), 4.0);
    basic.add_constraint(sparse_row(&[(0, 1.0)]), 2.0);
    basic.add_constraint(sparse_row(&[(1, 1.0)]), 3.0);

    // max 3x1 + 2x2
    let mut primal = LinearTriadModel::from_basic(basic);
    primal.set_objective(vec![3.0, 2.0], ObjectiveCategory::Maximum);

    let dual = primal.to_dual();
    assert_eq!(dual.objective_category, ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let primal_output = solver.solve_linear(&primal).unwrap();
    let dual_output = solver.solve_linear(&dual).unwrap();

    assert!(
        primal_output.status.is_feasible(),
        "primal status: {:?}",
        primal_output.status
    );
    assert!(
        dual_output.status.is_feasible(),
        "dual status: {:?}",
        dual_output.status
    );

    let primal_obj = primal_output.objective_value.unwrap();
    let dual_obj = dual_output.objective_value.unwrap();
    assert_close(primal_obj, 10.0);
    assert_close(dual_obj, 10.0);
    assert_close(primal_obj, dual_obj);
}

#[test]
fn gurobi_solves_bounded_primal_and_dual_with_equal_objectives() {
    let mut basic = BasicLinearTriadModel::new("lp_primal_bounded");
    let x = ContinuousVariableItem::create(VariableId::standalone(3101), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        1.0,
        3.0,
        VariableType::Continuous,
    );

    // max 2x, 1 <= x <= 3
    let mut primal = LinearTriadModel::from_basic(basic);
    primal.set_objective(vec![2.0], ObjectiveCategory::Maximum);

    let dual = primal.to_dual();
    assert_eq!(dual.objective_category, ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let primal_output = solver.solve_linear(&primal).unwrap();
    let dual_output = solver.solve_linear(&dual).unwrap();

    assert!(
        primal_output.status.is_feasible(),
        "primal status: {:?}",
        primal_output.status
    );
    assert!(
        dual_output.status.is_feasible(),
        "dual status: {:?}",
        dual_output.status
    );

    let primal_obj = primal_output.objective_value.unwrap();
    let dual_obj = dual_output.objective_value.unwrap();
    assert_close(primal_obj, 6.0);
    assert_close(dual_obj, 6.0);
    assert_close(primal_obj, dual_obj);
}

#[test]
fn gurobi_solves_min_with_lower_bound_primal_and_dual_with_equal_objectives() {
    let mut basic = BasicLinearTriadModel::new("lp_primal_min_lb");
    let x = ContinuousVariableItem::create(VariableId::standalone(3201), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        1.0,
        f64::INFINITY,
        VariableType::Continuous,
    );
    // x <= 4
    basic.add_constraint(sparse_row(&[(0, 1.0)]), 4.0);

    // min x, with x >= 1 and x <= 4
    let mut primal = LinearTriadModel::from_basic(basic);
    primal.set_objective(vec![1.0], ObjectiveCategory::Minimum);

    let dual = primal.to_dual();
    assert_eq!(dual.objective_category, ObjectiveCategory::Maximum);

    let solver = GurobiSolver::new();
    let primal_output = solver.solve_linear(&primal).unwrap();
    let dual_output = solver.solve_linear(&dual).unwrap();

    assert!(
        primal_output.status.is_feasible(),
        "primal status: {:?}",
        primal_output.status
    );
    assert!(
        dual_output.status.is_feasible(),
        "dual status: {:?}",
        dual_output.status
    );

    let primal_obj = primal_output.objective_value.unwrap();
    let dual_obj = dual_output.objective_value.unwrap();
    assert_close(primal_obj, 1.0);
    assert_close(dual_obj, 1.0);
    assert_close(primal_obj, dual_obj);
}

#[test]
fn gurobi_linear_report_preserves_certificate_provenance_and_fingerprints() {
    let mut basic = BasicLinearTriadModel::new("gurobi_report_linear");
    let x = ContinuousVariableItem::create(VariableId::standalone(3301), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        0.0,
        4.0,
        VariableType::Continuous,
    );
    basic.add_constraint(sparse_row(&[(0, 1.0)]), 3.0);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Maximum);

    let report = GurobiSolver::new()
        .solve_linear_report(&model)
        .expect("Gurobi report solve should succeed");

    assert!(report.is_optimal());
    assert_eq!(report.statistics.solution_count, Some(1));
    assert!(
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.dual_solution.as_ref())
            .is_some()
    );
    assert_eq!(
        report.fingerprints.model,
        Some(linear_model_fingerprint(&model).unwrap())
    );
    assert!(report.fingerprints.configuration.is_some());
    assert!(report.fingerprints.solver.is_some());
    assert!(report.provenance.backend_version.is_some());
    require_optimal_lp_certificate_for_model(&report, &linear_model_fingerprint(&model).unwrap())
        .expect("Gurobi optimal LP report should pass the certificate gate");
    require_optimal_lp_certificate_for_linear_model(&report, &model)
        .expect("Gurobi optimal LP report should pass the strict model-bound certificate gate");
}

#[test]
fn gurobi_quadratic_report_preserves_native_optimality() {
    let mut basic = BasicLinearTriadModel::new("gurobi_report_quadratic");
    let x = ContinuousVariableItem::create(VariableId::standalone(3302), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        -2.0,
        2.0,
        VariableType::Continuous,
    );
    let mut linear = LinearTriadModel::from_basic(basic);
    linear.set_objective(vec![0.0], ObjectiveCategory::Minimum);
    let mut model =
        QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::from_linear(linear.basic));
    let mut quadratic = ospf_rust_core::model::intermediate::SparseMatrix::new();
    let mut row = SparseVector::new();
    row.add(0, 1.0);
    quadratic.add_row(row);
    model.set_objective(vec![0.0], quadratic, ObjectiveCategory::Minimum);

    let report = GurobiSolver::new()
        .solve_quadratic_report(&model)
        .expect("Gurobi quadratic report solve should succeed");

    assert!(report.is_optimal());
    assert_eq!(report.statistics.solution_count, Some(1));
    assert!(report.fingerprints.model.is_some());
    assert!(report.fingerprints.configuration.is_some());
    assert!(report.fingerprints.solver.is_some());
}
