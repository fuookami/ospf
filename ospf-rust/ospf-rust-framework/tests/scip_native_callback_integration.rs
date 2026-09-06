#![cfg(feature = "scip")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::solvers::scip::{
    SCIPConfig, SCIPNativeCallback, SCIPNativeControl, SCIPNativeObserver, SCIPNativeWhere,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId, VariableType};
use ospf_rust_framework::{
    ColumnGenerationSolver, LinearBendersDecompositionSolver, ScipColumnGenerationSolver,
    ScipLinearBendersDecompositionSolver, ScipQuadraticBendersDecompositionSolver, SolveOptions,
};

#[cfg(feature = "async")]
fn resolve<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build")
        .block_on(future)
}

#[cfg(not(feature = "async"))]
fn resolve<T>(value: T) -> T {
    value
}

fn build_mip_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let item_count = 36usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(32000 + index),
            &format!("x_{}", index),
        );
        basic.add_variable_with_bounds(
            Token::from_generic(variable, index),
            0.0,
            1.0,
            VariableType::Binary,
        );
    }

    let mut row1 = SparseVector::new();
    let mut row2 = SparseVector::new();
    let mut objective = vec![0.0; item_count];
    for index in 0..item_count {
        row1.add(index, ((index % 6) + 1) as f64);
        row2.add(index, ((index % 4) + 1) as f64);
        objective[index] = ((index * 13 + 3) % 17 + 1) as f64;
    }
    basic.add_constraint(row1, 56.0);
    basic.add_constraint(row2, 44.0);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(objective, ObjectiveCategory::Maximum);
    model
}

