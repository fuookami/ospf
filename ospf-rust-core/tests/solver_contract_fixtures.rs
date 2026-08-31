use std::time::Duration;

use ospf_rust_core::error::{CoreError, SolverError};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};
use ospf_rust_core::solver::{
    CancellationOrigin, ProblemStatus, SolveHandle, SolverOutput, SolverStatus, TerminationReason,
    solver_output_to_report, solver_output_to_report_with_cancellation,
};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableType,
};

fn fixed_lp_model() -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new("contract_lp");
    let variable = ContinuousVariableItem::create(VariableId::standalone(70001), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        0.0,
        4.0,
        VariableType::Continuous,
    );
    let mut row = SparseVector::new();
    row.add(0, 1.0);
    basic.add_constraint(row, 3.0);
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Minimum);
    model
}

fn fixed_mip_model() -> LinearTriadModel {
    let mut basic = BasicLinearTriadModel::new("contract_mip");
    let variable = BinaryVariableItem::create(VariableId::standalone(70002), "x");
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        0.0,
        1.0,
        VariableType::Binary,
    );
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
    model
}

fn fixed_qp_model() -> QuadraticTetradModel {
    let linear = fixed_lp_model();
    let mut model =
        QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::from_linear(linear.basic));
    let mut quadratic = SparseMatrix::new();
    quadratic.add_row(SparseVector::new());
    quadratic.rows[0].add(0, 1.0);
    model.set_objective(vec![0.0], quadratic, ObjectiveCategory::Minimum);
    model
}

fn fixed_incumbent_output(status: SolverStatus) -> SolverOutput {
    SolverOutput::new(status)
        .with_objective(7.0)
        .with_solution(vec![1.0, 0.0])
        .with_time(Duration::from_millis(3))
        .with_solution_count(1)
}

#[test]
fn fixed_lp_mip_qp_fixture_preserves_incumbent_and_termination() {
    for status in [SolverStatus::Optimal, SolverStatus::Feasible] {
        let report = solver_output_to_report(fixed_incumbent_output(status))
            .expect("fixed feasible fixture should produce a report");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::Completed);
        assert!(report.has_incumbent());
        assert_eq!(report.statistics.solution_count, Some(1));
    }
}

#[test]
fn fixed_model_fixture_keeps_lp_mip_qp_shapes_distinct() {
    let lp = fixed_lp_model();
    let mip = fixed_mip_model();
    let qp = fixed_qp_model();

    assert!(!lp.has_integer_variables());
    assert!(mip.has_integer_variables());
    assert_eq!(qp.num_variables(), 1);
    assert_eq!(qp.num_quadratic_constraints(), 0);
    assert_eq!(qp.Q.rows.len(), 1);
    assert_eq!(qp.Q.rows[0].entries, vec![(0, 1.0)]);
}

#[test]
fn infeasible_and_unbounded_fixtures_remain_distinct() {
    let infeasible = solver_output_to_report(SolverOutput::infeasible())
        .expect("infeasible fixture should produce a report");
    assert_eq!(infeasible.problem_status, ProblemStatus::Infeasible);
    assert_eq!(infeasible.termination_reason, TerminationReason::Completed);
    assert!(!infeasible.has_incumbent());

    let unbounded = solver_output_to_report(SolverOutput::unbounded())
        .expect("unbounded fixture should produce a report");
    assert_eq!(unbounded.problem_status, ProblemStatus::Unbounded);
    assert_eq!(unbounded.termination_reason, TerminationReason::Completed);
    assert!(!unbounded.has_incumbent());
}

