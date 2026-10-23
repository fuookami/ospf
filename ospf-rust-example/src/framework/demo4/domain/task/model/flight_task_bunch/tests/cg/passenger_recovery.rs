//! 有界取消、舱位变更和独立生产校验回归。 / Bounded cancellation, cabin changes and independent production validation regressions.

use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::framework::demo4::domain::bunch_compilation::context::RecoveryBunchResolver;
use crate::framework::demo4::domain::passenger::recovery::{RecoveryPassengerConfiguration, RecoveryPassengerLegPolicy};
use super::*;

fn configuration(cancel: Option<f64>, change: Option<f64>) -> RecoveryPassengerConfiguration {
    RecoveryPassengerConfiguration { passengers: vec![passenger(1)], policies: ["F1", "F2"].into_iter().map(|flight| RecoveryPassengerLegPolicy {
        passenger: "P1".into(), flight: flight.into(), cancellation_cost: cancel,
        class_change_costs: change.map(|cost| BTreeMap::from([("J".into(), cost)])).unwrap_or_default(),
        flight_change_costs: BTreeMap::new(),
    }).collect() }
}

fn source(y: u64, j: u64, early: bool) -> RecoveryProblem<CostCalculator> {
    let base = fleet::fresh();
    let mut configuration = base.configuration().clone();
    let mut domains = base.domains().to_vec();
    let mut aircraft = aircraft();
    aircraft.capacity = AircraftCapacity::Passenger { capacities: vec![(PassengerClass("Y".into()), y), (PassengerClass("J".into()), j)] };
    configuration.aircraft[0] = aircraft.clone();
    domains[0].aircraft = aircraft.clone();
    for (index, candidate) in domains[0].candidates.iter_mut().enumerate() {
        let activity = &candidate.recovery;
        let time = if early && index == 1 { (at(9, 30), at(11, 30)) } else { activity.time() };
        let mut original = activity.original().clone();
        original.plan.dep = activity.route().dep.clone();
        original.plan.arr = activity.route().arr.clone();
        configuration.tasks[index] = original.clone();
        candidate.recovery = FlightTaskRecovery::new(original, aircraft.clone(), activity.route().clone(), time).unwrap();
    }
    RecoveryProblem::new(configuration, domains, vec![], CostCalculator).unwrap()
}

fn context(problem: &Arc<RecoveryProblem<CostCalculator>>) -> (BunchCompilationContext, MetaModel<f64>) {
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("passenger-recovery");
    context.register(&mut model).unwrap();
    (context, model)
}

#[test]
fn passenger_cancel_and_class_change_optima_match_independent_disposition_oracle() {
    for (y, j, cancel, change, expected, canceled, changed) in [
        (0, 0, Some(20.0), None, 340.0, 1, 0),
        (0, 1, Some(50.0), Some(3.0), 306.0, 0, 1),
        (0, 1, None, Some(3.0), 306.0, 0, 1),
        (1, 1, Some(50.0), Some(3.0), 300.0, 0, 0),
    ] {
        let problem = Arc::new(source(y, j, false).with_passenger_recovery(configuration(cancel, change)).unwrap());
        let (mut context, mut model) = context(&problem);
        context.add_candidates(0, vec![fleet::complete(&problem)], &mut model).unwrap();
        let (objective, values) = master_optimum(&model).unwrap();
        assert_eq!(objective, expected);
        let result = context.extract_solution(&model, &values).unwrap();
        assert_eq!(result.passengers.len(), 2);
        assert!(result.canceled.is_empty());
        for leg in &result.passengers {
            assert_eq!(leg.canceled, canceled);
            assert_eq!(leg.class_changes.get("J").copied().unwrap_or(0), changed);
            assert_eq!(leg.transported.values().sum::<u64>(), 1 - canceled);
        }
        let independent = (0..=1).flat_map(|cancel| (0..=1).map(move |change| (cancel, change)))
            .filter(|(c, s)| c + s <= 1 && 1 - c - s <= y && *s <= j && (cancel.is_some() || *c == 0) && (change.is_some() || *s == 0))
            .map(|(c, s)| 300.0 + 2.0 * (c as f64 * cancel.unwrap_or(0.0) + s as f64 * change.unwrap_or(0.0)))
            .min_by(f64::total_cmp).unwrap();
        assert_eq!(objective, independent);
    }
    let problem = Arc::new(source(0, 0, false).with_passenger_recovery(configuration(None, None)).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![fleet::complete(&problem)], &mut model).unwrap();
    assert!(master_optimum(&model).is_none());
}

