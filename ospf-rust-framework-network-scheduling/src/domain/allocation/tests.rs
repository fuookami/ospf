#![allow(clippy::borrow_interior_mutable_const)]

use std::collections::BTreeMap;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Celsius, Kelvin, Kilogram, Meter, NoneUnit};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use crate::domain::design::{CapacityLevel, NetworkDesignAggregation, NetworkDesignPipeline, NetworkDesignRegistration};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::CapacityBounds;
use super::*;

fn key(request: &str, service: &str) -> AllocationFlowKey { AllocationFlowKey { request: request.into(), service: service.into() } }

pub(crate) fn problem() -> AllocationProblem<f64> {
    let unit = Kilogram::INSTANT.clone();
    let cost_unit = NoneUnit::INSTANT.clone();
    let tariff = (&cost_unit / &unit).build();
    let locations = ["r", "h", "d"].into_iter().enumerate().map(|(layer, id)| {
        let mut location = AllocationLocation::new(id);
        location.layer = Some(layer as i64);
        location
    }).collect();
    let services = [("rh1", "r", "h", 1.0), ("rh2", "r", "h", 1.0), ("hd", "h", "d", 2.0)]
        .into_iter().map(|(id, from, to, capacity)| AllocationService {
            id: id.into(), from: from.into(), to: to.into(), cost: Quantity::new(10.0, tariff.clone()),
            capacity: CapacityBounds::try_new(Quantity::new(0.0, unit.clone()), Quantity::new(capacity, unit.clone())).unwrap(),
            capacity_levels: if id == "rh2" { vec![CapacityLevel { id: "built".into(), capacity: Quantity::new(1.0, unit.clone()),
                cost: Quantity::new(3.0, cost_unit.clone()) }] } else { Vec::new() }, required: false, attributes: BTreeMap::new(),
        }).collect();
    let mut request = AllocationRequest::new("order", "r", "d", Quantity::new(2.0, unit.clone()));
    request.integer_step = Some(Quantity::new(1.0, unit.clone()));
    request.costs = [("rh1", 1.0), ("rh2", 2.0), ("hd", 1.0)].into_iter()
        .map(|(id, value)| (id.into(), Quantity::new(value, tariff.clone()))).collect();
    AllocationProblem::new(locations, services, vec![request], AllocationOptions {
        capacity_unit: unit.clone(), cost_unit: cost_unit.clone(), quotas: vec![AllocationQuota {
            id: "departures".into(), bounds: CapacityBounds::try_new(Quantity::new(0.0, unit.clone()), Quantity::new(2.0, unit.clone())).unwrap(),
            usage: ["rh1", "rh2"].into_iter().map(|id| (key("order", id), Quantity::new(1.0, cost_unit.clone()))).collect(),
        }], hubs: vec![AllocationHub { location: "h".into(), opening_cost: Quantity::new(1.0, cost_unit.clone()),
            required: false, quota_ids: ["departures".into()].into() }], budget: Some(Quantity::new(4.0, cost_unit)),
        ..Default::default()
    }).unwrap()
}

pub(crate) fn values(
    context: &AllocationContext<f64>,
    model: &MetaModel<f64>,
    flow: [f64; 3],
    choices: [f64; 2],
) -> Vec<f64> {
    let aggregation = context.aggregation();
    let mut result = vec![0.0; model.tokens().len()];
    for (id, value) in ["rh1", "rh2", "hd"].into_iter().zip(flow) {
        result[aggregation.flow().aggregation.variable_index("order", &id.into()).unwrap()] = value;
    }
    result[aggregation.construction().facility_variable(&"h".into()).unwrap()] = choices[0];
    result[aggregation.construction().service_variables(&"rh2".into()).unwrap()[0]] = choices[1];
    result
}

