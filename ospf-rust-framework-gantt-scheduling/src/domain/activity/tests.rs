use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use time::{Duration, OffsetDateTime};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Celsius, Gram, Kelvin, Kilogram, Meter};
use crate::domain::task::{F64SolverValueAdapter, SchedulingSolverValueAdapter};
use crate::infrastructure::{TimeRange, distant_future};
use crate::{GanttError, GanttResult};
use super::*;
use super::numeric::{activity_value_from_solver, normalize_activity_value};

mod scheduling;

fn unit<U: CTUnit>() -> ospf_rust_quantities::unit::Unit {
    let unit = U::INSTANT;
    unit.clone()
}

fn time(hour: i64) -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH + Duration::hours(hour)
}

fn quantity(value: f64) -> Quantity<f64, ospf_rust_quantities::unit::Unit> {
    Quantity::new(value, unit::<Kilogram>())
}

fn problem() -> ActivitySchedulingProblem<f64> {
    ActivitySchedulingProblem {
        horizon: TimeRange::new(time(0), time(10)),
        activities: vec![],
        resources: vec![],
        dependencies: vec![],
        baselines: vec![],
        executions: vec![],
        rows: vec![],
        cost_unit: unit::<Kilogram>(),
        materials: vec![],
        material_events: vec![],
    }
}

fn activity() -> Activity<f64> {
    Activity {
        id: "job".into(),
        modes: vec![ActivityMode {
            id: "mode".into(),
            duration: Duration::hours(1),
            starts: vec![time(2), time(1)],
            resources: vec![],
            cost: quantity(2.0),
            material_flows: vec![],
        }],
        cancellation_cost: None,
    }
}

fn prepare(problem: ActivitySchedulingProblem<f64>) -> GanttResult<ActivityPreparation<f64>> {
    ActivityPreparation::new(problem, &F64SolverValueAdapter, None)
}

fn key(hour: i64) -> ActivityChoiceKey {
    ActivityChoiceKey { task: "job".into(), mode: "mode".into(), start: time(hour) }
}

#[cfg(any(feature = "big-decimal", feature = "big-rational"))]
fn numeric_problem<V>(cost: V) -> ActivitySchedulingProblem<V> {
    ActivitySchedulingProblem {
        horizon: TimeRange::new(time(0), time(10)),
        activities: vec![Activity {
            id: "job".into(),
            modes: vec![ActivityMode {
                id: "mode".into(), duration: Duration::hours(1), starts: vec![time(1)],
                resources: vec![], cost: Quantity::new(cost, unit::<Kilogram>()), material_flows: vec![],
            }],
            cancellation_cost: None,
        }],
        resources: vec![], dependencies: vec![], baselines: vec![], executions: vec![], rows: vec![],
        cost_unit: unit::<Kilogram>(), materials: vec![], material_events: vec![],
    }
}

#[test]
fn activity_empty_domain_and_mandatory_empty_choices_are_valid_inputs() {
    assert!(prepare(problem()).unwrap().choices().is_empty());
    let mut input = problem();
    let mut job = activity();
    job.modes.clear();
    input.activities.push(job);
    let prepared = prepare(input).unwrap();
    assert!(prepared.choices().is_empty());
    assert!(prepared.cancellation_costs().is_empty());
    assert!(!prepared.solver_model_only());
}

#[test]
fn activity_horizon_and_calendar_intervals_must_be_nonempty() {
    let mut input = problem();
    input.horizon.end = input.horizon.start;
    assert!(prepare(input).is_err());
    let mut input = problem();
    input.resources.push(ActivityResource {
        id: "crew".into(), capacity: quantity(1.0),
        unavailable_times: vec![TimeRange::new(time(1), time(1))], capacities: vec![],
    });
    assert!(prepare(input.clone()).is_err());
    input.resources[0].unavailable_times.clear();
    input.resources[0].capacities.push(ActivityCapacity {
        time: TimeRange::new(time(1), time(1)), capacity: quantity(1.0),
    });
    assert!(prepare(input).is_err());
}

#[test]
fn activity_additional_cost_failure_is_propagated_without_a_partial_snapshot() {
    let mut input = problem();
    input.activities.push(activity());
    let failing = |_: &ActivityPlacement<f64>| Err(GanttError::Conversion { message: "policy failed".into() });
    assert!(matches!(ActivityPreparation::new(input.clone(), &F64SolverValueAdapter, Some(&failing)), Err(GanttError::Conversion { .. })));
    assert_eq!(prepare(input).unwrap().choices().len(), 2);
}

