use std::collections::{BTreeMap, BTreeSet};
use num_traits::One;
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::flatten::Linear;
use ospf_rust_core::model::{MetaModel, SubObjective};
use ospf_rust_core::variable::{Binary, VariableType};
use ospf_rust_framework_bpp::infrastructure::{RuntimePackingValueAdapter, SameUnitPackingValueAdapter};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Kilogram, Meter};
use crate::domain::*;

fn problem() -> PackingProblem<f64, Meter, Kilogram> {
    PackingProblem {
        items: vec![Item { id: "a".into(), length: Quantity::new_ct(4.0), weight: Quantity::new_ct(3.0), amount: 2,
            materials: BTreeMap::from([("steel".into(), 2)]) },
            Item { id: "b".into(), length: Quantity::new_ct(2.0), weight: Quantity::new_ct(1.0), amount: 2, materials: BTreeMap::new() }],
        bin_types: vec![BinType { id: "box".into(), length: Quantity::new_ct(7.0), max_weight: Quantity::new_ct(5.0), cost: 2.0, inventory: Some(2) }],
        spacing: Quantity::new_ct(1.0), length_unit: Meter, weight_unit: Kilogram, objective: PackingObjective::BinCount,
    }
}

fn context() -> PackingContext<f64, Meter, Kilogram> {
    PackingContext::new(problem(), &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap()
}

fn pattern(amounts: &[(&str, u64)]) -> PackingPattern {
    PackingPattern { bin_type_id: "box".into(), amounts: amounts.iter().map(|(id, count)| (id.to_string(), *count)).collect() }
}

#[test]
fn spacing_has_no_trailing_gap_and_patterns_keep_identity() {
    let packing = context();
    packing.validate_pattern(&pattern(&[("a", 1), ("b", 1)])).unwrap();
    assert!(packing.validate_pattern(&pattern(&[("a", 2)])).is_err());
    assert!(packing.validate_pattern(&pattern(&[("a", 0)])).is_err());
    assert!(packing.validate_pattern(&pattern(&[("unknown", 1)])).is_err());
    assert!(packing.validate_pattern(&pattern(&[])).is_err());
    let mut over = problem();
    over.bin_types[0].max_weight.value = 3.0;
    let packing = PackingContext::new(over, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    assert!(packing.validate_pattern(&pattern(&[("a", 1), ("b", 1)])).is_err());
    assert_ne!(pattern(&[("a", 1)]).key(), pattern(&[("b", 1)]).key());
}

#[test]
fn validation_rejects_nonfinite_duplicate_and_negative_input() {
    for case in 0..5 {
        let mut input = problem();
        match case {
            0 => input.spacing.value = f64::NAN,
            1 => input.items[0].length.value = 0.0,
            2 => input.items[0].weight.value = -1.0,
            3 => input.bin_types[0].cost = f64::INFINITY,
            _ => input.items[1].id = input.items[0].id.clone(),
        }
        assert!(PackingContext::new(input, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).is_err());
    }
}

#[test]
fn enumeration_is_complete_only_when_every_node_is_exhausted() {
    let packing = context();
    let result = packing.generate_patterns(PatternGenerationConfig::default(), &AcceptAllPatterns).unwrap();
    assert!(result.complete);
    assert!(result.visited_nodes > 0);
    assert_eq!(result.patterns.into_iter().collect::<BTreeSet<_>>(), BTreeSet::from([
        pattern(&[("a", 1)]), pattern(&[("b", 1)]), pattern(&[("b", 2)]), pattern(&[("a", 1), ("b", 1)]),
    ]));
    let result = packing.generate_patterns(PatternGenerationConfig { max_nodes: 2, max_patterns: 100 }, &AcceptAllPatterns).unwrap();
    assert!(!result.complete);
    assert_eq!(result.visited_nodes, 2);
    let result = packing.generate_patterns(PatternGenerationConfig { max_nodes: 100, max_patterns: 1 }, &AcceptAllPatterns).unwrap();
    assert!(!result.complete);
    assert_eq!(result.patterns.len(), 1);
    assert!(packing.generate_patterns(PatternGenerationConfig { max_nodes: 0, max_patterns: 1 }, &AcceptAllPatterns).is_err());
}

#[test]
fn policies_propagate_errors_and_zero_stock_yields_no_candidates() {
    struct FailedPolicy;
    impl PatternPolicy for FailedPolicy {
        fn accept(&self, _: &PackingPattern) -> Result<bool> { Err(ModelError::InvalidConstraint("policy failed".into()).into()) }
    }
    assert!(context().generate_patterns(PatternGenerationConfig::default(), &FailedPolicy).is_err());
    assert!(context().initial_patterns(false, 100, &FailedPolicy).is_err());
    let mut input = problem();
    input.bin_types[0].inventory = Some(0);
    let packing = PackingContext::new(input, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    let result = packing.generate_patterns(PatternGenerationConfig::default(), &AcceptAllPatterns).unwrap();
    assert!(result.complete);
    assert!(result.patterns.is_empty());
    assert!(packing.initial_patterns(true, 100, &AcceptAllPatterns).unwrap().is_empty());
}

#[test]
fn ffd_and_bfd_seeds_are_valid_and_bounded_but_not_completeness_proofs() {
    for best_fit in [false, true] {
        let packing = context();
        let seeds = packing.initial_patterns(best_fit, 4, &AcceptAllPatterns).unwrap();
        assert!(seeds.contains(&pattern(&[("a", 1), ("b", 1)])));
        for seed in seeds { packing.validate_pattern(&seed).unwrap(); }
        assert!(packing.initial_patterns(best_fit, 3, &AcceptAllPatterns).is_err());
    }
}

#[test]
fn packing_verifies_demands_inventory_materials_and_exact_coordinates() {
    let packing = context();
    let usages = BTreeMap::from([(pattern(&[("a", 1), ("b", 1)]), 2)]);
    let result = packing.pack(&usages, 4).unwrap();
    assert_eq!(result.plans.len(), 2);
    assert_eq!(result.plans[0].placements[1].start.value, 5.0);
    assert_eq!(result.materials["steel"], 4);
    assert_eq!(result.total_length.value, 14.0);
    assert_eq!(result.packed_length.value, 12.0);
    assert_eq!(result.free_intervals.len(), 2);
    assert_eq!(result.free_intervals[0].length.value, 1.0);
    assert_eq!(result.objective, 2.0);
    assert!(packing.pack(&usages, 3).is_err());
    assert!(packing.verify_usages(&BTreeMap::from([(pattern(&[("a", 1)]), 2)])).is_err());
    for case in 0..4 {
        let mut plans = result.plans.clone();
        match case {
            0 => plans[0].placements[1].start.value = 4.5,
            1 => plans[0].placements[0].length.value = 1.0,
            2 => plans[1].bin_id = plans[0].bin_id.clone(),
            _ => plans[0].placements[0].start.value = f64::NAN,
        }
        assert!(packing.verify_plans(plans).is_err());
    }
}

#[test]
fn all_registration_stages_share_rows_objective_and_full_pool() {
    let packing = context();
    let pool = packing.generate_patterns(Default::default(), &AcceptAllPatterns).unwrap().patterns;
    for stage in [PackingModelStage::Milp, PackingModelStage::Lp, PackingModelStage::FinalMilp] {
        let assignment = PatternAssignmentContext::new(packing.clone(), pool.clone(), stage, NoPatternExtension).unwrap();
        let snapshot = assignment.model().linear_model_snapshot().unwrap();
        assert_eq!(snapshot.variables.len(), 4);
        assert_eq!(snapshot.rows.len(), 3);
        assert_eq!(snapshot.objective.len(), 4);
        assert!(snapshot.variables.iter().all(|state| state.var_type.is_integer() == (stage != PackingModelStage::Lp)));
        let final_master = assignment.final_master().unwrap();
        assert_eq!(final_master.patterns(), assignment.patterns());
        assert_ne!(final_master.model().model_identity(), assignment.model().model_identity());
    }
}

#[derive(Clone)]
struct Extension {
    fail_after_columns: usize,
}

impl PatternModelingExtension<f64, Meter, Kilogram> for Extension {
    fn register(
        &self,
        model: &mut MetaModel,
        _: &PackingContext<f64, Meter, Kilogram>,
        aggregation: &PatternAggregation<f64>,
        _: PackingModelStage,
    ) -> Result<()> {
        let helper = model.register_auto_variable::<Binary>("extension_helper")?;
        model.add_le_constraint(&[(helper, 1.0)], 0.0, "extension_limit")?;
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![], 7.0), "extension_cost"));
        if aggregation.variables.len() > self.fail_after_columns { return Err(ModelError::InvalidConstraint("late extension failure".into()).into()); }
        Ok(())
    }
}

