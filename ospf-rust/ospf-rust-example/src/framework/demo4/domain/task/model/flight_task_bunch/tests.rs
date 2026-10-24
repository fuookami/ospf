//! 恢复记录和实际业务规则回归 / Recovery record and effective business rule regressions.

mod cg;

use std::collections::HashMap;
use time::macros::datetime;
use super::*;
use ospf_rust_framework::model::{ColumnGenerationCompletion, ColumnGenerationScope};
use ospf_rust_framework_network_scheduling::domain::path::PathSearchBudget;
use crate::framework::demo4::domain::bunch_generation::recovery::{
    RecoveryActivityCandidate, RecoveryBunchCandidate, RecoveryBunchGenerationContext, RecoveryPathRequest,
};
use crate::framework::demo4::domain::bunch_compilation::context::{BunchCompilationContext, RecoveryMasterConfiguration};
use crate::framework::demo4::domain::passenger::model::Passenger;
use crate::framework::demo4::domain::rule::model::{Link, LinkType};
use crate::framework::demo4::domain::rule::service::{
    BunchRecoveryRules, CostCalculator, FeasibilityJudger, RecoveryCostPolicy,
};
use crate::framework::demo4::domain::task::model::{
    AircraftCapacity, AircraftMinorType, AircraftType, AircraftUsability, AirportType,
    FlightCyclePeriod, FlightTaskCategory, FlightTaskImpl, FlightTaskPlanImpl,
    FlightTaskStatus, FlightTaskTypeImpl, Route,
};
use crate::framework::demo4::infrastructure::{AircraftMinorTypeCode, AircraftRegisterNumber, AircraftTypeCode, Icao, PassengerClass};

fn airport(code: &str) -> Airport {
    Airport {
        icao: Icao(code.into()), airport_type: AirportType::Domestic,
        passenger_transfer_time: Duration::minutes(45), cargo_transfer_time: Duration::hours(1), base: false,
    }
}

fn aircraft() -> Aircraft {
    Aircraft {
        reg_no: AircraftRegisterNumber("B1001".into()),
        minor_type: AircraftMinorType {
            aircraft_type: AircraftType { code: AircraftTypeCode("A320".into()) },
            code: AircraftMinorTypeCode("A320-200".into()), cost_per_hour: 100.0,
            route_fly_time: HashMap::new(), connection_time: HashMap::from([(airport("B"), Duration::minutes(30))]),
            max_fly_time: None,
        },
        capacity: AircraftCapacity::Passenger { capacities: vec![(PassengerClass("Y".into()), 100)] },
    }
}

fn at(hour: i64, minute: i64) -> OffsetDateTime {
    datetime!(2026-06-07 00:00 UTC) + Duration::hours(hour) + Duration::minutes(minute)
}

fn activity(id: &str, dep: &str, arr: &str, start: i64, end: i64) -> FlightTaskRecovery {
    let original = FlightTaskImpl::new(
        FlightTaskPlanImpl {
            id: id.into(), name: format!("original-{id}"), aircraft: Some(aircraft()),
            dep: airport("PLANNED-DEP"), arr: airport("PLANNED-ARR"),
            time: Some((at(start - 1, 0), at(end - 1, 0))),
            scheduled_time: Some((at(start - 1, 0), at(end - 1, 0))), flight_task_status: vec![],
        },
        FlightTaskTypeImpl { category: FlightTaskCategory::Flight, name: "flight".into() },
    );
    FlightTaskRecovery::new(original, aircraft(), Route { dep: airport(dep), arr: airport(arr) }, (at(start, 0), at(end, 0))).unwrap()
}

fn bunch() -> FlightTaskBunch {
    FlightTaskBunch::new("recovered".into(), aircraft(), vec![
        activity("F1", "A", "B", 8, 9), activity("F2", "B", "C", 10, 12),
    ]).unwrap()
}

fn usability() -> AircraftUsability {
    AircraftUsability { location: airport("A"), enabled_time: at(7, 0), flight_cycle_periods: vec![] }
}

fn passenger(amount: u64) -> Passenger {
    Passenger { id: "P1".into(), amount, flights: vec![
        ("F1".into(), PassengerClass("Y".into())), ("F2".into(), PassengerClass("Y".into())),
    ] }
}

fn recovery_master() -> BunchCompilationContext {
    BunchCompilationContext::new(RecoveryMasterConfiguration {
        tasks: bunch().activities().iter().map(|activity| activity.original().clone()).collect(),
        aircraft: vec![aircraft()], cancellation_costs: vec![Some(1000.0), Some(1000.0)],
    }).unwrap()
}