#[test]
fn allocation_maps_to_one_kernel_and_matches_split_integer_shipment_oracle() {
    let mut context = AllocationContext::new(problem(), "allocation").unwrap();
    let mut model = MetaModel::new("allocation");
    context.register(&mut model, Default::default()).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert_eq!(model.tokens().len(), 5);
    let mut best = f64::INFINITY;
    let mut feasible = 0;
    for first in 0..=1 {
        for second in 0..=1 {
            for onward in 0..=2 {
                for open in 0..=1 {
                    for built in 0..=1 {
                        let candidate = values(&context, &model, [first as f64, second as f64, onward as f64], [open as f64, built as f64]);
                        let expected = first + second == 2 && onward == 2 && first + second == onward
                            && second <= built && first <= open && second <= open && onward <= 2 * open
                            && first + second <= 2 * open && open + 3 * built <= 4;
                        assert_eq!(evaluate_linear_solution(&compiled, &candidate, 1e-7).unwrap().issues.is_empty(), expected, "{candidate:?}");
                        model.set_solution(&candidate);
                        assert_eq!(context.extract_solution(&model).is_ok(), expected);
                        if expected {
                            feasible += 1;
                            best = best.min((first + 2 * second + onward + open + 3 * built) as f64);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(feasible, 1);
    assert_eq!(best, 9.0);
    model.set_solution(&values(&context, &model, [1.0, 1.0, 2.0], [1.0, 1.0]));
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.shipments[&key("order", "rh1")].value, 1.0);
    assert_eq!(solution.shipments[&key("order", "rh2")].value, 1.0);
    assert_eq!(solution.flow.operating_cost.value, 5.0);
    assert_eq!(solution.flow.resources["departures"].value, 2.0);
    assert_eq!(solution.construction_cost.value, 4.0);
    assert_eq!(solution.total_cost.value, best);
    assert_eq!(solution.open_hubs, ["h".into()].into());
    assert_eq!(solution.service_levels[&"rh2".into()], "built");
}

#[test]
fn heterogeneous_capacity_usage_and_physical_integer_steps_share_the_same_scaling() {
    let capacity = Kilogram::INSTANT.clone();
    let quantity = Meter::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    let locations = vec![AllocationLocation::new("a"), AllocationLocation::new("b")];
    let services = vec![AllocationService {
        id: "ab".into(), from: "a".into(), to: "b".into(), cost: Quantity::new(99.0, (&cost / &capacity).build()),
        capacity: CapacityBounds::try_new(Quantity::new(0.0, capacity.clone()), Quantity::new(6.0, capacity.clone())).unwrap(),
        capacity_levels: Vec::new(), required: false, attributes: BTreeMap::new(),
    }];
    let mut request = AllocationRequest::new("length", "a", "b", Quantity::new(2.0, quantity.clone()));
    request.integer_step = Some(Quantity::new(0.5, quantity.clone()));
    request.costs.insert("ab".into(), Quantity::new(4.0, (&cost / &quantity).build()));
    let mut options = AllocationOptions { capacity_unit: capacity.clone(), cost_unit: cost, ..Default::default() };
    assert!(AllocationProblem::new(locations.clone(), services.clone(), vec![request.clone()], options.clone()).is_err());
    options.service_usage.insert(key("length", "ab"), Quantity::new(3.0, (&capacity / &quantity).build()));
    let problem = AllocationProblem::new(locations, services, vec![request], options).unwrap();
    let mut context = AllocationContext::new(problem, "heterogeneous").unwrap();
    let mut model = MetaModel::new("heterogeneous");
    context.register(&mut model, Default::default()).unwrap();
    assert_eq!(model.tokens().len(), 1);
    assert_eq!(model.try_to_linear_triad_model().unwrap().c[0], 2.0);
    model.set_solution(&[4.0]);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.shipments[&key("length", "ab")].value, 2.0);
    assert_eq!(solution.total_cost.value, 8.0);
    for invalid in [4.1, 5.0, -1.0, f64::NAN, f64::INFINITY] {
        model.set_solution(&[invalid]);
        assert!(context.extract_solution(&model).is_err());
    }
}

#[derive(Debug)]
struct Extension(AtomicBool);

impl NetworkDesignPipeline<f64> for Extension {
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkDesignAggregation<f64>) -> Result<()> {
        let expression = aggregation.flow().aggregation.flow_expression("order", &"rh1".into())?;
        model.add_linear_constraint(&expression, ConstraintRelation::Equal, 1.0, "downstream_shipment")
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        model.add_linear_objective(&[(expression[0].0, 7.0)], "downstream_cost");
        if self.0.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("测试注入失败 / injected test failure")); }
        Ok(())
    }
}

#[test]
fn allocation_reuses_transactional_pipelines_and_preserves_external_objective() {
    let mut context = AllocationContext::new(problem(), "retry").unwrap();
    context.add_pipeline(Arc::new(Extension(AtomicBool::new(true)))).unwrap();
    let mut model = MetaModel::new("retry");
    assert!(context.register(&mut model, Default::default()).is_err());
    assert_eq!(model.tokens().len(), 0);
    assert!(context.aggregation().construction().cost_expression().is_empty());
    context.register(&mut model, Default::default()).unwrap();
    let index = context.aggregation().flow().aggregation.variable_index("order", &"rh1".into()).unwrap();
    assert_eq!(model.try_to_linear_triad_model().unwrap().c[index], 8.0);
    let count = model.tokens().len();
    context.register(&mut model, Default::default()).unwrap();
    assert_eq!(model.tokens().len(), count);
    assert!(context.register(&mut model, NetworkDesignRegistration { minimize_cost: false }).is_err());
    assert!(context.register(&mut MetaModel::new("other"), Default::default()).is_err());
    assert!(context.add_pipeline(Arc::new(Extension(AtomicBool::new(false)))).is_err());
    model.set_solution(&values(&context, &model, [1.0, 1.0, 2.0], [1.0, 1.0]));
    assert_eq!(context.extract_solution(&model).unwrap().total_cost.value, 9.0);
}

#[test]
fn allocation_rejects_duplicate_unknown_incompatible_and_layer_inputs() {
    let original = problem();
    for field in 0..18 {
        let mut locations = original.locations().to_vec();
        let mut services = original.services().to_vec();
        let mut requests = original.requests().to_vec();
        let mut options = original.options().clone();
        match field {
            0 => locations.push(locations[0].clone()),
            1 => services.push(services[0].clone()),
            2 => requests.push(requests[0].clone()),
            3 => services[0].from = "missing".into(),
            4 => locations[0].layer = Some(3),
            5 => requests[0].origin = "missing".into(),
            6 => requests[0].destination = requests[0].origin.clone(),
            7 => requests[0].quantity.value = 0.0,
            8 => requests[0].integer_step.as_mut().unwrap().value = 0.0,
            9 => requests[0].allowed_services = Some(["missing".into()].into()),
            10 => { let cost = requests[0].quantity.clone(); requests[0].costs.insert("missing".into(), cost); }
            11 => options.quotas.push(options.quotas[0].clone()),
            12 => { options.quotas[0].usage.insert(key("missing", "rh1"), Quantity::new(1.0, NoneUnit::INSTANT.clone())); }
            13 => { options.service_usage.insert(key("order", "missing"), Quantity::new(1.0, NoneUnit::INSTANT.clone())); }
            14 => requests[0].allowed_services = Some(["hd".into()].into()),
            15 => options.hubs.push(options.hubs[0].clone()),
            16 => { options.hubs[0].quota_ids.insert("missing".into()); }
            _ => options.hubs[0].location = "missing".into(),
        }
        assert!(AllocationProblem::new(locations, services, requests, options).is_err(), "field={field}");
    }
    let mut options = original.options().clone();
    options.hubs.push(AllocationHub { location: "r".into(), opening_cost: Quantity::new(0.0, options.cost_unit.clone()),
        required: false, quota_ids: ["departures".into()].into() });
    assert!(AllocationProblem::new(original.locations().to_vec(), original.services().to_vec(), original.requests().to_vec(), options).is_err());
}

#[test]
fn allocation_affine_units_never_panic_and_linear_temperature_differences_remain_legal() {
    let original = problem();
    for field in 0..10 {
        let mut locations = original.locations().to_vec();
        let mut services = original.services().to_vec();
        let mut requests = original.requests().to_vec();
        let mut options = original.options().clone();
        match field {
            0 => options.capacity_unit = Celsius::INSTANT.clone(),
            1 => options.cost_unit = Celsius::INSTANT.clone(),
            2 => services[0].capacity.lower.unit = Celsius::INSTANT.clone(),
            3 => services[0].cost.unit = Celsius::INSTANT.clone(),
            4 => requests[0].quantity.unit = Celsius::INSTANT.clone(),
            5 => requests[0].unit = Celsius::INSTANT.clone(),
            6 => options.quotas[0].bounds.lower.unit = Celsius::INSTANT.clone(),
            7 => options.hubs[0].opening_cost.unit = Celsius::INSTANT.clone(),
            8 => { locations[0].attributes.insert("temperature".into(), Quantity::new(1.0, Celsius::INSTANT.clone())); }
            _ => { services[0].attributes.insert("temperature".into(), Quantity::new(1.0, Celsius::INSTANT.clone())); }
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| AllocationProblem::new(locations, services, requests, options)));
        assert!(result.is_ok(), "field={field}");
        assert!(result.unwrap().is_err(), "field={field}");
    }
    let unit = Kelvin::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    let problem = AllocationProblem::new(vec![AllocationLocation::new("a"), AllocationLocation::new("b")], vec![AllocationService {
        id: "ab".into(), from: "a".into(), to: "b".into(), cost: Quantity::new(1.0, (&cost / &unit).build()),
        capacity: CapacityBounds::try_new(Quantity::new(0.0, unit.clone()), Quantity::new(1.0, unit.clone())).unwrap(),
        capacity_levels: Vec::new(), required: false, attributes: BTreeMap::new(),
    }], vec![AllocationRequest::new("temperature_difference", "a", "b", Quantity::new(1.0, unit.clone()))],
        AllocationOptions { capacity_unit: unit, cost_unit: cost, ..Default::default() }).unwrap();
    let mut context = AllocationContext::new(problem, "linear").unwrap();
    let mut model = MetaModel::new("linear");
    context.register(&mut model, Default::default()).unwrap();
    model.set_solution(&[1.0]);
    assert_eq!(context.extract_solution(&model).unwrap().total_cost.value, 1.0);
}

