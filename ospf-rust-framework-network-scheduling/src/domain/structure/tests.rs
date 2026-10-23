#![allow(clippy::borrow_interior_mutable_const)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Kilogram, NoneUnit};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use crate::domain::design::{CapacityLevel, NetworkDesignProblem, NetworkFacility, NetworkService};
use crate::domain::flow::{FlowArc, FlowCommodity, FlowNode, FlowProblem};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{CapacityBounds, NetworkCost};
use super::*;

pub(crate) fn problem() -> NetworkStructureProblem<f64> {
    let unit = Kilogram::INSTANT.clone();
    let tariff = NoneUnit::INSTANT.clone();
    let cost_unit = (&unit * &tariff).build();
    let arcs = [("rs", "r", "s", 1.0), ("st1", "s", "t", 1.0), ("st2", "s", "t", 2.0),
        ("rt", "r", "t", 5.0), ("ts", "t", "s", 0.0), ("ss", "s", "s", 0.0)];
    let flow = FlowProblem::new(
        ["r", "s", "t"].into_iter().map(FlowNode::new).collect(),
        arcs.iter().map(|(id, from, to, _)| FlowArc::new(*id, *from, *to, CapacityBounds::try_new(
            Quantity::new(0.0, unit.clone()), Quantity::new(10.0, unit.clone()),
        ).unwrap(), NetworkCost::new(Quantity::new(100.0, tariff.clone())))).collect(),
        // 不平衡名义流故意不可行；结构不能偷偷注册它。 / Deliberately infeasible nominal flow must not be registered by structure.
        vec![FlowCommodity::new("unbalanced").with_balance("r", Quantity::new(9.0, unit.clone()))],
    ).unwrap();
    let design = NetworkDesignProblem::new(flow, arcs.iter().map(|(id, _, _, cost)| NetworkService {
        arc: (*id).into(), required: false, levels: vec![CapacityLevel {
            id: "built".into(), capacity: Quantity::new(10.0, unit.clone()), cost: Quantity::new(*cost, cost_unit.clone()),
        }],
    }).collect(), vec![NetworkFacility {
        node: "s".into(), required: false, cost: Quantity::new(1.0, cost_unit), capacity_rows: BTreeSet::new(),
    }], None).unwrap();
    NetworkStructureProblem::new(design, "r".into(), ["t".into()].into(), ["s".into()].into(), Vec::new(), Vec::new()).unwrap()
}

pub(crate) fn optimal_values(context: &NetworkStructureContext<f64>, model: &MetaModel<f64>) -> Vec<f64> {
    let aggregation = context.aggregation();
    let mut values = vec![0.0; model.tokens().len()];
    values[aggregation.construction().facility_variable(&"s".into()).unwrap()] = 1.0;
    for id in ["rs", "st1"] {
        values[aggregation.construction().service_variables(&id.into()).unwrap()[0]] = 1.0;
        values[aggregation.tree().arc_variables()[&id.into()]] = 1.0;
    }
    for index in aggregation.tree().active_variables().values() { values[*index] = 1.0; }
    values[aggregation.tree().depth_variables()[&"s".into()]] = 1.0;
    values[aggregation.tree().depth_variables()[&"t".into()]] = 2.0;
    values
}

