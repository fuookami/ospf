#![allow(clippy::borrow_interior_mutable_const)]

use std::collections::BTreeSet;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Celsius, Kilogram, NoneUnit};
use crate::domain::flow::{FlowArc, FlowCommodity, FlowNode, FlowProblem};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{CapacityBounds, NetworkCost};
use super::*;

pub(crate) fn problem(lower: f64, demand: f64) -> NetworkDesignProblem<f64> {
    let unit = Kilogram::INSTANT.clone();
    let tariff = NoneUnit::INSTANT.clone();
    let cost_unit = (&unit * &tariff).build();
    let flow = FlowProblem::new(
        vec![FlowNode::new("a"), FlowNode::new("b")],
        vec![FlowArc::new("ab", "a", "b", CapacityBounds::try_new(
            Quantity::new(lower, unit.clone()), Quantity::new(10.0, unit.clone()),
        ).unwrap(), NetworkCost::new(Quantity::new(2.0, tariff)))],
        vec![FlowCommodity::new("goods").with_balance("a", Quantity::new(demand, unit.clone()))
            .with_balance("b", Quantity::new(-demand, unit.clone()))],
    ).unwrap();
    NetworkDesignProblem::new(flow, vec![NetworkService {
        arc: "ab".into(), required: false, levels: vec![
            CapacityLevel { id: "small".into(), capacity: Quantity::new(3.0, unit.clone()), cost: Quantity::new(1.0, cost_unit.clone()) },
            CapacityLevel { id: "large".into(), capacity: Quantity::new(10.0, unit), cost: Quantity::new(4.0, cost_unit.clone()) },
        ],
    }], vec![NetworkFacility { node: "a".into(), cost: Quantity::new(5.0, cost_unit.clone()), required: false, capacity_rows: BTreeSet::new() }],
        Some(Quantity::new(9.0, cost_unit))).unwrap()
}

fn values(context: &NetworkDesignContext<f64>, facility: f64, levels: [f64; 2], flow: f64) -> Vec<f64> {
    let construction = context.aggregation().construction();
    let mut result = vec![0.0; 4];
    result[construction.facility_variable(&"a".into()).unwrap()] = facility;
    for (index, value) in construction.service_variables(&"ab".into()).unwrap().iter().zip(levels) { result[*index] = value; }
    result[context.aggregation().flow().aggregation.variable_index("goods", &"ab".into()).unwrap()] = flow;
    result
}

#[test]
fn design_matches_independent_enumeration_and_checks_production_results() {
    let mut context = NetworkDesignContext::new(problem(0.0, 2.0), "design").unwrap();
    let mut model = MetaModel::new("design");
    context.register(&mut model, Default::default()).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    let mut best = f64::INFINITY;
    let mut feasible = 0;
    for open in 0..=1 {
        for small in 0..=1 {
            for large in 0..=1 {
                for flow in 0..=10 {
                    let candidate = values(&context, open as f64, [small as f64, large as f64], flow as f64);
                    let expected = flow == 2 && small + large <= 1 && flow <= 3 * small + 10 * large && flow <= 10 * open
                        && 5 * open + small + 4 * large <= 9;
                    let actual = evaluate_linear_solution(&compiled, &candidate, 1e-7).unwrap().issues.is_empty();
                    assert_eq!(actual, expected, "{candidate:?}");
                    model.set_solution(&candidate);
                    assert_eq!(context.extract_solution(&model).is_ok(), expected);
                    if expected { feasible += 1; best = best.min(5.0 * open as f64 + small as f64 + 4.0 * large as f64 + 2.0 * flow as f64); }
                }
            }
        }
    }
    assert_eq!(feasible, 2);
    assert_eq!(best, 10.0);
    for invalid in [f64::NAN, f64::INFINITY, -0.01, 0.5, 1.01] {
        model.set_solution(&values(&context, invalid, [1.0, 0.0], 2.0));
        assert!(context.extract_solution(&model).is_err());
    }
}

#[test]
fn construction_only_does_not_register_nominal_flows_and_optional_lower_is_gated() {
    let mut construction = NetworkConstruction::new(problem(2.0, 0.0), "structure").unwrap();
    let mut model = MetaModel::new("structure-only");
    construction.register(&mut model).unwrap();
    assert_eq!(model.tokens().len(), 3);
    model.set_solution(&[0.0, 0.0, 0.0]);
    assert_eq!(construction.extract_solution(&model).unwrap().cost.value, 0.0);
    let mut context = NetworkDesignContext::new(problem(2.0, 0.0), "design").unwrap();
    let mut model = MetaModel::new("closed");
    context.register(&mut model, Default::default()).unwrap();
    model.set_solution(&values(&context, 0.0, [0.0, 0.0], 0.0));
    assert!(context.extract_solution(&model).is_ok());
    model.set_solution(&values(&context, 1.0, [1.0, 0.0], 0.0));
    assert!(context.extract_solution(&model).is_err());
}