#[cfg(feature = "scip")]
#[test]
fn scip_allocation_service_executes_split_orders_compatibility_and_shared_quotas() {
    use ospf_rust_core::solver::{ProblemStatus, solvers::SCIPSolver};
    use crate::application::AllocationApplicationService;
    let original = problem();
    let service = AllocationApplicationService::new(SCIPSolver::new());
    let report = service.solve(&original).unwrap();
    assert!(report.is_optimal());
    let solution = report.incumbent().unwrap();
    assert_eq!(solution.total_cost.value, 9.0);
    assert_eq!(solution.shipments[&key("order", "rh1")].value, 1.0);
    assert_eq!(solution.shipments[&key("order", "rh2")].value, 1.0);
    let mut requests = original.requests().to_vec();
    requests[0].quantity.value = 1.0;
    let mut second = requests[0].clone();
    second.id = "second".into();
    second.allowed_services = Some(["rh1".into(), "hd".into()].into());
    requests.push(second);
    let mut options = original.options().clone();
    options.quotas[0].usage.insert(key("second", "rh1"), Quantity::new(1.0, NoneUnit::INSTANT.clone()));
    let two = AllocationProblem::new(original.locations().to_vec(), original.services().to_vec(), requests, options.clone()).unwrap();
    let report = service.solve(&two).unwrap();
    assert!(report.is_optimal());
    assert_eq!(report.incumbent().unwrap().total_cost.value, 9.0);
    assert!(!report.incumbent().unwrap().shipments.contains_key(&key("second", "rh2")));
    options.quotas[0].bounds.upper.value = 1.0;
    let limited = AllocationProblem::new(two.locations().to_vec(), two.services().to_vec(), two.requests().to_vec(), options).unwrap();
    let report = service.solve(&limited).unwrap();
    assert_eq!(report.problem_status, ProblemStatus::Infeasible);
    assert!(!report.has_incumbent());
}