#[test]
fn extension_helpers_relax_for_lp_restore_for_final_and_validate_on_extraction() {
    let pool = vec![pattern(&[("a", 1), ("b", 1)])];
    let extension = Extension { fail_after_columns: usize::MAX };
    let lp = PatternAssignmentContext::new(context(), pool.clone(), PackingModelStage::Lp, extension.clone()).unwrap();
    assert!(lp.model().linear_model_snapshot().unwrap().variables.iter().all(|state| state.var_type == VariableType::Continuous));
    let final_master = lp.final_master().unwrap();
    let snapshot = final_master.model().linear_model_snapshot().unwrap();
    assert_eq!(snapshot.variables[1].var_type, VariableType::Binary);
    assert_eq!(snapshot.objective_constant, 7.0);
    let solution = final_master.extract_solution(&[2.0, 0.0], &1e-8).unwrap();
    assert_eq!(solution.usages[&pool[0]], 2);
    assert!(final_master.extract_solution(&[2.0, 1.0], &1e-8).is_err());
    assert!(final_master.extract_solution(&[2.0], &1e-8).is_err());
    assert!(lp.extract_solution(&[2.0, 0.0], &1e-8).is_err());
}

#[derive(Clone)]
struct CancellingObjective;

impl PatternModelingExtension<f64, Meter, Kilogram> for CancellingObjective {
    fn register(
        &self,
        model: &mut MetaModel,
        _: &PackingContext<f64, Meter, Kilogram>,
        _: &PatternAggregation<f64>,
        _: PackingModelStage,
    ) -> Result<()> {
        use ospf_rust_core::model::flatten::LinearMonomial;
        use ospf_rust_core::variable::{Continuous, VariableRange};
        let mut terms = Vec::new();
        for (index, coefficient) in [1e16, -1.0, -1e16].into_iter().enumerate() {
            let variable = model.register_auto_variable_with_range::<Continuous>(&format!("objective_aux_{index}"), VariableRange::bounded(1.0, 1.0))?;
            terms.push(LinearMonomial::new(coefficient, variable));
        }
        model.add_sub_objective(SubObjective::minimize(Linear::new(terms, 7.0), "cancelling"));
        Ok(())
    }
}

