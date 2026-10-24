#![allow(clippy::borrow_interior_mutable_const)]

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Celsius, Kelvin, Kilogram, Kilometer, Meter, NoneUnit, Unit};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{CapacityBounds, NetworkCost};
use super::*;

fn node(id: &str, supply: f64, unit: &Unit) -> GeneralizedFlowNode<f64> {
    GeneralizedFlowNode { id: id.into(), net_supply: Quantity::new(supply, unit.clone()) }
}

fn arc(id: &str, from: &str, to: &str, unit: &Unit) -> GeneralizedFlowArc<f64> {
    let cost = NoneUnit::INSTANT.clone();
    GeneralizedFlowArc {
        id: id.into(), from: from.into(), to: to.into(),
        input_capacity: CapacityBounds::try_new(
            Quantity::new(0.0, unit.clone()), Quantity::new(10.0, unit.clone()),
        ).unwrap(),
        cost: NetworkCost::new(Quantity::new(3.0, (&cost / unit).build())),
        production_yield: Quantity::new(1.0, cost),
        input_to_output_factor: None,
    }
}

fn context(problem: GeneralizedFlowProblem<f64>) -> (GeneralizedFlowContext<f64>, MetaModel<f64>) {
    let mut context = GeneralizedFlowContext::new(problem, "test").unwrap();
    let mut model = MetaModel::new("generalized-flow");
    context.register(&mut model, Default::default()).unwrap();
    (context, model)
}

#[test]
fn physical_conversion_and_yield_are_separate_and_production_extraction_checks_balances() {
    let source = Kilometer::INSTANT.clone();
    let destination = Meter::INSTANT.clone();
    let mut service = arc("service", "source", "destination", &source);
    service.production_yield.value = 0.8;
    let problem = GeneralizedFlowProblem::new(
        vec![node("source", 2.0, &source), node("destination", -1600.0, &destination)],
        vec![service], NoneUnit::INSTANT.clone(),
    ).unwrap();
    let (context, mut model) = context(problem);
    model.set_solution(&[2.0]);
    let solution = context.extract_solution(&model).unwrap();
    let result = &solution.arcs[&"service".into()];
    assert_eq!(result.input.value, 2.0);
    assert!((result.output.value - 1600.0).abs() < 1e-9);
    assert!((result.loss_or_gain.value - 400.0).abs() < 1e-9);
    assert_eq!(solution.total_cost.value, 6.0);
    model.set_solution(&[1.99]);
    assert!(context.extract_solution(&model).is_err());
}

#[test]
fn cross_dimension_requires_explicit_factor_and_normalizes_its_scale() {
    let source = Kilogram::INSTANT.clone();
    let destination = Meter::INSTANT.clone();
    let mut service = arc("service", "a", "b", &source);
    let nodes = vec![node("a", 2.0, &source), node("b", -6.0, &destination)];
    assert!(GeneralizedFlowProblem::new(nodes.clone(), vec![service.clone()], NoneUnit::INSTANT.clone()).is_err());
    service.input_to_output_factor = Some(Quantity::new(0.003, (&Kilometer::INSTANT.clone() / &source).build()));
    let problem = GeneralizedFlowProblem::new(nodes, vec![service], NoneUnit::INSTANT.clone()).unwrap();
    let (context, mut model) = context(problem);
    model.set_solution(&[2.0]);
    assert!((context.extract_solution(&model).unwrap().arcs[&"service".into()].output.value - 6.0).abs() < 1e-9);
}

#[test]
fn illegal_incumbents_are_rejected_by_the_production_entry() {
    let unit = Kilogram::INSTANT.clone();
    let problem = GeneralizedFlowProblem::new(
        vec![node("a", 2.0, &unit), node("b", -2.0, &unit)],
        vec![arc("ab", "a", "b", &unit)], NoneUnit::INSTANT.clone(),
    ).unwrap();
    let (context, mut model) = context(problem);
    assert!(context.extract_solution(&model).is_err());
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1e-12, 10.1, 1.0] {
        model.set_solution(&[invalid]);
        assert!(context.extract_solution(&model).is_err(), "accepted {invalid}");
    }
    model.set_solution(&[2.0]);
    assert!(context.extract_solution(&model).is_ok());
    assert!(context.extract_solution(&MetaModel::new("different")).is_err());
}

