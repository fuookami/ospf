//! 独立半整数网格、运行费用生命周期及真实后端回归。 / Independent half-integral grid, operating-cost lifecycle and native backend regressions.

use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::framework::demo4::domain::bunch_compilation::context::RecoveryBunchResolver;
use super::*;

fn link(cost: f64) -> Link {
    Link { from_task: "F1".into(), to_task: "F2".into(), link_type: LinkType::Connecting { min_connection_time: Duration::minutes(45) }, split_cost: Some(cost) }
}

fn source(cost: f64, idle: f64) -> Arc<RecoveryProblem<CostCalculator>> {
    let base = problem(CostCalculator);
    Arc::new(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(), vec![link(cost)], CostCalculator).unwrap()
        .with_aircraft_idle_costs(BTreeMap::from([("B1001".into(), idle)])).unwrap())
}

fn split_source(cost: f64) -> Arc<RecoveryProblem<CostCalculator>> {
    let base = transfer_problem(1);
    Arc::new(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(), vec![link(cost)], CostCalculator).unwrap()
        .with_aircraft_idle_costs(BTreeMap::from([("B1001".into(), 10.0), ("B1002".into(), 20.0)])).unwrap())
}

fn context(problem: &Arc<RecoveryProblem<CostCalculator>>) -> (BunchCompilationContext, MetaModel<f64>) {
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("recovery-operating-costs");
    context.register(&mut model).unwrap();
    (context, model)
}

// 这些夹具只有二元整数和半整数松弛极点；不将网格当作通用 LP 求解器。 / These fixtures have only binary integers and half-integral slack vertices; the grid is not a general LP solver.
fn grid_optimum(model: &MetaModel<f64>) -> (f64, Vec<f64>) {
    let matrix = model.try_to_linear_triad_model().unwrap();
    assert!(matrix.num_variables() < 12);
    let mut states = vec![vec![]];
    for index in 0..matrix.num_variables() {
        let choices = if matrix.var_types[index].is_integer() { vec![0.0, 1.0] } else { vec![0.0, 0.5, 1.0] };
        states = states.into_iter().flat_map(|state| choices.iter().map(move |value| {
            let mut state = state.clone(); state.push(*value); state
        })).collect();
    }
    states.into_iter().filter(|values| {
        values.iter().enumerate().all(|(index, value)| *value >= matrix.lb[index] && *value <= matrix.ub[index])
            && matrix.A.rows.iter().zip(&matrix.b).all(|(row, bound)| row.entries.iter()
                .map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() <= *bound + 1e-9)
    }).map(|values| (values.iter().zip(&matrix.c).map(|(value, coefficient)| value * coefficient).sum::<f64>(), values))
        .min_by(|left, right| left.0.total_cmp(&right.0)).unwrap()
}

#[test]
fn link_losses_match_zero_half_and_full_physical_oracles_and_change_selection() {
    let source = source(400.0, 0.0);
    let (mut context, mut model) = context(&source);
    let (objective, values) = grid_optimum(&model);
    assert_eq!(objective, 2000.0);
    assert_eq!(context.extract_solution(&model, &values).unwrap().operating_costs.links[0].loss, 0.0);
    context.add_candidates(0, vec![single(&source, 0, 0)], &mut model).unwrap();
    let (objective, values) = grid_optimum(&model);
    assert_eq!(objective, 1300.0);
    let result = context.extract_solution(&model, &values).unwrap();
    assert_eq!(result.operating_costs.links[0].loss, 0.5);
    assert_eq!(result.operating_costs.links[0].modeled_loss, 0.5);
    assert_eq!(result.operating_costs.links[0].penalty, 200.0);
    context.add_candidates(1, vec![fleet::complete(&source)], &mut model).unwrap();
    let (objective, values) = grid_optimum(&model);
    assert_eq!(objective, 300.0);
    assert_eq!(context.extract_solution(&model, &values).unwrap().operating_costs.links[0].loss, 0.0);
    for (cost, objective, selected, loss) in [(400.0, 600.0, 2, 1.0), (4000.0, 2030.0, 0, 0.0)] {
        let source = split_source(cost);
        let (mut context, mut model) = self::context(&source);
        context.add_candidates(0, vec![single(&source, 0, 0), single(&source, 1, 1)], &mut model).unwrap();
        let (actual, values) = grid_optimum(&model);
        assert_eq!(actual, objective);
        let result = context.extract_solution(&model, &values).unwrap();
        assert_eq!(result.selected.len(), selected);
        assert_eq!(result.operating_costs.links[0].loss, loss);
    }
}