#[test]
fn application_accepts_exact_cancelling_extension_objective() {
    use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveHandle, SolveProof, SolveReport, SolveSolution, TerminationReason,
        linear_model_fingerprint, linear_model_mapping};
    use crate::application::Bpp1dMilp;
    let mut input = problem();
    input.items.truncate(1);
    input.items[0].length.value = 7.0;
    input.items[0].amount = 1;
    input.objective = PackingObjective::Cost;
    let packing = PackingContext::new(input, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    let result = Bpp1dMilp::solve_with_extension(packing, CancellingObjective, &AcceptAllPatterns, Default::default(),
        &SolveHandle::new(), |model, _, _| {
            assert_eq!(model.num_variables(), 4);
            let mut solution = SolveSolution::vector(vec![1.0; 4]);
            solution.objective_value = Some(1.0);
            SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution).proof(SolveProof::optimality())
                .model_mapping(Some(linear_model_mapping(model)?))
                .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
        }).unwrap();
    assert!(result.packing.is_some());
    assert_eq!(result.report.solution.unwrap().objective_value, Some(1.0));
    assert!(!result.globally_optimal);
}

#[test]
fn column_batch_and_extension_failure_leave_old_model_pool_and_indices_intact() {
    let seed = pattern(&[("a", 1)]);
    let mut assignment = PatternAssignmentContext::new(context(), vec![seed.clone()], PackingModelStage::Milp,
        Extension { fail_after_columns: 1 }).unwrap();
    let before = assignment.model().linear_model_snapshot().unwrap();
    assert!(assignment.add_columns(vec![pattern(&[("b", 1)]), pattern(&[("unknown", 1)])]).is_err());
    assert_eq!(assignment.model().linear_model_snapshot().unwrap(), before);
    assert!(assignment.add_columns(vec![pattern(&[("b", 1)])]).is_err());
    assert_eq!(assignment.model().linear_model_snapshot().unwrap(), before);
    assert_eq!(assignment.patterns(), &BTreeSet::from([seed.clone()]));
    assignment.remove_columns(&BTreeSet::from([seed])).unwrap();
    assert_eq!(assignment.model().num_tokens(), 1);
    assert!(assignment.patterns().is_empty());
}