#[test]
fn activity_complete_candidates_are_sorted_and_extra_cost_is_cached_once() {
    let mut input = problem();
    input.activities.push(activity());
    let calls = AtomicUsize::new(0);
    let policy = |_: &ActivityPlacement<f64>| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(quantity(-1.0))
    };
    let prepared = ActivityPreparation::new(input, &F64SolverValueAdapter, Some(&policy)).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(prepared.choices().keys().cloned().collect::<Vec<_>>(), vec![key(1), key(2)]);
    for choice in prepared.choices().values() {
        assert_eq!(choice.cost.value, 1.0);
        assert_eq!(choice.cost.solver, 1.0);
    }
    let _ = prepared.clone();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn activity_milestones_allow_both_horizon_endpoints_but_no_clipping() {
    let mut input = problem();
    let mut job = activity();
    job.modes[0].duration = Duration::ZERO;
    job.modes[0].starts = vec![time(10), time(0)];
    input.activities.push(job);
    assert_eq!(prepare(input.clone()).unwrap().choices().len(), 2);
    input.activities[0].modes[0].duration = Duration::seconds(1);
    assert!(prepare(input).is_err());
}

#[test]
fn activity_duplicate_and_unknown_identities_are_rejected() {
    let mut input = problem();
    input.activities = vec![activity(), activity()];
    assert!(prepare(input).is_err());
    let mut input = problem();
    input.activities.push(activity());
    input.activities[0].modes[0].starts.push(time(1));
    assert!(prepare(input).is_err());
    let mut input = problem();
    input.activities.push(activity());
    input.activities[0].modes[0].id.clear();
    assert!(prepare(input).is_err());
    let mut input = problem();
    input.dependencies.push(ActivityDependency {
        predecessor: "missing".into(), successor: "missing".into(),
        from: ActivityEndpoint::Finish, to: ActivityEndpoint::Start,
        minimum: Duration::ZERO, maximum: None,
        presence: ActivityDependencyPresence::RequirePredecessor,
    });
    assert!(prepare(input).is_err());
}

#[test]
fn activity_empty_mode_still_validates_resource_material_cost_and_offsets() {
    for kind in 0..4 {
        let mut input = problem();
        let mut job = activity();
        job.modes[0].starts.clear();
        match kind {
            0 => job.modes[0].resources.push(ActivityResourceUse {
                resource: "missing".into(), quantity: quantity(1.0),
                from: Duration::ZERO, until: None,
            }),
            1 => job.modes[0].material_flows.push(ActivityMaterialFlow {
                material: "missing".into(), quantity: quantity(-1.0), at: Duration::ZERO,
            }),
            2 => job.modes[0].cost = Quantity::new(1.0, unit::<Celsius>()),
            _ => job.modes[0].duration = -Duration::seconds(1),
        }
        input.activities.push(job);
        assert!(prepare(input).is_err());
    }
}

#[test]
fn activity_time_overflow_and_infinite_sentinels_return_errors() {
    let mut input = problem();
    input.activities.push(activity());
    input.activities[0].modes[0].duration = Duration::MAX - Duration::seconds(1);
    assert!(prepare(input).is_err());
    let mut input = problem();
    input.horizon.end = distant_future();
    assert!(prepare(input).is_err());
}

#[test]
fn activity_baseline_costs_are_per_change_and_cancellation_is_separate() {
    let mut input = problem();
    let mut job = activity();
    job.cancellation_cost = Some(quantity(7.0));
    input.activities.push(job);
    input.baselines.push(ActivityBaseline {
        choice: key(1), frozen: false,
        start_change_cost: quantity(3.0), mode_change_cost: quantity(9.0),
    });
    let prepared = prepare(input.clone()).unwrap();
    assert_eq!(prepared.choices()[&key(1)].cost.value, 2.0);
    assert_eq!(prepared.choices()[&key(2)].cost.value, 5.0);
    assert_eq!(prepared.cancellation_costs()["job"].value, 7.0);
    input.baselines[0].choice = key(9);
    assert!(prepare(input.clone()).is_ok());
    input.baselines[0].frozen = true;
    assert!(prepare(input).is_err());
}

