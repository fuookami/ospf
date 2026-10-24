use std::collections::BTreeSet;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::SubObjective;
use ospf_rust_core::solver::{SolveFingerprints, SolveProof, SolveSolution};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Kilogram;
use crate::{AssignmentKey, DiscreteLoadingProblem, LoadingItem, LoadingPartition, LoadingPosition, PartitionLoadingOptions, SameUnitPackingValueAdapter};
use super::*;

mod certification;

fn registered(cargo: bool) -> RegisteredDiscreteLoading<f64, Kilogram> {
    let input = DiscreteLoadingProblem {
        items: if cargo { vec![LoadingItem { id: "cargo".into(), weight: Quantity::new_ct(1.0), required: true, fixed_position_id: None }] } else { vec![] },
        positions: ["a", "b"].into_iter().map(|id| LoadingPosition {
            id: id.into(), max_count: 2, max_weight: Quantity::new_ct(5.0), fixed_occupied: false, enabled: true,
        }).collect(),
        allowed_assignments: if cargo { ["a", "b"].into_iter().map(|position| AssignmentKey { item_id: "cargo".into(), position_id: position.into() }).collect() } else { BTreeSet::new() },
        conflicts: BTreeSet::new(), layout_groups: vec![], weight_unit: Kilogram,
    };
    let mut context = DiscreteLoadingContext::new(input, &SameUnitPackingValueAdapter).unwrap();
    let mut model = MetaModel::new("application");
    context.register(&mut model).unwrap();
    let terms = context.aggregation().assignments.iter().map(|(key, variable)| {
        let index = model.tokens().iter().position(|token| token.id() == *variable).unwrap();
        LinearMonomial::new(if key.position_id == "a" { 1.0 } else { 2.0 }, index)
    }).collect();
    model.add_sub_objective(SubObjective::minimize(Linear::new(terms, 7.0), "business"));
    RegisteredDiscreteLoading::new(context, model).unwrap()
}

fn partitions() -> PartitionLoadingOptions<f64> {
    PartitionLoadingOptions { partitions: ["a", "b"].into_iter().map(|id| LoadingPartition { id: id.into(), position_ids: BTreeSet::from([id.into()]) }).collect(),
        extra_decisions: vec![], refinable_resources: BTreeSet::new(), tolerance: 1e-7 }
}

#[test]
fn ordinary_service_verifies_complete_source_objective_and_preserves_constant_once() {
    let result = DiscreteLoadingApplicationService::solve(|| Ok(registered(false)), Default::default(), &SolveHandle::new(), |model, _, _| {
        let mut solution = SolveSolution::vector(vec![0.0; model.num_variables()]);
        solution.objective_value = Some(0.0);
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
            .model_mapping(Some(linear_model_mapping(model)?))
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
    }).unwrap();
    assert!(result.solution.is_some());
    assert_eq!(result.objective, Some(7.0));
    assert!(DiscreteLoadingApplicationService::solve(|| Ok(registered(false)), Default::default(), &SolveHandle::new(), |_, _, _| {
        SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Completed).build()
    }).is_err());
}

fn registered_cancelling_objective() -> RegisteredDiscreteLoading<f64, Kilogram> {
    use ospf_rust_core::variable::{Continuous, VariableRange};
    let mut registration = registered(false);
    let mut terms = Vec::new();
    for (index, coefficient) in [1e16, -1.0, -1e16].into_iter().enumerate() {
        let variable = registration.model.register_auto_variable_with_range::<Continuous>(
            &format!("objective_aux_{index}"), VariableRange::bounded(1.0, 1.0)).unwrap();
        terms.push(LinearMonomial::new(coefficient, variable));
    }
    registration.model.add_sub_objective(SubObjective::minimize(Linear::new(terms, 0.0), "cancelling"));
    registration
}

