//! 真实恢复记录、策略配额和跨调用状态回归。 / Actual recovery records, strategy quotas and cross-call state regressions.

use std::collections::HashMap;
use time::{Duration, macros::datetime};
use crate::framework::demo4::domain::task::model::{AircraftCapacity, AircraftMinorType, AircraftType, Airport, AirportType,
    FlightTaskBunch, FlightTaskCategory, FlightTaskImpl, FlightTaskPlanImpl, FlightTaskRecovery, FlightTaskTypeImpl, Route};
use crate::framework::demo4::infrastructure::{AircraftMinorTypeCode, AircraftRegisterNumber, AircraftTypeCode, Icao};
use super::*;

fn aircraft(id: &str) -> Aircraft {
    Aircraft { reg_no: AircraftRegisterNumber(id.into()), minor_type: AircraftMinorType {
        aircraft_type: AircraftType { code: AircraftTypeCode("A320".into()) }, code: AircraftMinorTypeCode("A320-200".into()),
        cost_per_hour: 10.0, route_fly_time: HashMap::new(), connection_time: HashMap::new(), max_fly_time: None,
    }, capacity: AircraftCapacity::Passenger { capacities: vec![] } }
}

fn candidate(id: &str, cost: f64, changes: usize) -> RecoveryBunchCandidate {
    let executor = aircraft(id);
    let airport = Airport { icao: Icao("A".into()), airport_type: AirportType::Domestic,
        passenger_transfer_time: Duration::ZERO, cargo_transfer_time: Duration::ZERO, base: false };
    let activities = (0..changes).map(|index| {
        let start = datetime!(2026-06-07 08:00 UTC) + Duration::hours(index as i64);
        FlightTaskRecovery::new(FlightTaskImpl::new(FlightTaskPlanImpl {
            id: format!("{id}-{index}"), name: "changed".into(), aircraft: Some(aircraft("ORIGINAL")),
            dep: airport.clone(), arr: airport.clone(), time: None, scheduled_time: None, flight_task_status: vec![],
        }, FlightTaskTypeImpl { category: FlightTaskCategory::Flight, name: "flight".into() }), executor.clone(),
            Route { dep: airport.clone(), arr: airport.clone() }, (start, start + Duration::minutes(30))).unwrap()
    }).collect();
    RecoveryBunchCandidate { bunch: FlightTaskBunch::new(id.into(), executor, activities).unwrap(), cost }
}

fn none() -> FreeAircraftSelectorConfiguration {
    FreeAircraftSelectorConfiguration { bad_reduced_amount: 0, high_cost_amount: 0, high_aircraft_change_amount: 0,
        random_amount: 0, ..Default::default() }
}

fn select(
    selector: &mut FreeAircraftSelector,
    fixed: &[RecoveryBunchCandidate],
    prices: &[f64],
    hidden: &[&str],
) -> GanttResult<BTreeSet<String>> {
    let aircraft = fixed.iter().map(|candidate| candidate.bunch.aircraft().clone()).collect::<Vec<_>>();
    let prices = fixed.iter().zip(prices).map(|(candidate, price)| (candidate.bunch.aircraft().reg_no.0.clone(), *price)).collect();
    selector.select(&FreeAircraftSelectionRequest { fixed_bunches: fixed, aircraft: &aircraft,
        hidden_aircraft: &hidden.iter().map(|id| id.to_string()).collect(), reduced_costs: &prices })
}

fn ids(ids: &[&str]) -> BTreeSet<String> { ids.iter().map(|id| id.to_string()).collect() }

#[test]
fn default_quotas_match_snapshot_and_nonfinite_threshold_is_rejected() {
    let default = FreeAircraftSelectorConfiguration::default();
    assert_eq!((default.bad_reduced_amount, default.high_cost_amount, default.high_aircraft_change_amount, default.random_amount), (3, 3, 3, 3));
    assert_eq!((default.tabu_amount, default.fix_bar, default.random_seed), (10, 0.0, 42));
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FreeAircraftSelector::new(FreeAircraftSelectorConfiguration { fix_bar: value, ..default.clone() }).is_err());
    }
}

#[test]
fn bad_cost_high_cost_and_actual_changes_have_separate_ordered_quotas() {
    let fixed = vec![candidate("A", 20.0, 0), candidate("B", 90.0, 0), candidate("C", 30.0, 2), candidate("D", 0.0, 1)];
    let mut selector = FreeAircraftSelector::new(FreeAircraftSelectorConfiguration {
        bad_reduced_amount: 1, high_cost_amount: 1, high_aircraft_change_amount: 1, ..none()
    }).unwrap();
    assert_eq!(select(&mut selector, &fixed, &[10.0, 0.0, -1.0, 0.0], &[]).unwrap(), ids(&["A", "B", "C"]));
    assert_eq!(selector.tabu, VecDeque::from(["B".to_string(), "C".to_string()]));
    assert_eq!(select(&mut selector, &fixed, &[10.0, 0.0, -1.0, 0.0], &[]).unwrap(), ids(&["A", "D"]));
    let mut strict = FreeAircraftSelector::new(FreeAircraftSelectorConfiguration { bad_reduced_amount: 1, fix_bar: 10.0, ..none() }).unwrap();
    assert!(select(&mut strict, &fixed, &[10.0, 0.0, -1.0, 0.0], &[]).unwrap().is_empty());
}

