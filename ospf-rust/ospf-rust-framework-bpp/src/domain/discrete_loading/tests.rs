use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{MetaModel, SubObjective};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Kilogram;
use crate::infrastructure::SameUnitPackingValueAdapter;
use super::*;

fn key(item: &str, position: &str) -> AssignmentKey {
    AssignmentKey { item_id: item.into(), position_id: position.into() }
}

fn problem() -> DiscreteLoadingProblem<f64, Kilogram> {
    DiscreteLoadingProblem {
        items: ["a", "b", "c"].into_iter().map(|id| LoadingItem {
            id: id.into(), weight: Quantity::new_ct(4.0), required: id != "c", fixed_position_id: None,
        }).collect(),
        positions: ["p", "q"].into_iter().map(|id| LoadingPosition {
            id: id.into(), max_count: if id == "p" { 2 } else { 1 },
            max_weight: Quantity::new_ct(if id == "p" { 10.0 } else { 6.0 }), fixed_occupied: false, enabled: true,
        }).collect(),
        allowed_assignments: BTreeSet::from([key("a", "p"), key("a", "q"), key("b", "p"), key("c", "q")]),
        conflicts: BTreeSet::new(), layout_groups: vec![], weight_unit: Kilogram,
    }
}

fn solution() -> DiscreteLoadingSolution {
    DiscreteLoadingSolution {
        assignments: BTreeMap::from([("a".into(), "p".into()), ("b".into(), "p".into())]),
        occupied_positions: BTreeSet::from(["p".into()]), selected_layouts: BTreeMap::new(),
    }
}

fn values(context: &DiscreteLoadingContext<f64, Kilogram>, model: &MetaModel) -> Vec<f64> {
    let mut values = vec![0.0; model.num_tokens()];
    for (key, variable) in &context.aggregation().assignments {
        if key.position_id == "p" { values[model.find_token(*variable).unwrap().solver_index] = 1.0; }
    }
    values[model.find_token(context.aggregation().occupied["p"]).unwrap().solver_index] = 1.0;
    values
}

#[test]
fn sparse_registration_allows_multiple_items_in_one_position() {
    let mut context = DiscreteLoadingContext::new(problem(), &SameUnitPackingValueAdapter).unwrap();
    let mut model = MetaModel::new("loading");
    context.register(&mut model).unwrap();
    assert_eq!(context.aggregation().assignments.len(), 4);
    assert_eq!(model.num_tokens(), 6);
    let result = context.extract_solution(&model, &values(&context, &model), &1e-8).unwrap();
    assert_eq!(result, solution());
    assert!(context.register(&mut model).is_err());
    assert!(context.extract_solution(&MetaModel::new("other"), &[], &1e-8).is_err());
}

#[test]
fn extension_rows_and_objectives_participate_and_failed_registration_is_retryable() {
    let mut context = DiscreteLoadingContext::new(problem(), &SameUnitPackingValueAdapter).unwrap();
    let mut model = MetaModel::new("extension");
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![], 7.0), "outside"));
    let result = context.register_with(&mut model, |model, _| {
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![], 99.0), "failed"));
        Err(super::context::invalid("extension failed"))
    });
    assert!(result.is_err());
    assert_eq!(model.num_tokens(), 0);
    assert_eq!(model.num_constraints(), 0);
    assert!(context.aggregation().assignments.is_empty());
    assert_eq!(model.objective().len(), 1);
    context.register_with(&mut model, |model, aggregation| {
        let variable = aggregation.assignments[&key("a", "p")];
        let index = model.find_token(variable).unwrap().solver_index;
        model.add_eq_constraint(&[(index, 1.0)], 0.0, "extension_rule")?;
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![], 3.0), "extension_cost"));
        Ok(())
    }).unwrap();
    assert_eq!(model.linear_model_snapshot().unwrap().objective_constant, 10.0);
    assert!(context.extract_solution(&model, &values(&context, &model), &1e-8).is_err());
}

#[test]
fn all_conflict_edges_are_covered_without_transitive_overrestriction() {
    let edges = BTreeSet::from([PositionConflict::new("a", "b"), PositionConflict::new("b", "c"),
        PositionConflict::new("b", "d"), PositionConflict::new("c", "d")]);
    let index = ConflictIndex::build(&edges);
    for edge in &edges {
        assert!(index.cliques().iter().any(|clique| clique.contains(&edge.first) && clique.contains(&edge.second)));
    }
    assert!(!index.cliques().iter().any(|clique| clique.contains("a") && clique.contains("c")));
    assert!(!index.cliques().iter().any(|clique| clique.contains("a") && clique.contains("d")));
    assert_eq!(PositionConflict::new("a", "b"), PositionConflict::new("b", "a"));
}