fn cancelling_report(model: &LinearTriadModel) -> Result<SolveReport<f64>> {
    use ospf_rust_core::model::LinearObjectiveEvaluationFlt64;
    let values = model.basic.variables.iter().map(|token| if token.variable.name().starts_with("objective_aux_")
        || token.variable.name().starts_with("column_") { 1.0 } else { 0.0 }).collect::<Vec<_>>();
    let mut report = optimal_report(model, values)?;
    let solution = report.solution.as_mut().unwrap();
    solution.objective_value = Some(LinearObjectiveEvaluationFlt64::new(
        model.c.iter().copied().zip(solution.values.iter().copied()), 0.0)?.variable_value()?);
    solution.dual_solution = Some(vec![0.0; model.num_constraints()]);
    Ok(report)
}

#[test]
fn all_application_entries_share_exact_objective_and_constant_evaluation() {
    let ordinary = DiscreteLoadingApplicationService::solve(|| Ok(registered_cancelling_objective()), Default::default(),
        &SolveHandle::new(), |model, _, _| cancelling_report(model)).unwrap();
    assert_eq!(ordinary.objective, Some(6.0));
    assert_eq!(ordinary.report.solution.unwrap().objective_value, Some(-1.0));
    let benders = DiscreteLoadingBendersApplicationService::solve(|| Ok(registered_cancelling_objective()), Default::default(),
        &SolveHandle::new(), |model, _, _| cancelling_report(model)).unwrap();
    assert_eq!(benders.result.objective, Some(6.0));
    assert_eq!(benders.result.report.solution.unwrap().objective_value, Some(-1.0));
    let registration = registered_cancelling_objective();
    let partition = PartitionColumnGenerationApplicationService::solve(&registration, partitions(), Default::default(),
        &SolveHandle::new(), |model, _, _| cancelling_report(model)).unwrap();
    assert_eq!(partition.objective, Some(6.0));
    assert_eq!(partition.report.solution.unwrap().objective_value, Some(-1.0));
    assert!(partition.solution.is_some());
}

#[test]
fn ordinary_and_partition_zero_budget_do_not_launch_backends() {
    let result = DiscreteLoadingApplicationService::solve::<Kilogram, _, _>(|| panic!("no factory on zero budget"),
        DiscreteLoadingSolveOptions { time_limit: Duration::ZERO, ..Default::default() }, &SolveHandle::new(), |_, _, _| panic!("no solve")).unwrap();
    assert_eq!(result.report.termination_reason, TerminationReason::TimeLimit);
    let result = PartitionColumnGenerationApplicationService::solve(&registered(true), partitions(),
        PartitionColumnGenerationConfig { time_limit: Duration::ZERO, ..Default::default() }, &SolveHandle::new(), |_, _, _| panic!("no solve")).unwrap();
    assert_eq!(result.report.termination_reason, TerminationReason::TimeLimit);
    assert!(!result.lp_converged);
}

fn registered_resource() -> RegisteredDiscreteLoading<f64, Kilogram> {
    use ospf_rust_core::variable::{Continuous, VariableRange};
    let mut registration = registered(false);
    let resource = registration.model.register_auto_variable_with_range::<Continuous>("resource", VariableRange::bounded(0.0, 2.0)).unwrap();
    registration.context.link_additional_weight(&mut registration.model, "a", Linear::new(vec![LinearMonomial::new(1.0, resource)], 0.0)).unwrap();
    registration.model.add_ge_constraint(&[(resource, 1.0)], 1.0, "extension_resource_demand").unwrap();
    registration
}

fn optimal_report(model: &LinearTriadModel, values: Vec<f64>) -> Result<SolveReport<f64>> {
    let mut solution = SolveSolution::vector(values);
    solution.objective_value = Some(model.c.iter().zip(&solution.values).map(|(coefficient, value)| coefficient * value).sum());
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
        .model_mapping(Some(linear_model_mapping(model)?))
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
}