// 独立枚举实际后端矩阵；这不是替代 Gantt 列生成流程的应用求解器。 / Independently enumerate the actual backend matrix; this is not an application solver replacing Gantt column generation.
fn master_optimum(model: &ospf_rust_core::model::MetaModel<f64>) -> Option<(f64, Vec<f64>)> {
    let matrix = model.try_to_linear_triad_model().unwrap();
    assert!(matrix.num_variables() < 12);
    (0..(1usize << matrix.num_variables())).filter_map(|mask| {
        let values = (0..matrix.num_variables()).map(|index| ((mask >> index) & 1) as f64).collect::<Vec<_>>();
        if values.iter().enumerate().any(|(index, value)| *value < matrix.lb[index] || *value > matrix.ub[index])
            || matrix.A.rows.iter().zip(&matrix.b).any(|(row, bound)| {
                row.entries.iter().map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() > *bound + 1e-9
            })
        { return None; }
        Some((values.iter().zip(&matrix.c).map(|(value, coefficient)| value * coefficient).sum::<f64>(), values))
    }).min_by(|left, right| left.0.total_cmp(&right.0))
}

#[test]
fn recovery_master_preserves_records_through_real_gantt_rows_retirement_and_readdition() {
    let mut context = recovery_master();
    let mut model = ospf_rust_core::model::MetaModel::new("recovery-managed-master");
    context.register(&mut model).unwrap();
    let initial = master_optimum(&model).unwrap();
    assert_eq!(initial.0, 2000.0);
    assert_eq!(context.extract_solution(&model, &initial.1).unwrap().canceled.len(), 2);
    let candidate = RecoveryBunchCandidate { bunch: bunch(), cost: 300.0 };
    assert_eq!(context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap(), vec![0]);
    let original_x = context.master().compilation.base.x_index(0).unwrap();
    let optimum = master_optimum(&model).unwrap();
    assert_eq!(optimum.0, 300.0);
    let schedule = context.extract_solution(&model, &optimum.1).unwrap();
    assert!(schedule.canceled.is_empty());
    assert_eq!(schedule.selected.len(), 1);
    let selected = &schedule.selected[0].bunch;
    assert_eq!(selected.recovery("F1").unwrap().original().plan.time, Some((at(7, 0), at(8, 0))));
    assert_eq!(selected.recovery("F1").unwrap().time(), (at(8, 0), at(9, 0)));
    assert!(selected.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
    assert!(selected.departed_when(&airport("B"), &(at(10, 0), at(10, 0))));
    assert!(context.add_candidates(1, vec![candidate.clone()], &mut model).unwrap().is_empty());
    assert_eq!(model.as_basic().num_tokens(), 3);
    context.remove_columns(&[0], &mut model).unwrap();
    assert_eq!(master_optimum(&model).unwrap().0, 2000.0);
    assert_eq!(context.add_candidates(2, vec![candidate], &mut model).unwrap(), vec![1]);
    assert_ne!(context.master().compilation.base.x_index(1).unwrap(), original_x);
    assert_eq!(master_optimum(&model).unwrap().0, 300.0);
    assert_eq!(model.variable_range_by_index(original_x), Some(ospf_rust_core::variable::VariableRange::bounded(0.0, 0.0)));
}

#[test]
fn recovery_master_rejects_conflicting_complete_payloads_before_mutation() {
    let mut context = recovery_master();
    let mut model = ospf_rust_core::model::MetaModel::new("recovery-payload-conflicts");
    context.register(&mut model).unwrap();
    let candidate = RecoveryBunchCandidate { bunch: bunch(), cost: 300.0 };
    context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    let tokens = model.as_basic().num_tokens();
    let before = ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    let mut route = candidate.bunch.activities()[1].route().clone();
    route.arr.passenger_transfer_time += Duration::minutes(1);
    let mut activities = candidate.bunch.activities().to_vec();
    activities[1] = FlightTaskRecovery::new(activities[1].original().clone(), aircraft(), route, activities[1].time()).unwrap();
    let changed_metadata = RecoveryBunchCandidate { bunch: FlightTaskBunch::new("recovered".into(), aircraft(), activities).unwrap(), cost: 300.0 };
    let changed_time = RecoveryBunchCandidate { bunch: FlightTaskBunch::new("recovered".into(), aircraft(), vec![
        candidate.bunch.activities()[0].clone(),
        FlightTaskRecovery::new(candidate.bunch.activities()[1].original().clone(), aircraft(), candidate.bunch.activities()[1].route().clone(), (at(11, 0), at(13, 0))).unwrap(),
    ]).unwrap(), cost: 300.0 };
    let mut changed_aircraft = aircraft();
    changed_aircraft.minor_type.connection_time.clear();
    let mut airport = airport("B");
    airport.passenger_transfer_time += Duration::minutes(1);
    changed_aircraft.minor_type.connection_time.insert(airport, Duration::minutes(30));
    let activities = candidate.bunch.activities().iter().map(|activity| FlightTaskRecovery::new(
        activity.original().clone(), changed_aircraft.clone(), activity.route().clone(), activity.time(),
    ).unwrap()).collect();
    let changed_aircraft = RecoveryBunchCandidate { bunch: FlightTaskBunch::new("recovered".into(), changed_aircraft, activities).unwrap(), cost: 300.0 };
    for bad in [RecoveryBunchCandidate { cost: 301.0, ..candidate.clone() }, changed_metadata, changed_time,
        changed_aircraft, RecoveryBunchCandidate { cost: f64::NAN, ..candidate }]
    {
        assert!(context.add_candidates(1, vec![bad], &mut model).is_err());
        assert_eq!(model.as_basic().num_tokens(), tokens);
        assert_eq!(ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
        assert_eq!(context.master().compilation.active_bunch_count(), 1);
    }
}

#[test]
fn recovery_master_rejects_unknown_and_changed_original_tasks_atomically() {
    let mut context = recovery_master();
    let mut model = ospf_rust_core::model::MetaModel::new("recovery-original-contract");
    context.register(&mut model).unwrap();
    for change_id in [false, true] {
        let recovery = activity("F1", "A", "B", 8, 9);
        let mut original = recovery.original().clone();
        if change_id { original.plan.id = "unknown".into(); } else { original.plan.name = "mutated-original".into(); }
        let bad = RecoveryBunchCandidate { bunch: FlightTaskBunch::new("bad".into(), aircraft(), vec![
            FlightTaskRecovery::new(original, aircraft(), recovery.route().clone(), recovery.time()).unwrap(),
        ]).unwrap(), cost: 10.0 };
        let good = RecoveryBunchCandidate { bunch: bunch(), cost: 300.0 };
        assert!(context.add_candidates(0, vec![good, bad], &mut model).is_err());
        assert_eq!(model.as_basic().num_tokens(), 2);
        assert_eq!(context.master().compilation.active_bunch_count(), 0);
    }
}

#[test]
fn recovery_master_preserves_downstream_rules_and_rolls_back_registration_failure() {
    let mut context = recovery_master();
    let mut model = ospf_rust_core::model::MetaModel::new("recovery-extension-contract");
    context.register(&mut model).unwrap();
    model.retain_constraints(|row| row.name != "executor_compilation_B1001");
    model.add_eq_constraint(&[], 0.0, "executor_compilation_B1001").unwrap();
    let candidate = RecoveryBunchCandidate { bunch: bunch(), cost: 300.0 };
    assert!(context.add_candidates(0, vec![candidate.clone()], &mut model).is_err());
    assert_eq!(context.master().compilation.active_bunch_count(), 0);
    assert_eq!(model.as_basic().num_tokens(), 2);
    model.retain_constraints(|row| row.name != "executor_compilation_B1001");
    context.add_candidates(0, vec![RecoveryBunchCandidate { cost: 301.0, ..candidate }], &mut model).unwrap();
    let x = context.master().compilation.base.x_index(0).unwrap();
    let previous = master_optimum(&model).unwrap();
    model.add_eq_constraint(&[(x, 1.0)], 0.0, "downstream_aircraft_restriction").unwrap();
    assert!(context.extract_solution(&model, &previous.1).is_err());
    assert_eq!(master_optimum(&model).unwrap().0, 2000.0);
    context.remove_columns(&[0], &mut model).unwrap();
    assert!(model.as_basic().constraints().iter().any(|row| row.name == "downstream_aircraft_restriction"));
}

#[test]
fn recovery_master_namespaces_idle_bunches_and_handles_empty_and_hard_domains() {
    let mut other = aircraft();
    other.reg_no.0 = "B1002".into();
    let mut context = BunchCompilationContext::new(RecoveryMasterConfiguration {
        tasks: vec![], aircraft: vec![aircraft(), other.clone()], cancellation_costs: vec![],
    }).unwrap();
    let mut model = ospf_rust_core::model::MetaModel::new("idle-aircraft-namespaces");
    context.register(&mut model).unwrap();
    let candidates = [aircraft(), other].into_iter().map(|aircraft| RecoveryBunchCandidate {
        bunch: FlightTaskBunch::new("idle".into(), aircraft, vec![]).unwrap(), cost: 0.0,
    }).collect();
    assert_eq!(context.add_candidates(0, candidates, &mut model).unwrap(), vec![0, 1]);
    let mut empty = BunchCompilationContext::new(RecoveryMasterConfiguration { tasks: vec![], aircraft: vec![], cancellation_costs: vec![] }).unwrap();
    let mut model = ospf_rust_core::model::MetaModel::new("empty-recovery-master");
    empty.register(&mut model).unwrap();
    assert_eq!(master_optimum(&model).unwrap().0, 0.0);
    assert!(empty.extract_solution(&model, &[]).unwrap().selected.is_empty());
    let mut task = activity("F1", "A", "B", 8, 9).original().clone();
    task.plan.flight_task_status.push(FlightTaskStatus::NotCancel);
    assert!(BunchCompilationContext::new(RecoveryMasterConfiguration {
        tasks: vec![task.clone()], aircraft: vec![aircraft()], cancellation_costs: vec![Some(1.0)],
    }).is_err());
    let mut hard = BunchCompilationContext::new(RecoveryMasterConfiguration {
        tasks: vec![task], aircraft: vec![aircraft()], cancellation_costs: vec![None],
    }).unwrap();
    let mut model = ospf_rust_core::model::MetaModel::new("hard-recovery-master");
    hard.register(&mut model).unwrap();
    assert!(master_optimum(&model).is_none());
}

#[test]
fn recovery_retains_original_and_derives_all_views_from_actual_records() {
    let bunch = bunch();
    let record = bunch.recovery("F1").unwrap();
    assert_eq!(record.original().plan.name, "original-F1");
    assert_eq!(record.original().plan.dep, airport("PLANNED-DEP"));
    assert_eq!(record.original().plan.time, Some((at(7, 0), at(8, 0))));
    assert_eq!(record.route().dep, airport("A"));
    assert_eq!(record.time(), (at(8, 0), at(9, 0)));
    assert_eq!(bunch.task_ids().collect::<Vec<_>>(), vec!["F1", "F2"]);
    assert_eq!(bunch.time(), Some((at(8, 0), at(12, 0))));
    assert_eq!(bunch.dep(), Some(&airport("A")));
    assert_eq!(bunch.arr(), Some(&airport("C")));
    assert!(bunch.consecutive("F1", "F2"));
    assert!(!bunch.consecutive("F2", "F1"));
}

#[test]
fn internal_airport_events_use_recovered_times_and_inclusive_boundaries() {
    let bunch = bunch();
    assert!(bunch.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
    assert!(bunch.departed_when(&airport("B"), &(at(10, 0), at(10, 0))));
    assert!(!bunch.arrived_when(&airport("B"), &(at(8, 0), at(8, 59))));
    assert!(!bunch.departed_when(&airport("B"), &(at(9, 0), at(9, 59))));
    assert!(!bunch.arrived_when(&airport("B"), &(at(10, 0), at(8, 0))));
}

#[test]
fn empty_bunch_has_no_fabricated_events_or_clock_dependency() {
    let bunch = FlightTaskBunch::new("idle".into(), aircraft(), vec![]).unwrap();
    assert_eq!(bunch.time(), None);
    assert_eq!(bunch.dep(), None);
    assert_eq!(bunch.arr(), None);
    assert_eq!(bunch.task_ids().count(), 0);
    assert_eq!(bunch.flight_hour(), FlightHour::ZERO);
    assert_eq!(bunch.flight_cycle(), FlightCycle::ZERO);
    assert!(!bunch.arrived_when(&airport("A"), &(at(0, 0), at(23, 0))));
    assert_eq!(CostCalculator.calculate_bunch(&bunch, &CostCalculator).unwrap(), 0.0);
}

#[test]
fn structural_errors_reject_duplicate_aircraft_disconnection_and_time_overlap() {
    let first = activity("F1", "A", "B", 8, 9);
    assert!(matches!(FlightTaskBunch::new("x".into(), aircraft(), vec![first.clone(), first.clone()]), Err(RecoveryError::DuplicateTask(_))));
    assert!(matches!(FlightTaskBunch::new("x".into(), aircraft(), vec![first.clone(), activity("F2", "X", "C", 10, 11)]), Err(RecoveryError::Disconnected(_))));
    assert!(matches!(FlightTaskBunch::new("x".into(), aircraft(), vec![first.clone(), activity("F2", "B", "C", 8, 10)]), Err(RecoveryError::Disconnected(_))));
    let mut other = aircraft();
    other.reg_no.0 = "B1002".into();
    assert!(matches!(FlightTaskBunch::new("x".into(), other, vec![first.clone()]), Err(RecoveryError::AircraftMismatch(_))));
    assert!(matches!(FlightTaskRecovery::new(first.original().clone(), aircraft(), first.route().clone(), (at(8, 0), at(8, 0))), Err(RecoveryError::InvalidTime(_))));
}

#[test]
fn actual_flight_resources_and_terminal_cost_use_the_same_records() {
    struct Terminal;
    impl RecoveryCostPolicy for Terminal {
        fn activity_cost(&self, activity: &FlightTaskRecovery) -> Result<f64, RecoveryError> {
            CostCalculator.activity_cost(activity)
        }
        fn terminal_cost(&self, bunch: &FlightTaskBunch) -> Result<f64, RecoveryError> {
            assert_eq!(bunch.arr(), Some(&airport("C")));
            assert_eq!(bunch.time().unwrap().1, at(12, 0));
            Ok(17.0)
        }
    }
    let bunch = bunch();
    assert_eq!(bunch.flight_hour(), FlightHour::new(Duration::hours(3)));
    assert_eq!(bunch.flight_cycle(), FlightCycle::new(2));
    assert_eq!(CostCalculator.calculate_bunch(&bunch, &Terminal).unwrap(), 317.0);
}

#[test]
fn nonflight_ground_activity_does_not_consume_flight_resources_or_hourly_cost() {
    let record = activity("M1", "A", "A", 8, 10);
    let mut original = record.original().clone();
    original.task_type.category = FlightTaskCategory::Maintenance;
    let ground = FlightTaskRecovery::new(original, aircraft(), record.route().clone(), record.time()).unwrap();
    let bunch = FlightTaskBunch::new("ground".into(), aircraft(), vec![ground]).unwrap();
    assert_eq!(bunch.flight_hour(), FlightHour::ZERO);
    assert_eq!(bunch.flight_cycle(), FlightCycle::ZERO);
    assert_eq!(CostCalculator.calculate_bunch(&bunch, &CostCalculator).unwrap(), 0.0);
}

#[test]
fn cost_policy_rejects_nan_infinite_and_overflowing_contributions() {
    struct Invalid(f64);
    impl RecoveryCostPolicy for Invalid {
        fn activity_cost(&self, _: &FlightTaskRecovery) -> Result<f64, RecoveryError> { Ok(self.0) }
        fn terminal_cost(&self, _: &FlightTaskBunch) -> Result<f64, RecoveryError> { Ok(self.0) }
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        assert_eq!(CostCalculator.calculate_bunch(&bunch(), &Invalid(value)), Err(RecoveryError::InvalidCost));
    }
}

#[test]
fn aircraft_availability_and_actual_predecessor_connection_are_effective() {
    let bunch = bunch();
    let mut usable = usability();
    let mut rules = BunchRecoveryRules { usability: &usable, previous: None, links: &[] };
    assert!(FeasibilityJudger.judge_bunch(&bunch, &rules).unwrap().feasible);
    let previous = activity("PREV", "Z", "A", 6, 9);
    rules.previous = Some(&previous);
    assert!(!FeasibilityJudger.judge_bunch(&bunch, &rules).unwrap().feasible);
    usable.enabled_time = at(8, 1);
    rules = BunchRecoveryRules { usability: &usable, previous: None, links: &[] };
    assert!(!FeasibilityJudger.judge_bunch(&bunch, &rules).unwrap().feasible);
    usable.enabled_time = at(7, 0);
    usable.location = airport("X");
    assert!(!FeasibilityJudger.judge_bunch(&bunch, &BunchRecoveryRules { usability: &usable, previous: None, links: &[] }).unwrap().feasible);
}

#[test]
fn actual_hours_and_cycles_enforce_period_allowances() {
    let mut usable = usability();
    usable.flight_cycle_periods.push(FlightCyclePeriod {
        expiration_time: at(20, 0), remaining_flight_hour: Some(FlightHour::new(Duration::hours(2))),
        remaining_flight_cycle: Some(FlightCycle::new(2)),
    });
    let check = |usable: &AircraftUsability| FeasibilityJudger.judge_bunch(&bunch(), &BunchRecoveryRules { usability: usable, previous: None, links: &[] }).unwrap().feasible;
    assert!(!check(&usable));
    usable.flight_cycle_periods[0].remaining_flight_hour = Some(FlightHour::new(Duration::hours(3)));
    assert!(check(&usable));
    usable.flight_cycle_periods[0].remaining_flight_cycle = Some(FlightCycle::new(1));
    assert!(!check(&usable));
}

#[test]
fn links_are_not_ignored_and_invalid_rule_is_distinct_from_infeasibility() {
    let usable = usability();
    let mut links = vec![Link { split_cost: None, from_task: "F1".into(), to_task: "F2".into(), link_type: LinkType::Connecting { min_connection_time: Duration::minutes(60) } }];
    let check = |links: &[Link]| FeasibilityJudger.judge_bunch(&bunch(), &BunchRecoveryRules { usability: &usable, previous: None, links });
    assert!(check(&links).unwrap().feasible);
    links[0].link_type = LinkType::Connecting { min_connection_time: Duration::minutes(61) };
    assert!(!check(&links).unwrap().feasible);
    links[0].to_task = "F3".into();
    assert!(!check(&links).unwrap().feasible);
    links[0].link_type = LinkType::Connecting { min_connection_time: Duration::minutes(-1) };
    assert!(matches!(check(&links), Err(RecoveryError::InvalidRule(_))));
}

#[test]
fn hard_passenger_rules_enforce_actual_transfer_and_aggregated_capacity() {
    assert!(FeasibilityJudger.judge_passengers(&[bunch()], &[passenger(100)]).unwrap().feasible);
    assert!(!FeasibilityJudger.judge_passengers(&[bunch()], &[passenger(101)]).unwrap().feasible);
    let mut second = passenger(50);
    second.id = "P2".into();
    assert!(!FeasibilityJudger.judge_passengers(&[bunch()], &[passenger(60), second]).unwrap().feasible);
    let short = FlightTaskBunch::new("short".into(), aircraft(), vec![
        activity("F1", "A", "B", 8, 9),
        FlightTaskRecovery::new(activity("F2", "B", "C", 10, 12).original().clone(), aircraft(), Route { dep: airport("B"), arr: airport("C") }, (at(9, 30), at(11, 30))).unwrap(),
    ]).unwrap();
    let result = FeasibilityJudger.judge_passengers(&[short], &[passenger(10)]).unwrap();
    assert!(!result.feasible);
    assert!(result.violations.iter().any(|reason| reason.contains("transfer")));
}

#[test]
fn passenger_transfers_can_cross_aircraft_and_missing_flights_are_not_silently_served() {
    let first = FlightTaskBunch::new("one".into(), aircraft(), vec![activity("F1", "A", "B", 8, 9)]).unwrap();
    let mut other = aircraft();
    other.reg_no.0 = "B1002".into();
    let record = activity("F2", "B", "C", 10, 12);
    let second = FlightTaskBunch::new("two".into(), other.clone(), vec![FlightTaskRecovery::new(record.original().clone(), other, record.route().clone(), record.time()).unwrap()]).unwrap();
    assert!(FeasibilityJudger.judge_passengers(&[first.clone(), second], &[passenger(20)]).unwrap().feasible);
    assert!(!FeasibilityJudger.judge_passengers(&[first], &[passenger(20)]).unwrap().feasible);
    assert!(FeasibilityJudger.judge_passengers(&[], &[]).unwrap().feasible);
}

#[test]
fn original_task_status_is_checked_against_actual_recovery() {
    let first = activity("F1", "A", "B", 8, 9);
    let mut original = first.original().clone();
    original.plan.flight_task_status = vec![FlightTaskStatus::NotDelay, FlightTaskStatus::NotTerminalChange];
    let record = FlightTaskRecovery::new(original, aircraft(), first.route().clone(), first.time()).unwrap();
    let bunch = FlightTaskBunch::new("invalid".into(), aircraft(), vec![record]).unwrap();
    let result = FeasibilityJudger.judge_bunch(&bunch, &BunchRecoveryRules { usability: &usability(), previous: None, links: &[] }).unwrap();
    assert!(!result.feasible);
    assert!(result.violations.iter().any(|reason| reason.contains("time change")));
    assert!(result.violations.iter().any(|reason| reason.contains("route change")));
}

fn scope() -> ColumnGenerationScope {
    ColumnGenerationScope {
        request: "recovery-request".into(), model: "model".into(), dual: "heuristic".into(),
        branch: "root".into(), groups: ["B1001".into()].into(), executor_slots: Default::default(), global: false,
    }
}

fn candidates() -> Vec<RecoveryActivityCandidate> {
    vec![
        RecoveryActivityCandidate { id: "F1-recovered".into(), recovery: activity("F1", "A", "B", 8, 9) },
        RecoveryActivityCandidate { id: "F2-recovered".into(), recovery: activity("F2", "B", "C", 10, 12) },
    ]
}

#[test]
fn shared_path_bridge_preserves_records_trace_and_effective_link_rules() {
    let aircraft = aircraft();
    let usable = usability();
    let candidates = candidates();
    let links = [Link { split_cost: None, from_task: "F1".into(), to_task: "F2".into(), link_type: LinkType::Connecting { min_connection_time: Duration::hours(1) } }];
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &links },
        cost: &CostCalculator, scope: scope(), maximum_results: None,
    };
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).unwrap();
    // 手算可行域：空束和 F1/F2；单独 F1 违反链接，F2 无法从 A 起飞。 / Hand oracle: idle and F1/F2; F1 violates the link and F2 cannot start at A.
    assert_eq!(generated.batch.columns.len(), 2);
    assert_eq!(generated.search.diagnostics.total_results, 2);
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Exhausted);
    assert_eq!(generated.batch.no_improving_columns, None);
    let full = &generated.batch.columns[1];
    assert_eq!(full.bunch.task_ids().collect::<Vec<_>>(), vec!["F1", "F2"]);
    assert_eq!(full.cost, 300.0);
    assert!(full.bunch.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
    assert_eq!(full.bunch.recovery("F1").unwrap().original().plan.time, Some((at(7, 0), at(8, 0))));
    let trace = generated.search.trace(generated.search.paths[1].label).unwrap();
    assert_eq!(trace.len(), 3);
    assert!(trace[0].as_str().starts_with("start:"));
    assert!(trace[1].as_str().starts_with("connection:"));
    assert!(trace[2].as_str().starts_with("finish:"));
}

