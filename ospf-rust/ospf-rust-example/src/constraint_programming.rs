//! 约束规划示例 / Constraint-programming examples.
//!
//! 这些示例刻意保持离线可运行，分别验证精确 `i64` CP 模型、framework 注册边界和
//! Logic-Based Benders 报告合同，不把 fake solver 冒充为生产级原生 CP backend。
//! These examples are deliberately offline. They exercise the exact `i64` CP model, the
//! framework registration boundary, and the Logic-Based Benders report contract without
//! pretending that the fake solver is a native production CP backend.

use ospf_rust_core::error::Result;
use ospf_rust_core::model::constraint_programming::{
    ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
    ConstraintProgrammingSnapshot, IntegerDomain, IntegerExpression, IntegerObjective,
    IntegerRelation, IntegerTerm, IntegerVariable, IntervalDuration, IntervalVariable,
};
use ospf_rust_core::model::mechanism::ConstraintGroup;
use ospf_rust_core::solver::{
    ConstraintProgrammingAssumption, ConstraintProgrammingSolver, FakeConstraintProgrammingSolver,
    ProblemStatus, SolveProof, SolveReport, SolveSolution, StableVariableId, TerminationReason,
};
use ospf_rust_framework::model::{
    ConstraintProgrammingContext, ConstraintProgrammingModelComponent,
};
use ospf_rust_framework::solver::{
    ConstraintProgrammingSubproblemResult, FrameworkSolveOptions, LogicBasedBendersEngine,
    MasterAssignment, MasterBinding, MasterVariableBinding,
};

fn linear_expression(
    constant: i64,
    terms: impl IntoIterator<Item = (IntegerVariable, i64)>,
) -> Result<IntegerExpression> {
    IntegerExpression::linear(
        constant,
        terms
            .into_iter()
            .map(|(variable, coefficient)| IntegerTerm {
                variable,
                coefficient,
            }),
    )
}

fn report_solution_value(report: &SolveReport<i64>, id: &str) -> Result<i64> {
    report
        .solution
        .as_ref()
        .and_then(|solution| solution.stable_values.get(&StableVariableId::from(id)))
        .copied()
        .ok_or_else(|| {
            ospf_rust_core::error::CoreError::contract_error("example solution is missing")
        })
}

/// 使用精确离线 CP solver 求解小型 assignment 模型。
/// Solve a small assignment model with the exact offline CP solver.
pub fn run_assignment_example() -> Result<SolveReport<i64>> {
    let variables = [
        IntegerVariable::new("assignment/worker-0/job-0"),
        IntegerVariable::new("assignment/worker-0/job-1"),
        IntegerVariable::new("assignment/worker-1/job-0"),
        IntegerVariable::new("assignment/worker-1/job-1"),
    ];
    let mut model = ConstraintProgrammingModel::new("cp-assignment-example");
    for variable in &variables {
        model.register_variable(variable.clone(), IntegerDomain::boolean())?;
    }

    let job_zero = linear_expression(0, [(variables[0].clone(), 1), (variables[2].clone(), 1)])?;
    let job_one = linear_expression(0, [(variables[1].clone(), 1), (variables[3].clone(), 1)])?;
    let worker_zero = linear_expression(0, [(variables[0].clone(), 1), (variables[1].clone(), 1)])?;
    let worker_one = linear_expression(0, [(variables[2].clone(), 1), (variables[3].clone(), 1)])?;
    model.add_constraint_by_id(
        "assignment/job-0",
        ConstraintProgrammingConstraint::integer(job_zero, IntegerRelation::Equal, 1),
    )?;
    model.add_constraint_by_id(
        "assignment/job-1",
        ConstraintProgrammingConstraint::integer(job_one, IntegerRelation::Equal, 1),
    )?;
    model.add_constraint_by_id(
        "assignment/worker-0",
        ConstraintProgrammingConstraint::integer(worker_zero, IntegerRelation::LessOrEqual, 1),
    )?;
    model.add_constraint_by_id(
        "assignment/worker-1",
        ConstraintProgrammingConstraint::integer(worker_one, IntegerRelation::LessOrEqual, 1),
    )?;
    model.set_objective(IntegerObjective::minimize(linear_expression(
        0,
        [
            (variables[0].clone(), 4),
            (variables[1].clone(), 1),
            (variables[2].clone(), 2),
            (variables[3].clone(), 5),
        ],
    )?));

    let snapshot = model.freeze()?;
    let report = FakeConstraintProgrammingSolver::new()
        .solve_constraint_programming(&snapshot, &Default::default())?;
    if report.problem_status != ProblemStatus::Feasible || !report.is_optimal() {
        return Err(ospf_rust_core::error::CoreError::contract_error(
            "assignment example did not produce an optimal report",
        ));
    }
    if report_solution_value(&report, "assignment/worker-0/job-1")? != 1
        || report_solution_value(&report, "assignment/worker-1/job-0")? != 1
    {
        return Err(ospf_rust_core::error::CoreError::contract_error(
            "assignment example selected an unexpected optimum",
        ));
    }
    Ok(report)
}

