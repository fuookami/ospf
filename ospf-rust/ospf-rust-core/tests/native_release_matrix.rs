//! 原生 release 矩阵 / Native release matrix.
//!
//! 这些测试只在显式 native feature 和 `--include-ignored` 下运行。
//! These tests run only with an explicit native feature and `--include-ignored`.

#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
use ospf_rust_core::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};
#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
use ospf_rust_core::model::ObjectiveCategory;
#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
use ospf_rust_core::solver::{
    CancellationOrigin, LinearSolver, ProblemStatus, SolutionPresence, SolveHandle, SolveOptions,
    SolveReport, TerminationReason,
};
#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
use ospf_rust_core::token::Token;
#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
use ospf_rust_core::variable::ContinuousVariableItem;
#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
use ospf_rust_core::variable::{BinaryVariableItem, VariableId, VariableType};

#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
fn build_mip_model(name: &str, with_initial_solution: bool) -> LinearTriadModel {
    let item_count = 48usize;
    let mut basic = BasicLinearTriadModel::new(name);
    for index in 0..item_count {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(40_000 + index),
            &format!("x_{}", index),
        );
        let token = Token::from_generic(variable, index);
        if with_initial_solution && index == 0 {
            token.set_result(1.0);
        } else if with_initial_solution {
            token.set_result(0.0);
        }
        basic.add_variable_with_bounds(token, 0.0, 1.0, VariableType::Binary);
    }

    let mut weight = SparseVector::new();
    let mut cardinality = SparseVector::new();
    let mut objective = vec![0.0; item_count];
    for (index, objective_value) in objective.iter_mut().enumerate() {
        weight.add(index, ((index % 9) + 1) as f64);
        cardinality.add(index, 1.0);
        *objective_value = (index + 2) as f64;
    }
    basic.add_constraint(weight, 96.0);
    // 保留根 LP 的分数松弛，便于验证 node limit 下无 incumbent 的合同。
    // Keep a fractional root relaxation so the no-incumbent node-limit contract is exercised.
    basic.add_constraint(cardinality, 17.5);

    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(objective, ObjectiveCategory::Maximum);
    model
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
fn build_lp_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let variable_count = 16usize;
    for index in 0..variable_count {
        let variable = ContinuousVariableItem::create(
            VariableId::standalone(41_000 + index),
            &format!("y_{}", index),
        );
        basic.add_variable_with_bounds(
            Token::from_generic(variable, index),
            0.0,
            1.0,
            VariableType::Continuous,
        );
    }
    let mut total = SparseVector::new();
    let mut weighted = SparseVector::new();
    for index in 0..variable_count {
        total.add(index, 1.0);
        weighted.add(index, ((index % 5) + 1) as f64);
    }
    basic.add_constraint(total, 7.25);
    basic.add_constraint(weighted, 22.5);
    let objective = (0..variable_count)
        .map(|index| (index + 1) as f64)
        .collect();
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(objective, ObjectiveCategory::Maximum);
    model
}

#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
fn build_unbounded_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    let variable = ContinuousVariableItem::create(VariableId::standalone(43_000), "u");
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        0.0,
        f64::INFINITY,
        VariableType::Continuous,
    );
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
    model
}

#[cfg(feature = "scip")]
fn build_integer_infeasible_root_model(name: &str) -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new(name);
    for index in 0..2usize {
        let variable = BinaryVariableItem::create(
            VariableId::standalone(42_000 + index),
            &format!("z_{}", index),
        );
        basic.add_variable_with_bounds(
            Token::from_generic(variable, index),
            0.0,
            1.0,
            VariableType::Binary,
        );
    }
    let mut upper = SparseVector::new();
    upper.add(0, 1.0);
    upper.add(1, 1.0);
    basic.add_constraint(upper, 1.5);
    let mut lower = SparseVector::new();
    lower.add(0, -1.0);
    lower.add(1, -1.0);
    basic.add_constraint(lower, -1.5);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0, 1.0], ObjectiveCategory::Maximum);
    model
}

