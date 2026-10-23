//! 机队终点、松弛费用及完整定价行回归。 / Fleet terminals, slack costs and full pricing-row regressions.

use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::framework::demo4::domain::bunch_compilation::fleet_balance::{RecoveryFleetBalance, RecoveryFleetCheckpoint};
use super::*;

pub(super) fn fresh() -> RecoveryProblem<CostCalculator> { Arc::try_unwrap(problem(CostCalculator)).unwrap() }

fn checkpoint(airport: &str, cost: Option<f64>) -> RecoveryFleetCheckpoint {
    RecoveryFleetCheckpoint { airport: airport.into(), aircraft_minor_type: "A320-200".into(), required: 1, shortage_cost: cost }
}

pub(super) fn complete(problem: &RecoveryProblem<CostCalculator>) -> RecoveryBunchCandidate {
    let domain = &problem.domains()[0];
    let id = domain.candidates.iter().fold("idle".to_string(), |id, candidate| format!("{id}:{}:{}", candidate.id.len(), candidate.id));
    let bunch = FlightTaskBunch::new(id, domain.aircraft.clone(), domain.candidates.iter().map(|candidate| candidate.recovery.clone()).collect()).unwrap();
    RecoveryBunchCandidate { cost: CostCalculator.calculate_bunch(&bunch, &CostCalculator).unwrap(), bunch }
}

fn context(problem: &Arc<RecoveryProblem<CostCalculator>>) -> (BunchCompilationContext, MetaModel<f64>) {
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("fleet-recovery");
    context.register(&mut model).unwrap();
    (context, model)
}

#[test]
fn fleet_hard_minimum_and_soft_shortage_change_actual_finite_master_optima() {
    for (cost, objective, selected, shortage) in [(None, 2000.0, 0, 0), (Some(500.0), 800.0, 1, 1), (Some(3000.0), 2000.0, 0, 0)] {
        let problem = Arc::new(fresh().with_fleet_balance(vec![checkpoint("A", cost)]).unwrap());
        let (mut context, mut model) = context(&problem);
        context.add_candidates(0, vec![complete(&problem)], &mut model).unwrap();
        let (actual, values) = master_optimum(&model).unwrap();
        assert_eq!(actual, objective);
        let schedule = context.extract_solution(&model, &values).unwrap();
        assert_eq!(schedule.selected.len(), selected);
        let fleet = &schedule.fleet_balance[0];
        assert_eq!(fleet.shortage, shortage);
        assert_eq!(fleet.available, 1 - shortage);
        assert_eq!(fleet.penalty, cost.unwrap_or(0.0) * shortage as f64);
    }
    let mut target = checkpoint("C", None);
    target.required = 2;
    let problem = Arc::new(fresh().with_fleet_balance(vec![target]).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![complete(&problem)], &mut model).unwrap();
    assert!(master_optimum(&model).is_none());
}

#[test]
fn fleet_refresh_preserves_downstream_objective_and_uses_new_variables_after_retirement() {
    let problem = Arc::new(fresh().with_fleet_balance(vec![checkpoint("A", Some(500.0))]).unwrap());
    let (mut context, mut model) = context(&problem);
    let extension = model.register_auto_variable_with_range::<Binary>("downstream_fleet_variable", VariableRange::fixed(1.0)).unwrap();
    model.add_eq_constraint(&[(extension, 1.0)], 1.0, "downstream_fleet_rule").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(7.0, extension)], 0.0), "downstream_fleet_objective"));
    assert_eq!(master_optimum(&model).unwrap().0, 2007.0);
    let candidate = complete(&problem);
    context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    let first = context.master().compilation.bunch_x_map[&0];
    let tokens = model.as_basic().num_tokens();
    context.register(&mut model).unwrap();
    assert_eq!(model.as_basic().num_tokens(), tokens);
    assert_eq!(master_optimum(&model).unwrap().0, 807.0);
    context.remove_columns(&[0], &mut model).unwrap();
    assert_eq!(master_optimum(&model).unwrap().0, 2007.0);
    context.add_candidates(1, vec![candidate], &mut model).unwrap();
    assert_ne!(first, context.master().compilation.bunch_x_map[&1]);
    assert_eq!(master_optimum(&model).unwrap().0, 807.0);
    assert!(model.constraints().iter().any(|row| row.name == "downstream_fleet_rule"));
}

#[test]
fn fleet_original_baseline_counts_unused_aircraft_and_executed_predecessors_without_fabricated_events() {
    let source = fresh();
    let original = complete(&source);
    let empty = RecoveryFleetBalance::from_original(source.domains(), &[], None).unwrap();
    assert_eq!(empty.checkpoints()[0].airport, "A");
    let target = RecoveryFleetBalance::from_original(source.domains(), std::slice::from_ref(&original), None).unwrap();
    assert_eq!(target.checkpoints()[0].airport, "C");
    assert!(RecoveryFleetBalance::from_original(source.domains(), &[original.clone(), original], None).is_err());
    let mut domains = source.domains().to_vec();
    domains[0].previous = Some(activity("executed", "X", "B", 5, 6));
    let origin = RecoveryFleetBalance::from_original(&domains, &[], None).unwrap();
    assert_eq!(origin.checkpoints()[0].airport, "B");
    let mut idle = single(&source, 0, 0);
    idle.bunch = FlightTaskBunch::new("idle".into(), aircraft(), vec![]).unwrap();
    idle.cost = 0.0;
    assert_eq!(origin.column_coefficients(&idle).unwrap()["recovery_fleet_balance_0"], 0.0);
    let definition = RecoveryFleetBalance::new(vec![], &[]).unwrap();
    assert!(definition.checkpoints().is_empty());
}

