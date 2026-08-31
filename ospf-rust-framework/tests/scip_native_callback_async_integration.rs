#![cfg(all(feature = "scip", feature = "async"))]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ospf_rust_core::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};
use ospf_rust_core::model::{
    BasicMechanismModel, Linear, LinearConstraint, LinearInequality, LinearMonomial,
    MechanismModel, ObjectiveCategory,
};
use ospf_rust_core::solvers::scip::{
    SCIPConfig, SCIPNativeCallback, SCIPNativeControl, SCIPNativeObserver, SCIPNativeWhere,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, VariableType};
use ospf_rust_framework::{
    ColumnGenerationSolver, LinearBendersDecompositionSolver, QuadraticBendersDecompositionSolver,
    ScipColumnGenerationSolver, ScipLinearBendersDecompositionSolver,
    ScipQuadraticBendersDecompositionSolver, SolveOptions,
};

fn build_mip_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let item_count = 36usize;
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(42000 + index),
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

fn build_cut_context() -> (MechanismModel<f64>, Option<VariableId>, Vec<VariableId>) {
    let x = ContinuousVariableItem::auto("x");
    let y = ContinuousVariableItem::auto("y");
    let theta = ContinuousVariableItem::auto("theta");
    let x_id = x.id();
    let theta_id = theta.id();

    let mut basic = BasicMechanismModel::new("scip_async_quadratic_cut_context");
    basic.add_token(Token::from_generic(x, 0));
    basic.add_token(Token::from_generic(y, 1));
    basic.add_token(Token::from_generic(theta, 2));
    basic.add_constraint(LinearConstraint::new(
        LinearInequality::less_equal(
            Linear::new(
                vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                0.0,
            ),
            0.0,
        ),
        "link_xy",
    ));

    (
        MechanismModel::from_basic(basic),
        Some(theta_id),
        vec![x_id],
    )
}

fn build_true_quadratic_subproblem(rhs: f64) -> QuadraticTetradModel {
    let x = ContinuousVariableItem::auto("sub_x");
    let mut basic = BasicQuadraticTetradModel::new("scip_async_quadratic_subproblem");
    basic.linear.add_variable(Token::from_generic(x, 0));

    let mut row = SparseVector::new();
    row.add(0, 1.0);
    basic.linear.add_constraint(row, rhs);

    let mut model = QuadraticTetradModel::from_basic(basic);
    let mut q = SparseMatrix::new();
    let mut q_row = SparseVector::new();
    q_row.add(0, 1.0);
    q.add_row(q_row);
    model.set_objective(vec![0.0], q, ObjectiveCategory::Minimum);
    model
}

