#![allow(clippy::borrow_interior_mutable_const)]

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Celsius, Kilogram};
use crate::domain::design::tests::problem;
use crate::error::{NetworkSchedulingError, Result};
use super::*;

fn degraded(lower: f64, demand: f64, upper: f64) -> NetworkResilienceProblem<f64> {
    let design = problem(lower, demand);
    let mut scenario = FailureScenario::new("degraded");
    scenario.upper_capacities.insert("ab".into(), Quantity::new(upper, Kilogram::INSTANT.clone()));
    NetworkResilienceProblem::new(design, vec![scenario], Vec::new()).unwrap()
}

fn values(context: &NetworkResilienceContext<f64>, capacity: usize, flow: f64, epigraph: f64) -> Vec<f64> {
    let aggregation = context.aggregation();
    // 一个设施、两个等级、一个费用包络、两个独立流。 / One facility, two levels, one cost epigraph and two independent flows.
    let mut values = vec![0.0; 6];
    values[aggregation.construction().facility_variable(&"a".into()).unwrap()] = 1.0;
    values[aggregation.construction().service_variables(&"ab".into()).unwrap()[capacity]] = 1.0;
    for id in ["nominal", "degraded"] {
        let index = aggregation.flow(id).unwrap().aggregation.variable_index("goods", &"ab".into()).unwrap();
        values[index] = flow;
    }
    values[3] = epigraph;
    values
}

#[test]
fn large_built_level_cannot_relax_scenario_capacity_and_production_extraction_checks_it() {
    let mut context = NetworkResilienceContext::new(degraded(0.0, 2.0, 2.0), "resilience").unwrap();
    let mut model = MetaModel::new("degraded");
    context.register(&mut model).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    let valid = values(&context, 1, 2.0, 4.0);
    assert!(evaluate_linear_solution(&compiled, &valid, 1e-7).unwrap().issues.is_empty());
    model.set_solution(&valid);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.construction.service_levels[&"ab".into()], "large");
    assert_eq!(solution.effective_capacities["nominal"][&"ab".into()].value, 10.0);
    assert_eq!(solution.effective_capacities["degraded"][&"ab".into()].value, 2.0);
    assert_eq!(solution.total_cost.value, 13.0);
    let mut invalid = valid.clone();
    invalid[context.aggregation().flow("degraded").unwrap().aggregation.variable_index("goods", &"ab".into()).unwrap()] = 3.0;
    model.set_solution(&invalid);
    assert!(context.extract_solution(&model).is_err());
    for epigraph in [f64::NAN, f64::INFINITY, 3.0] {
        model.set_solution(&values(&context, 1, 2.0, epigraph));
        assert!(context.extract_solution(&model).is_err());
    }
    model.set_solution(&values(&context, 1, 2.0, 5.0));
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.worst_operating_cost.value, 4.0);
    assert_eq!(solution.operating_cost_epigraph.value, 5.0);
}

#[test]
fn degradation_below_hard_lower_registers_normally_and_has_no_feasible_incumbent() {
    let mut context = NetworkResilienceContext::new(degraded(3.0, 3.0, 2.0), "infeasible").unwrap();
    let mut model = MetaModel::new("hard-lower");
    context.register(&mut model).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    for capacity in 0..=1 {
        for flow in 0..=10 {
            let values = values(&context, capacity, flow as f64, 20.0);
            assert!(!evaluate_linear_solution(&compiled, &values, 1e-7).unwrap().issues.is_empty());
            model.set_solution(&values);
            assert!(context.extract_solution(&model).is_err());
        }
    }
}

#[test]
fn failed_nodes_keep_demand_and_explicit_balance_override_is_required() {
    let design = problem(0.0, 2.0);
    let mut scenario = FailureScenario::new("failure");
    scenario.failed_nodes.insert("b".into());
    let original = NetworkResilienceProblem::new(design.clone(), vec![scenario.clone()], Vec::new()).unwrap();
    let mut context = NetworkResilienceContext::new(original, "failed").unwrap();
    let mut model = MetaModel::new("failed");
    context.register(&mut model).unwrap();
    assert_eq!(context.aggregation().flow("failure").unwrap().problem().commodities()[0].balances[&"b".into()].value, -2.0);
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert!(compiled.b.iter().any(|rhs| *rhs == -2.0));
    scenario.balances.insert("goods".into(), Default::default());
    let problem = NetworkResilienceProblem::new(design, vec![scenario], Vec::new()).unwrap();
    let context = NetworkResilienceContext::new(problem, "override").unwrap();
    assert!(context.aggregation().flow("failure").unwrap().problem().commodities()[0].balances.is_empty());
}