#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
fn assert_limit_report(
    report: &SolveReport<f64>,
    termination: TerminationReason,
    has_incumbent: bool,
) {
    report
        .validate()
        .expect("native release report must validate");
    assert_eq!(report.termination_reason, termination);
    assert_eq!(report.has_incumbent(), has_incumbent);
    assert!(
        !report.is_optimal(),
        "a limit report is not an exact optimum"
    );
    assert_eq!(
        report.solution_presence,
        if has_incumbent {
            SolutionPresence::Incumbent
        } else {
            SolutionPresence::None
        }
    );
    assert_eq!(
        report.problem_status,
        if has_incumbent {
            ProblemStatus::Feasible
        } else {
            ProblemStatus::Unknown
        }
    );
    assert!(report.provenance.backend_version.is_some());
    assert!(!report.provenance.solver_id.is_empty());
    if has_incumbent {
        let solution = report.solution.as_ref().expect("incumbent vector");
        let objective = solution.objective_value.expect("incumbent objective");
        assert!(!solution.values.is_empty());
        assert!(report.statistics.solution_count.unwrap_or_default() >= 1);

        // 最大化 MIP 的 bound 必须是 incumbent 的有效上界，并且 gap 必须与两者一致。
        // For a maximization MIP, the bound must dominate the incumbent and both gaps must agree.
        let best_bound = report
            .statistics
            .best_bound_value
            .expect("incumbent MIP bound");
        let absolute_gap = report
            .statistics
            .absolute_gap
            .expect("incumbent MIP absolute gap");
        let relative_gap = report
            .statistics
            .relative_gap
            .expect("incumbent MIP relative gap");
        assert!(best_bound.is_finite());
        assert!(absolute_gap.is_finite());
        assert!(relative_gap.is_finite());
        assert!(
            best_bound + 1e-7 >= objective,
            "maximization bound {best_bound} is below incumbent {objective}"
        );
        assert!(absolute_gap >= 0.0);
        assert!(relative_gap >= 0.0);
        let expected_absolute_gap = (objective - best_bound).abs();
        let expected_relative_gap = expected_absolute_gap / objective.abs().max(1.0);
        assert!((absolute_gap - expected_absolute_gap).abs() <= 1e-7);
        assert!((relative_gap - expected_relative_gap).abs() <= 1e-7);
    } else {
        assert!(report.solution.is_none());
    }
}

#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
fn assert_cancelled_report(report: &SolveReport<f64>) {
    report.validate().expect("cancelled report must validate");
    assert_eq!(report.termination_reason, TerminationReason::Cancelled);
    assert_eq!(
        report.diagnostics.extensions.get("cancellation.origin"),
        Some(&"CALLBACK".to_owned())
    );
    assert!(!report.is_optimal());
}

