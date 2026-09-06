#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::{Arc, Mutex};

use ospf_rust_core::error::{CoreError, SolverError};
use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solvers::gurobi::{
    GurobiConfig, GurobiTelemetryCallback, GurobiTelemetryStatus,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId, VariableType};

#[test]
fn gurobi_mip_telemetry_callback_reports_progress_snapshots() {
    let mut basic = BasicLinearTriadModel::new("telemetry_knapsack");
    let item_count = 40usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(9600 + index),
            &format!("x_{}", index),
        );
        basic.add_variable_with_bounds(
            Token::from_generic(variable, index),
            0.0,
            1.0,
            VariableType::Binary,
        );
    }

    let mut weight_row = SparseVector::new();
    let mut diversity_row = SparseVector::new();
    let mut objective = vec![0.0; item_count];
    for index in 0..item_count {
        let weight = ((index % 7) + 1) as f64;
        let diversity = ((index % 5) + 1) as f64;
        let profit = ((index * 11 + 7) % 19 + 1) as f64;
        weight_row.add(index, weight);
        diversity_row.add(index, diversity);
        objective[index] = profit;
    }
    basic.add_constraint(weight_row, 70.0);
    basic.add_constraint(diversity_row, 55.0);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(objective, ObjectiveCategory::Maximum);

    let snapshots: Arc<Mutex<Vec<GurobiTelemetryStatus>>> = Arc::new(Mutex::new(Vec::new()));
    let snapshots_for_callback = snapshots.clone();
    let telemetry_callback: GurobiTelemetryCallback = Arc::new(move |status| {
        snapshots_for_callback.lock().unwrap().push(status.clone());
        Ok(())
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_telemetry_callback(Some(telemetry_callback)),
    );
    let output = solver
        .solve_linear(&model)
        .expect("telemetry callback solve should succeed");
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );

    let snapshots = snapshots.lock().unwrap();
    assert!(
        !snapshots.is_empty(),
        "telemetry callback should receive at least one snapshot"
    );
    assert!(
        snapshots
            .windows(2)
            .all(|window| window[1].solve_time >= window[0].solve_time),
        "telemetry snapshots should have non-decreasing solve_time"
    );
    assert!(
        snapshots.iter().any(|status| status.node_count.is_some()),
        "telemetry snapshots should include node count"
    );
}

#[test]
fn gurobi_mip_telemetry_callback_respects_min_interval() {
    let mut basic = BasicLinearTriadModel::new("telemetry_knapsack_throttled");
    let item_count = 40usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(9700 + index),
            &format!("x_{}", index),
        );
        basic.add_variable_with_bounds(
            Token::from_generic(variable, index),
            0.0,
            1.0,
            VariableType::Binary,
        );
    }

    let mut weight_row = SparseVector::new();
    let mut diversity_row = SparseVector::new();
    let mut objective = vec![0.0; item_count];
    for index in 0..item_count {
        let weight = ((index % 7) + 1) as f64;
        let diversity = ((index % 5) + 1) as f64;
        let profit = ((index * 11 + 7) % 19 + 1) as f64;
        weight_row.add(index, weight);
        diversity_row.add(index, diversity);
        objective[index] = profit;
    }
    basic.add_constraint(weight_row, 70.0);
    basic.add_constraint(diversity_row, 55.0);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(objective, ObjectiveCategory::Maximum);

    let snapshots: Arc<Mutex<Vec<GurobiTelemetryStatus>>> = Arc::new(Mutex::new(Vec::new()));
    let snapshots_for_callback = snapshots.clone();
    let telemetry_callback: GurobiTelemetryCallback = Arc::new(move |status| {
        snapshots_for_callback.lock().unwrap().push(status.clone());
        Ok(())
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_telemetry_callback(Some(telemetry_callback))
            .with_telemetry_min_interval(3600.0),
    );
    let output = solver
        .solve_linear(&model)
        .expect("throttled telemetry callback solve should succeed");
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );

    let snapshots = snapshots.lock().unwrap();
    assert_eq!(
        snapshots.len(),
        1,
        "large telemetry interval should throttle snapshots to first emit"
    );
}

#[test]
fn gurobi_mip_telemetry_callback_error_is_propagated() {
    let mut basic = BasicLinearTriadModel::new("telemetry_knapsack_error");
    let item_count = 30usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(9800 + index),
            &format!("x_{}", index),
        );
        basic.add_variable_with_bounds(
            Token::from_generic(variable, index),
            0.0,
            1.0,
            VariableType::Binary,
        );
    }

    let mut weight_row = SparseVector::new();
    let mut diversity_row = SparseVector::new();
    let mut objective = vec![0.0; item_count];
    for index in 0..item_count {
        let weight = ((index % 7) + 1) as f64;
        let diversity = ((index % 5) + 1) as f64;
        let profit = ((index * 11 + 7) % 19 + 1) as f64;
        weight_row.add(index, weight);
        diversity_row.add(index, diversity);
        objective[index] = profit;
    }
    basic.add_constraint(weight_row, 52.0);
    basic.add_constraint(diversity_row, 44.0);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(objective, ObjectiveCategory::Maximum);

    let telemetry_callback: GurobiTelemetryCallback = Arc::new(|_| {
        Err(CoreError::Solver(SolverError::SolveFailed(
            "telemetry callback expected failure".to_string(),
        )))
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_telemetry_callback(Some(telemetry_callback)),
    );
    let err = solver
        .solve_linear(&model)
        .expect_err("solver should fail when telemetry callback returns error");
    let message = err.to_string();
    assert!(
        message.to_lowercase().contains("callback"),
        "unexpected error message: {}",
        message
    );
}