#[test]
fn activity_cost_signs_and_derived_overflow_are_checked() {
    let mut input = problem();
    let mut job = activity();
    job.modes[0].cost = quantity(-4.0);
    input.activities.push(job);
    assert_eq!(prepare(input.clone()).unwrap().choices()[&key(1)].cost.value, -4.0);
    input.activities[0].cancellation_cost = Some(quantity(-1.0));
    assert!(prepare(input).is_err());
    let mut input = problem();
    input.activities.push(activity());
    input.activities[0].modes[0].cost = quantity(f64::MAX);
    let extra = |_: &ActivityPlacement<f64>| Ok(quantity(f64::MAX));
    assert!(ActivityPreparation::new(input, &F64SolverValueAdapter, Some(&extra)).is_err());
}

#[test]
fn activity_units_normalize_and_affine_units_fail_without_panicking() {
    let converted = normalize_activity_value(&Quantity::new(1000.0, unit::<Gram>()), &unit::<Kilogram>(), &F64SolverValueAdapter).unwrap();
    assert_eq!(converted.value, 1.0);
    assert_eq!(converted.solver, 1.0);
    assert!(normalize_activity_value(&quantity(1.0), &unit::<Meter>(), &F64SolverValueAdapter).is_err());
    assert!(normalize_activity_value(&Quantity::new(1.0, unit::<Celsius>()), &unit::<Kelvin>(), &F64SolverValueAdapter).is_err());
    assert!(normalize_activity_value(&Quantity::new(1.0, unit::<Kelvin>()), &unit::<Celsius>(), &F64SolverValueAdapter).is_err());
    assert!(normalize_activity_value(&Quantity::new(1.0, unit::<Kelvin>()), &unit::<Kelvin>(), &F64SolverValueAdapter).is_ok());
    let tiny = Quantity::new(f64::from_bits(1), unit::<Gram>());
    assert!(normalize_activity_value(&tiny, &unit::<Kilogram>(), &F64SolverValueAdapter).is_err());
    for scale in [0, -1] {
        let invalid_unit = ospf_rust_quantities::unit::Unit::new(
            "invalid-scale".into(),
            "invalid".into(),
            unit::<Kilogram>().dimension().clone(),
            ospf_rust_quantities::scale::Scale::from_int(scale),
        );
        assert!(normalize_activity_value(&quantity(1.0), &invalid_unit, &F64SolverValueAdapter).is_err());
        assert!(normalize_activity_value(&Quantity::new(1.0, invalid_unit), &unit::<Kilogram>(), &F64SolverValueAdapter).is_err());
    }
}

#[test]
fn activity_capacity_overrides_validate_even_without_candidates() {
    let mut input = problem();
    input.resources.push(ActivityResource {
        id: "crew".into(), capacity: quantity(2.0), unavailable_times: vec![],
        capacities: vec![
            ActivityCapacity { time: TimeRange::new(time(1), time(3)), capacity: quantity(1.0) },
            ActivityCapacity { time: TimeRange::new(time(2), time(4)), capacity: quantity(1.0) },
        ],
    });
    assert!(prepare(input.clone()).is_err());
    input.resources[0].capacities[1].time.start = time(3);
    assert!(prepare(input.clone()).is_ok());
    input.resources[0].capacity = quantity(-1.0);
    assert!(prepare(input).is_err());
}

#[test]
fn activity_phases_and_material_flows_preserve_absolute_times_and_units() {
    let mut input = problem();
    input.resources.push(ActivityResource { id: "crew".into(), capacity: quantity(2.0), unavailable_times: vec![], capacities: vec![] });
    input.materials.push(ActivityMaterial { id: "raw".into(), initial: quantity(1.0), minimum: None, maximum: None });
    let mut job = activity();
    job.modes[0].resources.push(ActivityResourceUse {
        resource: "crew".into(), quantity: Quantity::new(500.0, unit::<Gram>()),
        from: Duration::minutes(10), until: Some(Duration::minutes(20)),
    });
    job.modes[0].material_flows.push(ActivityMaterialFlow {
        material: "raw".into(), quantity: Quantity::new(-1000.0, unit::<Gram>()), at: Duration::hours(1),
    });
    input.activities.push(job);
    let prepared = prepare(input).unwrap();
    let choice = &prepared.choices()[&key(1)];
    assert_eq!(choice.placement.reservations[0].quantity.value, 0.5);
    assert_eq!(choice.placement.reservations[0].time.start, time(1) + Duration::minutes(10));
    assert_eq!(choice.material_events[0].time, time(2));
    assert_eq!(choice.material_events[0].quantity.value, -1.0);
    assert!(prepared.solver_model_only());
}

