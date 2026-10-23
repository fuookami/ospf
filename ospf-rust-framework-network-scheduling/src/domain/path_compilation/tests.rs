#![allow(clippy::borrow_interior_mutable_const)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Celsius, Gram, Kelvin, Kilogram, NoneUnit};
use ospf_rust_core::model::{ConstraintRelation, Linear, LinearMonomial, MetaModel, ObjectiveCategory, SubObjective};
use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveReport, SolveSolution, TerminationReason, linear_model_fingerprint};
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_framework::model::{ColumnGenerationScope, VerifiedPricingDuals};
use crate::error::{NetworkSchedulingError, Result};
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Payload {
    arcs: Vec<String>,
    actual_times: Vec<i64>,
}

fn column(id: &str, cost: f64, coverage: [f64; 2], capacity: f64) -> PathColumn<f64, Payload> {
    let unit = NoneUnit::INSTANT.clone();
    PathColumn { id: id.into(), payload: Payload { arcs: vec![format!("{id}-start"), format!("{id}-end")], actual_times: vec![10, 15, 23] },
        cost: Quantity::new(cost, unit.clone()), coefficients: [
            ("a".into(), Quantity::new(coverage[0], unit.clone())), ("b".into(), Quantity::new(coverage[1], unit.clone())),
            ("capacity".into(), Quantity::new(capacity, Kilogram::INSTANT.clone())), ("signed".into(), Quantity::new(-1.0, unit)),
        ].into() }
}

pub(crate) fn problem() -> PathCompilationProblem<f64, Payload> {
    let unit = NoneUnit::INSTANT.clone();
    let capacity = Kilogram::INSTANT.clone();
    let mut rows = ["a", "b"].into_iter().map(|id| PathCompilationRow { id: id.into(), unit: unit.clone(),
        lower: Some(Quantity::new(1.0, unit.clone())), upper: Some(Quantity::new(1.0, unit.clone())) }).collect::<Vec<_>>();
    rows.extend([
        PathCompilationRow { id: "capacity".into(), unit: capacity.clone(), lower: Some(Quantity::new(1.0, capacity.clone())),
            upper: Some(Quantity::new(4.0, capacity)) },
        PathCompilationRow { id: "signed".into(), unit: unit.clone(), lower: None, upper: Some(Quantity::new(0.0, unit.clone())) },
        PathCompilationRow { id: "unused".into(), unit: unit.clone(), lower: None, upper: Some(Quantity::new(0.0, unit.clone())) },
    ]);
    PathCompilationProblem::new(rows, vec![column("a", 4.0, [1.0, 0.0], 1.0), column("b", 3.0, [0.0, 1.0], 2.0),
        column("whole", 5.0, [1.0, 1.0], 3.0)], unit).unwrap()
}

fn registered() -> (PathCompilationContext<f64, Payload>, MetaModel<f64>) {
    let mut context = PathCompilationContext::new(problem(), "master").unwrap();
    let mut model = MetaModel::new("master");
    context.register(&mut model).unwrap();
    (context, model)
}

fn scope(context: &PathCompilationContext<f64, Payload>, model: &MetaModel<f64>) -> ColumnGenerationScope {
    ColumnGenerationScope { request: "request-1".into(), groups: ["aircraft".into()].into(),
        model: linear_model_fingerprint(&context.pricing_model(model).unwrap()).unwrap().value,
        dual: "dual-1".into(), branch: "root".into(), global: true, executor_slots: ["aircraft:0".into()].into() }
}

fn prices() -> BTreeMap<String, f64> {
    [("a", 2.0), ("b", 3.0), ("capacity", -2.0), ("signed", -1.0), ("unused", 0.0)].into_iter()
        .map(|(id, value)| (id.into(), value)).collect()
}

fn report(
    context: &PathCompilationContext<f64, Payload>,
    model: &MetaModel<f64>,
    duals: Vec<f64>,
) -> SolveReport<f64> {
    let lp = context.pricing_model(model).unwrap();
    let mut values = vec![0.0; model.tokens().len()];
    values[context.aggregation().compilation().variables()["whole"]] = 1.0;
    let mut solution = SolveSolution::vector(values);
    solution.objective_value = Some(5.0);
    solution.objective = Some(5.0);
    solution.dual_solution = Some(duals);
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&lp).unwrap()), ..Default::default() }).build().unwrap()
}

