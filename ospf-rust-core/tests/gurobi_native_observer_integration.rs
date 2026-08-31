#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solvers::gurobi::{
    GurobiConfig, GurobiNativeControl, GurobiNativeObserver, GurobiNativeWhere,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId, VariableType};

fn build_mip_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let item_count = 40usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(12000 + index),
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
fn gurobi_native_observers_are_aggregated() {
    let model = build_mip_model("native_observer_aggregate");
    let observer1_hits = Arc::new(AtomicUsize::new(0));
    let observer2_hits = Arc::new(AtomicUsize::new(0));
    let observer1_hits_ref = observer1_hits.clone();
    let observer2_hits_ref = observer2_hits.clone();

    let observer1: GurobiNativeObserver = Arc::new(move |snapshot| {
        if snapshot.where_point == GurobiNativeWhere::Mip {
            observer1_hits_ref.fetch_add(1, Ordering::SeqCst);
        }
        Ok(GurobiNativeControl::Continue)
    });
    let observer2: GurobiNativeObserver = Arc::new(move |snapshot| {
        if snapshot.where_point == GurobiNativeWhere::Mip {
            observer2_hits_ref.fetch_add(1, Ordering::SeqCst);
        }
        Ok(GurobiNativeControl::Continue)
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
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
fn gurobi_native_observer_can_request_terminate() {
    let model = build_mip_model("native_observer_terminate");
    let terminate_once = Arc::new(AtomicUsize::new(0));
    let terminate_once_ref = terminate_once.clone();
    let observer: GurobiNativeObserver = Arc::new(move |snapshot| {
        if snapshot.where_point == GurobiNativeWhere::Mip
            && terminate_once_ref.fetch_add(1, Ordering::SeqCst) == 0
        {
            return Ok(GurobiNativeControl::Terminate);
        }
        Ok(GurobiNativeControl::Continue)
    });

    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_output(false)
            .add_native_observer(observer),
    );
    let _ = solver
        .solve_linear(&model)
        .expect("native observer terminate should not crash solver");
    assert!(
        terminate_once.load(Ordering::SeqCst) > 0,
        "terminate observer should be called"
    );
}
