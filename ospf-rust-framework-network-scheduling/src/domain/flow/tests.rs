#![allow(clippy::borrow_interior_mutable_const)]

use std::collections::{HashMap, HashSet};
use ospf_rust_core::model::MetaModel;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Kilogram, Meter, NoneUnit, Unit};
use crate::infrastructure::{CapacityBounds, NetworkArcId, NetworkCost};
use super::*;

fn bounds(upper: f64, unit: &Unit) -> CapacityBounds<f64> {
    CapacityBounds::try_new(Quantity::new(0.0, unit.clone()), Quantity::new(upper, unit.clone())).unwrap()
}

fn cost(value: f64, unit: &Unit) -> NetworkCost<f64> {
    NetworkCost::new(Quantity::new(value, unit.clone()))
}

fn overflowing_problem(case: usize) -> FlowProblem<f64> {
    let unit = NoneUnit::INSTANT.clone();
    let mut invalid = FlowCommodity::new("invalid");
    invalid.integer_step = Some(Quantity::new(if case == 3 { 1.0 } else { 1e200 }, unit.clone()));
    invalid.capacity_consumption.insert("ab".into(), Quantity::new(match case {
        0 => 1e200,
        3 => 1e-200,
        _ => 1.0,
    }, unit.clone()));
    let problem = FlowProblem::new(vec![FlowNode::new("a"), FlowNode::new("b")],
        vec![FlowArc::new("ab", "a", "b", bounds(1e200, &unit), cost(if case == 2 { 1e200 } else { 0.0 }, &unit))],
        vec![FlowCommodity::new("valid-first"), invalid]).unwrap();
    if case == 1 {
        problem.with_resources(vec![FlowResource { id: "shared".into(), capacity: bounds(1e200, &unit),
            consumptions: HashMap::from([(("invalid".into(), "ab".into()), Quantity::new(1e200, unit))]),
        }]).unwrap()
    } else { problem }
}

#[test]
fn derived_capacity_resource_cost_and_bound_overflow_fail_before_any_registration() {
    use ospf_rust_core::variable::{Continuous, VariableRange};

    for case in 0..4 {
        for minimize_cost in [false, true] {
            let problem = overflowing_problem(case);
            let mut context = FlowContext::new(problem.clone());
            let mut model = MetaModel::new("overflow-rollback");
            let existing = model.register_auto_variable_with_range::<Continuous>("existing", VariableRange::bounded(0.0, 1.0)).unwrap();
            model.add_linear_objective(&[(existing, 7.0)], "external");
            let before = model.try_to_linear_triad_model().unwrap();
            assert!(context.register_with_options(&mut model, FlowRegistration { minimize_cost }).is_err(), "case {case}");
            assert_eq!(model.tokens().len(), 1);
            assert!(model.constraints().is_empty());
            assert_eq!(model.try_to_linear_triad_model().unwrap().c, before.c);
            assert!(context.aggregation.variable_indices().is_empty());
            assert!(context.aggregation.slack_indices().is_empty());
            assert!(context.register_variables(&mut model).is_err());
            assert!(context.aggregation.variable_indices().is_empty());
            let mut aggregation = FlowAggregation::new(problem);
            assert!(aggregation.register(&mut model).is_err());
            assert!(aggregation.variable_indices().is_empty());
            assert_eq!(model.tokens().len(), 1);
            assert!(model.constraints().is_empty());
        }
    }
}

