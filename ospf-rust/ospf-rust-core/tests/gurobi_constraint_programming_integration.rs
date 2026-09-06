//! Gurobi CP exact-lowering 集成门禁 / Gurobi CP exact-lowering integration gate.

#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use ospf_rust_core::model::constraint_programming::{
    ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
    IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerTerm,
    IntegerVariable, IntervalDuration, IntervalVariable,
};
use ospf_rust_core::solver::constraint_programming::MipBackedConstraintProgrammingSolver;
use ospf_rust_core::solver::solvers::gurobi::{GurobiConfig, GurobiSolver};
use ospf_rust_core::solver::{
    ConstraintProgrammingSolver, ProblemStatus, SolveReport, StableVariableId,
};

fn expression(
    constant: i64,
    terms: impl IntoIterator<Item = (IntegerVariable, i64)>,
) -> IntegerExpression {
    IntegerExpression::linear(
        constant,
        terms
            .into_iter()
            .map(|(variable, coefficient)| IntegerTerm {
                variable,
                coefficient,
            }),
    )
    .expect("CP test expression should be representable")
}

fn solver() -> MipBackedConstraintProgrammingSolver<GurobiSolver> {
    MipBackedConstraintProgrammingSolver::new(GurobiSolver::with_config(
        GurobiConfig::new().with_output(false),
    ))
}

fn value(report: &SolveReport<i64>, stable_id: &str) -> i64 {
    report
        .solution
        .as_ref()
        .and_then(|solution| {
            solution
                .stable_values
                .get(&StableVariableId::from(stable_id))
        })
        .copied()
        .expect("CP report should contain the requested stable value")
}

#[test]
fn gurobi_exact_lowering_solves_forbidden_assignments_and_revalidates_snapshot() {
    let x = IntegerVariable::new("forbidden/x");
    let y = IntegerVariable::new("forbidden/y");
    let mut model = ConstraintProgrammingModel::new("gurobi-forbidden-assignments");
    model
        .register_variable(x.clone(), IntegerDomain::boolean())
        .expect("x variable");
    model
        .register_variable(y.clone(), IntegerDomain::boolean())
        .expect("y variable");
    model
        .add_constraint(ConstraintDefinition::new(
            "forbidden/zero-zero",
            ConstraintProgrammingConstraint::ForbiddenAssignments {
                expressions: vec![
                    IntegerExpression::variable(x.clone()),
                    IntegerExpression::variable(y.clone()),
                ],
                tuples: vec![vec![0, 0]],
            },
        ))
        .expect("forbidden table");
    model.set_objective(IntegerObjective::maximize(expression(0, [(x, 1), (y, 1)])));
    let snapshot = model.freeze().expect("snapshot");

    let report = solver()
        .solve_constraint_programming(&snapshot, &Default::default())
        .expect("Gurobi CP solve");

    assert_eq!(report.problem_status, ProblemStatus::Feasible);
    assert!(report.is_optimal());
    assert_eq!(
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective),
        Some(2)
    );
    assert_eq!(value(&report, "forbidden/x"), 1);
    assert_eq!(value(&report, "forbidden/y"), 1);
    snapshot
        .validate_assignment(&report.solution.as_ref().expect("solution").stable_values)
        .expect("Gurobi incumbent must satisfy the source CP snapshot");
    assert_eq!(
        report.fingerprints.model.as_ref(),
        Some(&snapshot.fingerprint)
    );
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
        .expect("CP report should satisfy the shared contract");
}