#[test]
fn finite_binary_master_matches_independent_oracle_and_preserves_whole_payloads() {
    let (context, mut model) = registered();
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert_eq!(model.tokens().len(), 3);
    assert_eq!(compiled.A.rows.len(), 8);
    assert!(context.aggregation().row_expression("missing").is_err());
    assert_eq!(context.aggregation().problem().compile(column("new", 6.0, [1.0, 1.0], 2.0)).unwrap().coefficients()["unused"], 0.0);
    let mut best = f64::INFINITY;
    let mut feasible = 0;
    for mask in 0..8 {
        let a = mask & 1;
        let b = (mask >> 1) & 1;
        let whole = (mask >> 2) & 1;
        let values = [a as f64, b as f64, whole as f64];
        let expected = a + whole == 1 && b + whole == 1 && (1..=4).contains(&(a + 2 * b + 3 * whole));
        assert_eq!(evaluate_linear_solution(&compiled, &values, 1e-7).unwrap().issues.is_empty(), expected);
        model.set_solution(&values);
        let extracted = context.extract_solution(&model);
        assert_eq!(extracted.is_ok(), expected);
        if let Ok(solution) = extracted {
            feasible += 1;
            let cost = (4 * a + 3 * b + 5 * whole) as f64;
            assert_eq!(solution.cost.value, cost);
            assert_eq!(solution.candidate_ids, ["a".into(), "b".into(), "whole".into()].into());
            assert_eq!(solution.usage["capacity"].value, (a + 2 * b + 3 * whole) as f64);
            assert_eq!(solution.usage["unused"].value, 0.0);
            best = best.min(cost);
        }
    }
    assert_eq!(feasible, 2);
    assert_eq!(best, 5.0);
    model.set_solution(&[0.0, 0.0, 1.0]);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(solution.selected[0].payload.actual_times, vec![10, 15, 23]);
    assert_eq!(solution.selected[0].payload.arcs, vec!["whole-start", "whole-end"]);
}

