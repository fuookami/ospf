#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

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

fn assert_model_satisfied(model: &LinearTriadModel, solution: &[f64]) {
    for (row_index, row) in model.A.rows.iter().enumerate() {
        let lhs = row
            .entries
            .iter()
            .fold(0.0, |acc, (col, coeff)| acc + coeff * solution[*col]);
        let rhs = model.b[row_index];
        assert!(
            lhs <= rhs + 1e-6,
            "constraint {} violated: lhs={}, rhs={}",
            row_index,
            lhs,
            rhs
        );
    }
}

#[test]
fn gurobi_farkas_dual_is_feasible_for_conflicting_bounds() {
    let mut basic = BasicLinearTriadModel::new("primal_bound_conflict");
    let x = ContinuousVariableItem::create(VariableId::standalone(7101), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        1.0,
        0.0,
        VariableType::Continuous,
    );

    let mut primal = LinearTriadModel::from_basic(basic);
    primal.set_objective(vec![0.0], ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let primal_output = solver.solve_linear(&primal).unwrap();
    assert!(
        primal_output.status.is_infeasible(),
        "primal status: {:?}",
        primal_output.status
    );

    let farkas = primal.to_farkas_dual();
    let farkas_output = solver.solve_linear(&farkas).unwrap();
    assert!(
        farkas_output.status.is_feasible(),
        "farkas status: {:?}",
        farkas_output.status
    );

    let solution = farkas_output.solution.unwrap();
    assert_model_satisfied(&farkas, &solution);
    assert_close(solution[0], -1.0);
    assert_close(solution[1], 1.0);
}

#[test]
fn gurobi_farkas_dual_is_feasible_for_conflicting_constraints() {
    let mut basic = BasicLinearTriadModel::new("primal_constraint_conflict");
    let x = ContinuousVariableItem::create(VariableId::standalone(7201), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        f64::NEG_INFINITY,
        f64::INFINITY,
        VariableType::Continuous,
    );

    // x <= 0
    basic.add_constraint(sparse_row(&[(0, 1.0)]), 0.0);
    // x >= 1 => -x <= -1
    basic.add_constraint(sparse_row(&[(0, -1.0)]), -1.0);

    let mut primal = LinearTriadModel::from_basic(basic);
    primal.set_objective(vec![0.0], ObjectiveCategory::Minimum);

    let solver = GurobiSolver::new();
    let primal_output = solver.solve_linear(&primal).unwrap();
    assert!(
        primal_output.status.is_infeasible(),
        "primal status: {:?}",
        primal_output.status
    );

    let farkas = primal.to_farkas_dual();
    let farkas_output = solver.solve_linear(&farkas).unwrap();
    assert!(
        farkas_output.status.is_feasible(),
        "farkas status: {:?}",
        farkas_output.status
    );

    let solution = farkas_output.solution.unwrap();
    assert_model_satisfied(&farkas, &solution);
    assert_close(solution[0], 1.0);
    assert_close(solution[1], 1.0);
    let objective = farkas_output.objective_value.unwrap();
    assert_close(objective, 2.0);
}