struct SchedulingComponent;

impl ConstraintProgrammingModelComponent for SchedulingComponent {
    fn name(&self) -> &str {
        "offline-no-overlap-schedule"
    }

    fn constraint_group(&self) -> Option<ConstraintGroup> {
        Some(ConstraintGroup::new(41, "cp-scheduling"))
    }

    fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
        let first_start = IntegerVariable::new("schedule/task-0/start");
        let second_start = IntegerVariable::new("schedule/task-1/start");
        model.register_variable(first_start.clone(), IntegerDomain::range(0, 4)?)?;
        model.register_variable(second_start.clone(), IntegerDomain::range(0, 4)?)?;
        let first_end = linear_expression(2, [(first_start.clone(), 1)])?;
        let second_end = linear_expression(2, [(second_start.clone(), 1)])?;
        model.register_interval(IntervalVariable::new(
            "schedule/task-0",
            IntegerExpression::variable(first_start),
            IntervalDuration::Fixed(2),
            first_end,
            None,
        )?)?;
        model.register_interval(IntervalVariable::new(
            "schedule/task-1",
            IntegerExpression::variable(second_start),
            IntervalDuration::Fixed(2),
            second_end,
            None,
        )?)?;
        model.add_constraint(ConstraintDefinition::new(
            "schedule/no-overlap",
            ConstraintProgrammingConstraint::NoOverlap {
                intervals: vec!["schedule/task-0".into(), "schedule/task-1".into()],
            },
        ))?;
        Ok(())
    }
}

/// 通过 framework CP context 注册并求解 scheduling 模型。
/// Register and solve a scheduling model through the framework CP context.
pub fn run_scheduling_example() -> Result<SolveReport<i64>> {
    let mut context = ConstraintProgrammingContext::new();
    context.add_extra_component(SchedulingComponent);
    let mut model = ConstraintProgrammingModel::new("cp-scheduling-example");
    let snapshot = context.freeze(&mut model)?;
    let report = FakeConstraintProgrammingSolver::new()
        .solve_constraint_programming(&snapshot, &Default::default())?;
    if report.problem_status != ProblemStatus::Feasible || !report.has_incumbent() {
        return Err(ospf_rust_core::error::CoreError::contract_error(
            "scheduling example did not produce a feasible report",
        ));
    }
    Ok(report)
}

fn master_report(x: i64) -> Result<SolveReport<f64>> {
    let mut solution = SolveSolution::vector(Vec::new());
    solution
        .stable_values
        .insert(StableVariableId::from("master/use-cp"), x as f64);
    solution.objective = Some(x as f64);
    solution.objective_value = Some(x as f64);
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
        .solution(solution)
        .proof(SolveProof::optimality())
        .build()
}

