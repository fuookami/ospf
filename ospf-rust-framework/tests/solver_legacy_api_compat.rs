use ospf_rust_core::solver::{
    ProblemStatus, SolveProof, SolveReport, SolveSolution, SolveValueConversionPolicy,
    TerminationReason,
};
use ospf_rust_framework::{FeasibleSolution, LPResult, LinearDualSolution};

fn certified_lp_report() -> SolveReport<f64> {
    let mut solution = SolveSolution::vector(vec![1.0]);
    solution.objective = Some(1.0);
    solution.objective_value = Some(1.0);
    solution.dual_solution = Some(vec![1.0]);
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
        .solution(solution)
        .proof(SolveProof::optimality())
        .build()
        .expect("certified LP report should be valid")
}

#[test]
fn downstream_framework_legacy_solution_and_lp_result_preserve_report_semantics() {
    let report = certified_lp_report();
    let solution = FeasibleSolution::try_from_report(&report)
        .expect("verified report should project to the legacy framework solution");
    assert_eq!(solution.obj, 1.0);
    assert_eq!(solution.solution, vec![1.0]);

    let round_trip = solution
        .to_solve_report("downstream-legacy")
        .expect("legacy framework solution should project back");
    assert!(round_trip.is_optimal());
    assert_eq!(round_trip.solution.as_ref().unwrap().values, vec![1.0]);

    let lp_result = LPResult::try_from_solve_report(&report)
        .expect("verified report should project to the legacy LP result");
    assert_eq!(lp_result.dual_solution.constraints, vec![1.0]);
    let lp_round_trip = lp_result
        .to_solve_report("downstream-legacy-lp")
        .expect("legacy LP result should project back");
    assert!(lp_round_trip.is_optimal());
    assert_eq!(
        lp_round_trip
            .solution
            .as_ref()
            .unwrap()
            .dual_solution
            .as_ref(),
        Some(&vec![1.0])
    );

    let typed = solution
        .try_into_typed::<f64>(SolveValueConversionPolicy::Strict)
        .expect("legacy framework solution should retain typed conversion");
    assert_eq!(typed.solution, vec![1.0]);
}

#[test]
fn downstream_legacy_projection_keeps_nonoptimal_termination_and_lp_gate() {
    let mut solution = SolveSolution::vector(vec![1.0]);
    solution.objective = Some(1.0);
    solution.objective_value = Some(1.0);
    let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
        .solution(solution)
        .build()
        .expect("limited report should be valid");

    let legacy = FeasibleSolution::try_from_report(&report)
        .expect("incumbent at a time limit remains a legacy solution");
    let projected = legacy
        .to_solve_report("downstream-time-limit")
        .expect("legacy projection should retain the time limit");
    assert_eq!(projected.termination_reason, TerminationReason::TimeLimit);
    assert!(!projected.is_optimal());

    assert!(LPResult::try_from_solve_report(&report).is_err());
    let manually_constructed = LPResult::new(
        FeasibleSolution::new(1.0, vec![1.0]),
        LinearDualSolution::new(vec![1.0], Vec::new()),
    );
    assert!(
        manually_constructed
            .to_solve_report("legacy-manual")
            .is_ok()
    );
}