#[derive(Debug)]
struct Extension(AtomicBool);

impl NetworkResiliencePipeline<f64> for Extension {
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkResilienceAggregation<f64>) -> Result<()> {
        let terms = aggregation.flow("degraded")?.aggregation.capacity_expression(&"ab".into())?;
        model.add_linear_constraint(&terms, ConstraintRelation::LessEqual, 2.0, "business_scenario_limit")
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        model.add_linear_objective(aggregation.construction().cost_expression(), "downstream_construction_penalty");
        if self.0.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("注入失败 / injected failure")); }
        Ok(())
    }
}

#[test]
fn recourse_is_transactional_and_public_expressions_support_downstream_rules() {
    let mut context = NetworkResilienceContext::new(degraded(0.0, 2.0, 2.0), "retry").unwrap();
    context.add_pipeline(Arc::new(Extension(AtomicBool::new(true)))).unwrap();
    let mut model = MetaModel::new("retry");
    assert!(context.register(&mut model).is_err());
    assert!(model.tokens().is_empty());
    assert!(context.aggregation().construction().cost_expression().is_empty());
    context.register(&mut model).unwrap();
    context.register(&mut model).unwrap();
    assert_eq!(model.tokens().len(), 6);
    assert!(context.register(&mut MetaModel::new("other")).is_err());
    assert!(context.aggregation().flow("undeclared").is_err());
    let compiled = model.try_to_linear_triad_model().unwrap();
    let facility = context.aggregation().construction().facility_variable(&"a".into()).unwrap();
    assert_eq!(compiled.c[facility], 10.0);
    let nominal = context.aggregation().flow("nominal").unwrap().aggregation.variable_index("goods", &"ab".into()).unwrap();
    let degraded = context.aggregation().flow("degraded").unwrap().aggregation.variable_index("goods", &"ab".into()).unwrap();
    assert_ne!(nominal, degraded);
    assert_eq!(compiled.c[nominal], 0.0);
    assert_eq!(compiled.c[degraded], 0.0);
}

#[test]
fn invalid_scenarios_fail_before_any_model_mutation_and_affine_units_do_not_panic() {
    let design = problem(0.0, 2.0);
    for field in 0..8 {
        let mut scenario = FailureScenario::new("bad");
        match field {
            0 => { scenario.closed_arcs.insert("unknown".into()); }
            1 => { scenario.failed_nodes.insert("unknown".into()); }
            2 => { scenario.upper_capacities.insert("ab".into(), Quantity::new(-1.0, Kilogram::INSTANT.clone())); }
            3 => { scenario.upper_capacities.insert("ab".into(), Quantity::new(11.0, Kilogram::INSTANT.clone())); }
            4 => { scenario.upper_capacities.insert("ab".into(), Quantity::new(2.0, Celsius::INSTANT.clone())); }
            5 => { scenario.balances.insert("unknown".into(), Default::default()); }
            6 => { scenario.slacks.insert("unknown".into(), Default::default()); }
            _ => scenario.id = "nominal".into(),
        }
        assert!(NetworkResilienceProblem::new(design.clone(), vec![scenario], Vec::new()).is_err());
    }
}