#[test]
fn tolerance_uses_actual_net_supply_not_gross_circulating_flow() {
    let unit = Kilogram::INSTANT.clone();
    let mut ab = arc("ab", "a", "b", &unit);
    let mut ba = arc("ba", "b", "a", &unit);
    ab.input_capacity.upper.value = 1e9;
    ba.input_capacity.upper.value = 1e9;
    let problem = GeneralizedFlowProblem::new(
        vec![node("a", 0.0, &unit), node("b", 0.0, &unit)],
        vec![ab, ba], NoneUnit::INSTANT.clone(),
    ).unwrap();
    let (context, mut model) = context(problem);
    model.set_solution(&[1e9, 1e9 - 1.0]);
    assert!(context.extract_solution(&model).is_err());
    model.set_solution(&[1e9, 1e9]);
    assert!(context.extract_solution(&model).is_ok());
    assert_eq!(GeneralizedFlowSolverTolerance::default().for_values(&[2.0]).unwrap(), 3e-7);
    for tolerance in [
        GeneralizedFlowSolverTolerance { absolute: -1.0, relative: 0.0 },
        GeneralizedFlowSolverTolerance { absolute: f64::NAN, relative: 0.0 },
        GeneralizedFlowSolverTolerance { absolute: 0.0, relative: f64::INFINITY },
        GeneralizedFlowSolverTolerance { absolute: f64::MAX, relative: f64::MAX },
    ] {
        assert!(tolerance.for_values(&[2.0]).is_err());
    }
}

#[test]
fn affine_inputs_fail_without_panicking_but_linear_temperature_differences_work() {
    let unit = Kelvin::INSTANT.clone();
    let nodes = vec![node("a", 2.0, &unit), node("b", -2.0, &unit)];
    let valid = arc("ab", "a", "b", &unit);
    for field in 0..7 {
        let mut nodes = nodes.clone();
        let mut invalid = valid.clone();
        let mut cost = NoneUnit::INSTANT.clone();
        match field {
            0 => nodes[0].net_supply.unit = Celsius::INSTANT.clone(),
            1 => invalid.cost.quantity.unit = Celsius::INSTANT.clone(),
            2 => invalid.input_capacity.lower.unit = Celsius::INSTANT.clone(),
            3 => invalid.input_capacity.upper.unit = Celsius::INSTANT.clone(),
            4 => invalid.production_yield.unit = Celsius::INSTANT.clone(),
            5 => invalid.input_to_output_factor = Some(Quantity::new(1.0, Celsius::INSTANT.clone())),
            _ => cost = Celsius::INSTANT.clone(),
        }
        assert!(GeneralizedFlowProblem::new(nodes, vec![invalid], cost).is_err());
    }
    let (context, mut model) = context(GeneralizedFlowProblem::new(nodes, vec![valid], NoneUnit::INSTANT.clone()).unwrap());
    model.set_solution(&[2.0]);
    assert!(context.extract_solution(&model).is_ok());
}

#[derive(Debug)]
struct Downstream {
    fail: AtomicBool,
}