#[test]
fn embedded_design_resilience_and_allocation_reject_overflow_without_partial_state() {
    use crate::domain::allocation::{AllocationContext, AllocationLocation, AllocationOptions, AllocationProblem, AllocationRequest, AllocationService};
    use crate::domain::design::{NetworkDesignContext, NetworkDesignProblem};
    use crate::domain::resilience::{NetworkResilienceContext, NetworkResilienceProblem};

    for case in 0..4 {
        let design = NetworkDesignProblem::new(overflowing_problem(case), Vec::new(), Vec::new(), None).unwrap();
        let mut context = NetworkDesignContext::new(design.clone(), "design").unwrap();
        let mut model = MetaModel::new("embedded");
        assert!(context.register(&mut model, Default::default()).is_err());
        assert!(model.tokens().is_empty());
        assert!(model.constraints().is_empty());
        assert!(context.register(&mut model, Default::default()).is_err());
        let mut context = NetworkResilienceContext::new(
            NetworkResilienceProblem::new(design, Vec::new(), Vec::new()).unwrap(), "resilience",
        ).unwrap();
        assert!(context.register(&mut model).is_err());
        assert!(model.tokens().is_empty());
        assert!(model.constraints().is_empty());
    }
    let unit = NoneUnit::INSTANT.clone();
    let mut request = AllocationRequest::new("m", "a", "b", Quantity::new(1.0, unit.clone()));
    request.integer_step = Some(Quantity::new(1e200, unit.clone()));
    let mut options = AllocationOptions::default();
    options.service_usage.insert(crate::domain::allocation::AllocationFlowKey { request: "m".into(), service: "ab".into() },
        Quantity::new(1e200, unit.clone()));
    let problem = AllocationProblem::new(vec![AllocationLocation::new("a"), AllocationLocation::new("b")], vec![AllocationService {
        id: "ab".into(), from: "a".into(), to: "b".into(), cost: Quantity::new(0.0, unit.clone()), capacity: bounds(1e200, &unit),
        capacity_levels: Vec::new(), required: false, attributes: Default::default(),
    }], vec![request], options).unwrap();
    let mut context = AllocationContext::new(problem, "allocation").unwrap();
    let mut model = MetaModel::new("allocation-overflow");
    assert!(context.register(&mut model, Default::default()).is_err());
    assert!(model.tokens().is_empty());
    assert!(model.constraints().is_empty());
}

#[test]
fn finite_large_step_coefficients_are_reused_by_rows_and_downstream_expressions() {
    let unit = NoneUnit::INSTANT.clone();
    let mut commodity = FlowCommodity::new("m")
        .with_balance("a", Quantity::new(1e200, unit.clone()))
        .with_balance("b", Quantity::new(-1e200, unit.clone()));
    commodity.integer_step = Some(Quantity::new(1e200, unit.clone()));
    let problem = FlowProblem::new(vec![FlowNode::new("a"), FlowNode::new("b")],
        vec![FlowArc::new("ab", "a", "b", bounds(1e200, &unit), cost(0.0, &unit))], vec![commodity]).unwrap()
        .with_resources(vec![FlowResource { id: "shared".into(), capacity: bounds(1e200, &unit),
            consumptions: HashMap::from([(("m".into(), "ab".into()), Quantity::new(1.0, unit))]),
        }]).unwrap();
    let mut context = FlowContext::new(problem);
    let mut model = MetaModel::new("large-but-finite");
    context.register(&mut model).unwrap();
    assert_eq!(context.aggregation.flow_expression("m", &"ab".into()).unwrap(), vec![(0, 1e200)]);
    assert_eq!(context.aggregation.capacity_expression(&"ab".into()).unwrap(), vec![(0, 1e200)]);
    assert_eq!(context.aggregation.resource_expression("shared").unwrap(), vec![(0, 1e200)]);
    assert_eq!(context.aggregation.cost_expression().unwrap(), vec![(0, 0.0)]);
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert_eq!(compiled.ub, vec![1.0]);
    assert!(compiled.A.rows.iter().flat_map(|row| &row.entries).all(|(_, value)| value.is_finite()));
    model.set_solution(&[1.0]);
    assert_eq!(context.extract_solution(&model).unwrap().flows[&("m".into(), "ab".into())].value, 1e200);
}

