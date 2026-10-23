//! 实际机场事件、完整停场、容量生命周期和原生闭环回归。 / Actual airport events, complete stays, capacity lifecycle and native-loop regressions.

use std::collections::BTreeSet;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::framework::demo4::domain::bunch_compilation::context::RecoveryBunchResolver;
use crate::framework::demo4::domain::rule::model::{
    FlowControl, FlowControlCapacity, FlowControlCondition, FlowControlEvaluation, FlowControlPredicate, FlowControlScene, FlowControlSlot,
};
use crate::framework::demo4::domain::task::model::FlightType;
use super::*;

fn control(scene: FlowControlScene, cost: Option<f64>) -> FlowControl {
    FlowControl { id: "internal-B".into(), airport: "B".into(), time_range: (at(8, 30), at(10, 30)), scene,
        condition: None, capacity: FlowControlCapacity { max_capacity: 0, current_usage: 0, interval: Duration::hours(2) }, over_cost: cost }
}

fn source(controls: Vec<FlowControl>) -> Arc<RecoveryProblem<CostCalculator>> {
    Arc::new(fleet::fresh().with_flow_controls(controls).unwrap())
}

fn context(source: &Arc<RecoveryProblem<CostCalculator>>) -> (BunchCompilationContext, MetaModel<f64>) {
    let mut context = BunchCompilationContext::with_resolver(source.configuration().clone(), source.clone()).unwrap();
    let mut model = MetaModel::new("recovery-flow-controls"); context.register(&mut model).unwrap();
    (context, model)
}

// 穷举这些有限整数夹具，包含实际非二元超额；不冒充连续 LP 求解器。 / Enumerate these finite integer fixtures, including actual nonbinary excess, without impersonating a continuous LP solver.
fn optimum(model: &MetaModel<f64>) -> Option<(f64, Vec<f64>)> {
    let matrix = model.try_to_linear_triad_model().unwrap();
    assert!(matrix.num_variables() < 12 && matrix.var_types.iter().all(|kind| kind.is_integer()));
    let mut states = vec![vec![]];
    for index in 0..matrix.num_variables() {
        assert!(matrix.lb[index] >= 0.0 && matrix.ub[index] <= 6.0);
        states = states.into_iter().flat_map(|state| (matrix.lb[index].ceil() as u64..=matrix.ub[index].floor() as u64).map(move |value| {
            let mut next = state.clone(); next.push(value as f64); next
        })).collect();
    }
    states.into_iter().filter(|values| matrix.A.rows.iter().zip(&matrix.b).all(|(row, bound)| row.entries.iter()
        .map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() <= *bound + 1e-9))
        .map(|values| (values.iter().zip(&matrix.c).map(|(value, coefficient)| value * coefficient).sum::<f64>(), values))
        .min_by(|left, right| left.0.total_cmp(&right.0))
}

#[test]
fn flow_scenes_count_internal_actual_events_and_do_not_infer_from_planned_or_bunch_endpoints() {
    let bunch = bunch();
    assert_eq!(bunch.dep().unwrap().icao.0, "A"); assert_eq!(bunch.arr().unwrap().icao.0, "C");
    for (scene, expected) in [(FlowControlScene::Arrival, 1), (FlowControlScene::Departure, 1),
        (FlowControlScene::DepartureArrival, 2), (FlowControlScene::Stay, 1)] {
        let control = control(scene, None);
        assert_eq!(control.usage_in_slot(&bunch, None, &control.slots().unwrap()[0]), expected);
    }
    let early = FlowControl { time_range: (at(6, 30), at(7, 30)), ..control(FlowControlScene::DepartureArrival, None) };
    assert_eq!(early.usage_in_slot(&bunch, None, &early.slots().unwrap()[0]), 0);
    let circular = FlightTaskBunch::new("circular".into(), aircraft(), vec![activity("LOOP", "B", "B", 8, 9)]).unwrap();
    let both = FlowControl { time_range: (at(8, 0), at(10, 0)), ..control(FlowControlScene::DepartureArrival, None) };
    assert_eq!(both.usage_in_slot(&circular, None, &both.slots().unwrap()[0]), 1);
}