#[test]
fn bad_reduced_cost_bypasses_tabu_but_other_strategies_respect_it() {
    let fixed = vec![candidate("A", 50.0, 0), candidate("B", 10.0, 1)];
    let mut selector = FreeAircraftSelector::new(FreeAircraftSelectorConfiguration { bad_reduced_amount: 1, high_cost_amount: 1, ..none() }).unwrap();
    assert_eq!(select(&mut selector, &fixed, &[0.0, 0.0], &[]).unwrap(), ids(&["A"]));
    assert_eq!(select(&mut selector, &fixed, &[1.0, 0.0], &[]).unwrap(), ids(&["A", "B"]));
}

#[test]
fn zero_quotas_empty_pool_and_hidden_aircraft_do_not_create_phantom_releases() {
    let fixed = vec![candidate("A", 50.0, 0), candidate("B", 10.0, 1)];
    let mut selector = FreeAircraftSelector::new(none()).unwrap();
    assert!(select(&mut selector, &fixed, &[1.0, 1.0], &[]).unwrap().is_empty());
    assert_eq!(select(&mut selector, &fixed, &[1.0, 1.0], &["B"]).unwrap(), ids(&["B"]));
    let mut selector = FreeAircraftSelector::new(Default::default()).unwrap();
    assert!(select(&mut selector, &[], &[], &[]).unwrap().is_empty());
    let aircraft = [aircraft("UNUSED")];
    assert_eq!(selector.select(&FreeAircraftSelectionRequest { fixed_bunches: &[], aircraft: &aircraft,
        hidden_aircraft: &ids(&["UNUSED"]), reduced_costs: &BTreeMap::new() }).unwrap(), ids(&["UNUSED"]));
}

#[test]
fn tabu_fifo_eviction_and_zero_capacity_are_safe_and_effective() {
    let fixed = vec![candidate("A", 50.0, 0), candidate("B", 10.0, 0)];
    let mut selector = FreeAircraftSelector::new(FreeAircraftSelectorConfiguration { high_cost_amount: 1, tabu_amount: 1, ..none() }).unwrap();
    for expected in ["A", "B", "A"] {
        assert_eq!(select(&mut selector, &fixed, &[0.0; 2], &[]).unwrap(), ids(&[expected]));
    }
    let mut selector = FreeAircraftSelector::new(FreeAircraftSelectorConfiguration { high_cost_amount: 1, tabu_amount: 0, ..none() }).unwrap();
    for _ in 0..3 { assert_eq!(select(&mut selector, &fixed, &[0.0; 2], &[]).unwrap(), ids(&["A"])); }
    assert!(selector.tabu.is_empty());
}

#[test]
fn random_sampling_is_reproducible_without_replacement_and_handles_all_seeds() {
    let fixed = (0..8).map(|index| candidate(&index.to_string(), 0.0, 0)).collect::<Vec<_>>();
    let configuration = FreeAircraftSelectorConfiguration { random_amount: 3, ..none() };
    let mut left = FreeAircraftSelector::new(configuration.clone()).unwrap();
    let mut right = FreeAircraftSelector::new(configuration).unwrap();
    for expected in [3, 3, 2, 0] {
        let chosen = select(&mut left, &fixed, &[0.0; 8], &[]).unwrap();
        assert_eq!(chosen.len(), expected);
        assert_eq!(chosen, select(&mut right, &fixed, &[0.0; 8], &[]).unwrap());
    }
    for seed in [0, u64::MAX] {
        let mut selector = FreeAircraftSelector::new(FreeAircraftSelectorConfiguration { random_amount: usize::MAX, random_seed: seed, ..none() }).unwrap();
        assert_eq!(select(&mut selector, &fixed, &[0.0; 8], &[]).unwrap().len(), 8);
    }
}

#[test]
fn invalid_prices_and_references_fail_without_changing_random_or_tabu_state() {
    let fixed = vec![candidate("A", 1.0, 0), candidate("B", 2.0, 1)];
    let mut selector = FreeAircraftSelector::new(Default::default()).unwrap();
    let aircraft = fixed.iter().map(|candidate| candidate.bunch.aircraft().clone()).collect::<Vec<_>>();
    for prices in [BTreeMap::new(), BTreeMap::from([("A".into(), 0.0)]),
        BTreeMap::from([("A".into(), f64::NAN), ("B".into(), 0.0)]),
        BTreeMap::from([("A".into(), 0.0), ("B".into(), f64::INFINITY)]),
        BTreeMap::from([("A".into(), 0.0), ("B".into(), 0.0), ("unknown".into(), 0.0)])]
    {
        assert!(selector.select(&FreeAircraftSelectionRequest { fixed_bunches: &fixed, aircraft: &aircraft,
            hidden_aircraft: &BTreeSet::new(), reduced_costs: &prices }).is_err());
    }
    assert!(select(&mut selector, &fixed, &[0.0; 2], &["unknown"]).is_err());
    let mut invalid = fixed.clone();
    invalid[0].cost = f64::NEG_INFINITY;
    assert!(select(&mut selector, &invalid, &[0.0; 2], &[]).is_err());
    assert!(select(&mut selector, &[fixed[0].clone(), fixed[0].clone()], &[0.0; 2], &[]).is_err());
    let mut conflict = aircraft.clone();
    conflict[0].minor_type.cost_per_hour += 1.0;
    assert!(selector.select(&FreeAircraftSelectionRequest { fixed_bunches: &fixed, aircraft: &conflict,
        hidden_aircraft: &BTreeSet::new(), reduced_costs: &BTreeMap::from([("A".into(), 0.0), ("B".into(), 0.0)]) }).is_err());
    assert!(selector.tabu.is_empty());
    assert_eq!(selector.random, selector.configuration().random_seed);
    assert_eq!(select(&mut selector, &fixed, &[0.0; 2], &[]).unwrap(),
        select(&mut FreeAircraftSelector::new(Default::default()).unwrap(), &fixed, &[0.0; 2], &[]).unwrap());
}