#[test]
fn duplicate_pricing_in_an_earlier_block_does_not_spend_the_last_new_column_slot() {
    let mut registration = registered(false);
    let terms = ["a", "b"].into_iter().zip([-10.0, -1.0]).map(|(position, cost)| {
        let variable = registration.context.aggregation().occupied[position];
        let column = registration.model.tokens().iter().position(|token| token.id() == variable).unwrap();
        LinearMonomial::new(cost, column)
    }).collect();
    registration.model.add_sub_objective(SubObjective::minimize(Linear::new(terms, 0.0), "independent_rewards"));
    let result = PartitionColumnGenerationApplicationService::solve(&registration, partitions(),
        PartitionColumnGenerationConfig { column_limit: 3, ..Default::default() }, &SolveHandle::new(), |model, _, _| {
            let master = model.basic.variables.iter().any(|token| token.name().starts_with("column_"));
            let has_new_b = model.basic.variables.iter().any(|token| token.name() == "column_2");
            let values = model.basic.variables.iter().enumerate().map(|(column, token)| {
                if master {
                    if token.name() == "column_0" || token.name() == if has_new_b { "column_2" } else { "column_1" } { 1.0 } else { 0.0 }
                } else if model.c[column] < 0.0 || token.name().ends_with("_occupied_0") { 1.0 } else { 0.0 }
            }).collect();
            let mut report = optimal_report(model, values)?;
            // lambda 的上界承担全部负检验数，对偶行价格为零是合法退化最优证书。
            // Lambda upper bounds absorb negative reduced costs; zero row prices form a valid degenerate optimal certificate.
            report.solution.as_mut().unwrap().dual_solution = Some(vec![0.0; model.num_constraints()]);
            Ok(report)
        }).unwrap();
    assert_eq!(result.root_column_count, 3);
    assert!(result.trace.iter().any(|iteration| iteration.added_columns == 1));
    assert_eq!(result.objective, Some(-4.0));
    assert_eq!(result.solution.unwrap().occupied_positions, BTreeSet::from(["a".into(), "b".into()]));
}

#[test]
fn benders_fallback_uses_fresh_equivalent_factory_and_remaining_budget() {
    use ospf_rust_framework::solver::FixedRecourseStop;
    let mut factories = 0;
    let mut owners = Vec::new();
    let options = DiscreteLoadingBendersOptions { fallback_to_milp: true,
        config: ospf_rust_framework::solver::FixedRecourseConfig { recover_missing_certificates: false, ..Default::default() }, ..Default::default() };
    let total = options.config.time_limit;
    let result = DiscreteLoadingBendersApplicationService::solve(|| {
        factories += 1;
        let registration = registered_resource();
        owners.push(registration.model.linear_model_snapshot()?.revision.owner);
        Ok(registration)
    }, options, &SolveHandle::new(), |model, remaining, _| {
        assert!(remaining <= total);
        if model.has_integer_variables() && model.num_variables() == 2 { return optimal_report(model, vec![0.0, 0.0]); }
        if !model.has_integer_variables() {
            return SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed).proof(SolveProof::infeasibility())
                .model_mapping(Some(linear_model_mapping(model)?))
                .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build();
        }
        optimal_report(model, vec![1.0, 0.0, 1.0])
    }).unwrap();
    assert_eq!(factories, 2);
    assert_ne!(owners[0], owners[1]);
    assert!(result.fallback_used);
    assert_eq!(result.decomposition.unwrap().stop, FixedRecourseStop::CertificateMissing);
    assert!(result.result.solution.unwrap().occupied_positions.contains("a"));
    assert_eq!(result.result.objective, Some(7.0));
}

#[test]
fn benders_cancellation_and_input_errors_never_start_fallback() {
    let mut factories = 0;
    let handle = SolveHandle::new();
    let result = DiscreteLoadingBendersApplicationService::solve(|| { factories += 1; Ok(registered_resource()) },
        DiscreteLoadingBendersOptions { fallback_to_milp: true, ..Default::default() }, &handle, |model, _, handle| {
            if model.has_integer_variables() { return optimal_report(model, vec![0.0, 0.0]); }
            handle.cancel("during_subproblem");
            SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Cancelled).build()
        }).unwrap();
    assert_eq!(factories, 1);
    assert!(!result.fallback_used);
    assert_eq!(result.result.report.termination_reason, TerminationReason::Cancelled);
    let mut first = Some(registered(false));
    let variable = first.as_ref().unwrap().context.aggregation().occupied["a"];
    let options = DiscreteLoadingBendersOptions { partition: ospf_rust_framework::solver::LinearBendersPartitionOptions {
        stages: std::collections::BTreeMap::from([(variable, ospf_rust_framework::solver::LinearBendersStage::ContinuousRecourse)]) },
        fallback_to_milp: true, ..Default::default() };
    assert!(DiscreteLoadingBendersApplicationService::solve(|| Ok(first.take().expect("no fallback factory on input errors")), options,
        &SolveHandle::new(), |_, _, _| panic!("invalid input cannot solve")).is_err());
    let result = DiscreteLoadingBendersApplicationService::solve::<Kilogram, _, _>(|| panic!("zero budget cannot construct"),
        DiscreteLoadingBendersOptions { config: ospf_rust_framework::solver::FixedRecourseConfig { time_limit: Duration::ZERO, ..Default::default() },
            fallback_to_milp: true, ..Default::default() }, &SolveHandle::new(), |_, _, _| panic!("zero budget cannot solve")).unwrap();
    assert!(!result.fallback_used);
    assert_eq!(result.result.report.termination_reason, TerminationReason::TimeLimit);
}