#[test]
fn fractional_and_nonfinite_solution_values_never_become_pattern_usages() {
    let assignment = PatternAssignmentContext::new(context(), vec![pattern(&[("a", 1), ("b", 1)])],
        PackingModelStage::Milp, NoPatternExtension).unwrap();
    assert_eq!(assignment.extract_solution(&[1.999999999], &1e-8).unwrap().usages.values().next(), Some(&2));
    for value in [1.5, f64::NAN, f64::INFINITY, -1.0] {
        assert!(assignment.extract_solution(&[value], &1e-8).is_err());
    }
    assert!(assignment.extract_solution(&[2.0], &0.5).is_err());
}

#[test]
fn high_precision_capacities_never_pass_through_f64() {
    use std::str::FromStr;
    use bigdecimal::BigDecimal;
    let tiny = BigDecimal::from_str("0.00000000000000000000000000001").unwrap();
    let one = BigDecimal::one();
    let input = PackingProblem {
        items: vec![Item { id: "a".into(), length: Quantity::<_, Meter>::new_ct(one.clone() + tiny),
            weight: Quantity::<_, Kilogram>::new_ct(one.clone()), amount: 1, materials: BTreeMap::new() }],
        bin_types: vec![BinType { id: "box".into(), length: Quantity::new_ct(one.clone()), max_weight: Quantity::new_ct(one.clone()),
            cost: one.clone(), inventory: None }],
        spacing: Quantity::new_ct(BigDecimal::from(0)), length_unit: Meter, weight_unit: Kilogram, objective: PackingObjective::Cost,
    };
    let packing = PackingContext::new(input, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    assert!(packing.validate_pattern(&pattern(&[("a", 1)])).is_err());
    assert!(packing.generate_patterns(Default::default(), &AcceptAllPatterns).unwrap().patterns.is_empty());
    use num_rational::BigRational;
    let one = BigRational::one();
    let half = one.clone() / BigRational::from_integer(2.into());
    let input = PackingProblem {
        items: vec![Item { id: "a".into(), length: Quantity::<_, Meter>::new_ct(half),
            weight: Quantity::<_, Kilogram>::new_ct(one.clone()), amount: 2, materials: BTreeMap::new() }],
        bin_types: vec![BinType { id: "box".into(), length: Quantity::new_ct(one.clone()), max_weight: Quantity::new_ct(one.clone() + one.clone()),
            cost: one, inventory: None }],
        spacing: Quantity::new_ct(BigRational::from_integer(0.into())), length_unit: Meter, weight_unit: Kilogram, objective: PackingObjective::BinCount,
    };
    let packing = PackingContext::new(input, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    let assignment = PatternAssignmentContext::new(packing.clone(), vec![pattern(&[("a", 2)])], PackingModelStage::Milp, NoPatternExtension).unwrap();
    assert_eq!(assignment.extract_solution(&[BigRational::one()], &BigRational::from_integer(0.into())).unwrap().usages[&pattern(&[("a", 2)])], 1);
    assert_eq!(packing.pack(&BTreeMap::from([(pattern(&[("a", 2)]), 1)]), 2).unwrap().packed_length.value, BigRational::one());
}

#[test]
fn runtime_mixed_units_are_normalized_explicitly() {
    use ospf_rust_quantities::unit::Centimeter;
    let meter = Meter::INSTANT;
    let centimeter = Centimeter::INSTANT;
    let kilogram = Kilogram::INSTANT;
    let input = PackingProblem {
        items: vec![Item { id: "a".into(), length: Quantity::new(50.0, centimeter.clone()),
            weight: Quantity::new(1.0, kilogram.clone()), amount: 2, materials: BTreeMap::new() }],
        bin_types: vec![BinType { id: "box".into(), length: Quantity::new(1.0, meter.clone()),
            max_weight: Quantity::new(2.0, kilogram.clone()), cost: 1.0, inventory: None }],
        spacing: Quantity::new(0.0, meter.clone()), length_unit: meter.clone(), weight_unit: kilogram.clone(), objective: PackingObjective::BinCount,
    };
    assert!(PackingContext::new(input.clone(), &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).is_err());
    let packing = PackingContext::new(input, &RuntimePackingValueAdapter, &RuntimePackingValueAdapter).unwrap();
    assert_eq!(packing.problem().items[0].length.value, 0.5);
    packing.validate_pattern(&pattern(&[("a", 2)])).unwrap();
}

#[test]
fn csv_closed_schema_quotes_counts_and_native_parser_are_strict() {
    use crate::infrastructure::{PackingCsvOptions, load_packing_csv};
    let load = |items: &str, bins: &str| load_packing_csv(items, bins, PackingCsvOptions {
        length_unit: Meter, weight_unit: Kilogram, spacing: Quantity::new_ct(0.0), objective: PackingObjective::BinCount,
    }, |text| text.parse::<f64>().map_err(|_| ModelError::InvalidConstraint("number".into()).into()),
        &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter);
    let bins = "id,length,max_weight,cost,inventory\r\nbox,10,10,1,";
    let packing = load("\u{feff}id,length,weight,amount\r\n\"a,b\",4,2,2", bins).unwrap();
    assert_eq!(packing.problem().items[0].id, "a,b");
    assert_eq!(packing.problem().bin_types[0].inventory, None);
    for text in [
        "id,length,weight,amount\na,4,2,+1",
        "id,length,weight,amount\na,4,2,-1",
        "id,length,weight,amount\na,4,2,18446744073709551616",
        "id,length,weight,amount\n\"a,4,2,1",
        "id,length,weight,amount\n\"a\"x,4,2,1",
        "id,length,weight,amount\na,4,2,1,extra",
        "id,length,weight,length\na,4,2,1",
        "id,length,weight,amount\na,NaN,2,1",
        "id,length,weight,amount\na,4,2,1\na,4,2,1",
    ] { assert!(load(text, bins).is_err(), "{text}"); }
    use std::str::FromStr;
    use bigdecimal::BigDecimal;
    let precise = load_packing_csv("id,length,weight,amount\na,1.00000000000000000000000000001,0,1",
        "id,length,max_weight,cost,inventory\nbox,1,1,1,", PackingCsvOptions {
            length_unit: Meter, weight_unit: Kilogram, spacing: Quantity::new_ct(BigDecimal::from(0)), objective: PackingObjective::BinCount,
        }, |text| BigDecimal::from_str(text).map_err(|_| ModelError::InvalidConstraint("decimal".into()).into()),
        &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    assert!(precise.validate_pattern(&pattern(&[("a", 1)])).is_err());
}

#[test]
fn tail_preserves_prefix_coordinates_and_subtracts_demand_and_stock() {
    let packing = context();
    let current = packing.pack(&BTreeMap::from([(pattern(&[("a", 1), ("b", 1)]), 2)]), 4).unwrap();
    let result = packing.optimize_tail(&current, 1, |local| {
        assert_eq!(local.problem().items[0].amount, 1);
        assert_eq!(local.problem().items[1].amount, 1);
        assert_eq!(local.problem().bin_types[0].inventory, Some(1));
        Ok(local.pack(&BTreeMap::from([(pattern(&[("a", 1), ("b", 1)]), 1)]), 2)?.plans)
    }, |complete| {
        assert_eq!(complete.plans.len(), 2);
        Ok(())
    }).unwrap();
    assert_eq!(result.plans[0].bin_id, current.plans[0].bin_id);
    assert_eq!(result.plans[0].placements[1].start.value, current.plans[0].placements[1].start.value);
    assert_ne!(result.plans[0].bin_id, result.plans[1].bin_id);
    let mut forged = current.clone();
    forged.plans[0].placements[1].start.value = 0.0;
    assert!(packing.optimize_tail(&forged, 1, |_| panic!("invalid current must not be solved"), |_| Ok(())).is_err());
    assert!(packing.optimize_tail(&current, 0, |_| unreachable!(), |_| Ok(())).is_err());
    assert!(packing.optimize_tail(&current, 1, |local| {
        Ok(local.pack(&BTreeMap::from([(pattern(&[("a", 1), ("b", 1)]), 1)]), 2)?.plans)
    }, |complete| {
        assert_eq!(complete.plans.len(), 2);
        Err(ModelError::InvalidConstraint("global acceptance veto".into()).into())
    }).is_err());
    packing.verify_plans(current.plans.clone()).unwrap();
}

#[test]
fn kpi_and_render_revalidate_actual_plans_and_keep_display_conversion_explicit() {
    use crate::infrastructure::PackingRendererAdapter;
    let packing = context();
    let mut result = packing.pack(&BTreeMap::from([(pattern(&[("a", 1), ("b", 1)]), 2)]), 4).unwrap();
    result.packed_length.value = 999.0;
    let kpi = packing.kpi(&result).unwrap();
    assert_eq!(kpi.bin_count, 2);
    assert_eq!(kpi.item_count, 4);
    assert_eq!(kpi.cost, 4.0);
    assert_eq!(kpi.remnant_length.value, 0.0);
    assert_eq!(kpi.utilization, 12.0 / 14.0);
    let schema = PackingRendererAdapter::to_schema(&packing, &result, &Meter, &SameUnitPackingValueAdapter, |value| Ok(*value)).unwrap();
    assert_eq!(schema.bins.len(), 2);
    assert_eq!(schema.bins[0].items[1].start, 5.0);
    assert_eq!(schema.utilization, 12.0 / 14.0);
    assert!(PackingRendererAdapter::to_schema(&packing, &result, &Meter, &SameUnitPackingValueAdapter, |_| Ok(f64::NAN)).is_err());
    result.plans[0].placements[0].length.value = 1.0;
    assert!(packing.kpi(&result).is_err());
    assert!(PackingRendererAdapter::to_schema(&packing, &result, &Meter, &SameUnitPackingValueAdapter, |value| Ok(*value)).is_err());
}

#[test]
fn one_dimensional_loading_adapter_uses_shared_nontransitive_conflicts_and_spaces() {
    use ospf_rust_framework_bpp::{DiscreteLoadingProblem, LoadingInterval, LoadingPosition, PositionConflict};
    let intervals = [("a", "main", 0.0, 2.0), ("b", "main", 1.0, 3.0), ("c", "main", 2.0, 4.0), ("other", "lower", 0.0, 3.0)]
        .into_iter().map(|(id, space, start, end)| LoadingInterval { id: id.into(), space_id: space.into(), start: Quantity::<_, Meter>::new_ct(start), end: Quantity::new_ct(end) }).collect::<Vec<_>>();
    let problem = DiscreteLoadingProblem {
        items: vec![], positions: ["a", "b", "c", "other"].into_iter().map(|id| LoadingPosition {
            id: id.into(), max_count: 1, max_weight: Quantity::<_, Kilogram>::new_ct(1.0), fixed_occupied: false, enabled: true,
        }).collect(), allowed_assignments: BTreeSet::new(), conflicts: BTreeSet::new(), layout_groups: vec![], weight_unit: Kilogram,
    };
    let context = Bpp1dDiscreteLoading::context(problem.clone(), &intervals, &Meter, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
    assert_eq!(context.conflict_index().edges(), &BTreeSet::from([PositionConflict::new("a", "b"), PositionConflict::new("b", "c")]));
    assert!(context.problem().layout_groups.is_empty());
    assert!(Bpp1dDiscreteLoading::context(problem, &intervals[..1], &Meter, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).is_err());
}

mod application {
    use std::time::Duration;
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    use ospf_rust_core::solver::audit::linear_model_mapping;
    use ospf_rust_core::solver::cancellation::{CancellationOrigin, SolveHandle};
    use ospf_rust_core::solver::fingerprint::linear_model_fingerprint;
    use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveReport, SolveSolution, TerminationReason};
    use crate::application::*;
    use super::*;

    fn certified_report(model: &LinearTriadModel, values: Vec<f64>) -> Result<SolveReport<f64>> {
        let mut solution = SolveSolution::vector(values);
        solution.objective_value = Some(solution.values.iter().zip(&model.c).map(|(value, coefficient)| value * coefficient).sum());
        if !model.has_integer_variables() {
            let mut dual = vec![0.0; model.num_constraints()];
            // 本夹具两个需求等式的 GE 拆行。 / GE split rows of this fixture's two demand equalities.
            dual[1] = -0.5;
            dual[3] = -0.5;
            solution.dual_solution = Some(dual);
        }
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution).proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() })
            .model_mapping(Some(linear_model_mapping(model)?)).build()
    }

    fn ab_solution(model: &LinearTriadModel) -> Vec<f64> {
        let mut values = vec![0.0; model.num_variables()];
        let index = (0..model.num_variables()).find(|index| model.basic.A.rows[0].entries.contains(&(*index, 1.0))
            && model.basic.A.rows[2].entries.contains(&(*index, 1.0))).unwrap();
        values[index] = 2.0;
        values
    }

    #[test]
    fn ordinary_milp_checks_actual_report_and_candidate_completeness() {
        let solved = Bpp1dMilp::solve(context(), Default::default(), &SolveHandle::new(), |model, remaining, _| {
            assert!(remaining > Duration::ZERO && remaining <= Duration::from_secs(60));
            assert!(model.has_integer_variables());
            certified_report(model, ab_solution(model))
        }).unwrap();
        assert!(solved.candidate_complete);
        assert!(solved.globally_optimal);
        assert!(!solved.globally_infeasible);
        assert_eq!(solved.packing.unwrap().plans.len(), 2);
        assert_eq!(solved.column_count, 4);
    }

    #[test]
    fn cg_verifies_lp_duals_and_final_uses_full_pool_not_only_active_columns() {
        let mut calls = 0;
        let solved = Bpp1dColumnGeneration::solve(context(), Default::default(), &SolveHandle::new(), |model, _, _| {
            calls += 1;
            assert_eq!(model.num_variables(), 4);
            if calls == 1 { assert!(!model.has_integer_variables()); } else { assert!(model.has_integer_variables()); }
            certified_report(model, ab_solution(model))
        }).unwrap();
        assert_eq!(calls, 2);
        assert_eq!(solved.pricing_iterations, 1);
        assert!(solved.pricing_converged);
        assert!(solved.globally_optimal);
        assert_eq!(solved.column_count, 4);
    }

    #[test]
    fn incomplete_candidates_keep_candidate_model_optimality_scoped_to_that_model() {
        let solved = Bpp1dColumnGeneration::solve(context(), Bpp1dSolveConfig {
            generation: PatternGenerationConfig { max_nodes: 1, ..Default::default() }, ..Default::default()
        }, &SolveHandle::new(), |model, _, _| certified_report(model, ab_solution(model))).unwrap();
        assert!(solved.report.is_optimal());
        assert!(solved.packing.is_some());
        assert!(!solved.candidate_complete);
        assert!(!solved.globally_optimal);
        assert!(!solved.globally_infeasible);
    }

    #[test]
    fn invalid_lp_proof_or_duals_never_reach_final_or_claim_convergence() {
        for case in 0..4 {
            let mut calls = 0;
            assert!(Bpp1dColumnGeneration::solve(context(), Default::default(), &SolveHandle::new(), |model, _, _| {
                calls += 1;
                let mut report = certified_report(model, ab_solution(model))?;
                match case {
                    0 => report.model_mapping = None,
                    1 => report.proof = None,
                    2 => report.solution.as_mut().unwrap().dual_solution.as_mut().unwrap()[1] = -9.0,
                    _ => report.solution.as_mut().unwrap().dual_solution.as_mut().unwrap().clear(),
                }
                Ok(report)
            }).is_err());
            assert_eq!(calls, 1);
        }
    }

    #[test]
    fn zero_budget_and_cancellation_do_not_launch_a_backend() {
        let config = Bpp1dSolveConfig { time_limit: Duration::ZERO, ..Default::default() };
        let solved = Bpp1dMilp::solve(context(), config, &SolveHandle::new(), |_, _, _| panic!("no budget")).unwrap();
        assert_eq!(solved.report.termination_reason, TerminationReason::TimeLimit);
        let handle = SolveHandle::new();
        handle.cancel(CancellationOrigin::User);
        let solved = Bpp1dColumnGeneration::solve(context(), Default::default(), &handle, |_, _, _| panic!("cancelled")).unwrap();
        assert_eq!(solved.report.termination_reason, TerminationReason::Cancelled);
        let mut calls = 0;
        let handle = SolveHandle::new();
        let solved = Bpp1dColumnGeneration::solve(context(), Default::default(), &handle, |model, _, handle| {
            calls += 1;
            handle.cancel(CancellationOrigin::User);
            certified_report(model, ab_solution(model))
        }).unwrap();
        assert_eq!(calls, 1);
        assert!(solved.packing.is_none());
        assert!(!solved.pricing_converged);
    }

    #[test]
    fn inventory_infeasible_seed_lp_goes_to_full_final_without_convergence() {
        let mut calls = 0;
        let solved = Bpp1dColumnGeneration::solve(context(), Default::default(), &SolveHandle::new(), |model, _, _| {
            calls += 1;
            if !model.has_integer_variables() {
                // 此夹具仅测试报告终态分支，不构成原生不可行证据。 / This fixture tests the report branch, not native infeasibility evidence.
                SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                    .proof(SolveProof::infeasibility())
                    .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() })
                    .model_mapping(Some(linear_model_mapping(model)?)).build()
            } else { certified_report(model, ab_solution(model)) }
        }).unwrap();
        assert_eq!(calls, 2);
        assert!(!solved.pricing_converged);
        assert_eq!(solved.pricing_iterations, 0);
        assert!(solved.packing.is_some());
    }

    #[test]
    fn custom_extension_is_validated_but_not_promoted_to_global_optimality() {
        let solved = Bpp1dMilp::solve_with_extension(context(), Extension { fail_after_columns: usize::MAX }, &AcceptAllPatterns,
            Default::default(), &SolveHandle::new(), |model, _, _| certified_report(model, ab_solution(model))).unwrap();
        assert!(solved.report.is_optimal());
        assert!(solved.packing.is_some());
        assert!(!solved.globally_optimal);
        assert!(Bpp1dMilp::solve(context(), Default::default(), &SolveHandle::new(), |model, _, _| {
            let mut report = certified_report(model, ab_solution(model))?;
            report.solution.as_mut().unwrap().objective_value = Some(99.0);
            Ok(report)
        }).is_err());
    }
}