#[test]
fn passenger_cancel_propagates_and_recovered_transfer_can_cancel_only_the_later_leg() {
    let problem = Arc::new(source(1, 0, true).with_passenger_recovery(configuration(Some(20.0), None)).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![fleet::complete(&problem)], &mut model).unwrap();
    let (objective, values) = master_optimum(&model).unwrap();
    assert_eq!(objective, 320.0);
    let result = context.extract_solution(&model, &values).unwrap();
    assert_eq!(result.passengers[0].canceled, 0);
    assert_eq!(result.passengers[1].canceled, 1);
    let mut forged = result.clone();
    forged.passengers[1].canceled = 0;
    assert!(problem.validate_schedule(&forged).is_err());
    model.retain_constraints(|row| !row.name.starts_with("recovery_passenger_"));
    let invalid_connection = [0.0, 0.0, 0.0, 0.0, 1.0];
    assert!(context.extract_solution(&model, &invalid_connection).is_err());
    let invalid_propagation = [0.0, 0.0, 1.0, 0.0, 1.0];
    assert!(context.extract_solution(&model, &invalid_propagation).is_err());
}

#[test]
fn passenger_capacity_does_not_subtract_cancellation_from_unrelated_cabins() {
    let mut configuration = configuration(Some(10.0), None);
    configuration.passengers = vec![Passenger { id: "P1".into(), amount: 1, flights: vec![("F1".into(), PassengerClass("Y".into()))] },
        Passenger { id: "P2".into(), amount: 1, flights: vec![("F1".into(), PassengerClass("J".into()))] }];
    configuration.policies = ["P1", "P2"].into_iter().map(|passenger| RecoveryPassengerLegPolicy {
        passenger: passenger.into(), flight: "F1".into(), cancellation_cost: Some(10.0), class_change_costs: BTreeMap::new(),
        flight_change_costs: BTreeMap::new(),
    }).collect();
    let problem = Arc::new(source(0, 0, false).with_passenger_recovery(configuration).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![fleet::complete(&problem)], &mut model).unwrap();
    let (objective, values) = master_optimum(&model).unwrap();
    assert_eq!(objective, 320.0);
    assert_eq!(context.extract_solution(&model, &values).unwrap().passengers.iter().map(|leg| leg.canceled).sum::<u64>(), 2);
}

#[test]
fn passenger_recovery_rows_share_signed_pricing_coefficients_including_explicit_zero_rows() {
    let problem = Arc::new(source(1, 1, true).with_passenger_recovery(configuration(Some(20.0), Some(3.0))).unwrap());
    let candidate = fleet::complete(&problem);
    let rows = problem.column_coefficients(&candidate).unwrap();
    assert_eq!(rows["recovery_passenger_disposition_0"], 0.0);
    assert_eq!(rows["recovery_passenger_propagation_1"], 0.0);
    assert_eq!(rows["recovery_passenger_recovery_transfer_1_0_1"], -2.0);
    let mut request = request();
    request.row_duals = problem.row_names().into_iter().map(|name| (name, 0.0)).collect();
    request.row_duals.insert("recovery_passenger_recovery_transfer_1_0_1".into(), -500.0);
    let result = RecoveryPricingPolicy::new(problem.clone(), None).price(&request, &PathSearchBudget::default()).unwrap();
    let id = problem.entry(&candidate).unwrap().id;
    assert_eq!(result.reduced_costs[id.as_str()], -700.0);
    assert_eq!(result.batch.no_improving_columns, None);
    request.row_duals.remove("recovery_passenger_disposition_0");
    assert!(RecoveryPricingPolicy::new(problem, None).price(&request, &PathSearchBudget::default()).is_err());
}

#[test]
fn passenger_recovery_lifecycle_preserves_downstream_objective_and_rolls_back_row_collisions() {
    let problem = Arc::new(source(0, 1, false).with_passenger_recovery(configuration(Some(50.0), Some(3.0))).unwrap());
    let (mut context, mut model) = context(&problem);
    let extra = model.register_auto_variable_with_range::<Binary>("downstream_passenger", VariableRange::fixed(1.0)).unwrap();
    model.add_eq_constraint(&[(extra, 1.0)], 1.0, "downstream_passenger_row").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(7.0, extra)], 0.0), "downstream_passenger_cost"));
    let candidate = fleet::complete(&problem);
    context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    assert_eq!(master_optimum(&model).unwrap().0, 313.0);
    let first = context.master().compilation.bunch_x_map[&0];
    let tokens = model.as_basic().num_tokens();
    context.register(&mut model).unwrap();
    assert_eq!(tokens, model.as_basic().num_tokens());
    context.remove_columns(&[0], &mut model).unwrap();
    assert_eq!(master_optimum(&model).unwrap().0, 2107.0);
    context.add_candidates(1, vec![candidate], &mut model).unwrap();
    assert_ne!(first, context.master().compilation.bunch_x_map[&1]);
    assert_eq!(master_optimum(&model).unwrap().0, 313.0);
    model.retain_constraints(|row| row.name != "recovery_passenger_disposition_0");
    model.add_eq_constraint(&[], 0.0, "recovery_passenger_disposition_0").unwrap();
    let before = model.as_basic().num_tokens();
    assert!(context.add_candidates(2, vec![single(&problem, 0, 0)], &mut model).is_err());
    assert_eq!(context.column_count(), 1);
    assert_eq!(before, model.as_basic().num_tokens());
}

