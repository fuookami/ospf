#![cfg(feature = "demo5-offline")]

use ospf_rust_example::framework::demo5::infrastructure::{all100_instance, first25_instance};

#[cfg(feature = "demo5-gurobi-bp")]
use std::collections::HashSet;
#[cfg(feature = "demo5-gurobi-bp")]
use std::time::Duration;

#[cfg(feature = "demo5-gurobi-bp")]
use ospf_rust_core::solver::TerminationReason;
#[cfg(feature = "demo5-gurobi-bp")]
use ospf_rust_core::solver::solvers::gurobi::{GurobiConfig, GurobiSolver};
#[cfg(feature = "demo5-gurobi-bp")]
use ospf_rust_example::framework::demo5::application::solve_instance;
#[cfg(feature = "demo5-gurobi-bp")]
use ospf_rust_example::framework::demo5::full_route_master;
#[cfg(feature = "demo5-gurobi-bp")]
use ospf_rust_example::framework::demo5::infrastructure::{
    SemanticParameter, native_solver_available, small_instance,
};
#[cfg(feature = "demo5-gurobi-bp")]
use ospf_rust_framework_network_scheduling::domain::vrp::{
    DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator,
    FixedPlusArcCostPolicy, RouteValidator,
};

#[test]
fn demo5_gurobi_target_exposes_reproducible_fixtures() {
    let first25 = first25_instance().expect("Demo17 25-customer fixture");
    let all100 = all100_instance().expect("Demo17 100-customer fixture");
    assert_eq!(first25.customers.len(), 25);
    assert_eq!(all100.customers.len(), 100);
}

#[cfg(feature = "demo5-gurobi-bp")]
#[test]
#[ignore = "requires a native Gurobi installation; run with --include-ignored"]
fn demo5_gurobi_target_runs_branch_and_price_and_validates_incumbent() {
    let instance = first25_instance().expect("Demo17 25-customer fixture");
    let solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_time_limit(60.0)
            .with_mip_gap(1e-4)
            .with_output(false),
    );
    assert!(
        native_solver_available(&solver),
        "Gurobi native environment is required for demo5_gurobi_bp"
    );
    let result = solve_instance(
        instance.clone(),
        solver,
        SemanticParameter {
            time_limit: Some(Duration::from_secs(60)),
            node_limit: 50,
            ..SemanticParameter::default()
        },
    )
    .expect("Gurobi Branch-and-Price solve");

    assert_ne!(result.termination_reason, TerminationReason::Interrupted);
    assert!(result.trace.total_iterations > 0);
    let solution = result
        .incumbent()
        .expect("25-customer Gurobi gate requires an incumbent");
    assert!(result.trace.upper_bound.is_some_and(f64::is_finite));
    let served = solution
        .routes
        .iter()
        .flat_map(|route| route.customer_ids())
        .collect::<HashSet<_>>();
    let expected = instance
        .customers
        .iter()
        .map(|customer| customer.id.clone())
        .collect::<HashSet<_>>();
    assert_eq!(served, expected);
    for route in &solution.routes {
        RouteValidator::validate(
            &instance,
            route,
            &EuclideanDistanceCalculator,
            &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            &DistanceArcCostCalculator,
            &FixedPlusArcCostPolicy,
            None,
        )
        .expect("Gurobi route validation");
    }
}

#[cfg(feature = "demo5-gurobi-bp")]
#[test]
#[ignore = "requires a native Gurobi installation; run with --include-ignored"]
fn demo5_gurobi_small_branch_and_price_matches_full_route_master_oracle() {
    let instance = small_instance().expect("small Demo17 instance");
    let oracle_solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_time_limit(30.0)
            .with_mip_gap(0.0)
            .with_output(false),
    );
    assert!(
        native_solver_available(&oracle_solver),
        "Gurobi native environment is required for the full-route oracle"
    );
    let oracle =
        full_route_master::solve(&instance, &oracle_solver).expect("full-route master oracle");
    assert!(oracle.route_count >= instance.customers.len());
    assert!(oracle.objective.is_finite());

    let branch_solver = GurobiSolver::with_config(
        GurobiConfig::new()
            .with_time_limit(30.0)
            .with_mip_gap(0.0)
            .with_output(false),
    );
    let result = solve_instance(
        instance.clone(),
        branch_solver,
        SemanticParameter {
            time_limit: Some(Duration::from_secs(30)),
            ..SemanticParameter::default()
        },
    )
    .expect("small branch-and-price solve");
    assert!(result.is_optimal());
    let solution = result
        .incumbent()
        .expect("small branch-and-price incumbent");
    assert!((solution.total_cost.value - oracle.objective).abs() <= 1e-6);
    assert!(
        result
            .trace
            .upper_bound
            .is_some_and(|value| { (value - oracle.objective).abs() <= 1e-6 })
    );
    let served = solution
        .routes
        .iter()
        .flat_map(|route| route.customer_ids())
        .collect::<HashSet<_>>();
    let expected = instance
        .customers
        .iter()
        .map(|customer| customer.id.clone())
        .collect::<HashSet<_>>();
    assert_eq!(served, expected);
}

#[cfg(not(feature = "demo5-gurobi-bp"))]
#[test]
#[ignore = "enable demo5-gurobi-bp and run with --include-ignored for the native gate"]
fn demo5_gurobi_native_gate_requires_feature() {}
