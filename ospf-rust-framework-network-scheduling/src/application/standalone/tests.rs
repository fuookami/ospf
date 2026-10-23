#![allow(clippy::borrow_interior_mutable_const)]

use ospf_rust_core::error::{CoreError, SolverError};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{
    ProblemStatus, SolverCapability, SolverInfo, SolverOutput, TerminationReason,
};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Kilogram, NoneUnit};
use crate::domain::flow::{FlowArc, FlowCommodity, FlowNode};
use crate::domain::generalized_flow::{GeneralizedFlowArc, GeneralizedFlowNode};
use crate::infrastructure::{CapacityBounds, NetworkCost};
use super::*;

#[derive(Clone)]
enum Output {
    Values(Vec<f64>),
    Terminal(ProblemStatus, TerminationReason),
    UnboundProof,
    WrongObjective,
    TechnicalError,
}

struct FakeSolver(Output);

impl SolverInfo for FakeSolver {
    fn name(&self) -> &str { "fake-network" }

    fn capabilities(&self) -> Vec<SolverCapability> { Vec::new() }
}

impl LinearSolver for FakeSolver {
    fn solve_linear(&self, _model: &LinearTriadModel) -> ospf_rust_core::error::Result<SolverOutput> {
        Err(CoreError::Solver(SolverError::NumericalError("test backend failure".into())))
    }

    fn solve_linear_report(&self, model: &LinearTriadModel) -> ospf_rust_core::error::Result<SolveReport<f64>> {
        match &self.0 {
            Output::Values(values) => {
                let mut solution = SolveSolution::vector(values.clone());
                solution.objective_value = Some(model.c.iter().zip(values).map(|(coefficient, value)| coefficient * value).sum());
                SolveReport::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
                    .solution(solution).build()
            }
            Output::Terminal(status, reason) => SolveReport::builder(*status, *reason).build(),
            Output::UnboundProof => SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(SolveSolution::vector(vec![2.0])).proof(SolveProof::optimality()).build(),
            Output::WrongObjective => {
                let mut solution = SolveSolution::vector(vec![2.0]);
                solution.objective_value = Some(7.0);
                SolveReport::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit).solution(solution).build()
            }
            Output::TechnicalError => self.solve_linear(model).and_then(|_| unreachable!()),
        }
    }
}

fn flow_problem() -> FlowProblem<f64> {
    let unit = Kilogram::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    FlowProblem::new(
        vec![FlowNode::new("a"), FlowNode::new("b")],
        vec![FlowArc::new(
            "ab", "a", "b",
            CapacityBounds::try_new(Quantity::new(0.0, unit.clone()), Quantity::new(3.0, unit.clone())).unwrap(),
            NetworkCost::new(Quantity::new(3.0, cost)),
        )],
        vec![FlowCommodity::new("m")
            .with_balance("a", Quantity::new(2.0, unit.clone()))
            .with_balance("b", Quantity::new(-2.0, unit))],
    ).unwrap()
}

fn generalized_problem() -> GeneralizedFlowProblem<f64> {
    let unit = Kilogram::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    GeneralizedFlowProblem::new(
        vec![
            GeneralizedFlowNode { id: "a".into(), net_supply: Quantity::new(2.0, unit.clone()) },
            GeneralizedFlowNode { id: "b".into(), net_supply: Quantity::new(-1.0, unit.clone()) },
        ],
        vec![GeneralizedFlowArc {
            id: "ab".into(), from: "a".into(), to: "b".into(),
            input_capacity: CapacityBounds::try_new(Quantity::new(0.0, unit.clone()), Quantity::new(3.0, unit.clone())).unwrap(),
            cost: NetworkCost::new(Quantity::new(3.0, (&cost / &unit).build())),
            production_yield: Quantity::new(0.5, cost.clone()), input_to_output_factor: None,
        }], cost,
    ).unwrap()
}

#[test]
fn standalone_services_reuse_registration_and_validate_every_incumbent() {
    let service = FlowApplicationService::new(FakeSolver(Output::Values(vec![2.0])));
    for _ in 0..2 {
        let report = service.solve(&flow_problem()).unwrap();
        assert_eq!(report.incumbent().unwrap().total_cost.value, 6.0);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.is_optimal());
    }
    let report = GeneralizedFlowApplicationService::new(FakeSolver(Output::Values(vec![2.0])))
        .solve(&generalized_problem()).unwrap();
    assert_eq!(report.incumbent().unwrap().arcs[&"ab".into()].output.value, 1.0);
    for values in [vec![], vec![f64::NAN], vec![f64::INFINITY], vec![-0.1], vec![4.0], vec![1.0]] {
        assert!(FlowApplicationService::new(FakeSolver(Output::Values(values.clone()))).solve(&flow_problem()).is_err());
        assert!(GeneralizedFlowApplicationService::new(FakeSolver(Output::Values(values))).solve(&generalized_problem()).is_err());
    }
    for output in [Output::UnboundProof, Output::WrongObjective] {
        assert!(FlowApplicationService::new(FakeSolver(output.clone())).solve(&flow_problem()).is_err());
        assert!(GeneralizedFlowApplicationService::new(FakeSolver(output)).solve(&generalized_problem()).is_err());
    }
}

