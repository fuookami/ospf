use super::*;
use ospf_rust_core::model::{ConstraintRelation, LinearObjectiveInput};
use crate::domain::route_compilation::RouteCompilationExtensionFactory;
use crate::domain::vrp::{RoutePricingArc, RoutePricingRow, RoutePricingRows, RouteRowCoefficient};

struct SignedArcUsage;

impl RouteRowCoefficient<f64> for SignedArcUsage {
    fn route_coefficient(&self, route: &Route<f64>, _: &VrptwInstance<f64>) -> crate::Result<Quantity<f64, ospf_rust_quantities::unit::Unit>> {
        Ok(Quantity::new(-0.5 * route.effective_arc_ids().len() as f64, Meter::INSTANT.clone()))
    }

    fn arc_coefficient(&self, _: &RoutePricingArc<'_, f64>) -> crate::Result<Option<Quantity<f64, ospf_rust_quantities::unit::Unit>>> {
        Ok(Some(Quantity::new(-0.0005, Kilometer::INSTANT.clone())))
    }

    fn additive_decomposition_is_exact(&self) -> bool { true }
}

fn rows() -> Arc<RoutePricingRows<f64>> {
    Arc::new(RoutePricingRows::new(vec![RoutePricingRow {
        id: "signed".into(), unit: Meter::INSTANT.clone(),
        lower: Some(Quantity::new(-2.0, Meter::INSTANT.clone())),
        upper: Some(Quantity::new(0.003, Kilometer::INSTANT.clone())),
        coefficient: Arc::new(SignedArcUsage),
    }]).unwrap())
}

fn matrix(phase: PricingPhase, price: Option<f64>) {
    let instance = test_instance();
    let catalog = price.map_or_else(|| Arc::new(RoutePricingRows::default()), |_| rows());
    let mut duals = PricingDuals::new(phase, [("c1".into(), 20.0)], [("v1".into(), 0.0)]);
    if let Some(price) = price { duals = duals.with_extension_duals([("signed".into(), price)]); }
    let builder = RouteGraphBuilder::new(instance.clone(), EuclideanDistanceCalculator,
        DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator), DistanceArcCostCalculator,
        FixedPlusArcCostPolicy).with_pricing_rows(catalog.clone());
    let graph = builder.build(&"v1".into(), &duals, None).unwrap();
    for arc in &graph.arcs {
        assert_eq!(arc.row_coefficients.get("signed").copied(), price.map(|_| -0.5));
    }
    let result = EspprcPricer::new(builder).price_request(&PricingRequest::new(instance.clone(), duals, "v1".into())).unwrap();
    assert!(result.exact_pricing_complete);
    assert_eq!(result.routes.len(), 1);
    let expected = if phase == PricingPhase::PhaseOne { 0.0 } else { 3.0 } - 20.0 + price.unwrap_or(0.0);
    assert_eq!(result.min_reduced_cost, expected);
    let mut context = RouteCompilationContext::new(instance).with_pricing_rows(catalog).unwrap();
    let mut model = MetaModel::new("row-matrix");
    context.register(&mut model).unwrap();
    context.add_columns(0, result.routes, &mut model).unwrap();
    if phase == PricingPhase::PhaseTwo { context.switch_to_phase_two(&mut model).unwrap(); }
    context.aggregation.validate_pricing_model(&model).unwrap();
    let mut raw = vec![0.0; model.constraints().len()];
    for (index, row) in model.constraints().iter().enumerate() {
        if row.name == "customer_coverage_c1" { raw[index] = 20.0; }
        if row.name == "route_pricing_row:lower:signed" { raw[index] = price.unwrap() + 1.0; }
        if row.name == "route_pricing_row:upper:signed" { raw[index] = -1.0; }
    }
    let extracted = context.extract_pricing_duals(&model, &raw).unwrap();
    assert_eq!(extracted.extension.get("signed").copied(), price);
    if price.is_some() {
        let compiled = model.try_to_linear_triad_model().unwrap();
        let index = context.aggregation.route_variable_indices().values().next().unwrap().model_index;
        assert!(compiled.A.rows.iter().any(|row| row.entries.contains(&(index, -1.0))));
        assert_eq!(context.aggregation.pricing_rows.catalog().rows()[0].upper.as_ref().unwrap().value, 3.0);
    }
}