#[test]
fn activity_inventory_opening_violation_is_not_an_input_error() {
    let mut input = problem();
    input.materials.push(ActivityMaterial {
        id: "raw".into(), initial: quantity(-1.0), minimum: None, maximum: Some(quantity(2.0)),
    });
    input.material_events.push(ActivityMaterialEvent { material: "raw".into(), time: time(0), quantity: quantity(1.0) });
    assert!(prepare(input.clone()).is_ok());
    input.material_events[0].time = time(10);
    assert!(prepare(input.clone()).is_ok());
    input.material_events[0].time = time(11);
    assert!(prepare(input.clone()).is_err());
    input.material_events.clear();
    input.materials[0].minimum = Some(quantity(3.0));
    assert!(prepare(input).is_err());
}

#[test]
fn activity_running_facts_keep_full_end_but_require_explicit_remaining_usage() {
    let mut input = problem();
    input.resources.push(ActivityResource { id: "crew".into(), capacity: quantity(1.0), unavailable_times: vec![], capacities: vec![] });
    input.executions.push(ActivityExecution {
        placement: ActivityPlacement {
            key: key(-2), time: TimeRange::new(time(-2), time(12)),
            reservations: vec![ActivityReservation { resource: "crew".into(), time: TimeRange::new(time(0), time(12)), quantity: quantity(1.0) }],
        },
        status: ActivityExecutionStatus::Running,
    });
    assert_eq!(prepare(input.clone()).unwrap().problem().executions[0].placement.time.end, time(12));
    input.executions[0].placement.reservations[0].time.start = time(-1);
    assert!(prepare(input.clone()).is_err());
    input.executions[0].placement.reservations.clear();
    input.executions[0].status = ActivityExecutionStatus::Completed;
    assert!(prepare(input.clone()).is_err());
    input.executions[0].placement.time.end = time(0);
    assert!(prepare(input).is_ok());
}

#[test]
fn activity_rows_require_explicit_bounds_and_known_choice_or_cancellation() {
    let mut input = problem();
    input.activities.push(activity());
    input.rows.push(ActivityRow {
        id: "budget".into(), unit: unit::<Kilogram>(),
        coefficients: BTreeMap::from([(key(1), quantity(1.0))]),
        cancellation_coefficients: BTreeMap::new(), lower: None, upper: Some(quantity(2.0)),
    });
    assert!(prepare(input.clone()).is_ok());
    input.rows[0].coefficients.insert(key(9), quantity(1.0));
    assert!(prepare(input.clone()).is_err());
    input.rows[0].coefficients.remove(&key(9));
    input.rows[0].cancellation_coefficients.insert("job".into(), quantity(1.0));
    assert!(prepare(input.clone()).is_err());
    input.activities[0].cancellation_cost = Some(quantity(2.0));
    assert!(prepare(input.clone()).is_ok());
    input.rows[0].upper = None;
    assert!(prepare(input).is_err());
}

#[derive(Debug)]
struct BrokenAdapter(u8);

impl SchedulingSolverValueAdapter<f64> for BrokenAdapter {
    fn try_into_value(&self, value: f64) -> GanttResult<f64> {
        self.try_from_value(&value)
    }

    fn try_from_value(&self, value: &f64) -> GanttResult<f64> {
        match self.0 {
            0 => Err(GanttError::Conversion { message: "custom failure".into() }),
            1 => Ok(f64::NAN),
            2 => Ok(f64::INFINITY),
            3 => Ok(0.0),
            _ => Ok(value * 2.0),
        }
    }

    fn into_value(&self, _: f64) -> f64 { panic!("legacy conversion must not be called") }
    fn from_value(&self, _: &f64) -> f64 { panic!("legacy conversion must not be called") }
    fn round_solution(&self, _: f64) -> f64 { panic!("legacy conversion must not be called") }
    fn floor_to_u64(&self, _: f64) -> u64 { panic!("legacy conversion must not be called") }
    fn floor_value(&self, _: f64) -> f64 { panic!("legacy conversion must not be called") }
    fn zero(&self) -> f64 { panic!("legacy conversion must not be called") }
    fn one(&self) -> f64 { panic!("legacy conversion must not be called") }
}