#[test]
fn rooted_steiner_tree_matches_independent_enumeration_and_preserves_parallel_arcs() {
    let mut context = NetworkStructureContext::new(problem(), "tree").unwrap();
    let mut model = MetaModel::new("tree");
    context.register(&mut model, Default::default()).unwrap();
    let aggregation = context.aggregation();
    assert_eq!(model.tokens().len(), 18);
    assert_eq!(aggregation.tree().arc_variables().len(), 5);
    assert!(aggregation.selected_tree_arc(&"ss".into()).unwrap().is_empty());
    assert!(aggregation.selected_tree_arc(&"unknown".into()).is_err());
    assert!(aggregation.tree().parent_expression(&"unknown".into()).is_err());
    assert_eq!(aggregation.tree().parent_expression(&"t".into()).unwrap().len(), 3);
    let arcs = [("rs", "r", "s", 1.0), ("st1", "s", "t", 1.0), ("st2", "s", "t", 2.0),
        ("rt", "r", "t", 5.0), ("ts", "t", "s", 0.0)];
    let compiled = model.try_to_linear_triad_model().unwrap();
    let mut best = f64::INFINITY;
    let mut feasible = 0;
    for mask in 0..32 {
        for relay in 0..=1 {
            for s_depth in 0..=2 {
                for t_depth in 0..=2 {
                    let mut values = vec![0.0; model.tokens().len()];
                    values[aggregation.construction().facility_variable(&"s".into()).unwrap()] = relay as f64;
                    let active = BTreeMap::from([("r", true), ("s", relay == 1), ("t", true)]);
                    let depth = BTreeMap::from([("r", 0), ("s", s_depth), ("t", t_depth)]);
                    for node in ["r", "s", "t"] {
                        values[aggregation.tree().active_variables()[&node.into()]] = f64::from(active[node]);
                        values[aggregation.tree().depth_variables()[&node.into()]] = depth[node] as f64;
                    }
                    let mut parents = BTreeMap::new();
                    let mut expected = relay == 1 || s_depth == 0;
                    let mut cost = relay as f64;
                    for (bit, (id, from, to, arc_cost)) in arcs.iter().enumerate() {
                        let selected = (mask >> bit) & 1 == 1;
                        values[aggregation.tree().arc_variables()[&(*id).into()]] = f64::from(selected);
                        values[aggregation.construction().service_variables(&(*id).into()).unwrap()[0]] = f64::from(selected);
                        if selected {
                            expected &= active[from] && active[to] && parents.insert(*to, *from).is_none()
                                && depth[to] > depth[from];
                            cost += arc_cost;
                        }
                    }
                    for node in ["s", "t"] {
                        expected &= parents.contains_key(node) == active[node];
                        if !active[node] { continue; }
                        let mut current = node;
                        let mut seen = BTreeSet::new();
                        while current != "r" {
                            if !seen.insert(current) { expected = false; break; }
                            let Some(parent) = parents.get(current) else { expected = false; break; };
                            current = parent;
                        }
                    }
                    let audit = evaluate_linear_solution(&compiled, &values, 1e-7).unwrap();
                    assert_eq!(audit.issues.is_empty(), expected, "mask={mask}, relay={relay}, depths={depth:?}");
                    model.set_solution(&values);
                    assert_eq!(context.extract_solution(&model).is_ok(), expected);
                    if expected { feasible += 1; best = best.min(cost); }
                }
            }
        }
    }
    assert!(feasible > 0);
    assert_eq!(best, 3.0);
    model.set_solution(&optimal_values(&context, &model));
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.selected_tree_arcs, ["rs".into(), "st1".into()].into());
    assert_eq!(solution.parent_arcs[&"t".into()], "st1".into());
    assert_eq!(solution.construction.cost.value, best);
}

