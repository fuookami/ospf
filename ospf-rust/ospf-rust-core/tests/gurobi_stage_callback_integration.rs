#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::{Arc, Mutex};

use ospf_rust_core::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solvers::gurobi::{GurobiConfig, GurobiStage, GurobiStageCallback};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableType};

fn assert_linear_stages(stages: &[GurobiStage]) {
    assert!(
        stages.contains(&GurobiStage::AfterModeling),
        "missing AfterModeling stage, got {:?}",
        stages
    );
    assert!(
        stages.contains(&GurobiStage::Configuration),
        "missing Configuration stage, got {:?}",
        stages
    );
    assert!(
        stages.contains(&GurobiStage::AnalyzingSolution),
        "missing AnalyzingSolution stage, got {:?}",
        stages
    );
}

#[test]
fn gurobi_linear_stage_callback_reports_pipeline_stages() {
    let mut basic = BasicLinearTriadModel::new("stage_linear");
    let x = ContinuousVariableItem::create(VariableId::standalone(30001), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(x, 0),
        0.0,
        5.0,
        VariableType::Continuous,
    );
    let mut row = SparseVector::new();
    row.add(0, 1.0);
    basic.add_constraint(row, 3.0);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Maximum);

    let stages: Arc<Mutex<Vec<GurobiStage>>> = Arc::new(Mutex::new(Vec::new()));
    let stages_ref = stages.clone();
    let callback: GurobiStageCallback = Arc::new(move |status| {
        stages_ref.lock().unwrap().push(status.stage);
        Ok(())
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_stage_callback(Some(callback)),
    );
    let output = solver
        .solve_linear(&model)
        .expect("linear solve with stage callback should succeed");
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );

    let stages = stages.lock().unwrap();
    assert_linear_stages(stages.as_slice());
}

#[test]
fn gurobi_quadratic_stage_callback_reports_pipeline_stages() {
    let mut basic = BasicQuadraticTetradModel::new("stage_quadratic");
    let x = ContinuousVariableItem::create(VariableId::standalone(30011), "x");
    basic.linear.add_variable_with_bounds(
        Token::from_generic(x, 0),
        0.0,
        10.0,
        VariableType::Continuous,
    );

    let mut model = QuadraticTetradModel::from_basic(basic);
    let mut q = SparseMatrix::new();
    let mut q_row = SparseVector::new();
    q_row.add(0, 1.0);
    q.add_row(q_row);
    model.set_objective(vec![0.0], q, ObjectiveCategory::Minimum);

    let stages: Arc<Mutex<Vec<GurobiStage>>> = Arc::new(Mutex::new(Vec::new()));
    let stages_ref = stages.clone();
    let callback: GurobiStageCallback = Arc::new(move |status| {
        stages_ref.lock().unwrap().push(status.stage);
        Ok(())
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .with_stage_callback(Some(callback)),
    );
    let output = solver
        .solve_quadratic(&model)
        .expect("quadratic solve with stage callback should succeed");
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );

    let stages = stages.lock().unwrap();
    assert_linear_stages(stages.as_slice());
}