#[test]
fn capacity_windows_own_shared_endpoints_once_and_truncate_the_final_window() {
    let control = FlowControl { time_range: (at(8, 0), at(12, 30)), ..control(FlowControlScene::DepartureArrival, None) };
    let slots = control.slots().unwrap();
    assert_eq!(slots.len(), 3);
    assert_eq!(slots[0], FlowControlSlot { start: at(8, 0), end: at(10, 0), include_end: false });
    assert_eq!(slots[2], FlowControlSlot { start: at(12, 0), end: at(12, 30), include_end: true });
    assert!(!slots[0].contains_event(at(10, 0))); assert!(slots[1].contains_event(at(10, 0)));
    assert!(slots[2].contains_event(at(12, 30))); assert!(!slots[2].contains_event(at(12, 31)));
    let bunch = bunch();
    assert_eq!(slots.iter().map(|slot| control.usage_in_slot(&bunch, None, slot)).collect::<Vec<_>>(), [1, 1, 0]);
    let end = FlowControl { airport: "C".into(), ..control };
    assert_eq!(slots.iter().map(|slot| end.usage_in_slot(&bunch, None, slot)).collect::<Vec<_>>(), [0, 0, 1]);
}

#[test]
fn stays_require_complete_intervals_use_ground_end_times_and_actual_executed_predecessors() {
    let original = bunch();
    let partial = FlowControl { time_range: (at(9, 15), at(9, 45)), ..control(FlowControlScene::Stay, None) };
    assert_eq!(partial.usage_in_slot(&original, None, &partial.slots().unwrap()[0]), 0);
    let mut task = original.activities()[1].original().clone(); task.task_type.category = FlightTaskCategory::Maintenance;
    let ground = FlightTaskRecovery::new(task, aircraft(), Route { dep: airport("B"), arr: airport("B") }, (at(10, 0), at(12, 0))).unwrap();
    let bunch = FlightTaskBunch::new("ground".into(), aircraft(), vec![original.activities()[0].clone(), ground]).unwrap();
    for (end, expected) in [(11, 0), (12, 1)] {
        let control = FlowControl { time_range: (at(9, 0), at(end, 0)), capacity: FlowControlCapacity { interval: Duration::hours(4), ..partial.capacity.clone() }, ..partial.clone() };
        assert_eq!(control.usage_in_slot(&bunch, None, &control.slots().unwrap()[0]), expected);
    }
    let previous = activity("executed", "X", "A", 5, 6);
    let at_origin = FlowControl { airport: "A".into(), time_range: (at(7, 0), at(8, 0)), ..control(FlowControlScene::Stay, None) };
    let slot = at_origin.slots().unwrap()[0];
    assert_eq!(at_origin.usage_in_slot(&original, None, &slot), 1);
    assert_eq!(at_origin.usage_in_slot(&original, Some(&previous), &slot), 0);
    let empty = FlightTaskBunch::new("idle".into(), aircraft(), vec![]).unwrap();
    assert_eq!(at_origin.usage_in_slot(&empty, Some(&previous), &slot), 0);
    let tail = FlowControlSlot { start: at(5, 0), end: at(7, 0), include_end: true };
    assert!(FlowControlScene::Stay.evaluate(Some(&previous), None, &FlowControlEvaluation { airport: "A", slot: &tail, condition: None }));
    assert!(!FlowControlScene::Stay.evaluate(None, None, &FlowControlEvaluation { airport: "A", slot: &tail, condition: None }));
}

#[derive(Debug)]
struct SecondActivity;

impl FlowControlPredicate for SecondActivity {
    fn matches(&self, activity: &FlightTaskRecovery) -> bool { activity.original().plan.id == "F2" }
}