#[test]
fn fleet_coefficients_and_signed_duals_are_shared_with_pricing_and_zero_duals_are_required() {
    let problem = Arc::new(fresh().with_fleet_balance(vec![checkpoint("A", Some(500.0)), checkpoint("C", None)]).unwrap());
    let (mut context, mut model) = context(&problem);
    let candidate = complete(&problem);
    let expected = problem.column_coefficients(&candidate).unwrap();
    assert_eq!(expected["recovery_fleet_balance_0"], -1.0);
    assert_eq!(expected["recovery_fleet_balance_1"], 1.0);
    context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    let variable = context.master().compilation.bunch_x_map[&0];
    for row in model.constraints().iter().filter(|row| row.name.starts_with("recovery_fleet_balance_")) {
        let actual = row.inequality.polynomial.monomials().iter().filter(|term| term.var_index() == variable).map(|term| *term.coefficient()).sum::<f64>();
        assert_eq!(actual, expected[&row.name]);
    }
    let mut request = request();
    request.row_duals.insert("recovery_fleet_balance_0".into(), 500.0);
    request.row_duals.insert("recovery_fleet_balance_1".into(), 0.0);
    let policy = RecoveryPricingPolicy::new(problem.clone(), None);
    let result = policy.price(&request, &PathSearchBudget::default()).unwrap();
    let id = problem.entry(&candidate).unwrap().id;
    assert_eq!(result.reduced_costs[id.as_str()], 450.0);
    request.row_duals.remove("recovery_fleet_balance_1");
    assert!(policy.price(&request, &PathSearchBudget::default()).is_err());
}

#[test]
fn fleet_inputs_fail_before_registration_and_production_extraction_rechecks_physical_shortage() {
    for target in [checkpoint("unknown", None), RecoveryFleetCheckpoint { aircraft_minor_type: "unknown".into(), ..checkpoint("A", None) },
        checkpoint("A", Some(f64::NAN)), checkpoint("A", Some(-1.0)),
        RecoveryFleetCheckpoint { required: 1_u64 << 54, ..checkpoint("A", None) }]
    { assert!(fresh().with_fleet_balance(vec![target]).is_err()); }
    assert!(fresh().with_fleet_balance(vec![checkpoint("A", None), checkpoint("A", None)]).is_err());
    let problem = Arc::new(fresh().with_fleet_balance(vec![checkpoint("A", Some(500.0))]).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![complete(&problem)], &mut model).unwrap();
    let (_, mut values) = master_optimum(&model).unwrap();
    assert_eq!(values[2], 1.0);
    values[2] = 0.0;
    model.retain_constraints(|row| row.name != "recovery_fleet_balance_0");
    assert!(context.extract_solution(&model, &values).is_err());
}

#[test]
fn fleet_shortage_is_bounded_integer_not_binary_and_rejects_illegal_incumbents() {
    let target = RecoveryFleetCheckpoint { required: 3, ..checkpoint("C", Some(500.0)) };
    let problem = Arc::new(fresh().with_fleet_balance(vec![target]).unwrap());
    let (mut context, mut model) = context(&problem);
    context.add_candidates(0, vec![complete(&problem)], &mut model).unwrap();
    let values = [0.0, 0.0, 2.0, 1.0];
    let result = context.extract_solution(&model, &values).unwrap();
    assert_eq!(result.fleet_balance[0].available, 1);
    assert_eq!(result.fleet_balance[0].shortage, 2);
    assert_eq!(result.fleet_balance[0].penalty, 1000.0);
    for illegal in [f64::NAN, f64::INFINITY, -1.0, 1.0, 2.5, 4.0] {
        let mut values = values;
        values[2] = illegal;
        assert!(context.extract_solution(&model, &values).is_err());
    }
    let mut tolerant = values;
    tolerant[2] += 1e-9;
    assert_eq!(context.extract_solution(&model, &tolerant).unwrap().fleet_balance[0].modeled_shortage, Some(2));
}

#[test]
fn fleet_row_collision_rolls_back_the_whole_column_transaction() {
    let problem = Arc::new(fresh().with_fleet_balance(vec![checkpoint("C", Some(500.0))]).unwrap());
    let (mut context, mut model) = context(&problem);
    model.retain_constraints(|row| row.name != "recovery_fleet_balance_0");
    model.add_eq_constraint(&[], 0.0, "recovery_fleet_balance_0").unwrap();
    let before = model.as_basic().num_tokens();
    assert!(context.add_candidates(0, vec![complete(&problem)], &mut model).is_err());
    assert_eq!(context.column_count(), 0);
    assert_eq!(model.as_basic().num_tokens(), before);
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_fleet_shortage_uses_actual_terminal_cost_and_full_rmp_rows() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let problem = Arc::new(fresh().with_fleet_balance(vec![checkpoint("A", Some(500.0))]).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(problem, RecoveryApplicationConfiguration {
        column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(800.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(solution.schedule.canceled.is_empty());
    assert_eq!(solution.schedule.fleet_balance[0].modeled_shortage, Some(1));
    assert_eq!(solution.schedule.fleet_balance[0].penalty, 500.0);
    assert!(!solution.pricing.is_empty());
}
