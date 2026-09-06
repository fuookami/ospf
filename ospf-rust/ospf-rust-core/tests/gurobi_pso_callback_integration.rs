#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::callback::{AbstractCallBackModel, SolutionStatus};
use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::solver::heuristic::{HeuristicModelExt, HeuristicPolicy};
use ospf_rust_core::solver::solvers::{GurobiSolver, ParticleSwarmHeuristicSolver};
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

#[tokio::test]
async fn gurobi_pso_callback_chain_smoke() {
    let x = ContinuousVariableItem::create(VariableId::standalone(9601), "x");
    let x_id = x.id();

    let mut callback_model = AbstractCallBackModel::<f64>::new("gurobi_pso_callback_smoke");
    callback_model.set_solve_executor(Arc::new(move |model| {
        // 构造一个最小线性模型：min x, s.t. x >= 2, 0 <= x <= 10
        // Build a minimal linear model: min x, s.t. x >= 2, 0 <= x <= 10
        let mut basic = BasicLinearTriadModel::new("gurobi_pso_callback_inner");
        basic.add_variable_with_bounds(
            Token::from_generic(x.clone(), 0),
            0.0,
            10.0,
            VariableType::Continuous,
        );

        // x >= 2  ==>  -x <= -2
        let mut row = SparseVector::new();
        row.add(0, -1.0);
        basic.add_constraint(row, -2.0);

        let mut linear = LinearTriadModel::from_basic(basic);
        linear.set_objective(vec![1.0], ObjectiveCategory::Minimum);

        let solver = GurobiSolver::new();
        let output = solver.solve_linear(&linear)?;

        if let Some(solution) = output.solution.as_ref() {
            model.set_solution_internal(HashMap::from([(x_id, solution[0])]));
        }
        model.set_objective_internal(output.objective_value);
        Ok(output)
    }));

    let algorithm = ParticleSwarmHeuristicSolver::new(16, 1, 0.0, 0.0, 0.0, 10.0)
        .with_solve_on_objective_miss(true);
    let mut policy = HeuristicPolicy::new().with_iteration_limit(0);
    let result = callback_model
        .solve_with_heuristic(&algorithm, &mut policy)
        .await
        .expect("pso callback chain should run with gurobi backend");

    assert!(
        matches!(
            result.status,
            SolutionStatus::Optimal | SolutionStatus::Feasible
        ),
        "status = {:?}",
        result.status
    );
    assert_close(
        result.best_objective.expect("best objective should exist"),
        2.0,
    );
    assert_close(
        result
            .best_solution
            .and_then(|solution| solution.get(&x_id).copied())
            .expect("best solution should include x"),
        2.0,
    );
}