#[test]
fn flow_filters_use_actual_flight_category_minor_type_and_custom_predicates_without_excluding_all_flights() {
    let original = bunch();
    let activity = &original.activities()[0];
    assert!(FlowControlCondition::default().matches(activity));
    let mut route = activity.route().clone(); route.arr.airport_type = AirportType::International;
    let actual = FlightTaskRecovery::new(activity.original().clone(), aircraft(), route, activity.time()).unwrap();
    let international = FlowControlCondition { flight_types: BTreeSet::from([FlightType::International]),
        aircraft_minor_types: BTreeSet::from(["A320-200".into()]), predicate: None };
    assert!(!international.matches(activity)); assert!(international.matches(&actual));
    let wrong_minor = FlowControlCondition { aircraft_minor_types: BTreeSet::from(["other".into()]), ..international };
    assert!(!wrong_minor.matches(&actual));
    let filtered = FlowControl { condition: Some(FlowControlCondition { predicate: Some(Arc::new(SecondActivity)), ..Default::default() }),
        ..control(FlowControlScene::DepartureArrival, None) };
    assert_eq!(filtered.usage_in_slot(&original, None, &filtered.slots().unwrap()[0]), 1);
    let mut task = activity.original().clone(); task.task_type.category = FlightTaskCategory::Maintenance;
    let ground = FlightTaskRecovery::new(task, aircraft(), activity.route().clone(), activity.time()).unwrap();
    assert!(!FlowControlCondition::default().matches(&ground));
}

#[test]
fn hard_capacity_soft_overage_and_external_usage_change_actual_master_choices_and_allow_nonbinary_counts() {
    for (cost, initial, expected, selected, overage) in [(None, 0, 2000.0, 0, 0), (Some(400.0), 0, 1100.0, 1, 2),
        (Some(1000.0), 0, 2000.0, 0, 0), (Some(20.0), 3, 400.0, 1, 5)] {
        let mut control = control(FlowControlScene::DepartureArrival, cost); control.capacity.current_usage = initial;
        let source = source(vec![control]); let (mut context, mut model) = context(&source);
        context.add_candidates(0, vec![single(&source, 0, 0), fleet::complete(&source)], &mut model).unwrap();
        let (actual, values) = optimum(&model).unwrap(); assert_eq!(actual, expected);
        let result = context.extract_solution(&model, &values).unwrap(); assert_eq!(result.selected.len(), selected);
        assert_eq!(result.flow_controls[0].initial_usage, initial);
        assert_eq!(result.flow_controls[0].overage, overage);
        assert_eq!(result.flow_controls[0].modeled_overage, cost.map(|_| overage));
        source.validate_schedule(&result).unwrap();
    }
    let mut impossible = control(FlowControlScene::Arrival, None); impossible.capacity.current_usage = 1;
    let source = source(vec![impossible]); let (_, model) = context(&source);
    assert!(optimum(&model).is_none());
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(source, Default::default()).unwrap();
    assert_eq!(report.problem_status, ProblemStatus::Unknown); assert!(!report.has_incumbent());
}

#[test]
fn executed_predecessor_and_every_capacity_row_share_the_same_coefficients_and_signed_pricing_duals() {
    let base = problem(CostCalculator); let mut domains = base.domains().to_vec();
    domains[0].previous = Some(activity("executed", "X", "A", 5, 6));
    let origin = FlowControl { id: "origin-A".into(), airport: "A".into(), time_range: (at(7, 0), at(8, 0)), ..control(FlowControlScene::Stay, None) };
    let split = FlowControl { time_range: (at(8, 0), at(12, 0)), ..control(FlowControlScene::DepartureArrival, Some(20.0)) };
    let source = Arc::new(RecoveryProblem::new(base.configuration().clone(), domains, vec![], CostCalculator).unwrap().with_flow_controls(vec![origin, split]).unwrap());
    let (mut context, mut model) = context(&source); let all = vec![single(&source, 0, 0), fleet::complete(&source)];
    context.add_candidates(0, all.clone(), &mut model).unwrap();
    for (index, candidate) in all.iter().enumerate() {
        let expected = source.column_coefficients(candidate).unwrap(); assert_eq!(expected["recovery_flow_control_0"], 0.0);
        let variable = context.master().compilation.bunch_x_map[&index];
        for row in model.constraints() {
            let actual = row.inequality.polynomial.monomials().iter().filter(|term| term.var_index() == variable).map(|term| *term.coefficient()).sum::<f64>();
            assert_eq!(actual, expected[&row.name], "{}", row.name);
        }
    }
    let mut request = request(); request.row_duals = source.row_names().into_iter().map(|name| (name, 0.0)).collect();
    request.row_duals.insert("recovery_flow_control_1".into(), -71.0); request.row_duals.insert("recovery_flow_control_2".into(), 13.0);
    let policy = RecoveryPricingPolicy::new(source.clone(), None); let priced = policy.price(&request, &PathSearchBudget::default()).unwrap();
    for candidate in all {
        let expected = source.column_coefficients(&candidate).unwrap().iter().fold(candidate.cost, |value, (name, coefficient)| value - request.row_duals[name] * coefficient);
        assert_eq!(priced.reduced_costs[source.entry(&candidate).unwrap().id.as_str()], expected);
    }
    request.row_duals.remove("recovery_flow_control_0"); assert!(policy.price(&request, &PathSearchBudget::default()).is_err());
    assert!(priced.batch.no_improving_columns.is_none());
}

