use ospf_rust_core::model::SubObjective;
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_quantities::unit::Kilogram;
use crate::{AssignmentKey, DiscreteLoadingProblem, LoadingItem, LoadingLayout, LoadingPosition, LayoutSelectionGroup, PositionConflict, SameUnitPackingValueAdapter};
use super::*;

fn input(cargo: bool) -> DiscreteLoadingProblem<f64, Kilogram> {
    DiscreteLoadingProblem {
        items: if cargo { vec![LoadingItem { id: "cargo".into(), weight: Quantity::new_ct(1.0), required: false, fixed_position_id: None }] } else { vec![] },
        positions: ["a", "b", "c"].into_iter().map(|id| LoadingPosition {
            id: id.into(), max_count: 2, max_weight: Quantity::new_ct(5.0), fixed_occupied: false, enabled: true,
        }).collect(),
        allowed_assignments: if cargo { ["a", "b", "c"].into_iter().map(|position| AssignmentKey { item_id: "cargo".into(), position_id: position.into() }).collect() } else { BTreeSet::new() },
        conflicts: BTreeSet::new(), layout_groups: vec![], weight_unit: Kilogram,
    }
}

fn options() -> PartitionLoadingOptions<f64> {
    PartitionLoadingOptions { partitions: vec![LoadingPartition { id: "left".into(), position_ids: BTreeSet::from(["a".into(), "b".into()]) },
        LoadingPartition { id: "right".into(), position_ids: BTreeSet::from(["c".into()]) }], extra_decisions: vec![], refinable_resources: BTreeSet::new(), tolerance: 1e-8 }
}

fn values(context: &PartitionLoadingContext<f64, Kilogram>, block: &str, occupied: &[&str]) -> BTreeMap<String, f64> {
    context.decisions().iter().filter(|decision| decision.block_id == block).map(|decision| {
        let active = context.loading().aggregation().occupied.iter().any(|(position, variable)| *variable == decision.variable && occupied.contains(&position.as_str()));
        (decision.id.clone(), if active { 1.0 } else { 0.0 })
    }).collect()
}

#[test]
fn partition_ownership_is_explicit_complete_unique_and_same_model() {
    let mut loading = DiscreteLoadingContext::new(input(true), &SameUnitPackingValueAdapter).unwrap();
    let mut source = MetaModel::new("partitions");
    loading.register(&mut source).unwrap();
    assert!(PartitionLoadingContext::new(&loading, &source, options()).is_ok());
    assert!(PartitionLoadingContext::new(&loading, &MetaModel::new("other"), options()).is_err());
    for case in 0..4 {
        let mut options = options();
        match case {
            0 => { options.partitions[0].position_ids.insert("c".into()); }
            1 => { options.partitions.pop(); }
            2 => { options.partitions[0].id = "right".into(); }
            _ => { options.refinable_resources.insert(loading.aggregation().occupied["a"]); }
        }
        assert!(PartitionLoadingContext::new(&loading, &source, options).is_err());
    }
}

#[test]
fn pricing_preserves_local_cargo_cross_partition_clique_pieces_and_layout_compatibility() {
    let mut input = input(true);
    input.conflicts = BTreeSet::from([PositionConflict::new("a", "b"), PositionConflict::new("a", "c"), PositionConflict::new("b", "c")]);
    input.layout_groups = vec![LayoutSelectionGroup { id: "layout".into(), position_ids: BTreeSet::from(["a".into(), "b".into(), "c".into()]),
        layouts: vec![LoadingLayout { id: "one".into(), position_ids: BTreeSet::from(["a".into(), "c".into()]) },
            LoadingLayout { id: "two".into(), position_ids: BTreeSet::from(["b".into(), "c".into()]) }] }];
    let mut loading = DiscreteLoadingContext::new(input, &SameUnitPackingValueAdapter).unwrap();
    let mut source = MetaModel::new("local");
    loading.register(&mut source).unwrap();
    let mut context = PartitionLoadingContext::new(&loading, &source, options()).unwrap();
    let request = context.pricing_request(&source, None, "left", false).unwrap();
    let pricing = context.pricing_model(&source, None, &request).unwrap();
    let snapshot = pricing.linear_model_snapshot().unwrap();
    assert!(snapshot.rows.iter().any(|row| row.source_name == "local_cargo_0" && row.coefficients.len() == 2));
    assert!(snapshot.rows.iter().any(|row| row.source_name == "local_conflict_0" && row.coefficients.len() == 2));
    assert!(snapshot.rows.iter().any(|row| row.source_name == "local_layout_choice_0"));
    assert!(snapshot.variables.iter().filter(|state| state.name.starts_with("local_layout_")).all(|state| state.var_type == VariableType::Binary));
    let bad = values(&context, "left", &["a", "b"]);
    assert!(context.add_columns(&source, vec![("left".into(), bad)]).is_err());
    assert!(context.columns().is_empty());
    let mut duplicate = values(&context, "left", &["a"]);
    for decision in context.decisions().iter().filter(|decision| decision.block_id == "left") {
        if loading.aggregation().assignments.values().any(|variable| *variable == decision.variable) { duplicate.insert(decision.id.clone(), 1.0); }
    }
    assert!(context.add_columns(&source, vec![("left".into(), duplicate)]).is_err());
}