#[test]
fn limit_fixtures_depend_on_actual_incumbent() {
    let limited_statuses = [
        (SolverStatus::TimeLimit, TerminationReason::TimeLimit),
        (SolverStatus::NodeLimit, TerminationReason::NodeLimit),
        (
            SolverStatus::IterationLimit,
            TerminationReason::IterationLimit,
        ),
        (
            SolverStatus::SolutionLimit,
            TerminationReason::SolutionLimit,
        ),
        (SolverStatus::GapLimit, TerminationReason::GapLimit),
        (SolverStatus::MemoryLimit, TerminationReason::MemoryLimit),
        (
            SolverStatus::TotalNodeLimit,
            TerminationReason::TotalNodeLimit,
        ),
        (
            SolverStatus::StallNodeLimit,
            TerminationReason::StallNodeLimit,
        ),
        (
            SolverStatus::BestSolutionLimit,
            TerminationReason::BestSolutionLimit,
        ),
        (SolverStatus::WorkLimit, TerminationReason::WorkLimit),
        (
            SolverStatus::ObjectiveLimit,
            TerminationReason::ObjectiveLimit,
        ),
        (SolverStatus::RestartLimit, TerminationReason::RestartLimit),
        (SolverStatus::Suboptimal, TerminationReason::Suboptimal),
    ];

    for (status, termination) in limited_statuses {
        let with_incumbent = solver_output_to_report(fixed_incumbent_output(status))
            .expect("limited fixture with incumbent should produce a report");
        assert_eq!(with_incumbent.problem_status, ProblemStatus::Feasible);
        assert_eq!(with_incumbent.termination_reason, termination);
        assert!(with_incumbent.has_incumbent());

        let without_incumbent = solver_output_to_report(SolverOutput::new(status))
            .expect("limited fixture without incumbent should produce a report");
        assert_eq!(without_incumbent.problem_status, ProblemStatus::Unknown);
        assert_eq!(without_incumbent.termination_reason, termination);
        assert!(!without_incumbent.has_incumbent());
    }
}

#[test]
fn cancellation_fixture_preserves_origin_and_does_not_invent_solution() {
    let handle = SolveHandle::new();
    assert!(handle.cancel(CancellationOrigin::RemoteStop));
    let report = solver_output_to_report_with_cancellation(
        fixed_incumbent_output(SolverStatus::UserInterrupt),
        Default::default(),
        &handle,
    )
    .expect("cancelled fixture should produce a report");
    assert_eq!(report.problem_status, ProblemStatus::Feasible);
    assert_eq!(report.termination_reason, TerminationReason::Cancelled);
    assert!(report.has_incumbent());
    assert_eq!(
        report.diagnostics.extensions.get("cancellation.origin"),
        Some(&"REMOTE_STOP".to_owned())
    );
}

#[test]
fn numerical_fixture_is_rejected_before_report_creation() {
    let error = solver_output_to_report(SolverOutput::optimal(f64::NAN, vec![1.0]))
        .expect_err("non-finite fixture must not become a solve report");
    assert!(matches!(
        error,
        CoreError::Solver(SolverError::ContractViolation(_))
    ));
}

#[test]
fn numeric_and_unknown_fixtures_keep_incomplete_terminal_semantics() {
    let numeric_with_incumbent =
        solver_output_to_report(fixed_incumbent_output(SolverStatus::NumericError))
            .expect("numeric terminal with an incumbent should remain representable");
    assert_eq!(
        numeric_with_incumbent.problem_status,
        ProblemStatus::Feasible
    );
    assert_eq!(
        numeric_with_incumbent.termination_reason,
        TerminationReason::NumericalFailure
    );
    assert!(numeric_with_incumbent.has_incumbent());
    assert!(!numeric_with_incumbent.is_optimal());

    let numeric_without_incumbent =
        solver_output_to_report(SolverOutput::new(SolverStatus::NumericError))
            .expect("numeric terminal without an incumbent should remain representable");
    assert_eq!(
        numeric_without_incumbent.problem_status,
        ProblemStatus::Unknown
    );
    assert_eq!(
        numeric_without_incumbent.termination_reason,
        TerminationReason::NumericalFailure
    );
    assert!(!numeric_without_incumbent.has_incumbent());

    let unknown = solver_output_to_report(SolverOutput::new(SolverStatus::Unknown))
        .expect("unknown terminal should remain representable");
    assert_eq!(unknown.problem_status, ProblemStatus::Unknown);
    assert_eq!(unknown.termination_reason, TerminationReason::Unknown);
    assert!(!unknown.has_incumbent());
}