macro_rules! case {
    ($name:ident, $phase:ident, $price:expr) => {
        #[test]
        fn $name() { matrix(PricingPhase::$phase, $price); }
    };
}

case!(ordinary_phase_one_no_extension, PhaseOne, None);
case!(ordinary_phase_two_no_extension, PhaseTwo, None);
case!(ordinary_phase_one_zero_extension, PhaseOne, Some(0.0));
case!(ordinary_phase_two_zero_extension, PhaseTwo, Some(0.0));
case!(ordinary_phase_one_nonzero_extension, PhaseOne, Some(2.0));
case!(ordinary_phase_two_nonzero_extension, PhaseTwo, Some(2.0));

#[test]
fn nonadditive_row_usage_controls_terminal_cost_without_dominance() {
    struct WholeUsage;
    impl RouteRowCoefficient<f64> for WholeUsage {
        fn route_coefficient(&self, route: &Route<f64>, _: &VrptwInstance<f64>) -> crate::Result<Quantity<f64, ospf_rust_quantities::unit::Unit>> {
            Ok(Quantity::new(100.0 * (route.customer_ids().len() as f64).powi(2), Meter::INSTANT.clone()))
        }
    }
    let instance = multi_customer_instance(2);
    let catalog = Arc::new(RoutePricingRows::new(vec![RoutePricingRow {
        id: "whole".into(), unit: Meter::INSTANT.clone(), lower: None,
        upper: Some(Quantity::new(500.0, Meter::INSTANT.clone())), coefficient: Arc::new(WholeUsage),
    }]).unwrap());
    for phase in [PricingPhase::PhaseOne, PricingPhase::PhaseTwo] {
        let duals = PricingDuals::new(phase, [], [("v1".into(), 0.0)]).with_extension_duals([("whole".into(), 1.0)]);
        let builder = RouteGraphBuilder::new(instance.clone(), EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator), DistanceArcCostCalculator,
            FixedPlusArcCostPolicy).with_pricing_rows(catalog.clone());
        let result = EspprcPricer::new(builder).price_request(&PricingRequest::new(instance.clone(), duals, "v1".into())).unwrap();
        assert!(result.exact_pricing_complete);
        assert_eq!(result.statistics.dominance_checks, 0);
        assert_eq!(result.routes.len(), 4);
        assert_eq!(result.routes[0].customer_ids().len(), 2);
        assert_eq!(result.min_reduced_cost, if phase == PricingPhase::PhaseOne { -400.0 } else { -395.0 });
    }
}

#[test]
fn invalid_row_coefficients_roll_back_and_affine_rows_do_not_panic() {
    struct InvalidUsage;
    impl RouteRowCoefficient<f64> for InvalidUsage {
        fn route_coefficient(&self, _: &Route<f64>, _: &VrptwInstance<f64>) -> crate::Result<Quantity<f64, ospf_rust_quantities::unit::Unit>> {
            Ok(Quantity::new(f64::INFINITY, Meter::INSTANT.clone()))
        }
    }
    let invalid = RoutePricingRow { id: "bad".into(), unit: Meter::INSTANT.clone(), lower: None,
        upper: Some(Quantity::new(1.0, Meter::INSTANT.clone())), coefficient: Arc::new(InvalidUsage) as Arc<dyn RouteRowCoefficient<f64>> };
    let instance = test_instance();
    let route = InitialRouteGenerator::new(instance.clone(), EuclideanDistanceCalculator,
        DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator), DistanceArcCostCalculator,
        FixedPlusArcCostPolicy).generate(None).unwrap().remove(0);
    let mut context = RouteCompilationContext::new(instance).with_pricing_rows(Arc::new(RoutePricingRows::new(vec![invalid.clone()]).unwrap())).unwrap();
    let mut model = MetaModel::new("bad-row-rollback");
    context.register(&mut model).unwrap();
    let before = (model.tokens().len(), model.constraints().len());
    assert!(context.add_columns(0, [route], &mut model).is_err());
    assert_eq!((model.tokens().len(), model.constraints().len()), before);
    assert!(context.aggregation.routes().is_empty());
    context.aggregation.validate_pricing_model(&model).unwrap();
    let mut affine = invalid;
    affine.unit = ospf_rust_quantities::unit::Celsius::INSTANT.clone();
    assert!(RoutePricingRows::new(vec![affine]).is_err());
}

