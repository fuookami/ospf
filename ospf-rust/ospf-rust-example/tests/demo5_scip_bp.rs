#![cfg(feature = "demo5-offline")]

use ospf_rust_example::framework::demo5::infrastructure::{instance_from_solomon, parse_solomon};

#[cfg(feature = "demo5-scip-bp")]
use std::collections::HashSet;
#[cfg(feature = "demo5-scip-bp")]
use std::time::Duration;

#[cfg(feature = "demo5-scip-bp")]
use ospf_rust_core::solver::ProblemStatus;
#[cfg(feature = "demo5-scip-bp")]
use ospf_rust_core::solver::solvers::scip::{SCIPConfig, SCIPSolver};
#[cfg(feature = "demo5-scip-bp")]
use ospf_rust_example::framework::demo5::application::solve_instance;
#[cfg(feature = "demo5-scip-bp")]
use ospf_rust_example::framework::demo5::infrastructure::{
    SemanticParameter, first25_instance, native_solver_available,
};
#[cfg(feature = "demo5-scip-bp")]
use ospf_rust_framework_network_scheduling::domain::vrp::{
    DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator,
    FixedPlusArcCostPolicy, RouteValidator,
};

const INPUT: &str = "C101\nVEHICLE\nNUMBER CAPACITY\n2 10\nCUSTOMER\nCUST NO. XCOORD YCOORD DEMAND READY DUE SERVICE\n0 0 0 0 0 1000 0\n1 1 0 1 0 1000 0\n2 2 0 1 0 1000 0\n";

#[test]
fn demo5_scip_target_covers_parser_and_adapter() {
    let data = parse_solomon(INPUT).expect("valid Solomon fixture");
    assert_eq!(data.customers.len(), 2);
    let instance = instance_from_solomon(&data).expect("valid VRPTW adapter output");
    assert_eq!(instance.customers.len(), 2);
    assert_eq!(instance.vehicle_types[0].amount, 2);
}

#[test]
#[cfg(feature = "demo5-scip-bp")]
#[ignore = "requires a native SCIP installation; run with --include-ignored"]
fn demo5_scip_target_solves_25_customer_instance_with_a_legal_result() {
    let instance = first25_instance().expect("Demo17 25-customer fixture");
    let solver = SCIPSolver::with_config(
        SCIPConfig::recommended_lp_subproblem_defaults()
            .with_time_limit(60.0)
            .with_output(false),
    );
    assert!(
        native_solver_available(&solver),
        "SCIP native environment is required for demo5_scip_bp"
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
    .expect("SCIP Branch-and-Price solve");

    if result.problem_status == ProblemStatus::Infeasible {
        assert!(!result.has_incumbent());
    }
    if result.problem_status == ProblemStatus::Feasible {
        assert!(result.has_incumbent());
    }
    assert!(
        result
            .statistics
            .best_bound_value
            .is_none_or(f64::is_finite)
    );
    assert!(result.trace.upper_bound.is_none_or(f64::is_finite));
    assert!(result.statistics.relative_gap.is_none_or(f64::is_finite));
    if let Some(solution) = result.incumbent() {
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
            .expect("SCIP route validation");
        }
    }
}

#[cfg(not(feature = "demo5-scip-bp"))]
#[test]
#[ignore = "enable demo5-scip-bp and run with --include-ignored for the native gate"]
fn demo5_scip_native_gate_requires_feature() {}
