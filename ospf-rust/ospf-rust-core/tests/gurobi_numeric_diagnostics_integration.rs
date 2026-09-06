#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::{Arc, Mutex};

use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solvers::gurobi::{
    GurobiConfig, GurobiNumericDiagnosticsCallback, GurobiNumericProfile,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn build_model(name: &str, objective: f64, coefficient: f64, rhs: f64) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let x = ContinuousVariableItem::create(VariableId::standalone(41001), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        1.0,
        1.0,
        VariableType::Continuous,
    );
    let mut row = SparseVector::new();
    row.add(0, coefficient);
    basic.add_constraint(row, rhs);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![objective], ObjectiveCategory::Minimum);
    model
}

#[test]
fn gurobi_numeric_diagnostics_callback_reports_recommended_profile() {
    let model = build_model("numeric_diagnostics_profile", 1e-12, 1e12, 1e12);
    let profiles: Arc<Mutex<Vec<GurobiNumericProfile>>> = Arc::new(Mutex::new(Vec::new()));
    let profiles_ref = profiles.clone();
    let callback: GurobiNumericDiagnosticsCallback = Arc::new(move |diagnostics| {
        profiles_ref
            .lock()
            .expect("profile lock poisoned")
            .push(diagnostics.recommended_profile);
        Ok(())
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_numeric_diagnostics(true)
            .with_auto_apply_numeric_profile(true)
            .with_numeric_diagnostics_callback(Some(callback)),
    );
    let output = solver
        .solve_linear(&model)
        .expect("numeric diagnostics solve should succeed");
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );
    assert!(
        output
            .objective_value
            .expect("objective value should be present")
            .abs()
            <= 1e-12
    );

    let profiles = profiles.lock().expect("profile lock poisoned");
    assert!(
        !profiles.is_empty(),
        "expected diagnostics callback to be called"
    );
    assert_eq!(profiles[0], GurobiNumericProfile::Robust);
}

#[test]
fn gurobi_numeric_diagnostics_rejects_non_finite_model_coefficients() {
    let model = build_model("numeric_diagnostics_non_finite", f64::NAN, 1.0, 1.0);
    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_numeric_diagnostics(true),
    );
    let error = solver
        .solve_linear(&model)
        .expect_err("non-finite model coefficients should fail");
    let message = error.to_string();
    assert!(
        message.to_lowercase().contains("non-finite"),
        "unexpected error message: {}",
        message
    );
}