#[test]
fn passenger_production_extraction_rejects_illegal_decisions_even_without_managed_rows() {
    let problem = Arc::new(source(0, 1, false).with_passenger_recovery(configuration(Some(50.0), Some(3.0))).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![fleet::complete(&problem)], &mut model).unwrap();
    let values = [0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0];
    assert_eq!(context.extract_solution(&model, &values).unwrap().passengers[0].class_change_penalty, 3.0);
    model.retain_constraints(|row| !row.name.starts_with("recovery_passenger_"));
    let mut over_disposed = values;
    over_disposed[2] = 1.0;
    assert!(context.extract_solution(&model, &over_disposed).is_err());
    let mut over_capacity = values;
    over_capacity[3] = 0.0;
    assert!(context.extract_solution(&model, &over_capacity).is_err());
    for illegal in [f64::NAN, f64::INFINITY, -1.0, 0.5, 2.0] {
        let mut invalid = values;
        invalid[3] = illegal;
        assert!(context.extract_solution(&model, &invalid).is_err());
    }
    assert!(context.extract_solution(&model, &values[..3]).is_err());
}

#[test]
fn passenger_decisions_are_bounded_integers_and_preserve_partial_group_transport() {
    let mut input = configuration(Some(50.0), Some(3.0));
    input.passengers[0].amount = 3;
    let problem = Arc::new(source(1, 1, false).with_passenger_recovery(input).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![fleet::complete(&problem)], &mut model).unwrap();
    let result = context.extract_solution(&model, &[0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0]).unwrap();
    for leg in result.passengers {
        assert_eq!(leg.canceled, 1);
        assert_eq!(leg.transported["Y"], 1);
        assert_eq!(leg.transported["J"], 1);
        assert_eq!(leg.cancellation_penalty, 50.0);
        assert_eq!(leg.class_change_penalty, 3.0);
    }
    let result = context.extract_solution(&model, &[0.0, 0.0, 2.0, 1.0, 2.0, 1.0, 1.0]).unwrap();
    assert_eq!(result.passengers[0].canceled, 2);
    model.retain_constraints(|row| !row.name.starts_with("recovery_passenger_"));
    assert!(context.extract_solution(&model, &[0.0, 0.0, 2.0, 0.0, 1.0, 1.0, 1.0]).is_err());
}

#[test]
fn passenger_recovery_rejects_bad_policy_inputs_and_empty_configuration_adds_no_variables() {
    for bad in 0..8 {
        let mut input = configuration(Some(10.0), Some(3.0));
        match bad {
            0 => { input.policies.pop(); }
            1 => input.policies.push(input.policies[0].clone()),
            2 => input.policies[0].passenger = "unknown".into(),
            3 => input.policies[0].cancellation_cost = Some(f64::NAN),
            4 => { input.policies[0].class_change_costs.insert("J".into(), f64::INFINITY); }
            5 => { input.policies[0].class_change_costs.insert("Y".into(), 0.0); }
            6 => { input.policies[0].class_change_costs.insert(String::new(), 0.0); }
            _ => input.policies[0].cancellation_cost = Some(-1.0),
        }
        assert!(source(1, 1, false).with_passenger_recovery(input).is_err());
    }
    let problem = Arc::new(source(1, 1, false).with_passenger_recovery(RecoveryPassengerConfiguration { passengers: vec![], policies: vec![] }).unwrap());
    let (context, model) = context(&problem);
    assert_eq!(model.as_basic().num_tokens(), 2);
    assert!(context.extract_solution(&model, &[1.0, 1.0]).unwrap().passengers.is_empty());
}

#[test]
fn passenger_recovery_application_uses_the_real_gantt_loop_without_a_private_passenger_solver() {
    let problem = Arc::new(source(0, 1, false).with_passenger_recovery(configuration(Some(50.0), Some(3.0))).unwrap());
    let initial = vec![fleet::complete(&problem)];
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(problem, RecoveryApplicationConfiguration {
        initial, column_generation: ColumnGenerationPolicy { max_iterations: 3, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(306.0));
    let result = report.solution.unwrap().value.unwrap();
    assert_eq!(result.schedule.passengers[0].class_changes["J"], 1);
    assert!(!result.pricing.is_empty());
    assert!(report.proof.is_none());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_passenger_recovery_prices_dynamic_cabin_capacity_and_extracts_actual_changes() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let problem = Arc::new(source(0, 1, false).with_passenger_recovery(configuration(Some(50.0), Some(3.0))).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(problem, RecoveryApplicationConfiguration {
        column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(306.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(solution.schedule.canceled.is_empty());
    for leg in &solution.schedule.passengers {
        assert_eq!(leg.canceled, 0);
        assert_eq!(leg.transported["Y"], 0);
        assert_eq!(leg.transported["J"], 1);
        assert_eq!(leg.class_change_penalty, 3.0);
    }
    assert!(!solution.pricing.is_empty());
}