#[tokio::test]
async fn scip_column_generation_native_callback_interrupt_async() {
    let model = build_mip_model("framework_scip_native_callback_interrupt_async");
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
    let result = solver
        .solve_milp_with_options(&model, SolveOptions::default())
        .await;
    assert!(
        result.is_err(),
        "framework async solve should surface user interrupt as error"
    );
    let error_text = format!("{:?}", result.err().expect("error expected"));
    assert!(
        error_text.contains("UserInterrupt"),
        "expected UserInterrupt in error, got: {}",
        error_text
    );
    assert!(callback_hits.load(Ordering::SeqCst) > 0);
    assert!(node_hits.load(Ordering::SeqCst) > 0);
    assert!(interrupted.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn scip_linear_benders_native_callback_interrupt_async() {
    let model = build_mip_model("framework_scip_linear_benders_native_callback_interrupt_async");
    let callback_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let callback_hits_ref = callback_hits.clone();
    let interrupted_ref = interrupted.clone();

    let callback: SCIPNativeCallback = Arc::new(move |_snapshot| {
        callback_hits_ref.fetch_add(1, Ordering::SeqCst);
        if interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = ScipLinearBendersDecompositionSolver::with_config(SCIPConfig::new().with_output(false))
        .with_native_callback(Some(callback))
        .with_telemetry_min_interval(0.1);
    let result = solver.solve_master(&model, &[]).await;
    match result {
        Ok(output) => {
            assert!(
                matches!(output.status, ospf_rust_core::solver::SolverStatus::UserInterrupt),
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

#[tokio::test]
async fn scip_column_generation_native_observers_aggregated_async() {
    let model = build_mip_model("framework_scip_native_observers_aggregated_async");
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
    let result = solver
        .solve_milp_with_options(&model, SolveOptions::default())
        .await
        .expect("framework async native observers should solve");
    assert!(result.obj.is_finite(), "expected finite objective");
    let hits1 = observer1_hits.load(Ordering::SeqCst);
    let hits2 = observer2_hits.load(Ordering::SeqCst);
    assert!(hits1 > 0, "observer1 should be called");
    assert_eq!(hits1, hits2, "observers should receive same callback count");
}

#[tokio::test]
async fn scip_column_generation_native_observer_interrupt_async() {
    let model = build_mip_model("framework_scip_native_observer_interrupt_async");
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
    let result = solver
        .solve_milp_with_options(&model, SolveOptions::default())
        .await;
    assert!(
        result.is_err(),
        "framework async solve should surface user interrupt as error"
    );
    let error_text = format!("{:?}", result.err().expect("error expected"));
    assert!(
        error_text.contains("UserInterrupt"),
        "expected UserInterrupt in error, got: {}",
        error_text
    );
    assert!(node_hits.load(Ordering::SeqCst) > 0);
    assert!(interrupt_once.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn scip_quadratic_benders_native_callback_interrupt_async() {
    let model = build_mip_model("framework_scip_quadratic_benders_native_callback_interrupt_async");
    let callback_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let callback_hits_ref = callback_hits.clone();
    let interrupted_ref = interrupted.clone();

    let callback: SCIPNativeCallback = Arc::new(move |_snapshot| {
        callback_hits_ref.fetch_add(1, Ordering::SeqCst);
        if interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = ScipQuadraticBendersDecompositionSolver::with_config(
        SCIPConfig::new().with_output(false),
    )
    .with_native_callback(Some(callback))
    .with_telemetry_min_interval(0.1);
    let result = solver.solve_master(&model, &[]).await;
    match result {
        Ok(output) => {
            assert!(
                matches!(output.status, ospf_rust_core::solver::SolverStatus::UserInterrupt),
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

#[tokio::test]
async fn scip_quadratic_benders_subproblem_native_callback_interrupt_async() {
    let (mechanism_model, objective_variable, fixed_variable_ids) = build_cut_context();
    let model = build_true_quadratic_subproblem(1.0);
    let callback_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let callback_hits_ref = callback_hits.clone();
    let interrupted_ref = interrupted.clone();

    let callback: SCIPNativeCallback = Arc::new(move |_snapshot| {
        callback_hits_ref.fetch_add(1, Ordering::SeqCst);
        if interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = ScipQuadraticBendersDecompositionSolver::with_config(
        SCIPConfig::new().with_output(false),
    )
    .with_cut_context(mechanism_model, objective_variable, fixed_variable_ids)
    .with_native_callback(Some(callback))
    .with_telemetry_min_interval(0.1);

    let result = solver.solve_sub_quadratic(&model, &[0.0]).await;
    assert!(
        result.is_err(),
        "quadratic sub-problem should surface interrupt as error"
    );
    let error_text = format!("{:?}", result.err().expect("error expected"));
    assert!(
        error_text.contains("UserInterrupt"),
        "expected UserInterrupt in error, got: {}",
        error_text
    );
    assert!(callback_hits.load(Ordering::SeqCst) > 0);
    assert!(interrupted.load(Ordering::SeqCst) > 0);
}

#[tokio::test]
async fn scip_quadratic_benders_subproblem_native_observer_interrupt_async() {
    let (mechanism_model, objective_variable, fixed_variable_ids) = build_cut_context();
    let model = build_true_quadratic_subproblem(1.0);
    let observer_hits = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicUsize::new(0));
    let observer_hits_ref = observer_hits.clone();
    let interrupted_ref = interrupted.clone();

    let observer: SCIPNativeObserver = Arc::new(move |_snapshot| {
        observer_hits_ref.fetch_add(1, Ordering::SeqCst);
        if interrupted_ref.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(SCIPNativeControl::Interrupt);
        }
        Ok(SCIPNativeControl::Continue)
    });

    let solver = ScipQuadraticBendersDecompositionSolver::with_config(
        SCIPConfig::new().with_output(false),
    )
    .with_cut_context(mechanism_model, objective_variable, fixed_variable_ids)
    .add_native_observer(observer)
    .with_telemetry_min_interval(0.1);

    let result = solver.solve_sub_quadratic(&model, &[0.0]).await;
    assert!(
        result.is_err(),
        "quadratic sub-problem should surface interrupt as error"
    );
    let error_text = format!("{:?}", result.err().expect("error expected"));
    assert!(
        error_text.contains("UserInterrupt"),
        "expected UserInterrupt in error, got: {}",
        error_text
    );
    assert!(observer_hits.load(Ordering::SeqCst) > 0);
    assert!(interrupted.load(Ordering::SeqCst) > 0);
}