#[test]
fn gurobi_exact_lowering_solves_optional_variable_duration_no_overlap() {
    let presence = IntegerVariable::new("schedule/first/presence");
    let first_start = IntegerVariable::new("schedule/first/start");
    let first_duration = IntegerVariable::new("schedule/first/duration");
    let first_end = IntegerVariable::new("schedule/first/end");
    let second_start = IntegerVariable::new("schedule/second/start");
    let second_end = IntegerVariable::new("schedule/second/end");
    let mut model = ConstraintProgrammingModel::new("gurobi-optional-variable-no-overlap");
    for (variable, domain) in [
        (presence.clone(), IntegerDomain::boolean()),
        (
            first_start.clone(),
            IntegerDomain::range(0, 3).expect("first start domain"),
        ),
        (
            first_duration.clone(),
            IntegerDomain::range(1, 2).expect("duration domain"),
        ),
        (
            first_end.clone(),
            IntegerDomain::range(1, 5).expect("first end domain"),
        ),
        (
            second_start.clone(),
            IntegerDomain::range(0, 3).expect("second start domain"),
        ),
        (
            second_end.clone(),
            IntegerDomain::range(1, 4).expect("second end domain"),
        ),
    ] {
        model
            .register_variable(variable, domain)
            .expect("schedule variable");
    }
    model
        .register_interval(
            IntervalVariable::new(
                "schedule/first",
                IntegerExpression::variable(first_start.clone()),
                IntervalDuration::Variable(first_duration.clone()),
                IntegerExpression::variable(first_end.clone()),
                Some(
                    ospf_rust_core::model::constraint_programming::BooleanLiteral::positive(
                        presence.clone(),
                    ),
                ),
            )
            .expect("first interval"),
        )
        .expect("register first interval");
    model
        .register_interval(
            IntervalVariable::new(
                "schedule/second",
                IntegerExpression::variable(second_start.clone()),
                IntervalDuration::Fixed(1),
                IntegerExpression::variable(second_end.clone()),
                None,
            )
            .expect("second interval"),
        )
        .expect("register second interval");
    model
        .add_constraint(ConstraintDefinition::new(
            "schedule/first-present",
            ConstraintProgrammingConstraint::integer(
                IntegerExpression::variable(presence.clone()),
                IntegerRelation::Equal,
                1,
            ),
        ))
        .expect("presence constraint");
    model
        .add_constraint(ConstraintDefinition::new(
            "schedule/no-overlap",
            ConstraintProgrammingConstraint::NoOverlap {
                intervals: vec!["schedule/first".into(), "schedule/second".into()],
            },
        ))
        .expect("NoOverlap constraint");
    model.set_objective(IntegerObjective::minimize(expression(
        0,
        [(first_start, 1), (second_start, 1), (first_duration, 1)],
    )));
    let snapshot = model.freeze().expect("snapshot");

    let report = solver()
        .solve_constraint_programming(&snapshot, &Default::default())
        .expect("Gurobi CP scheduling solve");

    assert_eq!(report.problem_status, ProblemStatus::Feasible);
    assert!(report.is_optimal());
    let assignment = report
        .solution
        .as_ref()
        .expect("schedule solution")
        .stable_values
        .clone();
    assert_eq!(
        assignment[&StableVariableId::from("schedule/first/presence")],
        1
    );
    snapshot
        .validate_assignment(&assignment)
        .expect("Gurobi schedule incumbent must satisfy NoOverlap and interval semantics");
    let first_start = assignment[&StableVariableId::from("schedule/first/start")];
    let first_end = assignment[&StableVariableId::from("schedule/first/end")];
    let second_start = assignment[&StableVariableId::from("schedule/second/start")];
    let second_end = assignment[&StableVariableId::from("schedule/second/end")];
    assert!(first_end <= second_start || second_end <= first_start);
    assert_eq!(
        report.fingerprints.model.as_ref(),
        Some(&snapshot.fingerprint)
    );
    report
        .validate()
        .expect("CP report should satisfy the shared contract");
}

#[test]
fn gurobi_cp_capability_reports_the_exact_subset_before_solving() {
    let x = IntegerVariable::new("capability/x");
    let mut model = ConstraintProgrammingModel::new("gurobi-capability");
    model
        .register_variable(x.clone(), IntegerDomain::boolean())
        .expect("x variable");
    model
        .add_constraint(ConstraintDefinition::new(
            "capability/forbidden",
            ConstraintProgrammingConstraint::ForbiddenAssignments {
                expressions: vec![IntegerExpression::variable(x)],
                tuples: vec![vec![1]],
            },
        ))
        .expect("forbidden table");
    let snapshot = model.freeze().expect("snapshot");

    let support = solver().analyze_support(&snapshot);
    assert!(support.satisfaction);
    assert!(support.integer_objective);
    assert_eq!(support.constraints.len(), 1);
    assert!(support.constraints.values().all(|support| matches!(
        support,
        ospf_rust_core::solver::ConstraintProgrammingSupport::ExactLowering
    )));
    assert!(
        support
            .notes
            .iter()
            .any(|note| note.contains("exact finite"))
    );
}