fn cp_subproblem_fixture() -> Result<(ConstraintProgrammingSnapshot, IntegerVariable)> {
    let variable = IntegerVariable::new("subproblem/selected");
    let mut model = ConstraintProgrammingModel::new("cp-lbb-example");
    model.register_variable(variable.clone(), IntegerDomain::boolean())?;
    model.set_objective(IntegerObjective::minimize(IntegerExpression::variable(
        variable.clone(),
    )));
    model.add_constraint_by_id(
        "subproblem/selected",
        ConstraintProgrammingConstraint::integer(
            IntegerExpression::variable(variable.clone()),
            IntegerRelation::Equal,
            1,
        ),
    )?;
    Ok((model.freeze()?, variable))
}

fn cp_subproblem_report(
    snapshot: &ConstraintProgrammingSnapshot,
    variable: &IntegerVariable,
    use_cp: i64,
    master_assignment: &MasterAssignment,
) -> Result<ConstraintProgrammingSubproblemResult> {
    let solver = FakeConstraintProgrammingSolver::new();
    let assumption = ConstraintProgrammingAssumption::Equal(variable.clone(), use_cp);
    let binding = ospf_rust_framework::solver::MasterAssignmentAssumptionBinding::new(
        [assumption.clone()],
        [(
            assumption.stable_id(),
            StableVariableId::from("master/use-cp"),
        )],
    )?;
    let report = solver.solve_constraint_programming_with_assumptions_and_conflict(
        snapshot,
        &[assumption],
        &Default::default(),
    )?;
    if use_cp == 0 {
        ConstraintProgrammingSubproblemResult::infeasible_from_snapshot_for_master(
            report,
            &snapshot,
            master_assignment,
            &binding,
            None,
        )
    } else {
        ConstraintProgrammingSubproblemResult::feasible_from_snapshot_for_master(
            report,
            snapshot,
            master_assignment,
            &binding,
            Some(1.0),
            Some(1.0),
        )
    }
}

/// 运行两轮 linear master + CP subproblem 的 LBB 示例。
/// Run a two-iteration linear-master plus CP-subproblem LBB example.
pub fn run_logic_based_benders_example() -> Result<SolveReport<f64>> {
    let (snapshot, variable) = cp_subproblem_fixture()?;
    let binding = MasterBinding::new([MasterVariableBinding::binary("master/use-cp")])?;
    let engine = LogicBasedBendersEngine::new(binding);
    let mut master = |cuts: &[ospf_rust_framework::solver::MasterCut],
                      _options: &FrameworkSolveOptions| {
        master_report(if cuts.is_empty() { 0 } else { 1 })
    };
    let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
        cp_subproblem_report(
            &snapshot,
            &variable,
            assignment
                .get(&StableVariableId::from("master/use-cp"))
                .unwrap_or(0),
            assignment,
        )
    };
    let report = engine.solve(
        &mut master,
        &mut subproblem,
        &FrameworkSolveOptions::default()
            .with_iterations(4, 1e-9)
            .with_expected_cp_snapshot_fingerprint(snapshot.fingerprint.clone()),
    )?;
    if !report.is_optimal() || report.trace.total_iterations != 2 {
        return Err(ospf_rust_core::error::CoreError::contract_error(format!(
            "LBB example did not close an exact two-iteration report: {report:?}"
        )));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_example_returns_a_verified_optimum() {
        let report = run_assignment_example().expect("assignment example");
        assert!(report.is_optimal());
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            Some(3)
        );
    }

    #[test]
    fn scheduling_example_uses_framework_context_and_no_overlap() {
        let report = run_scheduling_example().expect("scheduling example");
        assert!(report.has_incumbent());
        let values = &report.solution.as_ref().expect("solution").stable_values;
        assert!(
            values[&StableVariableId::from("schedule/task-0/start")] + 2
                <= values[&StableVariableId::from("schedule/task-1/start")]
                || values[&StableVariableId::from("schedule/task-1/start")] + 2
                    <= values[&StableVariableId::from("schedule/task-0/start")]
        );
    }

    #[test]
    fn lbb_example_returns_a_verified_global_report() {
        let report = run_logic_based_benders_example().expect("LBB example");
        assert!(report.is_optimal());
        assert_eq!(report.trace.total_iterations, 2);
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            Some(1.0)
        );
    }
}