#[test]
fn duplicate_identity_conflicts_are_atomic_but_normalized_equivalent_columns_are_idempotent() {
    let (mut context, mut model) = registered();
    let request = scope(&context, &model);
    let duals = context.refresh_heuristic_duals(&model, prices(), request.clone()).unwrap();
    let mut equivalent = context.aggregation().columns()["a"].column().clone();
    equivalent.coefficients.insert("capacity".into(), Quantity::new(1000.0, Gram::INSTANT.clone()));
    equivalent.coefficients.insert("unused".into(), Quantity::new(0.0, NoneUnit::INSTANT.clone()));
    assert!(context.add_columns(&mut model, vec![equivalent]).unwrap().is_empty());
    context.validate_pricing_duals(&model, &duals, &request).unwrap();
    let before = linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    for field in 0..5 {
        let mut conflicting = context.aggregation().columns()["a"].column().clone();
        match field {
            0 => conflicting.payload.actual_times[1] = 16,
            1 => conflicting.cost.value += 1.0,
            2 => { conflicting.coefficients.get_mut("capacity").unwrap().value = 2.0; }
            3 => { conflicting.coefficients.insert("unknown".into(), Quantity::new(0.0, NoneUnit::INSTANT.clone())); }
            _ => conflicting.cost.value = f64::NAN,
        }
        assert!(context.add_columns(&mut model, vec![column("new", 1.0, [1.0, 1.0], 1.0), conflicting]).is_err());
        assert_eq!(model.tokens().len(), 3);
        assert_eq!(context.aggregation().columns().len(), 3);
        assert_eq!(linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
        context.validate_pricing_duals(&model, &duals, &request).unwrap();
    }
    let mut conflicting = problem().columns().to_vec();
    let mut second = conflicting[0].clone();
    second.payload.actual_times[0] += 1;
    conflicting.push(second);
    assert!(PathCompilationProblem::new(problem().rows().to_vec(), conflicting, NoneUnit::INSTANT.clone()).is_err());
}

#[test]
fn deleting_and_readding_columns_cleans_owned_rows_and_all_objective_terms_without_overwriting_extensions() {
    let (mut context, mut model) = registered();
    let old = context.aggregation().compilation().variables()["a"];
    let extra = model.register_auto_variable_with_range::<Continuous>("external", VariableRange::bounded(0.0, 1.0)).unwrap();
    model.add_sub_objective(SubObjective::new_with_weight(ObjectiveCategory::Minimum,
        Linear::new(vec![LinearMonomial::new(2.0, old), LinearMonomial::new(7.0, extra)], 9.0), "external", 3.0));
    model.add_sub_objective(SubObjective::new_with_weight(ObjectiveCategory::Maximum,
        Linear::new(vec![LinearMonomial::new(2.0, old)], 4.0), "cancel_path_term", 3.0));
    let saved = context.aggregation().columns()["a"].column().clone();
    assert_eq!(context.remove_columns(&mut model, &["a".into(), "unknown".into()].into()).unwrap().len(), 1);
    let compiled = model.try_to_linear_triad_model().unwrap();
    assert_eq!(compiled.lb[old], 0.0);
    assert_eq!(compiled.ub[old], 0.0);
    assert_eq!(compiled.c[old], 0.0);
    assert_eq!(compiled.c[extra], 21.0);
    assert!(compiled.A.rows.iter().all(|row| row.entries.iter().all(|(index, _)| *index != old)));
    let external = model.objective().sub_objectives.iter().find(|objective| objective.name == "external").unwrap();
    assert_eq!(*external.polynomial.constant_term(), 9.0);
    assert_eq!(external.weight, 3.0);
    context.add_columns(&mut model, vec![saved]).unwrap();
    let new = context.aggregation().compilation().variables()["a"];
    assert_ne!(old, new);
    assert_ne!(model.tokens()[old].id(), model.tokens()[new].id());
    assert_eq!(model.tokens().len(), 5);
    assert_eq!(context.aggregation().compilation().retired_variables(), &[old].into());
    model.set_solution(&[0.0, 1.0, 0.0, 0.0, 1.0]);
    assert_eq!(context.extract_solution(&model).unwrap().cost.value, 7.0);
    assert!(context.remove_columns(&mut model, &BTreeSet::new()).unwrap().is_empty());
}

#[test]
fn heuristic_duals_require_complete_rows_and_scope_and_never_become_verified_sources() {
    let (mut context, model) = registered();
    let request = scope(&context, &model);
    let duals = context.refresh_heuristic_duals(&model, prices(), request.clone()).unwrap();
    assert!(!duals.has_verified_lp_source());
    assert!(context.require_verified_pricing_duals(&model, &duals, &request).is_err());
    let mut candidate = column("new", 6.0, [1.0, 1.0], 2.0);
    candidate.coefficients.get_mut("signed").unwrap().value = -3.0;
    assert_eq!(context.reduced_cost(&model, candidate.clone(), &duals, &request).unwrap(), 2.0);
    for field in 0..7 {
        let mut changed = request.clone();
        match field {
            0 => changed.request.push('x'),
            1 => changed.model.push('x'),
            2 => changed.dual.push('x'),
            3 => changed.branch.push('x'),
            4 => { changed.groups.insert("another".into()); }
            5 => { changed.executor_slots.insert("aircraft:1".into()); }
            _ => changed.global = false,
        }
        assert!(context.reduced_cost(&model, candidate.clone(), &duals, &changed).is_err());
    }
    for field in 0..4 {
        let mut invalid = prices();
        match field {
            0 => { invalid.remove("unused"); }
            1 => { invalid.insert("unknown".into(), 0.0); }
            2 => { invalid.insert("unused".into(), f64::NAN); }
            _ => { invalid.insert("unused".into(), f64::INFINITY); }
        }
        assert!(context.refresh_heuristic_duals(&model, invalid, request.clone()).is_err());
        assert!(context.validate_pricing_duals(&model, &duals, &request).is_err());
    }
    let first = context.refresh_heuristic_duals(&model, prices(), request.clone()).unwrap();
    let latest = context.refresh_heuristic_duals(&model, prices(), request.clone()).unwrap();
    assert!(context.validate_pricing_duals(&model, &first, &request).is_err());
    context.validate_pricing_duals(&model, &latest, &request).unwrap();
}

#[test]
fn verified_pricing_consumes_actual_certificate_duals_and_normalizes_both_sides_and_equalities() {
    for (lower, upper, expected, expanded) in [
        (1.0, 4.0, [2.0, 3.0, 0.0], vec![0.0, -2.0, 0.0, -3.0, 0.0, 0.0, 0.0, 0.0]),
        (1.0, 3.0, [4.0, 4.0, -1.0], vec![0.0, -4.0, 0.0, -4.0, 0.0, -1.0, 0.0, 0.0]),
        (3.0, 4.0, [1.0, 1.0, 1.0], vec![0.0, -1.0, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0]),
    ] {
        let mut rows = problem().rows().to_vec();
        rows[2].lower.as_mut().unwrap().value = lower;
        rows[2].upper.as_mut().unwrap().value = upper;
        let problem = PathCompilationProblem::new(rows, problem().columns().to_vec(), NoneUnit::INSTANT.clone()).unwrap();
        let mut context = PathCompilationContext::new(problem, "master").unwrap();
        let mut model = MetaModel::new("certificate");
        context.register(&mut model).unwrap();
        let report = report(&context, &model, expanded);
        let request = scope(&context, &model);
        let duals = context.refresh_shadow_prices(&model, &report, request.clone()).unwrap();
        assert!(duals.has_verified_lp_source());
        context.require_verified_pricing_duals(&model, &duals, &request).unwrap();
        assert_eq!(duals.prices()["a"], expected[0]);
        assert_eq!(duals.prices()["b"], expected[1]);
        assert_eq!(duals.prices()["capacity"], expected[2]);
        assert_eq!(duals.prices()["unused"], 0.0);
        assert_eq!(context.reduced_cost(&model, column("new", 1.0, [1.0, 1.0], 3.0), &duals, &request).unwrap(), -4.0);
        let certificate = VerifiedPricingDuals::new(&context.pricing_model(&model).unwrap(), &report).unwrap();
        let latest = context.refresh_verified_duals(&model, &certificate, request.clone()).unwrap();
        assert!(context.validate_pricing_duals(&model, &duals, &request).is_err());
        context.require_verified_pricing_duals(&model, &latest, &request).unwrap();
        let heuristic = context.refresh_heuristic_duals(&model, latest.prices().clone(), request.clone()).unwrap();
        assert!(!heuristic.has_verified_lp_source());
        assert!(context.require_verified_pricing_duals(&model, &heuristic, &request).is_err());
    }
}

#[test]
fn stale_unbound_missing_nonfinite_or_inconsistent_lp_reports_cannot_drive_verified_pricing() {
    let (mut context, mut model) = registered();
    let valid = report(&context, &model, vec![0.0, -2.0, 0.0, -3.0, 0.0, 0.0, 0.0, 0.0]);
    for field in 0..7 {
        let mut invalid = valid.clone();
        match field {
            0 => invalid.proof = None,
            1 => invalid.fingerprints.model = None,
            2 => { invalid.solution.as_mut().unwrap().dual_solution.as_mut().unwrap().pop(); }
            3 => invalid.solution.as_mut().unwrap().dual_solution.as_mut().unwrap()[0] = f64::NAN,
            4 => invalid.solution.as_mut().unwrap().dual_solution.as_mut().unwrap()[0] = f64::INFINITY,
            5 => { invalid.solution.as_mut().unwrap().values[2] = 0.0; }
            _ => { invalid.solution.as_mut().unwrap().objective_value = Some(4.0); invalid.solution.as_mut().unwrap().objective = Some(4.0); }
        }
        let request = scope(&context, &model);
        assert!(context.refresh_shadow_prices(&model, &invalid, request).is_err(), "field={field}");
    }
    let request = scope(&context, &model);
    let duals = context.refresh_shadow_prices(&model, &valid, request.clone()).unwrap();
    let whole = context.aggregation().compilation().variables()["whole"];
    model.set_variable_range_by_index(whole, VariableRange::fixed(0.0)).unwrap();
    assert!(context.require_verified_pricing_duals(&model, &duals, &request).is_err());
    let changed = scope(&context, &model);
    assert!(context.refresh_shadow_prices(&model, &valid, changed).is_err());
    model.set_variable_range_by_index(whole, VariableRange::bounded(0.0, 1.0)).unwrap();
    context.add_columns(&mut model, vec![column("new", 4.0, [1.0, 1.0], 2.0)]).unwrap();
    let changed = scope(&context, &model);
    assert!(context.refresh_shadow_prices(&model, &valid, changed).is_err());
    let (mut foreign, other) = registered();
    let other_scope = scope(&foreign, &other);
    assert!(foreign.refresh_shadow_prices(&other, &valid, other_scope).is_err());
}

#[test]
fn observable_objectives_rows_bounds_columns_and_context_identity_invalidate_snapshots() {
    for field in 0..7 {
        let (mut context, mut model) = registered();
        let request = scope(&context, &model);
        let duals = context.refresh_heuristic_duals(&model, prices(), request.clone()).unwrap();
        let a = context.aggregation().compilation().variables()["a"];
        match field {
            0 => model.set_variable_range_by_index(a, VariableRange::fixed(0.0)).unwrap(),
            1 => model.add_sub_objective(SubObjective::minimize(Linear::new(Vec::new(), 1.0), "external_constant")),
            2 => model.add_linear_objective(&[(a, 1.0)], "tampered_path_cost"),
            3 => { model.add_linear_constraint(&[(a, 1.0)], ConstraintRelation::LessEqual, 1.0, "unmapped").unwrap(); }
            4 => { model.retain_constraints(|constraint| !constraint.name.ends_with("0_eq")); }
            5 => { context.add_columns(&mut model, vec![column("new", 4.0, [1.0, 1.0], 3.0)]).unwrap(); }
            _ => { context.remove_columns(&mut model, &["a".into()].into()).unwrap(); }
        }
        assert!(context.validate_pricing_duals(&model, &duals, &request).is_err(), "field={field}");
        if field == 2 {
            assert!(context.extract_solution(&model).is_err());
            assert!(context.add_columns(&mut model, vec![column("new", 1.0, [1.0, 1.0], 3.0)]).is_err());
        }
    }
    let (mut first, model) = registered();
    let request = scope(&first, &model);
    let duals = first.refresh_heuristic_duals(&model, prices(), request.clone()).unwrap();
    let (mut second, other) = registered();
    let other_scope = scope(&second, &other);
    second.refresh_heuristic_duals(&other, prices(), other_scope.clone()).unwrap();
    assert!(second.validate_pricing_duals(&other, &duals, &other_scope).is_err());
    assert!(first.register(&mut MetaModel::new("foreign")).is_err());
}

#[derive(Debug)]
struct Extension {
    register: AtomicBool,
    add: AtomicBool,
    remove: AtomicBool,
}

impl PathCompilationPipeline<f64, Payload> for Extension {
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &PathCompilationAggregation<f64, Payload>) -> Result<()> {
        let index = model.register_auto_variable_with_range::<Continuous>("business", VariableRange::bounded(0.0, 1.0)).unwrap();
        model.add_linear_objective(&[(index, 7.0)], "business_cost");
        model.add_linear_constraint(&[(aggregation.compilation().variables()["whole"], 1.0)], ConstraintRelation::LessEqual, 0.0, "business_no_whole").unwrap();
        if self.register.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("注入注册失败 / injected registration failure")); }
        Ok(())
    }

    fn add_columns(&self, model: &mut MetaModel<f64>, _aggregation: &PathCompilationAggregation<f64, Payload>, _columns: &[PathColumn<f64, Payload>]) -> Result<()> {
        model.add_sub_objective(SubObjective::minimize(Linear::new(Vec::new(), 2.0), "business_added"));
        if self.add.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("注入增列失败 / injected add failure")); }
        Ok(())
    }

    fn remove_columns(&self, model: &mut MetaModel<f64>, _aggregation: &PathCompilationAggregation<f64, Payload>, _columns: &[PathColumn<f64, Payload>]) -> Result<()> {
        model.add_sub_objective(SubObjective::minimize(Linear::new(Vec::new(), 3.0), "business_removed"));
        if self.remove.swap(false, Ordering::SeqCst) { return Err(NetworkSchedulingError::validation("注入删列失败 / injected removal failure")); }
        Ok(())
    }
}