#[test]
fn extension_row_artificials_preserve_phase_one_feasibility_and_convergence() {
    let instance = test_instance();
    let catalog = Arc::new(RoutePricingRows::new(vec![RoutePricingRow {
        id: "lower".into(), unit: Meter::INSTANT.clone(),
        lower: Some(Quantity::new(2.0, Meter::INSTANT.clone())), upper: None,
        coefficient: Arc::new(SignedArcUsage),
    }]).unwrap());
    let mut context = RouteCompilationContext::new(instance).with_pricing_rows(catalog).unwrap();
    let mut model = MetaModel::new("phase-one-row-slack");
    context.register(&mut model).unwrap();
    let compiled = model.try_to_linear_triad_model().unwrap();
    let values = [1.0, 2.0];
    assert!(ospf_rust_core::solver::evaluate_linear_solution(&compiled, &values, 1e-7).unwrap().issues.is_empty());
    model.set_solution(&[0.0, 2.0]);
    assert!(!context.is_phase_one_converged(&model, 1e-7).unwrap());
    model.set_solution(&[0.0, 0.0]);
    assert!(context.is_phase_one_converged(&model, 1e-7).unwrap());
    context.switch_to_phase_two(&mut model).unwrap();
    assert_eq!(model.variable_range_by_index(1), Some(ospf_rust_core::variable::VariableRange::fixed(0.0)));
    context.aggregation.validate_pricing_model(&model).unwrap();
}

#[test]
fn changed_provider_or_shadow_hook_cannot_replace_registered_pricing_coefficients() {
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MutableUsage(Arc<AtomicU64>);
    impl RouteRowCoefficient<f64> for MutableUsage {
        fn route_coefficient(&self, _: &Route<f64>, _: &VrptwInstance<f64>) -> crate::Result<Quantity<f64, ospf_rust_quantities::unit::Unit>> {
            Ok(Quantity::new(f64::from_bits(self.0.load(Ordering::Relaxed)), Meter::INSTANT.clone()))
        }
    }
    struct Reprice;
    impl RouteCompilationExtension<f64> for Reprice {
        fn refresh_shadow_price(&self, _: &MetaModel<f64>, _: &[f64], prices: &mut crate::domain::vrp::VrpShadowPriceMap) -> crate::Result<()> {
            prices.set_extension_dual("changing".into(), 100.0); Ok(())
        }
    }
    let value = Arc::new(AtomicU64::new(1.0_f64.to_bits()));
    let catalog = Arc::new(RoutePricingRows::new(vec![RoutePricingRow {
        id: "changing".into(), unit: Meter::INSTANT.clone(), lower: None,
        upper: Some(Quantity::new(10.0, Meter::INSTANT.clone())), coefficient: Arc::new(MutableUsage(value.clone())),
    }]).unwrap());
    let instance = test_instance();
    let route = InitialRouteGenerator::new(instance.clone(), EuclideanDistanceCalculator,
        DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator), DistanceArcCostCalculator,
        FixedPlusArcCostPolicy).generate(None).unwrap().remove(0);
    let mut model = MetaModel::new("changed-provider");
    let mut context = RouteCompilationContext::with_extensions(instance, vec![Box::new(Reprice)]).with_pricing_rows(catalog).unwrap();
    context.register(&mut model).unwrap();
    context.add_columns(0, [route], &mut model).unwrap();
    context.aggregation.validate_pricing_model(&model).unwrap();
    assert!(context.extract_pricing_duals(&model, &vec![0.0; model.constraints().len()]).is_err());
    value.store(2.0_f64.to_bits(), Ordering::Relaxed);
    assert!(context.aggregation.validate_pricing_model(&model).is_err());
}