impl GeneralizedFlowPipeline for Downstream {
    fn register(&self, model: &mut MetaModel<f64>, expressions: &GeneralizedFlowExpressions) -> Result<()> {
        model.add_linear_constraint(
            &expressions.outputs[&"ab".into()], ConstraintRelation::LessEqual, 8.0, "downstream",
        ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        model.add_linear_objective(&expressions.cost, "downstream_cost");
        if self.fail.load(Ordering::SeqCst) {
            return Err(NetworkSchedulingError::contract("测试失败 / test failure"));
        }
        Ok(())
    }
}

#[test]
fn downstream_extension_is_transactional_registration_is_bound_and_idempotent() {
    let unit = Kilogram::INSTANT.clone();
    let problem = GeneralizedFlowProblem::new(
        vec![node("a", 2.0, &unit), node("b", -2.0, &unit)],
        vec![arc("ab", "a", "b", &unit)], NoneUnit::INSTANT.clone(),
    ).unwrap();
    let pipeline = Arc::new(Downstream { fail: AtomicBool::new(true) });
    let mut context = GeneralizedFlowContext::new(problem.clone(), "test").unwrap();
    context.add_pipeline(pipeline.clone()).unwrap();
    let mut model = MetaModel::new("extended");
    assert!(context.register(&mut model, Default::default()).is_err());
    assert_eq!(model.tokens().len(), 0);
    assert!(context.aggregation().input_variable(&"ab".into()).is_none());
    pipeline.fail.store(false, Ordering::SeqCst);
    context.register(&mut model, Default::default()).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert_eq!(compiled.c, vec![6.0]);
    let rows = compiled.num_constraints();
    context.register(&mut model, Default::default()).unwrap();
    assert_eq!(model.try_to_linear_triad_model().unwrap().num_constraints(), rows);
    assert!(context.register(&mut model, GeneralizedFlowRegistration { minimize_cost: false }).is_err());
    assert!(context.register(&mut MetaModel::new("foreign"), Default::default()).is_err());
    let mut independent = GeneralizedFlowContext::new(problem, "independent").unwrap();
    independent.register(&mut MetaModel::new("independent"), Default::default()).unwrap();
}

#[test]
fn gain_cycle_parallel_arcs_and_self_loop_match_an_independent_exhaustive_oracle() {
    let unit = Kilogram::INSTANT.clone();
    let mut ab = arc("ab", "a", "b", &unit);
    ab.production_yield.value = 2.0;
    let mut ba = arc("ba", "b", "a", &unit);
    ba.cost.quantity.value = 0.0;
    let mut bs = arc("bs", "b", "sink", &unit);
    bs.cost.quantity.value = 0.0;
    let mut parallel = ab.clone();
    parallel.id = "ab-expensive".into();
    parallel.cost.quantity.value = 5.0;
    let mut self_loop = arc("loop", "a", "a", &unit);
    self_loop.cost.quantity.value = 1.0;
    let mut arcs = vec![ab, ba, bs, parallel, self_loop];
    for arc in &mut arcs { arc.input_capacity.upper.value = 2.0; }
    let problem = GeneralizedFlowProblem::new(
        vec![node("a", 0.0, &unit), node("b", 0.0, &unit), node("sink", -1.0, &unit)],
        arcs, NoneUnit::INSTANT.clone(),
    ).unwrap();
    let (context, mut model) = context(problem);
    let compiled = model.try_to_linear_triad_model().unwrap();
    let mut best = f64::INFINITY;
    for code in 0..243 {
        let mut remainder = code;
        let values = (0..5).map(|_| { let value = (remainder % 3) as f64; remainder /= 3; value }).collect::<Vec<_>>();
        let [ab, ba, bs, parallel, _self_loop] = values.as_slice() else { unreachable!() };
        let oracle = ab + parallel == *ba && ba + bs == 2.0 * (ab + parallel) && *bs == 1.0;
        let feasible = compiled.A.rows.iter().zip(&compiled.b).all(|(row, rhs)|
            row.entries.iter().map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() <= rhs + 1e-9);
        assert_eq!(feasible, oracle, "{values:?}");
        model.set_solution(&values);
        assert_eq!(context.extract_solution(&model).is_ok(), oracle);
        if oracle { best = best.min(3.0 * ab + 5.0 * parallel + values[4]); }
    }
    assert_eq!(best, 3.0);
}

#[test]
fn disconnected_hard_demand_is_model_infeasibility_not_an_input_error() {
    let unit = Kilogram::INSTANT.clone();
    let problem = GeneralizedFlowProblem::new(
        vec![node("isolated", -1.0, &unit)], vec![], NoneUnit::INSTANT.clone(),
    ).unwrap();
    let (context, model) = context(problem);
    assert!(model.try_to_linear_triad_model().is_ok());
    assert!(context.extract_solution(&model).is_err());
}