#[test]
fn downstream_constraints_and_objectives_survive_transactional_registration_addition_and_removal() {
    let mut context = PathCompilationContext::new(problem(), "master").unwrap();
    context.add_pipeline(Arc::new(Extension { register: AtomicBool::new(true), add: AtomicBool::new(true), remove: AtomicBool::new(true) })).unwrap();
    let mut model = MetaModel::new("downstream");
    assert!(context.register(&mut model).is_err());
    assert_eq!(model.tokens().len(), 0);
    assert!(model.constraints().is_empty());
    assert!(model.objective().is_empty());
    assert!(context.aggregation().columns().is_empty());
    context.register(&mut model).unwrap();
    assert_eq!(model.tokens().len(), 4);
    model.set_solution(&[0.0, 0.0, 1.0, 0.0]);
    assert!(context.extract_solution(&model).is_err());
    model.set_solution(&[1.0, 1.0, 0.0, 0.0]);
    assert_eq!(context.extract_solution(&model).unwrap().cost.value, 7.0);
    assert!(context.pricing_model(&model).is_err());
    let before = linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    let objective = format!("{:?}", model.objective_snapshot());
    assert!(context.add_columns(&mut model, vec![column("new", 1.0, [1.0, 1.0], 2.0)]).is_err());
    assert_eq!(linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
    assert_eq!(format!("{:?}", model.objective_snapshot()), objective);
    assert_eq!(context.aggregation().columns().len(), 3);
    context.add_columns(&mut model, vec![column("new", 1.0, [1.0, 1.0], 2.0)]).unwrap();
    let before = linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    let objective = format!("{:?}", model.objective_snapshot());
    assert!(context.remove_columns(&mut model, &["new".into()].into()).is_err());
    assert_eq!(linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
    assert_eq!(format!("{:?}", model.objective_snapshot()), objective);
    assert!(context.aggregation().columns().contains_key("new"));
    context.remove_columns(&mut model, &["new".into()].into()).unwrap();
    assert_eq!(model.try_to_linear_triad_model().unwrap().c[3], 7.0);
    assert!(model.constraints().iter().any(|constraint| constraint.name == "business_no_whole"));
    assert!(context.add_pipeline(Arc::new(Extension { register: AtomicBool::new(false), add: AtomicBool::new(false), remove: AtomicBool::new(false) })).is_err());
}

#[test]
fn production_extraction_rejects_missing_nonfinite_negative_fractional_and_row_violating_incumbents() {
    let (context, mut model) = registered();
    assert!(context.extract_solution(&model).is_err());
    for values in [vec![0.0; 3], vec![1.0; 3], vec![0.5, 0.5, 0.5], vec![-0.1, 0.0, 1.0],
        vec![0.0, 0.0, 1.1], vec![f64::NAN, 0.0, 1.0], vec![0.0, f64::INFINITY, 1.0], vec![0.0, 0.0, f64::NEG_INFINITY]]
    {
        model.set_solution(&values);
        assert!(context.extract_solution(&model).is_err(), "{values:?}");
    }
    assert!(context.extract_solution(&MetaModel::new("other")).is_err());
}

#[test]
fn input_preparation_rejects_invalid_rows_nonfinite_and_affine_quantities_without_panicking() {
    let original = problem();
    for field in 0..10 {
        let mut rows = original.rows().to_vec();
        let mut columns = original.columns().to_vec();
        let mut unit = NoneUnit::INSTANT.clone();
        match field {
            0 => rows.push(rows[0].clone()),
            1 => rows[0].id = " ".into(),
            2 => { rows[0].lower = None; rows[0].upper = None; }
            3 => rows[0].lower.as_mut().unwrap().value = 2.0,
            4 => rows[0].unit = Celsius::INSTANT.clone(),
            5 => rows[0].upper.as_mut().unwrap().unit = Celsius::INSTANT.clone(),
            6 => columns[0].cost.unit = Celsius::INSTANT.clone(),
            7 => columns[0].coefficients.get_mut("capacity").unwrap().unit = Celsius::INSTANT.clone(),
            8 => unit = Celsius::INSTANT.clone(),
            _ => rows[0].lower.as_mut().unwrap().value = f64::INFINITY,
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| PathCompilationProblem::new(rows, columns, unit)));
        assert!(result.is_ok(), "field={field}");
        assert!(result.unwrap().is_err(), "field={field}");
    }
    let unit = Kelvin::INSTANT.clone();
    let problem = PathCompilationProblem::new(vec![PathCompilationRow { id: "temperature_difference".into(), unit: unit.clone(),
        lower: Some(Quantity::new(1.0, unit.clone())), upper: None }], vec![PathColumn { id: "path".into(), payload: (),
        cost: Quantity::new(1.0, NoneUnit::INSTANT.clone()), coefficients: [("temperature_difference".into(), Quantity::new(1.0, unit))].into() }],
        NoneUnit::INSTANT.clone()).unwrap();
    let mut context = PathCompilationContext::new(problem, "linear").unwrap();
    let mut model = MetaModel::new("linear");
    context.register(&mut model).unwrap();
    model.set_solution(&[1.0]);
    assert_eq!(context.extract_solution(&model).unwrap().cost.value, 1.0);
}

#[test]
fn invalid_new_columns_scope_and_retired_token_reactivation_are_rejected_without_partial_mutation() {
    let (mut context, mut model) = registered();
    let fingerprint = linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    for field in 0..6 {
        let mut candidate = column("new", 1.0, [1.0, 1.0], 3.0);
        match field {
            0 => candidate.id = " ".into(),
            1 => { candidate.coefficients.insert("unknown".into(), Quantity::new(0.0, NoneUnit::INSTANT.clone())); }
            2 => candidate.cost.value = f64::NAN,
            3 => candidate.cost.value = f64::INFINITY,
            4 => candidate.coefficients.get_mut("capacity").unwrap().value = f64::NEG_INFINITY,
            _ => candidate.coefficients.get_mut("capacity").unwrap().unit = Celsius::INSTANT.clone(),
        }
        assert!(context.add_columns(&mut model, vec![candidate]).is_err());
        assert_eq!(context.aggregation().columns().len(), 3);
        assert_eq!(linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), fingerprint);
    }
    let request = scope(&context, &model);
    for field in 0..6 {
        let mut invalid = request.clone();
        match field {
            0 => invalid.request.clear(),
            1 => invalid.model.push('x'),
            2 => invalid.dual.clear(),
            3 => invalid.branch.clear(),
            4 => { invalid.groups.insert(" ".into()); }
            _ => { invalid.executor_slots.insert(" ".into()); }
        }
        assert!(context.refresh_heuristic_duals(&model, prices(), invalid).is_err());
    }
    let retired = context.aggregation().compilation().variables()["a"];
    context.remove_columns(&mut model, &["a".into()].into()).unwrap();
    model.set_variable_range_by_index(retired, VariableRange::bounded(0.0, 1.0)).unwrap();
    assert!(context.pricing_model(&model).is_err());
    assert!(context.add_columns(&mut model, vec![column("new", 1.0, [1.0, 1.0], 3.0)]).is_err());
    model.set_solution(&[0.0, 0.0, 1.0]);
    assert!(context.extract_solution(&model).is_err());
}