#[test]
fn idle_cost_changes_selection_and_empty_bunch_does_not_fake_aircraft_use() {
    for (idle_cost, objective, selected) in [(0.0, 200.0, 0), (500.0, 300.0, 1)] {
        let base = problem(CostCalculator);
        let mut configuration = base.configuration().clone();
        configuration.cancellation_costs = vec![Some(100.0); 2];
        let source = Arc::new(RecoveryProblem::new(configuration, base.domains().to_vec(), vec![], CostCalculator).unwrap()
            .with_aircraft_idle_costs(BTreeMap::from([("B1001".into(), idle_cost)])).unwrap());
        let (mut context, mut model) = context(&source);
        let empty = RecoveryBunchCandidate { bunch: FlightTaskBunch::new("idle".into(), aircraft(), vec![]).unwrap(), cost: 0.0 };
        context.add_candidates(0, vec![empty], &mut model).unwrap();
        model.add_eq_constraint(&[(context.master().compilation.bunch_x_map[&0], 1.0)], 1.0, "select-empty").unwrap();
        let (actual, values) = grid_optimum(&model);
        assert_eq!(actual, 200.0 + idle_cost);
        assert!(context.extract_solution(&model, &values).unwrap().operating_costs.aircraft[0].idle);
        model.retain_constraints(|row| row.name != "select-empty");
        context.add_candidates(1, vec![fleet::complete(&source)], &mut model).unwrap();
        let (actual, values) = grid_optimum(&model);
        assert_eq!(actual, objective);
        let result = context.extract_solution(&model, &values).unwrap();
        assert_eq!(result.selected.iter().filter(|candidate| !candidate.bunch.activities().is_empty()).count(), selected);
        assert_eq!(result.operating_costs.aircraft[0].idle, selected == 0);
    }
}

#[test]
fn soft_link_allows_splits_but_preserved_links_still_obey_connection_time_and_hard_links_remain_hard() {
    let base = problem(CostCalculator);
    for cost in [None, Some(1.0)] {
        let link = Link { split_cost: cost, ..link(1.0) };
        let single = single(&base, 0, 0);
        let rules = BunchRecoveryRules { usability: &usability(), previous: None, links: std::slice::from_ref(&link) };
        assert_eq!(FeasibilityJudger.judge_bunch(&single.bunch, &rules).unwrap().feasible, cost.is_some());
        let link = Link { link_type: LinkType::Connecting { min_connection_time: Duration::hours(2) }, ..link };
        let rules = BunchRecoveryRules { usability: &usability(), previous: None, links: &[link] };
        assert!(!FeasibilityJudger.judge_bunch(&fleet::complete(&base).bunch, &rules).unwrap().feasible);
    }
}