#[test]
fn output_limit_and_budget_never_invent_a_no_improvement_certificate() {
    let aircraft = aircraft();
    let usable = usability();
    let candidates = candidates();
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &CostCalculator, scope: scope(), maximum_results: Some(0),
    };
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).unwrap();
    assert!(generated.batch.columns.is_empty());
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Exhausted);
    assert_eq!(generated.search.diagnostics.total_results, 3);
    assert_eq!(generated.batch.no_improving_columns, None);
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget { maximum_labels: Some(1), ..Default::default() }).unwrap();
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::BudgetLimit);
    assert_eq!(generated.batch.no_improving_columns, None);
    let interrupted = || true;
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget { interrupted: Some(&interrupted), ..Default::default() }).unwrap();
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Interrupted);
}

#[test]
fn nonadditive_terminal_policy_sorts_after_complete_cost_and_cannot_use_dominance() {
    struct Reward;
    impl RecoveryCostPolicy for Reward {
        fn activity_cost(&self, activity: &FlightTaskRecovery) -> Result<f64, RecoveryError> { CostCalculator.activity_cost(activity) }
        fn terminal_cost(&self, bunch: &FlightTaskBunch) -> Result<f64, RecoveryError> {
            Ok(if bunch.activities().len() == 2 { -500.0 } else { 0.0 })
        }
    }
    let aircraft = aircraft();
    let usable = usability();
    let candidates = candidates();
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &Reward, scope: scope(), maximum_results: Some(1),
    };
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).unwrap();
    assert_eq!(generated.batch.columns.len(), 1);
    assert_eq!(generated.batch.columns[0].cost, -200.0);
    assert_eq!(generated.batch.columns[0].bunch.task_ids().collect::<Vec<_>>(), vec!["F1", "F2"]);
    assert_eq!(generated.search.diagnostics.total_results, 3);
    assert_eq!(generated.search.diagnostics.dominance_checks, 0);
}