#[test]
fn zero_output_limit_cannot_turn_a_known_negative_column_into_a_bound() {
    struct RestrictedLp;
    impl LinearProgrammingSolver for RestrictedLp {
        fn solve_lp(&self, request: &LinearProgrammingRequest<'_>) -> crate::Result<LinearProgrammingResult> {
            assert_eq!(request.model.tokens().len(), 4);
            let solution = vec![0.0, 0.0, 1.0, 1.0];
            let phase_two = request.phase == PricingPhase::PhaseTwo;
            let objective = if phase_two { 8.0 } else { 0.0 };
            let duals = request.model.constraints().iter().map(|row| {
                if !phase_two { 0.0 }
                else if row.name == "customer_coverage_c1" { 3.0 }
                else if row.name == "customer_coverage_c2" { 5.0 }
                else { 0.0 }
            }).collect::<Vec<_>>();
            Ok(LinearProgrammingResult { status: LinearProgrammingStatus::Optimal, objective: Some(objective),
                solution: Some(solution.clone()), dual_solution: Some(duals.clone()),
                report: Some(verified_optimal_lp_report(request, objective, solution, duals)) })
        }
    }
    let instance = multi_customer_instance(2);
    let provider = BranchNodeSolver::new(instance.clone(), Arc::new(RestrictedLp), BranchNodeSolverConfig {
        distance_calculator: EuclideanDistanceCalculator,
        travel_time_calculator: DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
        arc_cost_calculator: DistanceArcCostCalculator, route_cost_policy: FixedPlusArcCostPolicy,
        arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
        dominance_policy: DefaultLabelDominancePolicy, column_selector: DefaultPricingColumnSelector,
        extension_factories: vec![], pricing_rows: Arc::new(Default::default()),
    }).unwrap();
    let result = provider.solve_node(&BranchNode::root(instance.start_depot.node.id.clone(), instance.end_depot.node.id.clone()).unwrap(),
        &[], &BranchNodeSolveContext { max_cg_iterations: 3, max_columns_per_pricing: 0,
            max_labels: None, deadline: None, cancellation: None }).unwrap();
    assert_eq!(result.status, BranchNodeSolveStatus::SolverStopped);
    assert!(!result.pricing_complete);
    assert!(result.lower_bound <= 5.0);
}

#[test]
fn extension_rows_cover_removal_readdition_missing_prices_and_proof_guards() {
    let instance = test_instance();
    let catalog = rows();
    let builder = RouteGraphBuilder::new(instance.clone(), EuclideanDistanceCalculator,
        DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator), DistanceArcCostCalculator,
        FixedPlusArcCostPolicy).with_pricing_rows(catalog.clone());
    let duals = PricingDuals::new(PricingPhase::PhaseTwo, [("c1".into(), 20.0)], [("v1".into(), 0.0)]);
    assert!(builder.build(&"v1".into(), &duals, None).is_err());
    assert!(builder.build(&"v1".into(), &duals.clone().with_extension_duals([("unknown".into(), 0.0)]), None).is_err());
    let request = PricingRequest::new(instance.clone(), duals.with_extension_duals([("signed".into(), 0.0)]), "v1".into());
    let route = EspprcPricer::new(builder).price_request(&request).unwrap().routes.remove(0);
    let mut context = RouteCompilationContext::new(instance).with_pricing_rows(catalog).unwrap();
    let mut model = MetaModel::new("row-lifecycle");
    context.register(&mut model).unwrap();
    context.add_columns(0, [route.clone()], &mut model).unwrap();
    let first = context.aggregation.route_variable_indices()[&route.signature()].model_index;
    context.remove_columns([route.clone()], &mut model).unwrap();
    context.add_columns(1, [route.clone()], &mut model).unwrap();
    let second = context.aggregation.route_variable_indices()[&route.signature()].model_index;
    assert_ne!(first, second);
    assert_eq!(model.variable_range_by_index(first), Some(ospf_rust_core::variable::VariableRange::fixed(0.0)));
    context.aggregation.validate_pricing_model(&model).unwrap();
    model.set_variable_range_by_index(first, ospf_rust_core::variable::VariableRange::with_lower(0.0)).unwrap();
    assert!(context.aggregation.validate_pricing_model(&model).is_err());
    model.set_variable_range_by_index(first, ospf_rust_core::variable::VariableRange::fixed(0.0)).unwrap();
    let raw = vec![0.0; model.constraints().len()];
    assert!(context.extract_pricing_duals(&MetaModel::new("foreign"), &raw).is_err());
    assert!(context.extract_pricing_duals(&model, &raw[..raw.len() - 1]).is_err());
    assert!(context.extract_pricing_duals(&model, &vec![f64::NAN; raw.len()]).is_err());
    model.add_linear_objective_input(LinearObjectiveInput::minimize("tampered-column").term(second, 1.0));
    assert!(context.extract_pricing_duals(&model, &raw).is_err());
    model.remove_objective_terms(&[second].into());
    context.aggregation.validate_pricing_model(&model).unwrap();
    model.add_linear_constraint(&[(second, 0.0)], ConstraintRelation::LessEqual, 1.0, "unmapped-zero-price-row").unwrap();
    assert!(context.extract_pricing_duals(&model, &vec![0.0; model.constraints().len()]).is_err());
}

