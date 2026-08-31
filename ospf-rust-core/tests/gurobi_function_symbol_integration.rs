#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::Arc;

use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{ConstraintRelation, LinearConstraint, LinearInequality, MetaModel};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::symbol::functions::{CosFunction, ModFunction, RoundingFunction, SinFunction};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableRange};

fn equality_constraint(index: usize, rhs: f64, name: &str) -> LinearConstraint<f64> {
    LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0),
            ConstraintRelation::Equal,
            rhs,
        ),
        name,
    )
}

#[test]
fn gurobi_solves_trigonometric_function_symbol_model() {
    let mut model = MetaModel::<f64>::new("gurobi_trig_integration");

    let sin_fn = SinFunction::new(7000, "sin_zero", Linear::new(vec![], 0.0));
    let sin_id = sin_fn.result_variable().id();
    model.add_symbol(Arc::new(sin_fn)).unwrap();

    let cos_fn = CosFunction::new(7001, "cos_zero", Linear::new(vec![], 0.0));
    let cos_id = cos_fn.result_variable().id();
    model.add_symbol(Arc::new(cos_fn)).unwrap();

    let mut mechanism = model.try_into_mechanism_model().unwrap();
    let sin_index = mechanism.find_token(sin_id).unwrap().solver_index;
    let cos_index = mechanism.find_token(cos_id).unwrap().solver_index;

    mechanism.add_constraint(equality_constraint(sin_index, 0.0, "sin_target"));
    mechanism.add_constraint(equality_constraint(cos_index, 1.0, "cos_target"));

    let linear = mechanism.into_linear_triad_model();
    let solver = GurobiSolver::new();
    let output = solver
        .solve_linear(&linear)
        .expect("gurobi should solve trigonometric integration model");

    assert!(
        output.status.is_feasible(),
        "expected feasible solve status, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .expect("expected a feasible solution vector");
    assert!((solution[sin_index] - 0.0).abs() <= 1e-6);
    assert!((solution[cos_index] - 1.0).abs() <= 1e-6);
}

#[test]
fn gurobi_solves_rounding_and_mod_function_symbol_model() {
    let mut model = MetaModel::<f64>::new("gurobi_round_mod_integration");

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(7100),
        "x_fixed",
        VariableRange::fixed(5.0),
    );
    let x_index = model.register_variable(x).unwrap();

    let input = Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0);
    let floor_fn = RoundingFunction::floor(7101, "x_floor", input.clone());
    let floor_id = floor_fn.result_variable().id();
    model.add_symbol(Arc::new(floor_fn)).unwrap();

    let mod_fn = ModFunction::new(7102, "x_mod2", input, 2.0);
    let mod_id = mod_fn.result_variable().id();
    model.add_symbol(Arc::new(mod_fn)).unwrap();

    let mut mechanism = model.try_into_mechanism_model().unwrap();
    let floor_index = mechanism.find_token(floor_id).unwrap().solver_index;
    let mod_index = mechanism.find_token(mod_id).unwrap().solver_index;

    mechanism.add_constraint(equality_constraint(floor_index, 5.0, "floor_target"));
    mechanism.add_constraint(equality_constraint(mod_index, 1.0, "mod_target"));

    let linear = mechanism.into_linear_triad_model();
    let solver = GurobiSolver::new();
    let output = solver
        .solve_linear(&linear)
        .expect("gurobi should solve rounding+mod integration model");

    assert!(
        output.status.is_feasible(),
        "expected feasible solve status, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .expect("expected a feasible solution vector");
    assert!((solution[floor_index] - 5.0).abs() <= 1e-6);
    assert!((solution[mod_index] - 1.0).abs() <= 1e-6);
}