#[test]
fn activity_custom_adapter_errors_nonfinite_and_illegal_zero_never_call_legacy_methods() {
    for mode in 0..4 {
        let mut input = problem();
        input.activities.push(activity());
        assert!(ActivityPreparation::new(input, &BrokenAdapter(mode), None).is_err());
        assert!(activity_value_from_solver(1.0, &BrokenAdapter(mode)).is_err());
    }
    assert!(activity_value_from_solver(f64::NAN, &F64SolverValueAdapter).is_err());
    assert!(activity_value_from_solver(f64::INFINITY, &F64SolverValueAdapter).is_err());
    assert_eq!(activity_value_from_solver(1.0, &F64SolverValueAdapter).unwrap(), 1.0);
}

#[test]
fn activity_builtin_fallible_adapters_reject_nonfinite_values() {
    use crate::domain::task::SolverValueAdapter;
    let adapter = SolverValueAdapter::<f64>::new();
    assert!(adapter.try_into_value(f64::NAN).is_err());
    assert!(adapter.try_from_value(&f64::INFINITY).is_err());
    assert_eq!(adapter.try_from_value(&2.0).unwrap(), 2.0);
    assert_eq!(adapter.try_into_value(2.0).unwrap(), 2.0);
}

#[test]
fn activity_f64_custom_adapter_does_not_imply_source_model_equivalence() {
    let mut input = problem();
    input.activities.push(activity());
    let prepared = ActivityPreparation::new(input, &BrokenAdapter(4), None).unwrap();
    assert!(prepared.solver_model_only());
    assert_eq!(prepared.choices()[&key(1)].cost.value, 2.0);
    assert_eq!(prepared.choices()[&key(1)].cost.solver, 4.0);
}

#[cfg(feature = "big-decimal")]
#[test]
fn activity_decimal_preserves_native_precision_and_rejects_nonzero_underflow() {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;
    use crate::domain::task::SolverValueAdapter;
    let adapter = SolverValueAdapter::<BigDecimal>::new();
    let precise = BigDecimal::from_str(&format!("1.{}1", "0".repeat(99))).unwrap();
    let prepared = normalize_activity_value(&Quantity::new(precise.clone(), unit::<Kilogram>()), &unit::<Kilogram>(), &adapter).unwrap();
    assert_eq!(prepared.value, precise);
    assert_eq!(prepared.solver, 1.0);
    let snapshot = ActivityPreparation::new(numeric_problem(precise.clone()), &adapter, None).unwrap();
    assert!(snapshot.solver_model_only());
    assert_eq!(snapshot.choices()[&key(1)].cost.value, precise);
    let tiny = BigDecimal::from_str("1e-1000").unwrap();
    assert!(normalize_activity_value(&Quantity::new(tiny, unit::<Kilogram>()), &unit::<Kilogram>(), &adapter).is_err());
    let large = BigDecimal::from_str("1e310").unwrap();
    let converted = normalize_activity_value(&Quantity::new(large, unit::<Gram>()), &unit::<Kilogram>(), &adapter).unwrap();
    assert_eq!(converted.value, BigDecimal::from_str("1e307").unwrap());
    assert!(converted.solver.is_finite());
    assert!(adapter.try_into_value(f64::INFINITY).is_err());
}

#[cfg(feature = "big-rational")]
#[test]
fn activity_rational_preserves_native_precision_and_rejects_nonzero_underflow() {
    use num_rational::BigRational;
    use crate::domain::task::SolverValueAdapter;
    let adapter = SolverValueAdapter::<BigRational>::new();
    let precise = BigRational::new(1000000000000000001_i64.into(), 1000000000000000000_i64.into());
    let prepared = normalize_activity_value(&Quantity::new(precise.clone(), unit::<Kilogram>()), &unit::<Kilogram>(), &adapter).unwrap();
    assert_eq!(prepared.value, precise);
    assert_eq!(prepared.solver, 1.0);
    let snapshot = ActivityPreparation::new(numeric_problem(precise.clone()), &adapter, None).unwrap();
    assert!(snapshot.solver_model_only());
    assert_eq!(snapshot.choices()[&key(1)].cost.value, precise);
    let denominator = num_rational::BigRational::from_integer(10.into()).pow(1000);
    let tiny = BigRational::from_integer(1.into()) / denominator;
    assert!(normalize_activity_value(&Quantity::new(tiny, unit::<Kilogram>()), &unit::<Kilogram>(), &adapter).is_err());
}
