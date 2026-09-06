#![cfg(feature = "scip")]

//! SCIP CP binding probes. / SCIP CP 绑定探针。
//!
//! These tests deliberately use the `russcip 0.9.1` surface directly.  They are
//! capability evidence only; they do not turn the MIP-backed CP facade into a
//! native CP solver. / 这些测试有意直接使用 `russcip 0.9.1` 接口，只提供能力
//! 证据，不会把 MIP-backed CP facade 冒充为原生 CP 求解器。

use std::ffi::CString;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::constraint_programming::{
    ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
    IntegerDomain, IntegerExpression, IntegerRelation, IntegerVariable,
};
use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
use ospf_rust_core::solver::constraint_programming::{
    ConstraintProgrammingSolver, ScipConstraintProgrammingSolver,
};
use ospf_rust_core::solver::solvers::scip::{
    SCIPConfig, SCIPNativeControl, SCIPNativeObserver, SCIPNativeWhere, SCIPSolver,
};
use ospf_rust_core::solver::{
    LinearSolver, ProblemStatus, SolveHandle, StableVariableId, TerminationReason,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId, VariableType};
use russcip::prelude::{
    ModelWithProblem, ProblemOrSolving, WithSolutions, WithSolvingStats, cons, eventhdlr, var,
};
use russcip::{Event, EventMask, Eventhdlr, Model, SCIPEventhdlr, Solving, Status, ffi};

fn assert_ok(code: ffi::SCIP_Retcode, context: &str) {
    assert_eq!(
        russcip::Retcode::from(code),
        russcip::Retcode::Okay,
        "SCIP call failed in {context}"
    );
}

#[test]
fn safe_indicator_and_sos1_report_status_bound_and_solution() {
    let mut indicator = Model::default().hide_output().maximize();
    let x = indicator.add(var().name("x").obj(1.0).int(0..=1));
    let b = indicator.add(var().name("b").bin());
    indicator.add(cons().name("force-b").eq(1.0).coef(&b, 1.0));
    indicator.add_cons_indicator(&b, vec![&x], &mut [1.0], 0.0, "b-implies-x-zero");
    let solved_indicator = indicator.solve();
    assert_eq!(solved_indicator.status(), Status::Optimal);
    assert!(solved_indicator.best_sol().is_some());
    assert!((solved_indicator.obj_val() - 0.0).abs() <= 1e-9);
    assert!(solved_indicator.best_bound().is_finite());

    let mut sos = Model::default().hide_output().maximize();
    let first = sos.add(var().name("first").obj(1.0).int(0..=1));
    let second = sos.add(var().name("second").obj(1.0).int(0..=1));
    sos.add_cons_sos1(vec![&first, &second], None, "at-most-one-nonzero");
    let solved_sos = sos.solve();
    assert_eq!(solved_sos.status(), Status::Optimal);
    assert!((solved_sos.obj_val() - 1.0).abs() <= 1e-9);
    assert!(solved_sos.n_sols() >= 1);
}

#[test]
fn event_probing_is_scoped_and_releases_its_native_handle() {
    struct ProbeObserver {
        called: Arc<AtomicBool>,
        depth: Arc<AtomicUsize>,
    }

    impl Eventhdlr for ProbeObserver {
        fn get_type(&self) -> EventMask {
            EventMask::LP_EVENT | EventMask::NODE_EVENT | EventMask::SOL_EVENT
        }

        fn execute(&mut self, mut model: Model<Solving>, _eventhdlr: SCIPEventhdlr, _event: Event) {
            if self.called.swap(true, Ordering::SeqCst) {
                return;
            }
            let variables = model.vars();
            let mut prober = model.start_probing();
            prober.new_node();
            self.depth.store(prober.depth(), Ordering::SeqCst);
            if let Some(variable) = variables.first() {
                prober.fix_var(variable, 0.0);
                let _ = prober.propagate(Some(1));
            }
            drop(prober);
            assert_eq!(unsafe { ffi::SCIPinProbing(model.scip_ptr()) }, 0);
        }
    }

    let called = Arc::new(AtomicBool::new(false));
    let depth = Arc::new(AtomicUsize::new(0));
    let mut model = Model::default()
        .hide_output()
        .set_presolving(russcip::ParamSetting::Off)
        .set_heuristics(russcip::ParamSetting::Off)
        .maximize();
    let variables = (0..12)
        .map(|index| model.add(var().name(&format!("x{index}")).obj(1.0).bin()))
        .collect::<Vec<_>>();
    model.add(
        cons()
            .name("branching-row")
            .le(6.0)
            .expr(variables.iter().map(|variable| (variable, 1.0))),
    );
    model.add(eventhdlr(ProbeObserver {
        called: Arc::clone(&called),
        depth: Arc::clone(&depth),
    }));
    let solved = model.solve();
    assert_eq!(solved.status(), Status::Optimal);
    assert!(called.load(Ordering::SeqCst));
    assert!(depth.load(Ordering::SeqCst) > 0);
    assert_eq!(unsafe { ffi::SCIPinProbing(solved.scip_ptr()) }, 0);
}