#[test]
fn alternative_recoveries_never_cover_the_same_original_task_twice() {
    let aircraft = aircraft();
    let usable = usability();
    let mut candidates = candidates();
    candidates.push(RecoveryActivityCandidate { id: "F1-again".into(), recovery: activity("F1", "B", "C", 13, 14) });
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &CostCalculator, scope: scope(), maximum_results: None,
    };
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).unwrap();
    assert_eq!(generated.search.diagnostics.total_results, 3);
    for column in generated.batch.columns {
        let ids = column.bunch.task_ids().collect::<Vec<_>>();
        assert_eq!(ids.len(), ids.iter().collect::<std::collections::HashSet<_>>().len());
    }
}

#[test]
fn recovery_bridge_rejects_duplicate_candidates_and_overclaimed_aircraft_scope() {
    let aircraft = aircraft();
    let usable = usability();
    let mut candidates = candidates();
    candidates.push(candidates[0].clone());
    let mut request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &CostCalculator, scope: scope(), maximum_results: None,
    };
    assert!(RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).is_err());
    request.candidates = &[];
    request.scope.groups.insert("B1002".into());
    assert!(RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).is_err());
}

#[test]
fn recovery_bridge_rejects_empty_business_identities_before_emitting_unreplayable_columns() {
    let mut aircraft = aircraft();
    let usable = usability();
    let mut candidates = candidates();
    candidates[0].id.clear();
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &CostCalculator, scope: scope(), maximum_results: None,
    };
    assert!(RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).is_err());
    aircraft.reg_no.0.clear();
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &[],
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &CostCalculator, scope: ColumnGenerationScope { groups: [String::new()].into(), ..scope() }, maximum_results: None,
    };
    assert!(RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).is_err());
}