#[test]
fn benders_all_integer_models_finish_without_an_empty_recourse_solve_or_proof_promotion() {
    let mut solves = 0;
    let result = DiscreteLoadingBendersApplicationService::solve(|| Ok(registered(false)), Default::default(), &SolveHandle::new(), |model, _, _| {
        solves += 1;
        optimal_report(model, vec![0.0; model.num_variables()])
    }).unwrap();
    assert_eq!(solves, 1);
    assert_eq!(result.decomposition.unwrap().stop, ospf_rust_framework::solver::FixedRecourseStop::Converged);
    assert!(!result.result.report.is_optimal());
    assert!(result.result.report.has_incumbent());
    assert_eq!(result.result.objective, Some(7.0));
}

#[test]
fn unsupported_partition_and_backend_errors_use_explicit_fresh_fallback_but_input_errors_do_not() {
    use ospf_rust_core::error::{CoreError, SolverError, SolverErrorClass};
    use ospf_rust_core::token::TokenVariableData;
    use ospf_rust_core::variable::{VariableType, new_standalone_id};
    use ospf_rust_framework::solver::{LinearBendersPartitionOptions, LinearBendersStage};
    let variable = new_standalone_id();
    let mut factories = 0;
    let mut solves = 0;
    let result = DiscreteLoadingBendersApplicationService::solve(|| {
        factories += 1;
        let mut registration = registered(false);
        let resource = registration.model.register_variable_data(TokenVariableData { id: variable, index: 0, name: "unboxed_resource".into(),
            display_name: None, var_type: VariableType::Continuous, lower_bound: None, upper_bound: None })?;
        registration.model.add_ge_constraint(&[(resource, 1.0)], 1.0, "lower_from_row")?;
        registration.model.add_le_constraint(&[(resource, 1.0)], 2.0, "upper_from_row")?;
        registration.model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(1.0, resource)], 0.0), "resource_objective"));
        Ok(registration)
    }, DiscreteLoadingBendersOptions { partition: LinearBendersPartitionOptions { stages: std::collections::BTreeMap::from([(variable, LinearBendersStage::ContinuousRecourse)]) },
        fallback_to_milp: true, ..Default::default() }, &SolveHandle::new(), |model, _, _| {
        solves += 1;
        optimal_report(model, vec![0.0, 0.0, 1.0])
    }).unwrap();
    assert_eq!((factories, solves), (2, 1));
    assert!(result.fallback_used);
    assert_eq!(result.decomposition_error.unwrap().solver_error_class(), SolverErrorClass::Unsupported);
    assert_eq!(result.result.objective, Some(8.0));
    for invalid_input in [false, true] {
        let mut factories = 0;
        let mut solves = 0;
        let result = DiscreteLoadingBendersApplicationService::solve(|| { factories += 1; Ok(registered(false)) },
            DiscreteLoadingBendersOptions { fallback_to_milp: true, ..Default::default() }, &SolveHandle::new(), |model, _, _| {
                solves += 1;
                if solves == 1 { return Err(CoreError::Solver(if invalid_input { SolverError::InvalidInput("input".into()) } else { SolverError::SolveFailed("backend".into()) })); }
                optimal_report(model, vec![0.0; model.num_variables()])
            });
        if invalid_input { assert!(result.is_err()); assert_eq!(factories, 1); }
        else { assert!(result.unwrap().fallback_used); assert_eq!(factories, 2); }
    }
}