#[test]
fn fixed_loading_and_external_occupancy_require_whole_position_and_reject_conflicts() {
    let mut input = problem();
    input.items[0].fixed_position_id = Some("p".into());
    input.positions[0].enabled = false;
    input.items[1].required = false;
    let mut context = DiscreteLoadingContext::new(input.clone(), &SameUnitPackingValueAdapter).unwrap();
    let mut model = MetaModel::new("fixed");
    context.register(&mut model).unwrap();
    assert_eq!(context.aggregation().assignments.len(), 2);
    assert!(context.verify(&DiscreteLoadingSolution::default(), &BTreeMap::new(), &1e-8).is_err());
    input.positions[1].fixed_occupied = true;
    input.conflicts.insert(PositionConflict::new("p", "q"));
    assert!(DiscreteLoadingContext::new(input, &SameUnitPackingValueAdapter).is_err());
}

#[test]
fn empty_reservations_are_allowed_but_any_positive_additional_weight_needs_occupancy() {
    let context = DiscreteLoadingContext::new(problem(), &SameUnitPackingValueAdapter).unwrap();
    let mut solution = solution();
    assert!(context.verify(&solution, &BTreeMap::from([("q".into(), 1e-30)]), &1e-6).is_err());
    solution.occupied_positions.insert("q".into());
    context.verify(&solution, &BTreeMap::new(), &1e-8).unwrap();
    context.verify(&solution, &BTreeMap::from([("q".into(), 1e-30)]), &1e-8).unwrap();
    assert!(context.verify(&solution, &BTreeMap::from([("q".into(), -1e-30)]), &1e-6).is_err());
    assert!(context.verify(&solution, &BTreeMap::from([("q".into(), f64::NAN)]), &1e-6).is_err());
}

#[test]
fn additional_weight_links_are_atomic_unique_and_extracted_from_the_model() {
    let mut context = DiscreteLoadingContext::new(problem(), &SameUnitPackingValueAdapter).unwrap();
    let mut model = MetaModel::new("additional");
    context.register(&mut model).unwrap();
    let before = model.num_constraints();
    assert!(context.link_additional_weight(&mut model, "p",
        Linear::new(vec![LinearMonomial::new(1.0, 999)], 0.0)).is_err());
    assert_eq!(model.num_constraints(), before);
    context.link_additional_weight(&mut model, "p", Linear::new(vec![], 1.0)).unwrap();
    context.extract_solution(&model, &values(&context, &model), &1e-8).unwrap();
    assert!(context.link_additional_weight(&mut model, "p", Linear::new(vec![], 1.0)).is_err());
    assert_eq!(model.num_constraints(), before + 2);
    context.link_additional_weight(&mut model, "q", Linear::new(vec![], 1e-30)).unwrap();
    assert!(context.extract_solution(&model, &values(&context, &model), &1e-6).is_err());
}

#[test]
fn explicit_layouts_allow_shared_positions_and_reject_wrong_selection() {
    let mut input = problem();
    input.layout_groups.push(LayoutSelectionGroup {
        id: "layout".into(), position_ids: BTreeSet::from(["p".into(), "q".into()]),
        layouts: vec![LoadingLayout { id: "both".into(), position_ids: BTreeSet::from(["p".into(), "q".into()]) },
            LoadingLayout { id: "only_q".into(), position_ids: BTreeSet::from(["q".into()]) }],
    });
    let context = DiscreteLoadingContext::new(input, &SameUnitPackingValueAdapter).unwrap();
    let mut solution = solution();
    assert!(context.verify(&solution, &BTreeMap::new(), &1e-8).is_err());
    solution.selected_layouts.insert("layout".into(), "only_q".into());
    assert!(context.verify(&solution, &BTreeMap::new(), &1e-8).is_err());
    solution.selected_layouts.insert("layout".into(), "both".into());
    context.verify(&solution, &BTreeMap::new(), &1e-8).unwrap();
}

#[test]
fn invalid_inputs_and_fixed_capacity_fail_before_registration() {
    let mut cases = Vec::new();
    let mut input = problem(); input.items[1].id = "a".into(); cases.push(input);
    let mut input = problem(); input.positions[0].max_count = 0; cases.push(input);
    let mut input = problem(); input.items[0].weight.value = f64::INFINITY; cases.push(input);
    let mut input = problem(); input.conflicts.insert(PositionConflict::new("p", "missing")); cases.push(input);
    let mut input = problem(); input.items[0].fixed_position_id = Some("missing".into()); cases.push(input);
    let mut input = problem(); input.items[0].fixed_position_id = Some("p".into());
    input.items[1].fixed_position_id = Some("p".into()); input.positions[0].max_weight.value = 7.0; cases.push(input);
    for input in cases { assert!(DiscreteLoadingContext::new(input, &SameUnitPackingValueAdapter).is_err()); }
}