#[test]
fn aircraft_identity_cannot_hide_conflicting_cost_or_capacity_payloads() {
    let mut altered = aircraft();
    altered.minor_type.cost_per_hour = 1.0;
    assert!(matches!(FlightTaskBunch::new("bad".into(), altered, vec![activity("F1", "A", "B", 8, 9)]), Err(RecoveryError::AircraftMismatch(_))));
    let mut altered = aircraft();
    altered.capacity = AircraftCapacity::Passenger { capacities: vec![(PassengerClass("Y".into()), 999)] };
    assert!(matches!(FlightTaskBunch::new("bad".into(), altered, vec![activity("F1", "A", "B", 8, 9)]), Err(RecoveryError::AircraftMismatch(_))));
}

#[test]
fn predecessor_location_and_explicit_connection_override_use_actual_records() {
    let usable = usability();
    let previous = activity("PREV", "Z", "B", 6, 9);
    let record = activity("F2", "B", "C", 9, 11);
    let bunch = FlightTaskBunch::new("after-previous".into(), aircraft(), vec![record]).unwrap();
    let link = Link { split_cost: None, from_task: "PREV".into(), to_task: "F2".into(), link_type: LinkType::ConnectionTimeIgnoring };
    let rules = BunchRecoveryRules { usability: &usable, previous: Some(&previous), links: std::slice::from_ref(&link) };
    assert!(FeasibilityJudger.judge_bunch(&bunch, &rules).unwrap().feasible);
    let rules = BunchRecoveryRules { usability: &usable, previous: Some(&previous), links: &[] };
    assert!(!FeasibilityJudger.judge_bunch(&bunch, &rules).unwrap().feasible);
}