#[test]
fn scip_column_generation_native_callback_can_interrupt_on_node_where_point() {
    let model = build_mip_model("framework_scip_native_callback_interrupt_on_node");
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

    let solver = ScipColumnGenerationSolver::new()
        .with_native_callback(Some(callback))
        .with_telemetry_min_interval(0.1);
    let result = resolve(solver.solve_milp_with_options(&model, SolveOptions::default()));
    assert!(
        result.is_err(),
        "framework solve should surface user interrupt as error"
    );
    let error_text = format!("{:?}", result.err().expect("error expected"));
    assert!(
        error_text.contains("UserInterrupt"),
        "expected UserInterrupt in error, got: {}",
        error_text
    );
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

#[test]
fn scip_column_generation_native_observers_are_aggregated() {
    let model = build_mip_model("framework_scip_native_observer_aggregate");
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

    let solver = ScipColumnGenerationSolver::new()
        .add_native_observer(observer1)
        .add_native_observer(observer2)
        .with_telemetry_min_interval(0.1);
    let result = resolve(solver.solve_milp_with_options(&model, SolveOptions::default()))
        .expect("framework native observers should solve");
    assert!(result.obj.is_finite(), "expected finite objective");
    let hits1 = observer1_hits.load(Ordering::SeqCst);
    let hits2 = observer2_hits.load(Ordering::SeqCst);
    assert!(hits1 > 0, "observer1 should be called");
    assert_eq!(hits1, hits2, "observers should receive same callback count");
}

#[test]
fn scip_column_generation_native_observer_interrupt_surfaces_user_interrupt_error() {
    let model = build_mip_model("framework_scip_native_observer_interrupt");
    let interrupt_once = Arc::new(AtomicUsize::new(0));
    let node_hits = Arc::new(AtomicUsize::new(0));
    let interrupt_once_ref = interrupt_once.clone();
    let node_hits_ref = node_hits.clone();

    let observer: SCIPNativeObserver = Arc::new(move |snapshot| {
        if snapshot.where_point == SCIPNativeWhere::Node {
            node_hits_ref.fetch_add(1, Ordering::SeqCst);
            if interrupt_once_ref.fetch_add(1, Ordering::SeqCst) == 0 {
                return Ok(SCIPNativeControl::Interrupt);
            }
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = ScipColumnGenerationSolver::new()
        .add_native_observer(observer)
        .with_telemetry_min_interval(0.1);
    let result = resolve(solver.solve_milp_with_options(&model, SolveOptions::default()));
    assert!(
        result.is_err(),
        "framework solve should surface user interrupt as error"
    );
    let error_text = format!("{:?}", result.err().expect("error expected"));
    assert!(
        error_text.contains("UserInterrupt"),
        "expected UserInterrupt in error, got: {}",
        error_text
    );
    assert!(
        node_hits.load(Ordering::SeqCst) > 0,
        "observer should observe node where-point"
    );
    assert!(
        interrupt_once.load(Ordering::SeqCst) > 0,
        "observer should request interrupt at least once"
    );
}

#[test]
fn scip_linear_benders_native_callback_interrupt_surfaces_user_interrupt_error() {
    let model = build_mip_model("framework_scip_linear_benders_native_callback_interrupt");
    let callback_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let callback_hits_ref = callback_hits.clone();
    let interrupted_ref = interrupted.clone();

    let callback: SCIPNativeCallback = Arc::new(move |snapshot| {
        callback_hits_ref.fetch_add(1, Ordering::SeqCst);
        if matches!(
            snapshot.where_point,
            SCIPNativeWhere::Node
                | SCIPNativeWhere::Lp
                | SCIPNativeWhere::Solution
                | SCIPNativeWhere::Other
        ) && interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0
        {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver =
        ScipLinearBendersDecompositionSolver::with_config(SCIPConfig::new().with_output(false))
            .with_native_callback(Some(callback))
            .with_telemetry_min_interval(0.1);
    let result = resolve(solver.solve_master(&model, &[]));
    match result {
        Ok(output) => {
            assert!(
                matches!(
                    output.status,
                    ospf_rust_core::solver::SolverStatus::UserInterrupt
                ),
                "expected UserInterrupt status, got {:?}",
                output.status
            );
        }
        Err(error) => {
            let error_text = format!("{:?}", error);
            assert!(
                error_text.contains("UserInterrupt"),
                "expected UserInterrupt in error, got: {}",
                error_text
            );
        }
    }
    assert!(callback_hits.load(Ordering::SeqCst) > 0);
    assert!(interrupted.load(Ordering::SeqCst) > 0);
}

#[test]
fn scip_quadratic_benders_native_callback_interrupt_surfaces_user_interrupt_error() {
    let model = build_mip_model("framework_scip_quadratic_benders_native_callback_interrupt");
    let callback_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let callback_hits_ref = callback_hits.clone();
    let interrupted_ref = interrupted.clone();

    let callback: SCIPNativeCallback = Arc::new(move |snapshot| {
        callback_hits_ref.fetch_add(1, Ordering::SeqCst);
        if matches!(
            snapshot.where_point,
            SCIPNativeWhere::Node
                | SCIPNativeWhere::Lp
                | SCIPNativeWhere::Solution
                | SCIPNativeWhere::Other
        ) && interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0
        {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver =
        ScipQuadraticBendersDecompositionSolver::with_config(SCIPConfig::new().with_output(false))
            .with_native_callback(Some(callback))
            .with_telemetry_min_interval(0.1);
    let result = resolve(solver.solve_master(&model, &[]));
    match result {
        Ok(output) => {
            assert!(
                matches!(
                    output.status,
                    ospf_rust_core::solver::SolverStatus::UserInterrupt
                ),
                "expected UserInterrupt status, got {:?}",
                output.status
            );
        }
        Err(error) => {
            let error_text = format!("{:?}", error);
            assert!(
                error_text.contains("UserInterrupt"),
                "expected UserInterrupt in error, got: {}",
                error_text
            );
        }
    }
    assert!(callback_hits.load(Ordering::SeqCst) > 0);
    assert!(interrupted.load(Ordering::SeqCst) > 0);
}