#[test]
fn production_extraction_rejects_missing_nonfinite_fractional_and_unbuilt_tree_results() {
    let mut context = NetworkStructureContext::new(problem(), "extract").unwrap();
    let mut model = MetaModel::new("extract");
    context.register(&mut model, Default::default()).unwrap();
    assert!(context.extract_solution(&model).is_err());
    let valid = optimal_values(&context, &model);
    for index in 0..valid.len() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1] {
            let mut values = valid.clone();
            values[index] = invalid;
            model.set_solution(&values);
            assert!(context.extract_solution(&model).is_err(), "index={index}, invalid={invalid}");
        }
    }
    let aggregation = context.aggregation();
    for index in [aggregation.tree().arc_variables()[&"rs".into()], aggregation.tree().active_variables()[&"s".into()]] {
        let mut values = valid.clone();
        values[index] = 0.5;
        model.set_solution(&values);
        assert!(context.extract_solution(&model).is_err());
    }
    for index in [aggregation.construction().service_variables(&"rs".into()).unwrap()[0],
        aggregation.construction().facility_variable(&"s".into()).unwrap()]
    {
        let mut values = valid.clone();
        values[index] = 0.0;
        model.set_solution(&values);
        assert!(context.extract_solution(&model).is_err());
    }
    let mut cycle = valid;
    cycle[aggregation.tree().arc_variables()[&"rs".into()]] = 0.0;
    cycle[aggregation.tree().arc_variables()[&"ts".into()]] = 1.0;
    cycle[aggregation.construction().service_variables(&"ts".into()).unwrap()[0]] = 1.0;
    model.set_solution(&cycle);
    assert!(context.extract_solution(&model).is_err());
    assert!(context.extract_solution(&MetaModel::new("other")).is_err());
}

fn coverage_problem() -> NetworkStructureProblem<f64> {
    NetworkStructureProblem::new(problem().design().clone(), "r".into(), BTreeSet::new(), BTreeSet::new(), vec![
        FacilityPartitionRequest { id: "required".into(), eligible_facilities: ["s".into(), "t".into()].into(), required: true },
        FacilityPartitionRequest { id: "optional".into(), eligible_facilities: ["s".into(), "t".into()].into(), required: false },
    ], vec![
        FacilityAssignmentQuota { facility: "s".into(), minimum: 1, maximum: Some(1) },
        FacilityAssignmentQuota { facility: "t".into(), minimum: 0, maximum: Some(1) },
    ]).unwrap()
}

#[test]
fn eligible_owners_and_count_quotas_match_independent_enumeration_without_implicit_connectivity() {
    let mut context = NetworkStructureContext::new(coverage_problem(), "coverage").unwrap();
    let mut model = MetaModel::new("coverage");
    context.register(&mut model, Default::default()).unwrap();
    let aggregation = context.aggregation();
    assert!(aggregation.tree().arc_variables().is_empty());
    assert!(aggregation.coverage().assignment_expression("missing", &"s".into()).is_err());
    let compiled = model.try_to_linear_triad_model().unwrap();
    let mut feasible = 0;
    for mask in 0..16 {
        for open in 0..=1 {
            let mut values = vec![0.0; model.tokens().len()];
            values[aggregation.tree().active_variables()[&"r".into()]] = 1.0;
            values[aggregation.construction().facility_variable(&"s".into()).unwrap()] = open as f64;
            let mut selected = BTreeMap::new();
            for (bit, ((partition, facility), index)) in aggregation.coverage().assignment_variables().iter().enumerate() {
                let value = (mask >> bit) & 1;
                values[*index] = value as f64;
                selected.insert((partition.as_str(), facility.as_str()), value);
            }
            let rs = selected[&("required", "s")];
            let rt = selected[&("required", "t")];
            let os = selected[&("optional", "s")];
            let ot = selected[&("optional", "t")];
            let expected = rs + rt == 1 && os + ot <= 1 && rs + os == 1 && rt + ot <= 1 && rs <= open && os <= open;
            assert_eq!(evaluate_linear_solution(&compiled, &values, 1e-7).unwrap().issues.is_empty(), expected);
            model.set_solution(&values);
            let result = context.extract_solution(&model);
            assert_eq!(result.is_ok(), expected);
            if let Ok(solution) = result {
                feasible += 1;
                assert_eq!(solution.connected_nodes, ["r".into()].into());
                assert!(solution.partition_owners.contains_key("required"));
            }
        }
    }
    assert_eq!(feasible, 3);
}

#[derive(Debug)]
struct Extension(AtomicBool);