#[derive(Debug)]
struct Extension(AtomicBool);

impl NetworkDesignPipeline<f64> for Extension {
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkDesignAggregation<f64>) -> Result<()> {
        let index = aggregation.construction().facility_variable(&"a".into()).unwrap();
        model.add_linear_constraint(&[(index, 1.0)], ConstraintRelation::Equal, 1.0, "downstream_open")
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        model.add_linear_objective(&[(index, 7.0)], "downstream_cost");
        if self.0.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("测试注入失败 / injected test failure")); }
        Ok(())
    }
}

#[test]
fn transactional_registration_retries_and_preserves_downstream_objective_and_model_binding() {
    let mut context = NetworkDesignContext::new(problem(0.0, 2.0), "design").unwrap();
    context.add_pipeline(Arc::new(Extension(AtomicBool::new(true)))).unwrap();
    let mut model = MetaModel::new("retry");
    assert!(context.register(&mut model, Default::default()).is_err());
    assert_eq!(model.tokens().len(), 0);
    assert!(context.aggregation().construction().cost_expression().is_empty());
    context.register(&mut model, Default::default()).unwrap();
    let count = model.tokens().len();
    context.register(&mut model, Default::default()).unwrap();
    assert_eq!(count, model.tokens().len());
    assert!(context.register(&mut model, NetworkDesignRegistration { minimize_cost: false }).is_err());
    assert!(context.register(&mut MetaModel::new("different"), Default::default()).is_err());
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert_eq!(compiled.c[context.aggregation().construction().facility_variable(&"a".into()).unwrap()], 12.0);
}

#[test]
fn invalid_design_definitions_and_affine_inputs_fail_before_registration_without_panicking() {
    let original = problem(0.0, 2.0);
    for field in 0..8 {
        let mut services = original.services().to_vec();
        let mut facilities = original.facilities().to_vec();
        let mut budget = original.budget().cloned();
        match field {
            0 => services[0].levels[0].capacity.unit = Celsius::INSTANT.clone(),
            1 => services[0].levels[0].cost.unit = Celsius::INSTANT.clone(),
            2 => facilities[0].cost.unit = Celsius::INSTANT.clone(),
            3 => budget.as_mut().unwrap().unit = Celsius::INSTANT.clone(),
            4 => services[0].levels[0].capacity.value = 11.0,
            5 => services.push(services[0].clone()),
            6 => { facilities[0].capacity_rows.insert("unknown".into()); }
            _ => services[0].arc = "unknown".into(),
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| NetworkDesignProblem::new(original.flow().clone(), services, facilities, budget)));
        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }
}

#[test]
fn missing_finite_envelope_is_rejected_instead_of_using_arbitrary_big_m() {
    let original = problem(0.0, 2.0);
    let mut arcs = original.flow().arcs().to_vec();
    arcs[0].capacity.upper = Quantity::new(f64::INFINITY, Kilogram::INSTANT.clone());
    assert!(FlowProblem::new_with_units(original.flow().nodes().to_vec(), arcs, original.flow().commodities().to_vec(), original.flow().units().clone()).is_err());
}

#[test]
fn facility_resource_bounds_are_gated_without_changing_the_shared_resource_kernel() {
    use crate::domain::flow::FlowResource;
    let original = problem(0.0, 0.0);
    let unit = Kilogram::INSTANT.clone();
    let flow = original.flow().clone().with_resources(vec![FlowResource {
        id: "processing".into(), capacity: CapacityBounds::try_new(
            Quantity::new(1.0, unit.clone()), Quantity::new(3.0, unit.clone()),
        ).unwrap(),
        consumptions: [(("goods".into(), "ab".into()), Quantity::new(1.0, (&unit / &unit).build()))].into_iter().collect(),
    }]).unwrap();
    let mut facilities = original.facilities().to_vec();
    facilities[0].capacity_rows.insert("processing".into());
    let problem = NetworkDesignProblem::new(flow, original.services().to_vec(), facilities, None).unwrap();
    let mut context = NetworkDesignContext::new(problem, "resource").unwrap();
    let mut model = MetaModel::new("facility-resource");
    context.register(&mut model, Default::default()).unwrap();
    model.set_solution(&values(&context, 0.0, [0.0, 0.0], 0.0));
    assert!(context.extract_solution(&model).is_ok());
    model.set_solution(&values(&context, 1.0, [0.0, 0.0], 0.0));
    assert!(context.extract_solution(&model).is_err());
}
