//! Gurobi 后端与启发式 PSO 的回调链集成测试 / Integration test of the Gurobi backend and the heuristic PSO callback chain
//!
//! 链路：启发式骨架产生候选解 → 回调模型的目标评估器把候选写成内层线性模型的约束 → Gurobi 求解 → 目标值返回给骨架。
//! 因此 `best_solution` 与 `best_objective` 必须互相一致，这也是本测试的核心断言。
//! Chain: the heuristic skeleton produces a candidate → the callback model's objective evaluator turns the candidate into a
//! constraint of the inner linear model → Gurobi solves it → the objective returns to the skeleton. `best_solution` and
//! `best_objective` must therefore agree, which is the core assertion of this test.
//!
//! 仅在启用 Gurobi 特性时编译 / Compiled only when a Gurobi feature is enabled.

#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::callback::AbstractCallBackModel;
use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::solver::heuristic::composition::SearchConfig;
use ospf_rust_core::solver::heuristic::facade::particle_swarm;
use ospf_rust_core::solver::heuristic::{
    HeuristicSolutionStatus, HeuristicSolver, Iteration, SeededEntropy,
};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solver::LinearSolver;
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "expected {expected}, got {actual}"
    );
}

/// 对给定候选值求解内层线性模型 `min x, s.t. x = candidate, 2 <= x <= 10` / Solve the inner linear model pinning x to the candidate
///
/// 候选低于 2 时模型不可行，返回 `None`，从而让启发式学会「可行域在 x >= 2」。
/// The model is infeasible for candidates below 2, returning `None` so the heuristic learns that the feasible region is x >= 2.
fn solve_inner(x: &ContinuousVariableItem, candidate: f64) -> Option<f64> {
    let mut basic = BasicLinearTriadModel::new("gurobi_pso_callback_inner");
    basic.add_variable_with_bounds(
        Token::from_generic(x.clone(), 0),
        2.0,
        10.0,
        VariableType::Continuous,
    );

    // x = candidate  ==>  -x <= -candidate 且 x <= candidate
    // x = candidate  ==>  -x <= -candidate and x <= candidate
    let mut lower_row = SparseVector::new();
    lower_row.add(0, -1.0);
    basic.add_constraint(lower_row, -candidate);
    let mut upper_row = SparseVector::new();
    upper_row.add(0, 1.0);
    basic.add_constraint(upper_row, candidate);

    let mut linear = LinearTriadModel::from_basic(basic);
    linear.set_objective(vec![1.0], ObjectiveCategory::Minimum);

    let output = GurobiSolver::new().solve_linear(&linear).ok()?;
    output
        .solution
        .as_ref()
        .and_then(|solution| solution.first().copied())
}

#[test]
fn gurobi_pso_callback_chain_smoke() {
    let x = ContinuousVariableItem::create(VariableId::standalone(9601), "x");
    let x_id = x.id();

    let mut callback_model = AbstractCallBackModel::<f64>::new("gurobi_pso_callback_smoke");
    // 变量顺序必须显式登记，否则启发式会认为维度为 0。
    // The variable order must be registered explicitly; otherwise the heuristic sees dimension zero.
    callback_model.register_variable_name(x_id, "x".to_string());

    let evaluator_x = x.clone();
    callback_model.set_objective_evaluator(Arc::new(move |solution| {
        // 真实回调链：候选值固定为内层模型的等式约束 → Gurobi 求解 → 返回目标值；不可行时返回 None。
        // The real callback chain: the candidate pins the inner model's equality constraint → Gurobi solves → the objective
        // returns; infeasible candidates return None.
        let candidate = solution.get(&x_id).copied().unwrap_or(0.0);
        solve_inner(&evaluator_x, candidate)
    }));
    callback_model.set_initial_solution_provider(Arc::new(move |amount| {
        (0..amount)
            .map(|index| HashMap::from([(x_id, 0.5 + index as f64 * 0.5)]))
            .collect()
    }));
    callback_model.set_variable_bounds_provider(Arc::new(move |id| {
        if id == x_id {
            Some((Some(0.0), Some(10.0)))
        } else {
            None
        }
    }));

    let config = SearchConfig::new(40);
    let mut solver = particle_swarm::<f64>(
        16,
        config.stopping(),
        Box::new(SeededEntropy::new(9601)),
    );

    let result = solver
        .solve(&mut callback_model, &mut Iteration::new())
        .expect("pso should run against the callback model");

    assert_eq!(solver.name(), "pso");
    assert!(
        matches!(result.status, HeuristicSolutionStatus::Feasible),
        "status = {:?}",
        result.status
    );

    let objective = result.best_objective.expect("best objective");
    let value = result
        .best_solution
        .and_then(|solution| solution.get(&x_id).copied())
        .expect("best solution should include x");

    // 目标与解必须一致：这正是「候选 → Gurobi → 目标」链路成立的条件。
    // The objective and the solution must agree, which is what makes the "candidate → Gurobi → objective" chain hold.
    assert_close(value, 2.0);
    assert_close(objective, 2.0);
}
