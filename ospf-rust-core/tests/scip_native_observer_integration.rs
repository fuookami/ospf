#![cfg(feature = "scip")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solver::LinearSolver;
use ospf_rust_core::solver::solvers::SCIPSolver;
use ospf_rust_core::solvers::scip::{
    SCIPConfig, SCIPNativeCallback, SCIPNativeControl, SCIPNativeObserver, SCIPNativeWhere,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId, VariableType};

fn build_mip_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let item_count = 40usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(22000 + index),
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
    model
}

#[test]
fn scip_native_observers_are_aggregated() {
    let model = build_mip_model("scip_native_observer_aggregate");
    let observer1_hits = Arc::new(AtomicUsize::new(0));
    let observer2_hits = Arc::new(AtomicUsize::new(0));
    let observer1_hits_ref = observer1_hits.clone();
    let observer2_hits_ref = observer2_hits.clone();

    let observer1: SCIPNativeObserver = Arc::new(move |snapshot| {
        if matches!(
            snapshot.where_point,
            SCIPNativeWhere::Node | SCIPNativeWhere::Lp | SCIPNativeWhere::Solution
        ) {
            observer1_hits_ref.fetch_add(1, Ordering::SeqCst);
        }
        Ok(SCIPNativeControl::Continue)
    });
    let observer2: SCIPNativeObserver = Arc::new(move |snapshot| {
        if matches!(
            snapshot.where_point,
            SCIPNativeWhere::Node | SCIPNativeWhere::Lp | SCIPNativeWhere::Solution
        ) {
            observer2_hits_ref.fetch_add(1, Ordering::SeqCst);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = SCIPSolver::with_config(
        SCIPConfig::new()
            .with_output(false)
            .add_native_observer(observer1)
            .add_native_observer(observer2),
    );
    let output = solver
        .solve_linear(&model)
        .expect("native observers solve should succeed");
    assert!(
        output.status.is_feasible(),
        "expected feasible status, got {:?}",
        output.status
    );
    let hits1 = observer1_hits.load(Ordering::SeqCst);
    let hits2 = observer2_hits.load(Ordering::SeqCst);
    assert!(hits1 > 0, "observer1 should be called");
    assert_eq!(hits1, hits2, "observers should receive same callback count");
}

#[test]
fn scip_native_observer_can_request_interrupt() {
    let model = build_mip_model("scip_native_observer_interrupt");
    let interrupt_once = Arc::new(AtomicUsize::new(0));
    let interrupt_once_ref = interrupt_once.clone();
    let observer: SCIPNativeObserver = Arc::new(move |_snapshot| {
        if interrupt_once_ref.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = SCIPSolver::with_config(
        SCIPConfig::new()
            .with_output(false)
            .add_native_observer(observer),
    );
    let _ = solver
        .solve_linear(&model)
        .expect("native observer interrupt should not crash solver");
    assert!(
        interrupt_once.load(Ordering::SeqCst) > 0,
        "interrupt observer should be called"
    );
}

#[test]
fn scip_native_callback_can_interrupt_on_node_where_point() {
    let model = build_mip_model("scip_native_callback_interrupt_on_node");
    let callback_hits = Arc::new(AtomicUsize::new(0));
    let node_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let callback_hits_ref = callback_hits.clone();
    let node_hits_ref = node_hits.clone();
    let interrupted_ref = interrupted.clone();

    let callback: SCIPNativeCallback = Arc::new(move |snapshot| {
        callback_hits_ref.fetch_add(1, Ordering::SeqCst);
        if snapshot.where_point == SCIPNativeWhere::Node {
            node_hits_ref.fetch_add(1, Ordering::SeqCst);
            if interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0 {
                return Ok(SCIPNativeControl::Interrupt);
            }
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = SCIPSolver::with_config(
        SCIPConfig::new()
            .with_output(false)
            .with_native_callback(Some(callback)),
    );
    let _ = solver
        .solve_linear(&model)
        .expect("native callback interrupt should not crash solver");
    assert!(
        callback_hits.load(Ordering::SeqCst) > 0,
        "native callback should be called"
    );
    assert!(
        node_hits.load(Ordering::SeqCst) > 0,
        "native callback should observe node where-point"
    );
    assert!(
        interrupted.load(Ordering::SeqCst) > 0,
        "native callback should request interrupt at least once"
    );
}