#[test]
fn shared_path_matches_independent_permutation_oracle_on_recovery_alternatives() {
    fn enumerate(
        candidates: &[RecoveryActivityCandidate],
        path: &mut Vec<usize>,
        result: &mut Vec<String>,
    ) {
        result.push(path.iter().map(|&index| candidates[index].id.as_str()).collect::<Vec<_>>().join(","));
        for (index, candidate) in candidates.iter().enumerate() {
            if path.iter().any(|&visited| candidates[visited].recovery.original().plan.id == candidate.recovery.original().plan.id) {
                continue;
            }
            let feasible = if let Some(&previous) = path.last() {
                let from = &candidates[previous].recovery;
                let connection = if candidate.recovery.route().dep == airport("B") { Duration::minutes(30) } else { Duration::ZERO };
                from.route().arr == candidate.recovery.route().dep && candidate.recovery.time().0 - from.time().1 >= connection
            } else {
                candidate.recovery.route().dep == airport("A") && candidate.recovery.time().0 >= at(7, 0)
            };
            if feasible {
                path.push(index);
                enumerate(candidates, path, result);
                path.pop();
            }
        }
    }
    let aircraft = aircraft();
    let usable = usability();
    let mut candidates = candidates();
    candidates.push(RecoveryActivityCandidate { id: "F3-recovered".into(), recovery: activity("F3", "C", "A", 13, 14) });
    candidates.push(RecoveryActivityCandidate { id: "F2-delayed".into(), recovery: activity("F2", "B", "C", 12, 14) });
    let mut expected = Vec::new();
    enumerate(&candidates, &mut Vec::new(), &mut expected);
    expected.sort();
    let request = RecoveryPathRequest {
        aircraft: &aircraft, candidates: &candidates,
        rules: BunchRecoveryRules { usability: &usable, previous: None, links: &[] },
        cost: &CostCalculator, scope: scope(), maximum_results: None,
    };
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).unwrap();
    let mut actual: Vec<_> = generated.batch.columns.iter().map(|column| {
        column.bunch.activities().iter().map(|activity| {
            candidates.iter().find(|candidate| candidate.recovery.original().plan.id == activity.original().plan.id
                && candidate.recovery.time() == activity.time()).unwrap().id.as_str()
        }).collect::<Vec<_>>().join(",")
    }).collect();
    actual.sort();
    assert_eq!(actual, expected);
    assert!(actual.iter().any(|path| path.contains("F2-delayed")));
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Exhausted);
}