impl NetworkStructurePipeline<f64> for Extension {
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkStructureAggregation<f64>) -> Result<()> {
        let expression = aggregation.selected_tree_arc(&"rs".into())?;
        model.add_linear_constraint(&expression, ConstraintRelation::Equal, 1.0, "downstream_tree")
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        model.add_linear_objective(&[(expression[0].0, 7.0)], "downstream_cost");
        if self.0.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("测试注入失败 / injected test failure")); }
        Ok(())
    }
}

#[test]
fn transactional_registration_keeps_downstream_objectives_and_rejects_cross_model_reuse() {
    let mut context = NetworkStructureContext::new(problem(), "retry").unwrap();
    assert!(context.aggregation().selected_tree_arc(&"rs".into()).is_err());
    context.add_pipeline(Arc::new(Extension(AtomicBool::new(true)))).unwrap();
    let mut model = MetaModel::new("retry");
    assert!(context.register(&mut model, Default::default()).is_err());
    assert_eq!(model.tokens().len(), 0);
    assert!(context.aggregation().tree().arc_variables().is_empty());
    assert!(context.aggregation().construction().cost_expression().is_empty());
    context.register(&mut model, Default::default()).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    let index = context.aggregation().tree().arc_variables()[&"rs".into()];
    assert_eq!(compiled.c[index], 7.0);
    let count = model.tokens().len();
    context.register(&mut model, Default::default()).unwrap();
    assert_eq!(model.tokens().len(), count);
    assert!(context.register(&mut model, NetworkStructureRegistration { minimize_construction_cost: false }).is_err());
    assert!(context.register(&mut MetaModel::new("other"), Default::default()).is_err());
    assert!(context.add_pipeline(Arc::new(Extension(AtomicBool::new(false)))).is_err());
    model.set_solution(&optimal_values(&context, &model));
    assert_eq!(context.extract_solution(&model).unwrap().construction.cost.value, 3.0);
}

#[test]
fn invalid_references_duplicates_and_quotas_fail_but_valid_infeasibility_registers() {
    let original = coverage_problem();
    for field in 0..7 {
        let mut root = original.root().clone();
        let mut partitions = original.partitions().to_vec();
        let mut quotas = original.quotas().to_vec();
        match field {
            0 => root = "unknown".into(),
            1 => partitions.push(partitions[0].clone()),
            2 => partitions[0].id = " ".into(),
            3 => { partitions[0].eligible_facilities.insert("unknown".into()); }
            4 => quotas.push(quotas[0].clone()),
            5 => quotas[0].maximum = Some(0),
            _ => quotas[0].facility = "unknown".into(),
        }
        assert!(NetworkStructureProblem::new(original.design().clone(), root, BTreeSet::new(), BTreeSet::new(), partitions, quotas).is_err());
    }
    for problem in infeasible_problems() {
        let mut context = NetworkStructureContext::new(problem, "infeasible").unwrap();
        context.register(&mut MetaModel::new("infeasible"), Default::default()).unwrap();
    }
}

fn infeasible_problems() -> Vec<NetworkStructureProblem<f64>> {
    let original = problem();
    let design = original.design();
    let arcs = design.flow().arcs().iter().filter(|arc| arc.id != "rt".into()).cloned().collect();
    let services = design.services().iter().filter(|service| service.arc != "rt".into()).cloned().collect();
    let disconnected = NetworkDesignProblem::new(FlowProblem::new(
        design.flow().nodes().to_vec(), arcs, design.flow().commodities().to_vec(),
    ).unwrap(), services, design.facilities().to_vec(), None).unwrap();
    vec![
        NetworkStructureProblem::new(disconnected, "r".into(), ["t".into()].into(), BTreeSet::new(), Vec::new(), Vec::new()).unwrap(),
        NetworkStructureProblem::new(design.clone(), "r".into(), BTreeSet::new(), BTreeSet::new(), vec![
            FacilityPartitionRequest { id: "required".into(), eligible_facilities: BTreeSet::new(), required: true },
        ], Vec::new()).unwrap(),
        NetworkStructureProblem::new(design.clone(), "r".into(), BTreeSet::new(), BTreeSet::new(), Vec::new(), vec![
            FacilityAssignmentQuota { facility: "s".into(), minimum: 1, maximum: None },
        ]).unwrap(),
    ]
}