#[test]
fn extension_factory_creates_distinct_model_bound_objects_for_full_lifecycle() {
    struct BoundExtension {
        identity: std::sync::Mutex<Option<u64>>,
        events: Arc<std::sync::Mutex<Vec<&'static str>>>,
    }
    impl RouteCompilationExtension<f64> for BoundExtension {
        fn register(&self, model: &mut MetaModel<f64>, _: &crate::domain::route_compilation::RouteCompilationAggregation<f64>) -> crate::Result<()> {
            let mut identity = self.identity.lock().unwrap();
            assert!(identity.is_none());
            *identity = Some(model.model_identity());
            self.events.lock().unwrap().push("register"); Ok(())
        }
        fn add_columns(&self, _: &[Route<f64>], model: &mut MetaModel<f64>) -> crate::Result<()> {
            assert_eq!(*self.identity.lock().unwrap(), Some(model.model_identity()));
            self.events.lock().unwrap().push("add"); Ok(())
        }
        fn remove_columns(&self, _: &[Route<f64>], model: &mut MetaModel<f64>) -> crate::Result<()> {
            assert_eq!(*self.identity.lock().unwrap(), Some(model.model_identity()));
            self.events.lock().unwrap().push("remove"); Ok(())
        }
        fn refresh_shadow_price(&self, model: &MetaModel<f64>, _: &[f64], _: &mut crate::domain::vrp::VrpShadowPriceMap) -> crate::Result<()> {
            assert_eq!(*self.identity.lock().unwrap(), Some(model.model_identity()));
            self.events.lock().unwrap().push("dual"); Ok(())
        }
        fn extract_solution(&self, _: &mut Vec<Route<f64>>) -> crate::Result<()> {
            self.events.lock().unwrap().push("extract"); Ok(())
        }
    }
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = events.clone();
    let factory: Arc<dyn RouteCompilationExtensionFactory<f64>> = Arc::new(move || {
        Ok(Box::new(BoundExtension { identity: std::sync::Mutex::new(None), events: observed.clone() }) as Box<dyn RouteCompilationExtension<f64>>)
    });
    let instance = test_instance();
    let route = InitialRouteGenerator::new(instance.clone(), EuclideanDistanceCalculator,
        DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator), DistanceArcCostCalculator,
        FixedPlusArcCostPolicy).generate(None).unwrap().remove(0);
    for id in ["first-branch", "second-branch"] {
        let mut model = MetaModel::new(id);
        let mut context = RouteCompilationContext::with_extensions(instance.clone(), vec![factory.create().unwrap()]);
        context.register(&mut model).unwrap();
        context.add_columns(0, [route.clone()], &mut model).unwrap();
        context.extract_pricing_duals(&model, &vec![0.0; model.constraints().len()]).unwrap();
        context.remove_columns([route.clone()], &mut model).unwrap();
        context.extract_solution(&model, 1e-7).unwrap();
    }
    assert_eq!(*events.lock().unwrap(), ["register", "add", "dual", "remove", "extract"].repeat(2));
    let provider = BranchNodeSolver::new(instance.clone(), Arc::new(FakeLpSolver), BranchNodeSolverConfig {
        distance_calculator: EuclideanDistanceCalculator,
        travel_time_calculator: DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
        arc_cost_calculator: DistanceArcCostCalculator, route_cost_policy: FixedPlusArcCostPolicy,
        arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
        dominance_policy: DefaultLabelDominancePolicy, column_selector: DefaultPricingColumnSelector,
        extension_factories: vec![factory], pricing_rows: Arc::new(Default::default()),
    }).unwrap();
    for _ in 0..2 {
        let result = provider.solve_node(&BranchNode::root(instance.start_depot.node.id.clone(), instance.end_depot.node.id.clone()).unwrap(),
            &[], &BranchNodeSolveContext { max_cg_iterations: 3, max_columns_per_pricing: usize::MAX,
                max_labels: None, deadline: None, cancellation: None }).unwrap();
        assert_eq!(result.status, BranchNodeSolveStatus::Optimal);
    }
    assert_eq!(events.lock().unwrap().iter().filter(|event| **event == "register").count(), 4);
}