#[test]
fn actual_aircraft_capacity_category_is_checked_for_flights_but_not_ground_or_unassigned_requirements() {
    let original = activity("F1", "A", "B", 8, 9);
    let mut cargo = aircraft(); cargo.capacity = AircraftCapacity::Cargo { max_weight: 1000.0 };
    for (category, assigned, expected) in [(FlightTaskCategory::Flight, true, false),
        (FlightTaskCategory::VirtualFlight, true, false), (FlightTaskCategory::Maintenance, true, true),
        (FlightTaskCategory::Flight, false, true)] {
        let mut task = original.original().clone(); task.task_type.category = category;
        if !assigned { task.plan.aircraft = None; }
        let recovery = FlightTaskRecovery::new(task, cargo.clone(), original.route().clone(), original.time()).unwrap();
        let bunch = FlightTaskBunch::new("capacity-category".into(), cargo.clone(), vec![recovery]).unwrap();
        assert_eq!(FeasibilityJudger.judge_bunch(&bunch, &BunchRecoveryRules { usability: &usability(), previous: None, links: &[] })
            .unwrap().feasible, expected);
    }
    let mut task = original.original().clone(); task.plan.aircraft = Some(cargo);
    let recovery = FlightTaskRecovery::new(task, aircraft(), original.route().clone(), original.time()).unwrap();
    let bunch = FlightTaskBunch::new("cargo-to-passenger".into(), aircraft(), vec![recovery]).unwrap();
    assert!(!FeasibilityJudger.judge_bunch(&bunch, &BunchRecoveryRules { usability: &usability(), previous: None, links: &[] }).unwrap().feasible);
}