#[test]
fn normal_no_incumbent_reports_are_not_technical_errors_or_invented_solutions() {
    for output in [
        Output::Terminal(ProblemStatus::Infeasible, TerminationReason::Completed),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::TimeLimit),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::Cancelled),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::BackendFailure),
    ] {
        let Output::Terminal(status, reason) = &output else { unreachable!() };
        let report = FlowApplicationService::new(FakeSolver(output.clone())).solve(&flow_problem()).unwrap();
        assert_eq!(&report.problem_status, status);
        assert_eq!(&report.termination_reason, reason);
        assert!(report.incumbent().is_none());
        let report = GeneralizedFlowApplicationService::new(FakeSolver(output)).solve(&generalized_problem()).unwrap();
        assert!(!report.has_incumbent());
    }
    assert!(FlowApplicationService::new(FakeSolver(Output::TechnicalError)).solve(&flow_problem()).is_err());
    assert!(GeneralizedFlowApplicationService::new(FakeSolver(Output::TechnicalError)).solve(&generalized_problem()).is_err());
}

#[test]
fn path_compilation_service_preserves_finite_scope_and_rejects_invalid_solver_results() {
    let problem = crate::domain::path_compilation::tests::problem();
    let service = PathCompilationApplicationService::new(FakeSolver(Output::Values(vec![0.0, 0.0, 1.0])));
    for _ in 0..2 {
        let report = service.solve(&problem).unwrap();
        assert_eq!(report.incumbent().unwrap().cost.value, 5.0);
        assert_eq!(report.incumbent().unwrap().candidate_ids.len(), 3);
        assert_eq!(report.incumbent().unwrap().selected[0].id, "whole");
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.is_optimal());
    }
    for output in [
        Output::Terminal(ProblemStatus::Infeasible, TerminationReason::Completed),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::TimeLimit),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::Cancelled),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::BackendFailure),
    ] {
        let Output::Terminal(status, reason) = &output else { unreachable!() };
        let report = PathCompilationApplicationService::new(FakeSolver(output.clone())).solve(&problem).unwrap();
        assert_eq!(&report.problem_status, status);
        assert_eq!(&report.termination_reason, reason);
        assert!(!report.has_incumbent());
    }
    for output in [
        Output::TechnicalError, Output::UnboundProof, Output::WrongObjective,
        Output::Values(Vec::new()), Output::Values(vec![f64::NAN; 3]),
        Output::Values(vec![0.0, 0.0, f64::INFINITY]), Output::Values(vec![-0.1, 0.0, 1.0]),
        Output::Values(vec![0.5; 3]), Output::Values(vec![0.0; 3]), Output::Values(vec![1.0; 3]),
    ] {
        assert!(PathCompilationApplicationService::new(FakeSolver(output)).solve(&problem).is_err());
    }
}

#[test]
fn design_and_resilience_services_preserve_terminal_reports_and_reject_invalid_incumbents() {
    let design = crate::domain::design::tests::problem(0.0, 2.0);
    let resilience = crate::domain::resilience::NetworkResilienceProblem::new(design.clone(), Vec::new(), Vec::new()).unwrap();
    let report = NetworkDesignApplicationService::new(FakeSolver(Output::Values(vec![1.0, 1.0, 0.0, 2.0])))
        .solve(&design).unwrap();
    assert_eq!(report.incumbent().unwrap().total_cost.value, 10.0);
    let report = NetworkResilienceApplicationService::new(FakeSolver(Output::Values(vec![1.0, 1.0, 0.0, 4.0, 2.0])))
        .solve(&resilience).unwrap();
    assert_eq!(report.incumbent().unwrap().total_cost.value, 10.0);
    for output in [
        Output::Terminal(ProblemStatus::Infeasible, TerminationReason::Completed),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::TimeLimit),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::Cancelled),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::BackendFailure),
    ] {
        let Output::Terminal(status, reason) = &output else { unreachable!() };
        let report = NetworkDesignApplicationService::new(FakeSolver(output.clone())).solve(&design).unwrap();
        assert_eq!(&report.problem_status, status);
        assert_eq!(&report.termination_reason, reason);
        assert!(!report.has_incumbent());
        let report = NetworkResilienceApplicationService::new(FakeSolver(output)).solve(&resilience).unwrap();
        assert!(!report.has_incumbent());
    }
    for output in [Output::TechnicalError, Output::UnboundProof, Output::WrongObjective,
        Output::Values(Vec::new()), Output::Values(vec![f64::NAN; 4]),
        Output::Values(vec![0.0, 1.0, 0.0, 2.0]), Output::Values(vec![1.0, 1.0, 1.0, 2.0]),
    ] {
        assert!(NetworkDesignApplicationService::new(FakeSolver(output.clone())).solve(&design).is_err());
        assert!(NetworkResilienceApplicationService::new(FakeSolver(output)).solve(&resilience).is_err());
    }
}