#[cfg(feature = "scip")]
#[test]
fn scip_path_compilation_service_executes_finite_optimum_and_current_lp_certificate() {
    use ospf_rust_core::solver::solvers::SCIPSolver;
    use crate::application::PathCompilationApplicationService;
    let service = PathCompilationApplicationService::new(SCIPSolver::new());
    let report = service.solve(&problem()).unwrap();
    assert!(report.is_optimal());
    let solution = report.incumbent().unwrap();
    assert_eq!(solution.cost.value, 5.0);
    assert_eq!(solution.selected.len(), 1);
    assert_eq!(solution.selected[0].id, "whole");
    assert_eq!(solution.candidate_ids.len(), 3);
    let (mut context, mut model) = registered();
    let lp = context.pricing_model(&model).unwrap();
    let report = SCIPSolver::new().solve_linear_report(&lp).unwrap();
    let request = scope(&context, &model);
    let duals = context.refresh_shadow_prices(&model, &report, request.clone()).unwrap();
    context.require_verified_pricing_duals(&model, &duals, &request).unwrap();
    assert!(context.reduced_cost(&model, column("new", 1.0, [1.0, 1.0], 3.0), &duals, &request).unwrap() < 0.0);
    context.add_columns(&mut model, vec![column("new", 1.0, [1.0, 1.0], 3.0)]).unwrap();
    assert!(context.require_verified_pricing_duals(&model, &duals, &request).is_err());
    let mut rows = problem().rows().to_vec();
    rows[0].lower.as_mut().unwrap().value = 4.0;
    rows[0].upper.as_mut().unwrap().value = 4.0;
    let infeasible = PathCompilationProblem::new(rows, problem().columns().to_vec(), NoneUnit::INSTANT.clone()).unwrap();
    let report = service.solve(&infeasible).unwrap();
    assert_eq!(report.problem_status, ProblemStatus::Infeasible);
    assert!(!report.has_incumbent());
}