#[test]
fn raw_cumulative_probe_is_version_gated_and_releases_captured_constraint() {
    let mut model = Model::default().hide_output().minimize();
    let first = model.add(var().name("start-first").int(0..=2));
    let second = model.add(var().name("start-second").int(0..=2));
    let mut variables = vec![first.inner(), second.inner()];
    let mut durations = vec![2_i32, 2_i32];
    let mut demands = vec![1_i32, 1_i32];
    let name = CString::new("raw-cumulative").expect("static constraint name");
    let mut constraint = ptr::null_mut();

    // SAFETY: all pointers refer to live SCIP-owned variables or writable arrays for the
    // duration of the call. After adding the original constraint, the model owner performs the
    // final release; releasing it here would conflict with transformed-constraint ownership.
    unsafe {
        assert_ok(
            ffi::SCIPcreateConsBasicCumulative(
                model.scip_ptr(),
                &mut constraint,
                name.as_ptr(),
                variables.len() as i32,
                variables.as_mut_ptr(),
                durations.as_mut_ptr(),
                demands.as_mut_ptr(),
                1,
            ),
            "SCIPcreateConsBasicCumulative",
        );
        assert!(!constraint.is_null());
        assert_ok(
            ffi::SCIPaddCons(model.scip_ptr(), constraint),
            "SCIPaddCons(cumulative)",
        );
    }
    assert!(!constraint.is_null());
    let solved = model.solve();
    assert_eq!(solved.status(), Status::Optimal);
}

#[test]
fn project_cancellation_reaches_scip_and_preserves_uniform_report() {
    let mut basic = BasicLinearTriadModel::new("scip-cp-cancellation-probe");
    for index in 0..40 {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(50_000 + index),
            &format!("x{index}"),
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
    let mut objective = vec![0.0; 40];
    for index in 0..40 {
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

    let handle = SolveHandle::new();
    let observer_calls = Arc::new(AtomicUsize::new(0));
    let observer: SCIPNativeObserver = Arc::new({
        let handle = handle.clone();
        let observer_calls = Arc::clone(&observer_calls);
        move |snapshot| {
            observer_calls.fetch_add(1, Ordering::SeqCst);
            if matches!(
                snapshot.where_point,
                SCIPNativeWhere::Node | SCIPNativeWhere::Lp | SCIPNativeWhere::Solution
            ) {
                handle.cancel("cp-probe");
                return Ok(SCIPNativeControl::Interrupt);
            }
            Ok(SCIPNativeControl::Continue)
        }
    });
    let solver = SCIPSolver::with_config(
        SCIPConfig::recommended_lp_subproblem_defaults()
            .with_output(false)
            .with_native_event_mask(
                russcip::EventMask::LP_EVENT
                    | russcip::EventMask::NODE_EVENT
                    | russcip::EventMask::SOL_EVENT,
            )
            .add_native_observer(observer),
    );
    let report = solver
        .solve_linear_report_with_options(
            &model,
            &ospf_rust_core::solver::SolveOptions::builder()
                .cancellation_handle(Some(&handle))
                .finish(),
        )
        .expect("SCIP cancellation probe should return a report");
    assert!(handle.is_cancelled());
    assert!(observer_calls.load(Ordering::SeqCst) > 0);
    assert_eq!(report.termination_reason, TerminationReason::Cancelled);
    assert!(matches!(
        report.problem_status,
        ProblemStatus::Unknown | ProblemStatus::Feasible
    ));
    assert!(
        report
            .diagnostics
            .extensions
            .contains_key("cancellation.origin")
    );
}

#[test]
fn scip_cp_facade_returns_a_verified_uniform_report() {
    let mut model = ConstraintProgrammingModel::new("scip-cp-facade-report");
    let x = IntegerVariable::new("x");
    model
        .register_variable(x.clone(), IntegerDomain::boolean())
        .expect("x variable");
    model
        .add_constraint(ConstraintDefinition::new(
            "force-one",
            ConstraintProgrammingConstraint::integer(
                IntegerExpression::variable(x),
                IntegerRelation::Equal,
                1,
            ),
        ))
        .expect("force-one constraint");
    let snapshot = model.freeze().expect("CP snapshot");
    let solver = ScipConstraintProgrammingSolver::new();
    let report = solver
        .solve_constraint_programming(&snapshot, &Default::default())
        .expect("SCIP CP facade solve");

    assert_eq!(report.problem_status, ProblemStatus::Feasible);
    assert!(report.is_optimal());
    assert_eq!(
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.stable_values.get(&StableVariableId::from("x")))
            .copied(),
        Some(1)
    );
    assert_eq!(
        report.fingerprints.model.as_ref(),
        Some(&snapshot.fingerprint)
    );
    assert_eq!(report.provenance.backend_name, "SCIP");
    assert_eq!(
        report
            .provenance
            .effective_configuration
            .get("cp.lowering")
            .map(String::as_str),
        Some("exact-finite-linear")
    );
    report
        .validate()
        .expect("SCIP CP report should satisfy the shared contract");
}