#[test]
fn structure_service_reuses_context_and_preserves_terminal_reports_without_inventing_incumbents() {
    use crate::domain::structure::{NetworkStructureContext, tests::{optimal_values, problem}};
    let problem = problem();
    let mut context = NetworkStructureContext::new(problem.clone(), "fixture").unwrap();
    let mut model = MetaModel::new("fixture");
    context.register(&mut model, Default::default()).unwrap();
    let values = optimal_values(&context, &model);
    let service = NetworkStructureApplicationService::new(FakeSolver(Output::Values(values.clone())));
    for _ in 0..2 {
        let report = service.solve(&problem).unwrap();
        assert_eq!(report.incumbent().unwrap().construction.cost.value, 3.0);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.is_optimal());
    }
    for output in [
        Output::Terminal(ProblemStatus::Infeasible, TerminationReason::Completed),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::TimeLimit),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::Cancelled),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::BackendFailure),
    ] {
        let Output::Terminal(status, reason) = &output else { unreachable!() };
        let report = NetworkStructureApplicationService::new(FakeSolver(output.clone())).solve(&problem).unwrap();
        assert_eq!(&report.problem_status, status);
        assert_eq!(&report.termination_reason, reason);
        assert!(!report.has_incumbent());
    }
    for output in [Output::TechnicalError, Output::UnboundProof, Output::WrongObjective,
        Output::Values(Vec::new()), Output::Values(vec![f64::NAN; values.len()]),
        Output::Values(vec![f64::INFINITY; values.len()]), Output::Values(vec![0.0; values.len()]),
    ] {
        assert!(NetworkStructureApplicationService::new(FakeSolver(output)).solve(&problem).is_err());
    }
}

#[cfg(feature = "scip")]
#[test]
fn scip_standalone_flow_and_generalized_flow_execute_real_backend_and_extract_domain_results() {
    use ospf_rust_core::solver::solvers::SCIPSolver;
    let flow = FlowApplicationService::new(SCIPSolver::new()).solve(&flow_problem()).unwrap();
    assert!(flow.is_optimal());
    assert!((flow.incumbent().unwrap().total_cost.value - 6.0).abs() < 1e-7);
    let generalized = GeneralizedFlowApplicationService::new(SCIPSolver::new()).solve(&generalized_problem()).unwrap();
    assert!(generalized.is_optimal());
    assert!((generalized.incumbent().unwrap().arcs[&"ab".into()].output.value - 1.0).abs() < 1e-7);
}

#[test]
fn allocation_service_reuses_context_and_rejects_invalid_incumbents_and_reports() {
    use crate::domain::allocation::{AllocationContext, tests::{problem, values}};
    let problem = problem();
    let mut context = AllocationContext::new(problem.clone(), "fixture").unwrap();
    let mut model = MetaModel::new("fixture");
    context.register(&mut model, Default::default()).unwrap();
    let values = values(&context, &model, [1.0, 1.0, 2.0], [1.0, 1.0]);
    let service = AllocationApplicationService::new(FakeSolver(Output::Values(values.clone())));
    for _ in 0..2 {
        let report = service.solve(&problem).unwrap();
        assert_eq!(report.incumbent().unwrap().total_cost.value, 9.0);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.is_optimal());
    }
    for output in [
        Output::Terminal(ProblemStatus::Infeasible, TerminationReason::Completed),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::TimeLimit),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::Cancelled),
        Output::Terminal(ProblemStatus::Unknown, TerminationReason::BackendFailure),
    ] {
        let Output::Terminal(status, reason) = &output else { unreachable!() };
        let report = AllocationApplicationService::new(FakeSolver(output.clone())).solve(&problem).unwrap();
        assert_eq!(&report.problem_status, status);
        assert_eq!(&report.termination_reason, reason);
        assert!(!report.has_incumbent());
    }
    for output in [Output::TechnicalError, Output::UnboundProof, Output::WrongObjective,
        Output::Values(Vec::new()), Output::Values(vec![f64::NAN; values.len()]),
        Output::Values(vec![f64::INFINITY; values.len()]), Output::Values(vec![0.0; values.len()]),
    ] {
        assert!(AllocationApplicationService::new(FakeSolver(output)).solve(&problem).is_err());
    }
}