#[cfg(feature = "scip")]
#[test]
fn scip_structure_service_executes_steiner_coverage_and_valid_infeasibility() {
    use ospf_rust_core::solver::{ProblemStatus, solvers::SCIPSolver};
    use crate::application::NetworkStructureApplicationService;
    let service = NetworkStructureApplicationService::new(SCIPSolver::new());
    let report = service.solve(&problem()).unwrap();
    assert!(report.is_optimal());
    let solution = report.incumbent().unwrap();
    assert_eq!(solution.construction.cost.value, 3.0);
    assert_eq!(solution.selected_tree_arcs, ["rs".into(), "st1".into()].into());
    let report = service.solve(&coverage_problem()).unwrap();
    assert!(report.is_optimal());
    assert_eq!(report.incumbent().unwrap().construction.cost.value, 1.0);
    for problem in infeasible_problems() {
        let report = service.solve(&problem).unwrap();
        assert_eq!(report.problem_status, ProblemStatus::Infeasible);
        assert!(!report.has_incumbent());
    }
}

#[cfg(any(feature = "big-decimal", feature = "big-rational"))]
fn generic_structure_roundtrip<V>()
where
    V: ospf_rust_core::solver::value::SolveValue + ospf_rust_quantities::unit::UnitConversionValue,
{
    let adapter = crate::infrastructure::NetworkSchedulingSolverValueAdapter::<V>::default();
    let value = |number| adapter.from_solver(number).unwrap();
    let unit = Kilogram::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    let flow = FlowProblem::new(vec![FlowNode::new("a"), FlowNode::new("b")], vec![FlowArc::new("ab", "a", "b",
        CapacityBounds::try_new(Quantity::new(value(0.0), unit.clone()), Quantity::new(value(1.0), unit.clone())).unwrap(),
        NetworkCost::new(Quantity::new(value(100.0), (&cost / &unit).build())),
    )], vec![FlowCommodity::new("unused")]).unwrap();
    let design = NetworkDesignProblem::new(flow, vec![NetworkService { arc: "ab".into(), required: false, levels: vec![CapacityLevel {
        id: "built".into(), capacity: Quantity::new(value(1.0), unit), cost: Quantity::new(value(3.0), cost.clone()),
    }] }], vec![NetworkFacility { node: "a".into(), required: false, cost: Quantity::new(value(2.0), cost), capacity_rows: BTreeSet::new() }], None).unwrap();
    let problem = NetworkStructureProblem::new(design, "a".into(), ["b".into()].into(), BTreeSet::new(), Vec::new(), Vec::new()).unwrap();
    let mut context = NetworkStructureContext::new(problem, "generic").unwrap();
    let mut model = MetaModel::new("generic");
    context.register(&mut model, Default::default()).unwrap();
    let aggregation = context.aggregation();
    let mut values = vec![1.0; model.tokens().len()];
    values[aggregation.tree().depth_variables()[&"a".into()]] = 0.0;
    model.set_solution(&values);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(adapter.to_solver(&solution.construction.cost.value).unwrap(), 5.0);
    assert_eq!(solution.selected_tree_arcs, ["ab".into()].into());
}

#[cfg(feature = "big-decimal")]
#[test]
fn structure_big_decimal_registers_and_extracts_construction_quantities() { generic_structure_roundtrip::<bigdecimal::BigDecimal>(); }

#[cfg(feature = "big-rational")]
#[test]
fn structure_big_rational_registers_and_extracts_construction_quantities() { generic_structure_roundtrip::<num_rational::BigRational>(); }