#[cfg(feature = "scip")]
#[test]
#[ignore = "需要本机 SCIP runtime / requires a native SCIP runtime"]
fn scip_extension_rows_run_through_both_phases_and_prove_infeasible_usage() {
    use ospf_rust_core::solver::solvers::scip::{SCIPConfig, SCIPSolver};

    for (lower, upper, feasible) in [
        (None, Some(-1.0), true),
        (None, Some(-2.0), false),
        (Some(-1.0), Some(-1.0), true),
        (Some(0.0), None, false),
    ] {
        let instance = test_instance();
        let catalog = Arc::new(RoutePricingRows::new(vec![RoutePricingRow {
            id: "native".into(), unit: Meter::INSTANT.clone(),
            lower: lower.map(|value| Quantity::new(value, Meter::INSTANT.clone())),
            upper: upper.map(|value| Quantity::new(value, Meter::INSTANT.clone())), coefficient: Arc::new(SignedArcUsage),
        }]).unwrap());
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            arc_cost_calculator: DistanceArcCostCalculator, route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy, column_selector: DefaultPricingColumnSelector,
            extension_factories: vec![], pricing_rows: catalog,
        };
        let solver = SCIPSolver::with_config(SCIPConfig::recommended_lp_subproblem_defaults());
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(solver), config).unwrap();
        let result = provider.solve_node(&BranchNode::root(instance.start_depot.node.id.clone(), instance.end_depot.node.id.clone()).unwrap(),
            &[], &BranchNodeSolveContext { max_cg_iterations: 10, max_columns_per_pricing: usize::MAX,
                max_labels: None, deadline: None, cancellation: None }).unwrap();
        if feasible {
            assert_eq!(result.status, BranchNodeSolveStatus::Optimal);
            assert!(result.is_integer);
            assert_eq!(result.lp_objective, 3.0);
        } else {
            assert_eq!(result.status, BranchNodeSolveStatus::Infeasible);
        }
        assert!(result.pricing_complete);
    }
    let instance = multi_customer_instance(2);
    let provider = BranchNodeSolver::new(instance.clone(), Arc::new(SCIPSolver::with_config(
        SCIPConfig::recommended_lp_subproblem_defaults())), BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            arc_cost_calculator: DistanceArcCostCalculator, route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy, column_selector: DefaultPricingColumnSelector,
            extension_factories: vec![], pricing_rows: Arc::new(Default::default()),
        }).unwrap();
    let result = provider.solve_node(&BranchNode::root(instance.start_depot.node.id.clone(), instance.end_depot.node.id.clone()).unwrap(),
        &[], &BranchNodeSolveContext { max_cg_iterations: 10, max_columns_per_pricing: 0,
            max_labels: None, deadline: None, cancellation: None }).unwrap();
    assert_eq!(result.status, BranchNodeSolveStatus::SolverStopped);
    assert!(!result.pricing_complete);
}