#[cfg(any(feature = "scip", feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
mod native {
    use std::time::Duration;
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    use ospf_rust_core::solver::{LinearSolver, SolveHandle, SolveOptions, SolveReport};
    use crate::application::*;
    use super::*;

    fn verify_backend(solver: &dyn LinearSolver) {
        let solve = |model: &LinearTriadModel, remaining: Duration, handle: &SolveHandle| -> Result<SolveReport<f64>> {
            solver.solve_linear_report_with_options(model, &SolveOptions {
                time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default()
            })
        };
        let ordinary = Bpp1dMilp::solve(context(), Default::default(), &SolveHandle::new(), solve).unwrap();
        assert!(ordinary.globally_optimal);
        assert_eq!(ordinary.packing.unwrap().plans.len(), 2);
        let cg = Bpp1dColumnGeneration::solve(context(), Default::default(), &SolveHandle::new(), solve).unwrap();
        assert!(cg.globally_optimal);
        assert!(cg.pricing_converged);
        assert!(cg.pricing_iterations > 0);

        // FFD/BFD 均需三个箱，但完整池存在两个箱方案。 / FFD/BFD both need three bins, but the complete pool admits two.
        let mut input = problem();
        input.items = [("six", 6.0, 1), ("five", 5.0, 1), ("three", 3.0, 1), ("two", 2.0, 3)].into_iter().map(|(id, length, amount)| Item {
            id: id.into(), length: Quantity::new_ct(length), weight: Quantity::new_ct(0.0), amount, materials: BTreeMap::new(),
        }).collect();
        input.spacing.value = 0.0;
        input.bin_types[0].length.value = 10.0;
        input.bin_types[0].inventory = Some(2);
        let packing = PackingContext::new(input, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
        let cg = Bpp1dColumnGeneration::solve(packing, Default::default(), &SolveHandle::new(), solve).unwrap();
        assert_eq!(cg.packing.unwrap().plans.len(), 2);
        assert!(!cg.pricing_converged);
        assert_eq!(cg.pricing_iterations, 0);
        assert!(cg.globally_optimal);
    }

    #[cfg(feature = "scip")]
    #[test]
    fn scip_native_milp_optimal_lp_duals_and_inventory_seed_recovery() {
        use ospf_rust_core::solver::solvers::scip::{SCIPConfig, SCIPSolver};
        verify_backend(&SCIPSolver::with_config(SCIPConfig::new().with_lp_subproblem_defaults().with_output(false)));
    }

    #[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
    #[test]
    fn gurobi_native_milp_optimal_lp_duals_and_inventory_seed_recovery() {
        use ospf_rust_core::solver::solvers::gurobi::{GurobiConfig, GurobiSolver};
        verify_backend(&GurobiSolver::with_config(GurobiConfig::new().with_output(false)));
    }
}