#[test]
fn empty_pools_keep_required_rows_and_allow_zero_expression_models() {
    let unit = NoneUnit::INSTANT.clone();
    for required in [false, true] {
        let problem = PathCompilationProblem::<f64, ()>::new(vec![PathCompilationRow {
            id: "required".into(), unit: unit.clone(), lower: Some(Quantity::new(if required { 1.0 } else { 0.0 }, unit.clone())),
            upper: None,
        }], Vec::new(), unit.clone()).unwrap();
        let mut context = PathCompilationContext::new(problem, "empty").unwrap();
        let mut model = MetaModel::new("empty");
        context.register(&mut model).unwrap();
        assert_eq!(model.constraints().len(), 1);
        assert!(model.tokens().is_empty());
        let result = context.extract_solution(&model);
        assert_eq!(result.is_err(), required);
        if let Ok(solution) = result {
            assert!(solution.selected.is_empty());
            assert!(solution.candidate_ids.is_empty());
            assert_eq!(solution.cost.value, 0.0);
            assert_eq!(solution.usage["required"].value, 0.0);
        }
    }
}

#[cfg(any(feature = "big-decimal", feature = "big-rational"))]
fn generic_path_compilation_roundtrip<V>()
where
    V: ospf_rust_core::solver::value::SolveValue + ospf_rust_quantities::unit::UnitConversionValue,
{
    let adapter = crate::infrastructure::NetworkSchedulingSolverValueAdapter::<V>::default();
    let value = |number| adapter.from_solver(number).unwrap();
    let unit = Kilogram::INSTANT.clone();
    let cost = NoneUnit::INSTANT.clone();
    let column = PathColumn { id: "whole".into(), payload: vec!["start", "transfer", "end"], cost: Quantity::new(value(5.0), cost.clone()),
        coefficients: [("load".into(), Quantity::new(value(1000.0), Gram::INSTANT.clone()))].into() };
    let problem = PathCompilationProblem::new(vec![PathCompilationRow { id: "load".into(), unit: unit.clone(),
        lower: Some(Quantity::new(value(1.0), unit.clone())), upper: Some(Quantity::new(value(1.0), unit)) }],
        vec![column.clone()], cost).unwrap();
    let mut context = PathCompilationContext::new(problem, "quantities").unwrap();
    let mut model = MetaModel::new("quantities");
    context.register(&mut model).unwrap();
    assert!(context.add_columns(&mut model, vec![column]).unwrap().is_empty());
    model.set_solution(&[1.0]);
    let solution = context.extract_solution(&model).unwrap();
    assert_eq!(adapter.to_solver(&solution.cost.value).unwrap(), 5.0);
    assert_eq!(adapter.to_solver(&solution.usage["load"].value).unwrap(), 1.0);
    assert_eq!(solution.selected[0].payload, ["start", "transfer", "end"]);
}

#[cfg(feature = "big-decimal")]
#[test]
fn path_compilation_big_decimal_registers_and_extracts_physical_quantities() { generic_path_compilation_roundtrip::<bigdecimal::BigDecimal>(); }

#[cfg(feature = "big-rational")]
#[test]
fn path_compilation_big_rational_registers_and_extracts_physical_quantities() { generic_path_compilation_roundtrip::<num_rational::BigRational>(); }