#[test]
fn operating_cost_rows_share_complete_coefficients_and_signed_duals_with_pricing() {
    let source = source(400.0, 25.0);
    let (mut context, mut model) = context(&source);
    let all = vec![single(&source, 0, 0), fleet::complete(&source)];
    context.add_candidates(0, all.clone(), &mut model).unwrap();
    for (index, candidate) in all.iter().enumerate() {
        let expected = source.column_coefficients(candidate).unwrap();
        assert_eq!(expected.keys().cloned().collect::<std::collections::BTreeSet<_>>(), source.row_names());
        let variable = context.master().compilation.bunch_x_map[&index];
        for row in model.constraints() {
            let actual = row.inequality.polynomial.monomials().iter().filter(|term| term.var_index() == variable).map(|term| *term.coefficient()).sum::<f64>();
            assert_eq!(actual, expected[&row.name], "{}", row.name);
        }
    }
    let mut request = request();
    request.row_duals = source.row_names().into_iter().map(|name| (name, 0.0)).collect();
    request.row_duals.insert("recovery_link_split_0".into(), 71.0);
    request.row_duals.insert("recovery_aircraft_idle_B1001".into(), -13.0);
    let policy = RecoveryPricingPolicy::new(source.clone(), None);
    let result = policy.price(&request, &PathSearchBudget::default()).unwrap();
    for candidate in all {
        let expected = source.column_coefficients(&candidate).unwrap().iter().fold(candidate.cost, |value, (name, coefficient)| value - request.row_duals[name] * coefficient);
        assert_eq!(result.reduced_costs[source.entry(&candidate).unwrap().id.as_str()], expected);
    }
    request.row_duals.remove("recovery_aircraft_idle_B1001");
    assert!(policy.price(&request, &PathSearchBudget::default()).is_err());
    request.row_duals.insert("recovery_aircraft_idle_B1001".into(), 0.0);
    request.row_duals.remove("recovery_link_split_0");
    assert!(policy.price(&request, &PathSearchBudget::default()).is_err());
    assert!(result.batch.no_improving_columns.is_none());
}

#[test]
fn operating_cost_lifecycle_preserves_extensions_rolls_back_and_readds_fresh_variables() {
    let source = source(400.0, 25.0);
    let (mut context, mut model) = context(&source);
    let extension = model.register_auto_variable_with_range::<Binary>("operating_extension", VariableRange::fixed(1.0)).unwrap();
    model.add_eq_constraint(&[(extension, 1.0)], 1.0, "operating_extension_rule").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(7.0, extension)], 0.0), "operating_extension_cost"));
    let candidate = fleet::complete(&source);
    context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    let first = context.master().compilation.bunch_x_map[&0];
    let count = model.as_basic().num_tokens();
    context.register(&mut model).unwrap();
    assert_eq!(count, model.as_basic().num_tokens());
    assert_eq!(grid_optimum(&model).0, 307.0);
    context.remove_columns(&[0], &mut model).unwrap();
    assert_eq!(grid_optimum(&model).0, 2032.0);
    context.add_candidates(1, vec![candidate.clone()], &mut model).unwrap();
    assert_ne!(first, context.master().compilation.bunch_x_map[&1]);
    assert_eq!(grid_optimum(&model).0, 307.0);
    assert!(model.constraints().iter().any(|row| row.name == "operating_extension_rule"));
    let (mut other, mut other_model) = self::context(&source);
    assert!(context.add_candidates(2, vec![candidate.clone()], &mut other_model).is_err());
    assert_eq!(other.column_count(), 0);
    other_model.retain_constraints(|row| row.name != "recovery_link_split_0");
    other_model.add_eq_constraint(&[], 0.0, "recovery_link_split_0").unwrap();
    let before = other_model.try_to_linear_triad_model().unwrap();
    assert!(other.add_candidates(0, vec![candidate], &mut other_model).is_err());
    let after = other_model.try_to_linear_triad_model().unwrap();
    assert_eq!(before.c, after.c);
    assert_eq!(before.num_variables(), after.num_variables());
    assert_eq!(before.num_constraints(), after.num_constraints());
    assert_eq!(other.column_count(), 0);
}