#[test]
fn flow_lifecycle_preserves_extensions_readds_new_variables_separates_models_and_rolls_back_collisions() {
    let source = source(vec![control(FlowControlScene::DepartureArrival, Some(400.0))]);
    let (mut context, mut model) = context(&source);
    let extension = model.register_auto_variable_with_range::<Binary>("flow_extension", VariableRange::fixed(1.0)).unwrap();
    model.add_eq_constraint(&[(extension, 1.0)], 1.0, "flow_extension_rule").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(7.0, extension)], 0.0), "flow_extension_cost"));
    let candidate = fleet::complete(&source); context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    let first = context.master().compilation.bunch_x_map[&0]; assert_eq!(optimum(&model).unwrap().0, 1107.0);
    let tokens = model.as_basic().num_tokens(); context.register(&mut model).unwrap(); assert_eq!(tokens, model.as_basic().num_tokens());
    context.remove_columns(&[0], &mut model).unwrap(); assert_eq!(optimum(&model).unwrap().0, 2007.0);
    context.add_candidates(1, vec![candidate.clone()], &mut model).unwrap(); assert_ne!(first, context.master().compilation.bunch_x_map[&1]);
    assert_eq!(optimum(&model).unwrap().0, 1107.0); assert!(model.constraints().iter().any(|row| row.name == "flow_extension_rule"));
    let (mut other, mut other_model) = self::context(&source); assert!(context.add_candidates(2, vec![candidate.clone()], &mut other_model).is_err());
    other_model.retain_constraints(|row| row.name != "recovery_flow_control_0"); other_model.add_eq_constraint(&[], 0.0, "recovery_flow_control_0").unwrap();
    let before = other_model.try_to_linear_triad_model().unwrap(); assert!(other.add_candidates(0, vec![candidate], &mut other_model).is_err());
    let after = other_model.try_to_linear_triad_model().unwrap();
    assert_eq!(before.c, after.c); assert_eq!(before.num_variables(), after.num_variables()); assert_eq!(before.num_constraints(), after.num_constraints());
    assert_eq!(other.column_count(), 0);
}

