#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solvers::gurobi::GurobiConfig;
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn build_fixed_linear_model(name: &str, coefficient: f64) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let x = ContinuousVariableItem::create(VariableId::standalone(40001), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        1.0,
        1.0,
        VariableType::Continuous,
    );

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![coefficient], ObjectiveCategory::Minimum);
    model
}

#[test]
fn gurobi_numeric_profiles_contrast_tiny_objective_coefficient() {
    let coefficient = 5e-11;
    let model = build_fixed_linear_model("numeric_profile_tiny_obj", coefficient);

    let robust_solver =
        GurobiSolver::with_config(GurobiConfig::robust_defaults().with_output(false));
    let robust_output = robust_solver
        .solve_linear(&model)
        .expect("robust profile solve should succeed");
    assert!(
        robust_output.status.is_feasible(),
        "robust profile status: {:?}",
        robust_output.status
    );
    let robust_objective = robust_output
        .objective_value
        .expect("robust profile should provide objective value");

    let performance_solver =
        GurobiSolver::with_config(GurobiConfig::performance_defaults().with_output(false));
    let performance_output = performance_solver
        .solve_linear(&model)
        .expect("performance profile solve should succeed");
    assert!(
        performance_output.status.is_feasible(),
        "performance profile status: {:?}",
        performance_output.status
    );
    let performance_objective = performance_output
        .objective_value
        .expect("performance profile should provide objective value");

    assert!(
        robust_objective.abs() <= 1e-12,
        "robust profile should filter tiny objective term, got {}",
        robust_objective
    );
    assert!(
        performance_objective > 1e-11,
        "performance profile should keep tiny objective term, got {}",
        performance_objective
    );
}

#[test]
fn gurobi_numeric_profiles_contrast_middle_tiny_objective_coefficient() {
    let coefficient = 5e-13;
    let model = build_fixed_linear_model("numeric_profile_middle_tiny_obj", coefficient);

    let robust_solver =
        GurobiSolver::with_config(GurobiConfig::robust_defaults().with_output(false));
    let robust_objective = robust_solver
        .solve_linear(&model)
        .expect("robust profile solve should succeed")
        .objective_value
        .expect("robust profile should provide objective value");

    let balanced_solver =
        GurobiSolver::with_config(GurobiConfig::balanced_defaults().with_output(false));
    let balanced_objective = balanced_solver
        .solve_linear(&model)
        .expect("balanced profile solve should succeed")
        .objective_value
        .expect("balanced profile should provide objective value");

    let performance_solver =
        GurobiSolver::with_config(GurobiConfig::performance_defaults().with_output(false));
    let performance_objective = performance_solver
        .solve_linear(&model)
        .expect("performance profile solve should succeed")
        .objective_value
        .expect("performance profile should provide objective value");

    assert!(
        robust_objective.abs() <= 1e-12,
        "robust profile should filter this tiny objective term, got {}",
        robust_objective
    );
    assert!(
        balanced_objective.abs() <= 1e-12,
        "balanced profile should filter this tiny objective term, got {}",
        balanced_objective
    );
    assert!(
        performance_objective > 1e-13,
        "performance profile should keep this tiny objective term, got {}",
        performance_objective
    );
}