#[cfg(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
))]
fn assert_unbounded_report(report: &SolveReport<f64>) {
    report
        .validate()
        .expect("native unbounded report must validate");
    assert_eq!(report.problem_status, ProblemStatus::Unbounded);
    assert_eq!(report.termination_reason, TerminationReason::Completed);
    assert_eq!(report.solution_presence, SolutionPresence::None);
    assert!(!report.has_incumbent());
    assert!(!report.is_optimal());
    assert!(report.provenance.backend_version.is_some());
    assert!(!report.provenance.solver_id.is_empty());
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
mod gurobi {
    use super::*;
    use ospf_rust_core::error::CoreError;
    use ospf_rust_core::solver::solvers::GurobiSolver;
    use ospf_rust_core::solvers::gurobi::{
        GurobiConfig, GurobiEnvCallback, GurobiNativeControl, GurobiNativeObserver,
        GurobiNativeWhere,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn config_without_root_shortcuts() -> GurobiConfig {
        let callback: GurobiEnvCallback = Arc::new(|env| {
            env.set(grb::param::Presolve, 0).map_err(|error| {
                CoreError::solver_environment(format!("disable Gurobi presolve: {}", error))
            })?;
            env.set(grb::param::Heuristics, 0.0).map_err(|error| {
                CoreError::solver_environment(format!("disable Gurobi heuristics: {}", error))
            })?;
            env.set(grb::param::Cuts, 0).map_err(|error| {
                CoreError::solver_environment(format!("disable Gurobi cuts: {}", error))
            })?;
            Ok(())
        });
        GurobiConfig::new()
            .with_output(false)
            .with_threads(1)
            .add_env_callback(callback)
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn node_limit_without_incumbent() {
        let solver = GurobiSolver::with_config(config_without_root_shortcuts().with_node_limit(0));
        let report = solver
            .solve_linear_report(&build_mip_model("gurobi_release_node_no_incumbent", false))
            .expect("Gurobi node-limit report");
        assert_limit_report(&report, TerminationReason::NodeLimit, false);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn node_limit_with_incumbent() {
        let solver = GurobiSolver::with_config(config_without_root_shortcuts().with_node_limit(0));
        let report = solver
            .solve_linear_report(&build_mip_model("gurobi_release_node_incumbent", true))
            .expect("Gurobi node-limit report");
        assert_limit_report(&report, TerminationReason::NodeLimit, true);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn iteration_limit_without_incumbent() {
        let solver =
            GurobiSolver::with_config(config_without_root_shortcuts().with_max_iterations(0));
        let report = solver
            .solve_linear_report(&build_lp_model("gurobi_release_iteration_no_incumbent"))
            .expect("Gurobi iteration-limit report");
        assert_limit_report(&report, TerminationReason::IterationLimit, false);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn time_limit_without_incumbent() {
        let solver =
            GurobiSolver::with_config(config_without_root_shortcuts().with_time_limit(0.0));
        let report = solver
            .solve_linear_report(&build_mip_model("gurobi_release_time_no_incumbent", false))
            .expect("Gurobi time-limit report");
        assert_limit_report(&report, TerminationReason::TimeLimit, false);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn time_limit_with_incumbent() {
        let solver = GurobiSolver::with_config(
            config_without_root_shortcuts()
                .with_seed(1)
                .with_time_limit(0.005),
        );
        let report = solver
            .solve_linear_report(&build_mip_model("gurobi_release_time_incumbent", true))
            .expect("Gurobi time-limit report");
        assert_limit_report(&report, TerminationReason::TimeLimit, true);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn gap_limit_with_incumbent_preserves_non_exact_report() {
        let solver = GurobiSolver::with_config(
            config_without_root_shortcuts()
                .with_mip_gap(0.5)
                .with_node_limit(10_000),
        );
        let report = solver
            .solve_linear_report(&build_mip_model("gurobi_release_gap_incumbent", true))
            .expect("Gurobi gap-limit report");
        assert_limit_report(&report, TerminationReason::GapLimit, true);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn unbounded_model_preserves_native_terminal() {
        let solver = GurobiSolver::with_config(config_without_root_shortcuts());
        let report = solver
            .solve_linear_report(&build_unbounded_model("gurobi_release_unbounded"))
            .expect("Gurobi unbounded report");
        assert_unbounded_report(&report);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn solution_limit_with_incumbent() {
        let solver = GurobiSolver::with_config(
            GurobiConfig::new()
                .with_output(false)
                .with_threads(1)
                .with_solution_limit(1),
        );
        let report = solver
            .solve_linear_report(&build_mip_model("gurobi_release_solution_limit", true))
            .expect("Gurobi solution-limit report");
        assert_limit_report(&report, TerminationReason::SolutionLimit, true);
    }

    #[test]
    #[ignore = "requires matching Gurobi native library and license"]
    fn callback_cancellation_preserves_origin() {
        let handle = SolveHandle::new();
        let callback_handle = handle.clone();
        let callback_hits = Arc::new(AtomicUsize::new(0));
        let callback_hits_ref = callback_hits.clone();
        let observer: GurobiNativeObserver = Arc::new(move |snapshot| {
            if snapshot.where_point == GurobiNativeWhere::Mip
                && callback_hits_ref.fetch_add(1, Ordering::SeqCst) == 0
            {
                callback_handle.cancel(CancellationOrigin::Callback);
            }
            Ok(GurobiNativeControl::Continue)
        });
        let solver = GurobiSolver::with_config(
            GurobiConfig::new()
                .with_output(false)
                .with_threads(1)
                .add_native_observer(observer),
        );
        let options = SolveOptions::new().with_cancellation_handle(Some(&handle));
        let report = solver
            .solve_linear_report_with_options(
                &build_mip_model("gurobi_release_callback_cancel", false),
                &options,
            )
            .expect("Gurobi cancellation report");
        assert!(callback_hits.load(Ordering::SeqCst) > 0);
        assert_cancelled_report(&report);
    }
}

#[cfg(feature = "scip")]
mod scip {
    use super::*;
    use ospf_rust_core::solver::solvers::SCIPSolver;
    use ospf_rust_core::solvers::scip::{
        PresolvingMode, SCIPConfig, SCIPNativeControl, SCIPNativeObserver,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn config() -> SCIPConfig {
        SCIPConfig::recommended_lp_subproblem_defaults()
            .with_output(false)
            .with_presolving(PresolvingMode::Off)
            .with_heuristics_priority(0)
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn node_limit_without_incumbent() {
        let solver = SCIPSolver::with_config(config().with_node_limit(0));
        let report = solver
            .solve_linear_report(&build_integer_infeasible_root_model(
                "scip_release_node_no_incumbent",
            ))
            .expect("SCIP node-limit report");
        assert_limit_report(&report, TerminationReason::NodeLimit, false);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn node_limit_with_incumbent() {
        let solver = SCIPSolver::with_config(config().with_node_limit(0));
        let report = solver
            .solve_linear_report(&build_mip_model("scip_release_node_incumbent", true))
            .expect("SCIP node-limit report");
        assert_limit_report(&report, TerminationReason::NodeLimit, true);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn iteration_limit_without_incumbent() {
        let solver = SCIPSolver::with_config(config().with_max_iterations(0));
        let report = solver
            .solve_linear_report(&build_integer_infeasible_root_model(
                "scip_release_iteration_no_incumbent",
            ))
            .expect("SCIP iteration-limit report");
        assert_eq!(
            report.termination_reason,
            TerminationReason::IterationLimit,
            "SCIP iteration fixture report: iterations={:?}, problem_status={:?}, solution_presence={:?}",
            report.statistics.iterations,
            report.problem_status,
            report.solution_presence
        );
        assert_limit_report(&report, TerminationReason::IterationLimit, false);
        assert!(
            report.statistics.iterations.is_some(),
            "SCIP iteration-limit fixture should expose native LP statistics"
        );
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn time_limit_without_incumbent() {
        let solver = SCIPSolver::with_config(config().with_time_limit(0.0));
        let report = solver
            .solve_linear_report(&build_integer_infeasible_root_model(
                "scip_release_time_no_incumbent",
            ))
            .expect("SCIP time-limit report");
        assert_limit_report(&report, TerminationReason::TimeLimit, false);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn time_limit_with_incumbent() {
        let solver = SCIPSolver::with_config(config().with_time_limit(0.0));
        let report = solver
            .solve_linear_report(&build_mip_model("scip_release_time_incumbent", true))
            .expect("SCIP time-limit report");
        assert_limit_report(&report, TerminationReason::TimeLimit, true);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn gap_limit_with_incumbent() {
        let solver = SCIPSolver::with_config(config().with_mip_gap(1.0));
        let report = solver
            .solve_linear_report(&build_mip_model("scip_release_gap_incumbent", true))
            .expect("SCIP gap-limit report");
        assert_limit_report(&report, TerminationReason::GapLimit, true);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn memory_limit_without_incumbent() {
        let solver = SCIPSolver::with_config(config().with_mem_limit(0.0));
        let report = solver
            .solve_linear_report(&build_mip_model("scip_release_memory_no_incumbent", false))
            .expect("SCIP memory-limit report");
        assert_limit_report(&report, TerminationReason::MemoryLimit, false);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn unbounded_model_preserves_native_terminal() {
        let solver = SCIPSolver::with_config(config());
        let report = solver
            .solve_linear_report(&build_unbounded_model("scip_release_unbounded"))
            .expect("SCIP unbounded report");
        assert_unbounded_report(&report);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn solution_limit_with_incumbent() {
        let solver = SCIPSolver::with_config(config().with_solution_limit(1));
        let report = solver
            .solve_linear_report(&build_mip_model("scip_release_solution_limit", true))
            .expect("SCIP solution-limit report");
        assert_limit_report(&report, TerminationReason::SolutionLimit, true);
    }

    #[test]
    #[ignore = "requires matching SCIP native library"]
    fn callback_cancellation_preserves_origin() {
        let handle = SolveHandle::new();
        let callback_handle = handle.clone();
        let callback_hits = Arc::new(AtomicUsize::new(0));
        let callback_hits_ref = callback_hits.clone();
        let observer: SCIPNativeObserver = Arc::new(move |_snapshot| {
            if callback_hits_ref.fetch_add(1, Ordering::SeqCst) == 0 {
                callback_handle.cancel(CancellationOrigin::Callback);
            }
            Ok(SCIPNativeControl::Continue)
        });
        let solver = SCIPSolver::with_config(config().add_native_observer(observer));
        let options = SolveOptions::new().with_cancellation_handle(Some(&handle));
        let report = solver
            .solve_linear_report_with_options(
                &build_mip_model("scip_release_callback_cancel", false),
                &options,
            )
            .expect("SCIP cancellation report");
        assert!(callback_hits.load(Ordering::SeqCst) > 0);
        assert_cancelled_report(&report);
    }
}

#[cfg(not(any(
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12",
    feature = "scip"
)))]
#[test]
#[ignore = "native release matrix requires an explicit Gurobi or SCIP feature"]
fn native_release_matrix_requires_explicit_backend_feature() {
    panic!("native release matrix requires an explicit Gurobi or SCIP feature");
}