#[test]
fn last_column_failure_preserves_pool_registration_and_dynamic_source_rules() {
    let mut loading = DiscreteLoadingContext::new(input(false), &SameUnitPackingValueAdapter).unwrap();
    let mut source = MetaModel::new("atomic");
    loading.register(&mut source).unwrap();
    let mut context = PartitionLoadingContext::new(&loading, &source, options()).unwrap();
    let left = values(&context, "left", &[]);
    let right = values(&context, "right", &[]);
    context.add_columns(&source, vec![("left".into(), left.clone()), ("right".into(), right)]).unwrap();
    let master = context.master(&source, false, true, false).unwrap();
    let master_values = vec![1.0; master.snapshot().variables.len()];
    context.extract_solution(&source, &master, &master_values).unwrap();
    let before = context.columns().len();
    assert!(context.add_columns(&source, vec![("left".into(), values(&context, "left", &["a"])), ("right".into(), BTreeMap::new())]).is_err());
    assert_eq!(context.columns().len(), before);
    context.extract_solution(&source, &master, &master_values).unwrap();
    let occupied = index(&source, loading.aggregation().occupied["a"]).unwrap();
    source.add_ge_constraint(&[(occupied, 1.0)], 1.0, "new_local_rule").unwrap();
    assert!(context.master(&source, false, true, false).is_err());
    assert!(context.extract_solution(&source, &master, &master_values).is_err());
    assert!(context.add_columns(&source, vec![("left".into(), left)]).is_ok());
    // 去重不等于既有池重新可行，旧列仍须删除或重定价。 / Deduplication does not repair invalid old columns; remove or reprice them.
    assert!(context.master(&source, false, true, false).is_err());
}

#[test]
fn resource_completion_freezes_profiles_materializes_full_columns_and_restores_final() {
    verify_resource_completion(false);
    verify_resource_completion(true);
}

fn verify_resource_completion(integer: bool) {
    let mut loading = DiscreteLoadingContext::new(input(false), &SameUnitPackingValueAdapter).unwrap();
    let mut source = MetaModel::new("resources");
    loading.register(&mut source).unwrap();
    let (a, c) = if integer {
        (source.register_auto_variable_with_range::<ospf_rust_core::variable::UInteger>("resource_a", VariableRange::bounded(0.0, 3.0)).unwrap(),
            source.register_auto_variable_with_range::<ospf_rust_core::variable::UInteger>("resource_c", VariableRange::bounded(0.0, 3.0)).unwrap())
    } else {
        (source.register_auto_variable_with_range::<Continuous>("resource_a", VariableRange::bounded(0.0, 3.0)).unwrap(),
            source.register_auto_variable_with_range::<Continuous>("resource_c", VariableRange::bounded(0.0, 3.0)).unwrap())
    };
    loading.link_additional_weight(&mut source, "a", Linear::new(vec![LinearMonomial::new(1.0, a)], 0.0)).unwrap();
    loading.link_additional_weight(&mut source, "c", Linear::new(vec![LinearMonomial::new(1.0, c)], 0.0)).unwrap();
    source.add_ge_constraint(&[(a, 1.0), (c, 1.0)], 2.0, "global_resource").unwrap();
    source.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(1.0, a), LinearMonomial::new(1.0, c)], 7.0), "resource_cost"));
    let aid = source.tokens()[a].id();
    let cid = source.tokens()[c].id();
    let mut options = options();
    options.extra_decisions = vec![PartitionDecision { id: "resource_a".into(), partition_id: "left".into(), variable: aid },
        PartitionDecision { id: "resource_c".into(), partition_id: "right".into(), variable: cid }];
    options.refinable_resources = BTreeSet::from([aid, cid]);
    let mut context = PartitionLoadingContext::new(&loading, &source, options).unwrap();
    context.add_columns(&source, vec![("left".into(), values(&context, "left", &["a"])), ("right".into(), values(&context, "right", &["c"]))]).unwrap();
    let refinement = context.master(&source, false, true, true).unwrap();
    let original_type = source.linear_model_snapshot().unwrap().variables.iter().find(|state| state.variable == aid).unwrap().var_type;
    assert_eq!(refinement.snapshot().variables.iter().find(|state| state.variable == aid).unwrap().var_type, original_type);
    let values = refinement.snapshot().variables.iter().map(|state| {
        if state.variable == aid { 2.0 } else if refinement.column_variables().values().any(|variable| *variable == state.variable) { 1.0 } else { 0.0 }
    }).collect::<Vec<_>>();
    let completed = context.extract_solution(&source, &refinement, &values).unwrap();
    assert_eq!(completed[&aid], 2.0);
    if original_type.is_integer() {
        let mut fractional = values.clone();
        let index = refinement.snapshot().variables.iter().position(|state| state.variable == aid).unwrap();
        fractional[index] = 2.5;
        assert!(context.materialize_completed_resources(&source, &refinement, &fractional).is_err());
        assert_eq!(context.columns().len(), 2);
    }
    context.materialize_completed_resources(&source, &refinement, &values).unwrap();
    assert_eq!(context.columns().len(), 3);
    assert!(context.materialize_completed_resources(&source, &refinement, &values).is_err());
    let final_master = context.master(&source, false, true, false).unwrap();
    assert!(final_master.global_variables().is_empty());
    assert_eq!(final_master.snapshot().objective_constant, 7.0);
    let final_values = final_master.snapshot().variables.iter().map(|state| {
        let id = final_master.column_variables().iter().find(|(_, variable)| **variable == state.variable).unwrap().0;
        let column = &context.columns()[id];
        if column.partition_id == "right" || column.decisions["resource_a"] == 2.0 { 1.0 } else { 0.0 }
    }).collect::<Vec<_>>();
    let complete = context.extract_solution(&source, &final_master, &final_values).unwrap();
    let source_values = source.linear_model_snapshot().unwrap().variables.iter().map(|state| complete[&state.variable]).collect::<Vec<_>>();
    let domain = loading.extract_solution(&source, &source_values, &1e-8).unwrap();
    assert!(domain.occupied_positions.contains("a"));
}
