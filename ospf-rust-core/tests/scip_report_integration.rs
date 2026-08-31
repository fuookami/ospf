#![cfg(feature = "scip")]

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::solver::solvers::SCIPSolver;
use ospf_rust_core::solver::{
    ProblemStatus, linear_model_fingerprint, require_infeasibility_certificate_for_model,
    require_optimal_lp_certificate_for_linear_model, require_optimal_lp_certificate_for_model,
};
use ospf_rust_core::solvers::scip::SCIPConfig;
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn sparse_row(entries: &[(usize, f64)]) -> SparseVector<f64> {
    let mut row = SparseVector::new();
    for (index, value) in entries {
        row.add(*index, *value);
    }
    row
}

fn optimal_model() -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new("scip_report_optimal");
    let x = ContinuousVariableItem::create(VariableId::standalone(3401), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        0.0,
        4.0,
        VariableType::Continuous,
    );
    basic.add_constraint(sparse_row(&[(0, 1.0)]), 3.0);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
    model
}

fn infeasible_model() -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new("scip_report_infeasible");
    let x = ContinuousVariableItem::create(VariableId::standalone(3402), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );
    basic.add_constraint(sparse_row(&[(0, 1.0)]), 0.0);
    basic.add_constraint(sparse_row(&[(0, -1.0)]), -1.0);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![0.0], ObjectiveCategory::Minimum);
    model
}

fn solver() -> SCIPSolver {
    SCIPSolver::with_config(SCIPConfig::recommended_lp_subproblem_defaults().with_output(false))
}

#[test]
fn scip_linear_report_preserves_optimal_dual_and_provenance() {
    let model = optimal_model();
    let fingerprint = linear_model_fingerprint(&model).expect("model fingerprint");
    let report = solver()
        .solve_linear_report(&model)
        .expect("SCIP report solve should succeed");

    assert!(report.is_optimal());
    assert!(report.statistics.solution_count.unwrap_or_default() >= 1);
    assert!(
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.dual_solution.as_ref())
            .is_some()
    );
    assert_eq!(report.fingerprints.model.as_ref(), Some(&fingerprint));
    assert!(report.fingerprints.configuration.is_some());
    assert!(report.fingerprints.solver.is_some());
    assert!(report.provenance.backend_version.is_some());
    require_optimal_lp_certificate_for_model(&report, &fingerprint)
        .expect("SCIP optimal LP report should pass the certificate gate");
    require_optimal_lp_certificate_for_linear_model(&report, &model)
        .expect("SCIP optimal LP report should pass the strict model-bound certificate gate");
}

#[test]
fn scip_linear_report_preserves_verified_infeasibility() {
    let model = infeasible_model();
    let fingerprint = linear_model_fingerprint(&model).expect("model fingerprint");
    let report = solver()
        .solve_linear_report(&model)
        .expect("SCIP infeasible report should be returned normally");

    assert_eq!(report.problem_status, ProblemStatus::Infeasible);
    assert!(!report.has_incumbent());
    assert!(report.proof.is_some());
    assert_eq!(report.fingerprints.model.as_ref(), Some(&fingerprint));
    require_infeasibility_certificate_for_model(&report, &fingerprint)
        .expect("SCIP infeasibility report should pass the certificate gate");
}