#[test]
fn production_extraction_rejects_illegal_overages_and_hard_events_even_after_rows_are_removed() {
    let source = source(vec![control(FlowControlScene::DepartureArrival, Some(400.0))]);
    let (mut context, mut model) = context(&source); context.add_candidates(0, vec![fleet::complete(&source)], &mut model).unwrap();
    let (_, values) = optimum(&model).unwrap(); assert_eq!(values[2], 2.0);
    model.retain_constraints(|row| row.name != "recovery_flow_control_0");
    for illegal in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0, 1.0, 1.5, 3.0] {
        let mut forged = values.clone(); forged[2] = illegal; assert!(context.extract_solution(&model, &forged).is_err());
    }
    let mut result = context.extract_solution(&model, &values).unwrap(); result.flow_controls[0].penalty = 0.0;
    assert!(source.validate_schedule(&result).is_err());
    let source = self::source(vec![control(FlowControlScene::DepartureArrival, None)]);
    let (mut context, mut model) = self::context(&source); context.add_candidates(0, vec![fleet::complete(&source)], &mut model).unwrap();
    model.retain_constraints(|row| row.name != "recovery_flow_control_0"); let (_, values) = optimum(&model).unwrap();
    assert!(context.extract_solution(&model, &values).is_err());
    let source = self::source(vec![control(FlowControlScene::DepartureArrival, Some(400.0))]);
    let (mut context, mut model) = self::context(&source); context.add_candidates(0, vec![single(&source, 0, 0)], &mut model).unwrap();
    let (_, mut values) = optimum(&model).unwrap(); assert_eq!(values[2], 1.0); values[2] = 2.0;
    let result = context.extract_solution(&model, &values).unwrap();
    assert_eq!(result.flow_controls[0].overage, 1); assert_eq!(result.flow_controls[0].modeled_overage, Some(2)); assert_eq!(result.flow_controls[0].penalty, 800.0);
}

#[test]
fn flow_configuration_rejects_unknown_duplicate_nonpositive_nonfinite_and_inexact_inputs_before_model_mutation() {
    let base = control(FlowControlScene::Arrival, Some(1.0));
    let mut bad = vec![FlowControl { id: String::new(), ..base.clone() }, FlowControl { airport: "unknown".into(), ..base.clone() },
        FlowControl { time_range: (at(9, 0), at(8, 0)), ..base.clone() }, FlowControl { time_range: (at(9, 0), at(9, 0)), ..base.clone() },
        FlowControl { condition: Some(FlowControlCondition { aircraft_minor_types: BTreeSet::from(["unknown".into()]), ..Default::default() }), ..base.clone() }];
    for interval in [Duration::ZERO, Duration::seconds(-1)] { bad.push(FlowControl { capacity: FlowControlCapacity { interval, ..base.capacity.clone() }, ..base.clone() }); }
    for cost in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] { bad.push(FlowControl { over_cost: Some(cost), ..base.clone() }); }
    for capacity in [FlowControlCapacity { max_capacity: (1_u64 << 53) + 1, ..base.capacity.clone() },
        FlowControlCapacity { current_usage: (1_u64 << 53), ..base.capacity.clone() }] { bad.push(FlowControl { capacity, ..base.clone() }); }
    for control in bad { assert!(fleet::fresh().with_flow_controls(vec![control]).is_err()); }
    assert!(fleet::fresh().with_flow_controls(vec![base.clone(), base]).is_err());
    assert!(fleet::fresh().with_flow_controls(vec![]).is_ok());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_flow_controls_price_actual_internal_events_and_extract_nonbinary_overage() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let base = transfer_problem(1);
    let late = FlowControl { time_range: (at(12, 0), at(15, 0)), capacity: FlowControlCapacity { interval: Duration::hours(3), ..control(FlowControlScene::Departure, None).capacity },
        ..control(FlowControlScene::Departure, None) };
    let source = Arc::new(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(), vec![], CostCalculator).unwrap().with_flow_controls(vec![late]).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(source.clone(), RecoveryApplicationConfiguration {
        initial: vec![single(&source, 0, 0), single(&source, 1, 2)],
        column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(200.0)); assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap(); assert!(solution.candidate_ids.len() > 2);
    assert!(solution.pricing.iter().any(|round| !round.batch.columns.is_empty())); assert_eq!(solution.schedule.flow_controls[0].usage, 0);
    source.validate_schedule(&solution.schedule).unwrap();
    let source = self::source(vec![control(FlowControlScene::DepartureArrival, Some(400.0))]);
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(source.clone(), RecoveryApplicationConfiguration {
        initial: vec![fleet::complete(&source)], column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(1100.0)); let solution = report.solution.unwrap().value.unwrap();
    assert_eq!(solution.schedule.flow_controls[0].recovery_usage, 2); assert_eq!(solution.schedule.flow_controls[0].modeled_overage, Some(2));
    assert_eq!(solution.schedule.flow_controls[0].penalty, 800.0); source.validate_schedule(&solution.schedule).unwrap();
}
