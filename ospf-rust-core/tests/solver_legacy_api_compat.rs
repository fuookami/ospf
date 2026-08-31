use bigdecimal::BigDecimal;
use std::str::FromStr;

use ospf_rust_core::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
};
use ospf_rust_core::solver::{
    LinearSolver, ProblemStatus, QuadraticSolver, SolveReport, SolveSolution, SolveStatistics,
    SolveValueConversionPolicy, SolverCapability, SolverInfo, SolverOutput, SolverStatus,
    TerminationReason, solve_report_to_solver_output,
};

struct PublicCompatibilitySolver;

impl SolverInfo for PublicCompatibilitySolver {
    fn name(&self) -> &str {
        "public-compatibility"
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        vec![SolverCapability::Linear, SolverCapability::Quadratic]
    }
}

impl LinearSolver for PublicCompatibilitySolver {
    fn solve_linear(
        &self,
        _model: &LinearTriadModel,
    ) -> ospf_rust_core::error::Result<SolverOutput> {
        Ok(SolverOutput::optimal(1.25, vec![]).with_solution_count(1))
    }
}

impl QuadraticSolver for PublicCompatibilitySolver {
    fn solve_quadratic(
        &self,
        _model: &QuadraticTetradModel,
    ) -> ospf_rust_core::error::Result<SolverOutput> {
        Ok(SolverOutput::optimal(2.5, vec![]).with_solution_count(1))
    }
}

#[test]
fn public_legacy_linear_and_quadratic_entries_remain_callable() {
    let solver = PublicCompatibilitySolver;
    let linear = LinearTriadModel::from_basic(BasicLinearTriadModel::new("legacy-linear"));
    let quadratic =
        QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("legacy-quadratic"));

    let linear_output = solver
        .solve_linear(&linear)
        .expect("legacy linear entry should remain callable");
    let quadratic_output = solver
        .solve_quadratic(&quadratic)
        .expect("legacy quadratic entry should remain callable");

    assert_eq!(linear_output.status, SolverStatus::Optimal);
    assert_eq!(linear_output.objective_value, Some(1.25));
    assert_eq!(quadratic_output.status, SolverStatus::Optimal);
    assert_eq!(quadratic_output.objective_value, Some(2.5));

    let report = solver
        .solve_linear_report(&linear)
        .expect("report compatibility entry should remain callable");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "LegacyStatusMapping")
    );
}

#[test]
fn public_legacy_output_and_config_paths_keep_their_projection_contract() {
    #[allow(deprecated)]
    let legacy_output =
        ospf_rust_core::solver::solver_output::SolverOutput::optimal(1.25, vec![2.0]);
    let typed = legacy_output
        .try_into_feasible_typed::<BigDecimal>(SolveValueConversionPolicy::AllowRounding)
        .expect("legacy output should retain typed projection");
    assert_eq!(typed.solution.len(), 1);
    assert_eq!(
        typed.objective_value,
        Some(BigDecimal::from_str("1.25").expect("decimal literal should parse"))
    );

    #[allow(deprecated)]
    let legacy_config = ospf_rust_core::solver::solver_config::SolverConfig::new("legacy");
    assert_eq!(legacy_config.name, "legacy");
}

#[test]
fn unified_report_projects_to_legacy_status_and_bound_without_silent_optimality() {
    let mut solution = SolveSolution::vector(vec![1.0]);
    solution.objective = Some(3.0);
    solution.objective_value = Some(3.0);
    let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::NodeLimit)
        .solution(solution)
        .statistics(SolveStatistics {
            best_bound: Some(2.0),
            best_bound_value: Some(2.0),
            solution_count: Some(1),
            ..SolveStatistics::default()
        })
        .build()
        .expect("limited report should remain valid");

    let legacy = solve_report_to_solver_output(&report);
    assert_eq!(legacy.status, SolverStatus::NodeLimit);
    assert_eq!(legacy.objective_value, Some(3.0));
    assert_eq!(legacy.best_bound, Some(2.0));
    assert_eq!(legacy.solution, Some(vec![1.0]));

    let projected = legacy
        .try_into_solve_report()
        .expect("legacy output should remain readable");
    assert_eq!(projected.termination_reason, TerminationReason::NodeLimit);
    assert!(!projected.is_optimal());
    assert_eq!(projected.statistics.best_bound_value, Some(2.0));
}