#[test]
fn connectivity_witnesses_distinguish_edge_and_internal_node_disjointness() {
    use crate::domain::design::NetworkDesignProblem;
    use crate::domain::flow::{FlowArc, FlowCommodity, FlowNode, FlowProblem};
    use crate::infrastructure::{CapacityBounds, NetworkCost};
    use ospf_rust_quantities::unit::NoneUnit;
    let unit = Kilogram::INSTANT.clone();
    let arcs = [("su", "s", "u"), ("sv", "s", "v"), ("um", "u", "m"), ("vm", "v", "m"),
        ("mw", "m", "w"), ("mz", "m", "z"), ("wt", "w", "t"), ("zt", "z", "t")];
    let flow = FlowProblem::new(
        ["s", "u", "v", "m", "w", "z", "t"].into_iter().map(FlowNode::new).collect(),
        arcs.into_iter().map(|(id, from, to)| FlowArc::new(id, from, to, CapacityBounds::try_new(
            Quantity::new(0.0, unit.clone()), Quantity::new(1.0, unit.clone()),
        ).unwrap(), NetworkCost::new(Quantity::new(0.0, NoneUnit::INSTANT.clone())))).collect(),
        vec![FlowCommodity::new("goods")],
    ).unwrap();
    let design = NetworkDesignProblem::new(flow, Vec::new(), Vec::new(), None).unwrap();
    for kind in [ConnectivityKind::Reachable, ConnectivityKind::EdgeDisjoint, ConnectivityKind::NodeDisjoint] {
        let problem = NetworkResilienceProblem::new(design.clone(), Vec::new(), vec![ConnectivityRequirement {
            source: "s".into(), target: "t".into(), paths: 2, kind,
        }]).unwrap();
        let mut context = NetworkResilienceContext::new(problem, "connectivity").unwrap();
        let mut model = MetaModel::new("witness");
        context.register(&mut model).unwrap();
        let compiled = model.try_to_linear_triad_model().unwrap();
        let mut found = false;
        // 独立枚举辅助弧；唯一中间割点 m 排除节点不相交而不排除边不相交。 / Independently enumerate auxiliary arcs; cut vertex m excludes node-disjoint but not edge-disjoint paths.
        for mask in 0..256 {
            let mut values = vec![0.0; model.tokens().len()];
            for edge in 0..8 { values[9 + edge] = ((mask >> edge) & 1) as f64; }
            let expected = evaluate_linear_solution(&compiled, &values, 1e-7).unwrap().issues.is_empty();
            model.set_solution(&values);
            assert_eq!(context.extract_solution(&model).is_ok(), expected);
            found |= expected;
        }
        assert_eq!(found, kind != ConnectivityKind::NodeDisjoint);
    }
}

#[test]
fn parallel_arcs_support_disjoint_paths_while_self_loops_do_not_create_a_witness() {
    use crate::domain::design::NetworkDesignProblem;
    use crate::domain::flow::{FlowArc, FlowCommodity, FlowNode, FlowProblem};
    use crate::infrastructure::{CapacityBounds, NetworkCost};
    use ospf_rust_quantities::unit::NoneUnit;
    let unit = Kilogram::INSTANT.clone();
    for connected in [false, true] {
        let arcs = if connected { vec![("p1", "s", "t"), ("p2", "s", "t"), ("loop", "s", "s")] }
            else { vec![("p1", "s", "s"), ("p2", "t", "t"), ("loop", "s", "s")] };
        let flow = FlowProblem::new(
            vec![FlowNode::new("s"), FlowNode::new("t")],
            arcs.into_iter().map(|(id, from, to)| FlowArc::new(id, from, to, CapacityBounds::try_new(
                Quantity::new(0.0, unit.clone()), Quantity::new(1.0, unit.clone()),
            ).unwrap(), NetworkCost::new(Quantity::new(0.0, NoneUnit::INSTANT.clone())))).collect(),
            vec![FlowCommodity::new("goods")],
        ).unwrap();
        let design = NetworkDesignProblem::new(flow, Vec::new(), Vec::new(), None).unwrap();
        let problem = NetworkResilienceProblem::new(design, Vec::new(), vec![ConnectivityRequirement {
            source: "s".into(), target: "t".into(), paths: 2, kind: ConnectivityKind::NodeDisjoint,
        }]).unwrap();
        let mut context = NetworkResilienceContext::new(problem, "parallel").unwrap();
        let mut model = MetaModel::new("parallel");
        context.register(&mut model).unwrap();
        let mut values = vec![0.0; model.tokens().len()];
        for value in &mut values[4..] { *value = 1.0; }
        model.set_solution(&values);
        assert_eq!(context.extract_solution(&model).is_ok(), connected);
        assert_eq!(model.tokens().len(), if connected { 6 } else { 4 });
    }
}

#[cfg(feature = "scip")]
#[test]
fn scip_design_and_resilience_services_validate_real_optima_and_hard_lower_infeasibility() {
    use ospf_rust_core::solver::{ProblemStatus, solvers::SCIPSolver};
    use crate::application::{NetworkDesignApplicationService, NetworkResilienceApplicationService};
    let report = NetworkDesignApplicationService::new(SCIPSolver::new()).solve(&problem(0.0, 2.0)).unwrap();
    assert!(report.is_optimal());
    assert_eq!(report.incumbent().unwrap().total_cost.value, 10.0);
    let service = NetworkResilienceApplicationService::new(SCIPSolver::new());
    let report = service.solve(&degraded(0.0, 2.0, 2.0)).unwrap();
    assert!(report.is_optimal());
    assert_eq!(report.incumbent().unwrap().total_cost.value, 10.0);
    let report = service.solve(&degraded(3.0, 3.0, 2.0)).unwrap();
    assert_eq!(report.problem_status, ProblemStatus::Infeasible);
    assert!(!report.has_incumbent());
}