#[test]
fn fallback_without_an_incumbent_or_with_a_worse_one_keeps_the_verified_source_best() {
    use ospf_rust_core::token::TokenVariableData;
    use ospf_rust_core::variable::{VariableType, new_standalone_id};
    use ospf_rust_framework::solver::{FixedRecourseConfig, LinearBendersPartitionOptions, LinearBendersStage};
    for outcome in 0..3 {
        let no_incumbent = outcome == 0;
        let variable = new_standalone_id();
        let mut calls = 0;
        let result = DiscreteLoadingBendersApplicationService::solve(|| {
            let mut registration = registered(false);
            let resource = registration.model.register_variable_data(TokenVariableData { id: variable, index: 0, name: "resource".into(), display_name: None,
                var_type: VariableType::Continuous, lower_bound: Some(0.0), upper_bound: Some(2.0) })?;
            registration.context.link_additional_weight(&mut registration.model, "a", Linear::new(vec![LinearMonomial::new(1.0, resource)], 0.0))?;
            registration.model.add_ge_constraint(&[(resource, 1.0)], 1.0, "resource_demand")?;
            registration.model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(1.0, resource)], 0.0), "resource_cost"));
            Ok(registration)
        }, DiscreteLoadingBendersOptions { partition: LinearBendersPartitionOptions { stages: std::collections::BTreeMap::from([(variable, LinearBendersStage::ContinuousRecourse)]) },
            config: FixedRecourseConfig { recover_missing_certificates: false, ..Default::default() }, fallback_to_milp: true }, &SolveHandle::new(), |model, _, _| {
            calls += 1;
            match calls {
                1 => optimal_report(model, vec![1.0, 0.0, 0.0]),
                2 => optimal_report(model, vec![1.0]),
                _ if outcome == 2 => Err(ospf_rust_core::error::CoreError::NotImplemented("fallback backend failed after source incumbent".into())),
                _ if no_incumbent => SolveReport::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit)
                    .model_mapping(Some(linear_model_mapping(model)?))
                    .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build(),
                _ => {
                    let mut report = optimal_report(model, vec![1.0, 0.0, 2.0])?;
                    report.proof = None; report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
                    report.termination_reason = TerminationReason::TimeLimit;
                    Ok(report)
                }
            }
        }).unwrap();
        assert_eq!(calls, 3);
        assert!(result.fallback_used);
        assert_eq!(result.result.objective, Some(8.0));
        assert!(result.result.solution.is_some());
        assert!(!result.result.report.is_optimal());
        if no_incumbent { assert_eq!(result.result.report.termination_reason, TerminationReason::TimeLimit); }
        else if outcome == 2 {
            assert!(result.decomposition_error.is_some());
            assert!(result.fallback_report.is_none());
            assert_eq!(result.result.report.termination_reason, TerminationReason::BackendFailure);
        } else { assert_eq!(result.fallback_report.unwrap().solution.unwrap().objective_value, Some(2.0)); }
    }
}

#[cfg(any(feature = "scip", feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
mod native {
    use ospf_rust_core::solver::{LinearSolver, SolveOptions};
    use ospf_rust_core::variable::{UInteger, VariableRange};
    use crate::PartitionDecision;
    use super::*;