#[test]
fn heterogeneous_commodities_steps_cost_overrides_and_resources_match_oracle() {
    let capacity_unit = Kilogram::INSTANT.clone();
    let length = Meter::INSTANT.clone();
    let currency = NoneUnit::INSTANT.clone();
    let default_tariff = (&currency / &capacity_unit).build();
    let length_tariff = (&currency / &length).build();
    let mut heavy = FlowCommodity::new("heavy")
        .with_balance("a", Quantity::new(1.0, capacity_unit.clone()))
        .with_balance("b", Quantity::new(-1.0, capacity_unit.clone()));
    heavy.integer_step = Some(Quantity::new(0.5, capacity_unit.clone()));
    let mut long = FlowCommodity::new("long")
        .with_balance("a", Quantity::new(2.0, length.clone()))
        .with_balance("b", Quantity::new(-2.0, length.clone()));
    long.unit = Some(length.clone());
    long.integer_step = Some(Quantity::new(1.0, length.clone()));
    long.allowed_arcs = Some(HashSet::from([NetworkArcId::from("slow")]));
    long.capacity_consumption.insert("slow".into(), Quantity::new(2.0, (&capacity_unit / &length).build()));
    long.costs.insert("slow".into(), cost(4.0, &length_tariff));
    let arcs = vec![
        FlowArc::new("fast", "a", "b", bounds(2.0, &capacity_unit), cost(1.0, &default_tariff)),
        FlowArc::new("slow", "a", "b", bounds(6.0, &capacity_unit), cost(3.0, &default_tariff)),
    ];
    let resource_unit = Meter::INSTANT.clone();
    let resource = FlowResource {
        id: "handling".into(), capacity: bounds(4.0, &resource_unit),
        consumptions: HashMap::from([
            (("heavy".into(), "fast".into()), Quantity::new(2.0, (&resource_unit / &capacity_unit).build())),
            (("heavy".into(), "slow".into()), Quantity::new(2.0, (&resource_unit / &capacity_unit).build())),
            (("long".into(), "slow".into()), Quantity::new(1.0, (&resource_unit / &length).build())),
        ]),
    };
    let problem = FlowProblem::new_with_units(
        vec![FlowNode::new("a"), FlowNode::new("b")], arcs, vec![heavy, long],
        FlowUnits::new(capacity_unit.clone(), default_tariff).unwrap(),
    ).unwrap().with_resources(vec![resource]).unwrap();
    let oracle = solve_bounded_integral_min_cost_flow(&problem).unwrap().unwrap();
    assert_eq!(oracle.objective, 9.0);
    assert_eq!(oracle.flows[&("heavy".into(), "fast".into())], 2);
    assert_eq!(oracle.flows[&("long".into(), "slow".into())], 2);
    let mut context = FlowContext::new(problem);
    let mut model = MetaModel::new("heterogeneous");
    context.register(&mut model).unwrap();
    let mut values = vec![0.0; model.tokens().len()];
    for ((commodity, arc), count) in oracle.flows {
        let index = context.aggregation.variable_index(commodity.as_str(), &arc).unwrap();
        values[index] = count as f64;
    }
    model.set_solution(&values);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.total_cost.value, 9.0);
    assert_eq!(solution.resources["handling"].value, 4.0);
    assert_eq!(solution.flows[&("heavy".into(), "fast".into())].value, 1.0);
    assert!(context.aggregation.variable_index("long", &"fast".into()).is_none());
    let index = context.aggregation.variable_index("heavy", &"fast".into()).unwrap();
    assert_eq!(model.try_to_linear_triad_model().unwrap().c[index], 0.5);
    values[index] = 2.1;
    model.set_solution(&values);
    assert!(context.extract_solution(&model).is_err());
}

#[test]
fn bounded_spill_and_shortage_are_explicit_and_have_independent_costs() {
    let unit = Kilogram::INSTANT.clone();
    let currency = NoneUnit::INSTANT.clone();
    let tariff = (&currency / &unit).build();
    let mut commodity = FlowCommodity::new("m")
        .with_balance("a", Quantity::new(3.0, unit.clone()))
        .with_balance("b", Quantity::new(-4.0, unit.clone()));
    for (node, spill, shortage) in [("a", 1.0, 0.0), ("b", 0.0, 2.0)] {
        commodity.slacks.insert(node.into(), FlowBalanceSlack {
            spill_limit: Quantity::new(spill, unit.clone()),
            shortage_limit: Quantity::new(shortage, unit.clone()),
            spill_cost: cost(5.0, &tariff), shortage_cost: cost(7.0, &tariff),
        });
    }
    let problem = FlowProblem::new_with_units(
        vec![FlowNode::new("a"), FlowNode::new("b")],
        vec![FlowArc::new("ab", "a", "b", bounds(2.0, &unit), cost(1.0, &tariff))],
        vec![commodity], FlowUnits::new(unit, tariff).unwrap(),
    ).unwrap();
    assert_eq!(solve_bounded_integral_min_cost_flow(&problem).unwrap().unwrap().objective, 21.0);
    let mut context = FlowContext::new(problem);
    let mut model = MetaModel::new("soft-flow");
    context.register(&mut model).unwrap();
    let mut values = vec![0.0; model.tokens().len()];
    values[context.aggregation.variable_index("m", &"ab".into()).unwrap()] = 2.0;
    let (a_spill, _) = context.aggregation.slack_indices()[&("m".into(), "a".into())];
    let (_, b_shortage) = context.aggregation.slack_indices()[&("m".into(), "b".into())];
    values[a_spill] = 1.0;
    values[b_shortage] = 2.0;
    model.set_solution(&values);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.operating_cost.value, 2.0);
    assert_eq!(solution.spill_cost.value, 5.0);
    assert_eq!(solution.shortage_cost.value, 14.0);
    assert_eq!(solution.total_cost.value, 21.0);
    values[b_shortage] = 2.1;
    model.set_solution(&values);
    assert!(context.extract_solution(&model).is_err());
}