#[test]
fn production_extraction_rejects_illegal_cost_auxiliaries_even_after_rows_are_removed() {
    let source = source(400.0, 25.0);
    let (mut context, mut model) = context(&source);
    context.add_candidates(0, vec![single(&source, 0, 0)], &mut model).unwrap();
    let (_, values) = grid_optimum(&model);
    assert_eq!(values[2], 0.5);
    assert_eq!(values[3], 0.0);
    model.retain_constraints(|row| !row.name.starts_with("recovery_link_split_") && !row.name.starts_with("recovery_aircraft_idle_"));
    for illegal in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.5, 0.0, 1.5] {
        let mut forged = values.clone(); forged[2] = illegal;
        assert!(context.extract_solution(&model, &forged).is_err());
    }
    for illegal in [f64::NAN, f64::INFINITY, -1.0, 0.5, 1.0] {
        let mut forged = values.clone(); forged[3] = illegal;
        assert!(context.extract_solution(&model, &forged).is_err());
    }
    let mut nonminimal = values.clone(); nonminimal[2] = 1.0;
    let result = context.extract_solution(&model, &nonminimal).unwrap();
    assert_eq!(result.operating_costs.links[0].loss, 0.5);
    assert_eq!(result.operating_costs.links[0].penalty, 400.0);
    let mut forged = result; forged.operating_costs.links[0].penalty = 200.0;
    assert!(source.validate_schedule(&forged).is_err());
}

#[test]
fn operating_cost_configuration_rejects_unknown_negative_nonfinite_and_executed_soft_endpoints() {
    let base = problem(CostCalculator);
    for cost in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(), vec![link(cost)], CostCalculator).is_err());
        assert!(fleet::fresh().with_aircraft_idle_costs(BTreeMap::from([("B1001".into(), cost)])).is_err());
    }
    assert!(fleet::fresh().with_aircraft_idle_costs(BTreeMap::from([("unknown".into(), 0.0)])).is_err());
    assert!(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(), vec![link(0.0), link(0.0)], CostCalculator).is_err());
    let mut domains = base.domains().to_vec();
    domains[0].previous = Some(activity("PREV", "X", "A", 5, 6));
    assert!(RecoveryProblem::new(base.configuration().clone(), domains, vec![Link { from_task: "PREV".into(), ..link(0.0) }], CostCalculator).is_err());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_soft_link_and_idle_costs_use_real_rmp_duals_and_final_milp() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    for (cost, objective, selected, loss) in [(400.0, 600.0, 2, 1.0), (4000.0, 2030.0, 0, 0.0)] {
        let source = split_source(cost);
        let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(source.clone(), RecoveryApplicationConfiguration {
            initial: vec![single(&source, 0, 0), single(&source, 1, 2)],
            column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
        }).unwrap();
        assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(objective));
        assert!(report.proof.is_none());
        let solution = report.solution.unwrap().value.unwrap();
        assert_eq!(solution.schedule.selected.len(), selected);
        assert_eq!(solution.schedule.operating_costs.links[0].loss, loss);
        source.validate_schedule(&solution.schedule).unwrap();
        assert!(!solution.pricing.is_empty());
        if selected > 0 {
            assert!(solution.pricing.iter().any(|round| !round.batch.columns.is_empty()));
            assert!(solution.schedule.selected.iter().any(|candidate| candidate.bunch.recovery("F2")
                .is_some_and(|activity| activity.time().1 - activity.time().0 == Duration::hours(1))));
        }
    }
    let source = source(400.0, 25.0);
    let (mut context, mut model) = context(&source);
    context.add_candidates(0, vec![single(&source, 0, 0)], &mut model).unwrap();
    let expected = grid_optimum(&model).0;
    let matrix = model.try_to_linear_triad_model().unwrap();
    let solution = GurobiColumnGenerationSolver::new().solve_milp_with_options(&matrix, Default::default()).unwrap();
    assert_eq!(solution.obj, expected);
    assert_eq!(context.extract_solution(&model, &solution.solution).unwrap().operating_costs.links[0].modeled_loss, 0.5);
}