    fn verify(solver: &dyn LinearSolver) {
        let solve = |model: &LinearTriadModel, remaining: Duration, handle: &SolveHandle| {
            solver.solve_linear_report_with_options(model, &SolveOptions { time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default() })
        };
        let result = DiscreteLoadingApplicationService::solve(|| Ok(registered(true)), Default::default(), &SolveHandle::new(), solve).unwrap();
        assert_eq!(result.objective, Some(8.0));
        assert_eq!(result.solution.unwrap().assignments["cargo"], "a");
        let result = PartitionColumnGenerationApplicationService::solve(&registered(true), partitions(), Default::default(), &SolveHandle::new(), solve).unwrap();
        assert_eq!(result.objective, Some(8.0));
        assert_eq!(result.solution.unwrap().assignments["cargo"], "a");
        assert!(result.lp_converged);
        assert!(result.trace.iter().any(|iteration| iteration.phase_one && iteration.added_columns > 0));

        let mut registration = registered(false);
        let mut options = partitions();
        let mut terms = Vec::new();
        for position in ["a", "b"] {
            let index = registration.model.register_auto_variable_with_range::<UInteger>(&format!("resource_{position}"), VariableRange::bounded(0.0, 3.0)).unwrap();
            let variable = registration.model.tokens()[index].id();
            registration.context.link_additional_weight(&mut registration.model, position, Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0)).unwrap();
            options.extra_decisions.push(PartitionDecision { id: format!("resource_{position}"), partition_id: position.into(), variable });
            options.refinable_resources.insert(variable);
            terms.push((index, 1.0));
        }
        registration.model.add_ge_constraint(&terms, 2.0, "global_resource").unwrap();
        registration.model.add_sub_objective(SubObjective::minimize(Linear::new(terms.into_iter().map(|(index, coefficient)| LinearMonomial::new(coefficient, index)).collect(), 0.0), "resource_cost"));
        let result = PartitionColumnGenerationApplicationService::solve(&registration, options.clone(), Default::default(), &SolveHandle::new(), solve).unwrap();
        assert_eq!(result.objective, Some(9.0));
        assert!(result.completion_column_count > 0 && result.completion_column_count <= 2);
        assert!(result.solution.unwrap().occupied_positions.len() <= 2);
        let mut completed = false;
        let retained = PartitionColumnGenerationApplicationService::solve(&registration, options.clone(), Default::default(), &SolveHandle::new(), |model, remaining, handle| {
            if completed { return Err(ospf_rust_core::error::CoreError::NotImplemented("final failed after completed source incumbent".into())); }
            let report = solve(model, remaining, handle)?;
            completed = model.basic.variables.iter().any(|token| token.name().starts_with("column_"))
                && model.basic.variables.iter().any(|token| token.name().starts_with("resource_"));
            Ok(report)
        }).unwrap();
        assert!(completed);
        assert!(retained.acceleration_error.is_some());
        assert!(retained.solution.is_some());
        assert_eq!(retained.objective, Some(9.0));
        assert_eq!(retained.report.termination_reason, TerminationReason::BackendFailure);
        let mut refinement_seen = false;
        let result = PartitionColumnGenerationApplicationService::solve(&registration, options,
            PartitionColumnGenerationConfig { time_limit: Duration::from_secs(2), ..Default::default() }, &SolveHandle::new(), |model, remaining, handle| {
                assert!(!refinement_seen, "deadline stops before another final solve");
                let report = solve(model, remaining, handle)?;
                if model.basic.variables.iter().any(|token| token.name().starts_with("column_"))
                    && model.basic.variables.iter().any(|token| token.name().starts_with("resource_"))
                {
                    refinement_seen = true;
                    std::thread::sleep(remaining + Duration::from_millis(20));
                }
                Ok(report)
            }).unwrap();
        assert!(refinement_seen);
        assert_eq!(result.report.termination_reason, TerminationReason::TimeLimit);
        assert_eq!(result.objective, Some(9.0));
        assert!(result.solution.is_some());
        assert!(result.refinement_report.is_some());
        assert!(result.final_report.is_none());
        assert!(!result.report.is_optimal());
    }

    #[cfg(feature = "scip")]
    #[test]
    fn scip_ordinary_partition_phase_one_pricing_completion_and_final() {
        use ospf_rust_core::solver::solvers::scip::{SCIPConfig, SCIPSolver};
        verify(&SCIPSolver::with_config(SCIPConfig::new().with_lp_subproblem_defaults().with_output(false)));
    }

    #[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
    #[test]
    fn gurobi_ordinary_partition_phase_one_pricing_completion_and_final() {
        use ospf_rust_core::solver::solvers::gurobi::{GurobiConfig, GurobiSolver};
        verify(&GurobiSolver::with_config(GurobiConfig::new().with_output(false)));
    }
}