#[test]
fn shared_resource_can_make_an_otherwise_feasible_flow_infeasible() {
    let unit = Kilogram::INSTANT.clone();
    let tariff = NoneUnit::INSTANT.clone();
    let dimensionless = (&unit / &unit).build();
    let commodity = FlowCommodity::new("m")
        .with_balance("a", Quantity::new(2.0, unit.clone()))
        .with_balance("b", Quantity::new(-2.0, unit.clone()));
    let problem = FlowProblem::new_with_units(
        vec![FlowNode::new("a"), FlowNode::new("b")],
        vec![FlowArc::new("ab", "a", "b", bounds(3.0, &unit), cost(1.0, &tariff))],
        vec![commodity], FlowUnits::new(unit.clone(), tariff).unwrap(),
    ).unwrap().with_resources(vec![FlowResource {
        id: "dock".into(), capacity: bounds(1.0, &unit),
        consumptions: HashMap::from([(("m".into(), "ab".into()), Quantity::new(1.0, dimensionless))]),
    }]).unwrap();
    assert!(solve_bounded_integral_min_cost_flow(&problem).unwrap().is_none());
    let mut context = FlowContext::new(problem);
    let mut model = MetaModel::new("resource-infeasible");
    context.register(&mut model).unwrap();
    model.set_solution(&[2.0]);
    assert!(context.extract_solution(&model).is_err());
    let mut unregistered = FlowContext::new(context.problem().clone());
    assert!(unregistered.extract_solution(&model).is_err());
}

#[test]
fn node_processing_capacity_uses_the_same_resource_rows_and_oracle() {
    let unit = Kilogram::INSTANT.clone();
    let tariff = NoneUnit::INSTANT.clone();
    let problem = FlowProblem::new_with_units(
        vec![FlowNode::new("a"), FlowNode::new("b"), FlowNode::new("c")],
        vec![
            FlowArc::new("ab", "a", "b", bounds(3.0, &unit), cost(1.0, &tariff)),
            FlowArc::new("bc", "b", "c", bounds(3.0, &unit), cost(1.0, &tariff)),
        ],
        vec![FlowCommodity::new("m")
            .with_balance("a", Quantity::new(2.0, unit.clone()))
            .with_balance("c", Quantity::new(-2.0, unit.clone()))],
        FlowUnits::new(unit.clone(), tariff).unwrap(),
    ).unwrap();
    for direction in [FlowThroughput::Incoming, FlowThroughput::Outgoing] {
        let constrained = problem.clone().with_node_capacities(vec![FlowNodeCapacity {
            id: "processing".into(), node: "b".into(), direction,
            capacity: bounds(1.0, &unit), consumptions: HashMap::new(),
        }]).unwrap();
        assert!(solve_bounded_integral_min_cost_flow(&constrained).unwrap().is_none());
        let mut context = FlowContext::new(constrained);
        let mut model = MetaModel::new("node-processing");
        context.register(&mut model).unwrap();
        model.set_solution(&[2.0, 2.0]);
        assert!(context.extract_solution(&model).is_err());
    }
}