#[cfg(any(feature = "big-decimal", feature = "big-rational"))]
fn generic_allocation_roundtrip<V>()
where
    V: ospf_rust_core::solver::value::SolveValue + ospf_rust_quantities::unit::UnitConversionValue,
{
    let adapter = crate::infrastructure::NetworkSchedulingSolverValueAdapter::<V>::default();
    let value = |number| adapter.from_solver(number).unwrap();
    let unit = Kilogram::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    let tariff = (&cost / &unit).build();
    let mut request = AllocationRequest::new("order", "a", "b", Quantity::new(value(2.0), unit.clone()));
    request.integer_step = Some(Quantity::new(value(0.5), unit.clone()));
    request.costs.insert("ab".into(), Quantity::new(value(3.0), tariff.clone()));
    let problem = AllocationProblem::new(vec![AllocationLocation::new("a"), AllocationLocation::new("b")], vec![AllocationService {
        id: "ab".into(), from: "a".into(), to: "b".into(), cost: Quantity::new(value(10.0), tariff),
        capacity: CapacityBounds::try_new(Quantity::new(value(0.0), unit.clone()), Quantity::new(value(2.0), unit.clone())).unwrap(),
        capacity_levels: Vec::new(), required: false, attributes: BTreeMap::new(),
    }], vec![request], AllocationOptions { capacity_unit: unit, cost_unit: cost, ..Default::default() }).unwrap();
    let mut context = AllocationContext::new(problem, "generic").unwrap();
    let mut model = MetaModel::new("generic");
    context.register(&mut model, Default::default()).unwrap();
    model.set_solution(&[4.0]);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(adapter.to_solver(&solution.shipments[&key("order", "ab")].value).unwrap(), 2.0);
    assert_eq!(adapter.to_solver(&solution.total_cost.value).unwrap(), 6.0);
}

#[cfg(feature = "big-decimal")]
#[test]
fn allocation_big_decimal_registers_and_extracts_physical_quantities() { generic_allocation_roundtrip::<bigdecimal::BigDecimal>(); }

#[cfg(feature = "big-rational")]
#[test]
fn allocation_big_rational_registers_and_extracts_physical_quantities() { generic_allocation_roundtrip::<num_rational::BigRational>(); }
