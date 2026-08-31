//! Demo5 application / Demo5 application.

use std::error::Error;
use std::sync::Arc;

use ospf_rust_core::solver::{LinearSolver, SolveReport};
use ospf_rust_framework_network_scheduling::application::{
    BranchAndPriceConfig, BranchNodeSolver, BranchNodeSolverConfig, VrptwApplicationService,
};
use ospf_rust_framework_network_scheduling::domain::route_generation::{
    DefaultLabelDominancePolicy, DefaultPricingColumnSelector,
};
use ospf_rust_framework_network_scheduling::domain::vrp::{
    DefaultArcFeasibilityPolicy, DistanceArcCostCalculator, DistanceAsTravelTimeCalculator,
    EuclideanDistanceCalculator, FixedPlusArcCostPolicy, RouteValidator, VrptwInstance,
};

use super::infrastructure::{Demo5Input, SemanticParameter, instance_from_solomon, parse_solomon};

const INLINE_SOLOMON: &str = "C101\nVEHICLE\nNUMBER CAPACITY\n2 2\nCUSTOMER\nCUST NO. XCOORD YCOORD DEMAND READY DUE SERVICE\n0 0 0 0 0 1000 0\n1 1 0 1 0 1000 0\n2 2 0 1 0 1000 0\n3 3 0 1 0 1000 0\n";

/// 运行 demo5 / Run demo5.
#[cfg(feature = "demo5-scip-bp")]
pub fn run() -> Result<(), Box<dyn Error>> {
    let config =
        ospf_rust_core::solver::solvers::scip::SCIPConfig::recommended_lp_subproblem_defaults()
            .with_output(false);
    run_with_solver(
        ospf_rust_core::solver::solvers::SCIPSolver::with_config(config),
        demo5_input(),
    )
}

/// 运行 Gurobi 后端 demo5 / Run demo5 with the Gurobi backend.
#[cfg(all(
    not(feature = "demo5-scip-bp"),
    any(feature = "backend-gurobi", feature = "demo5-gurobi-bp")
))]
pub fn run() -> Result<(), Box<dyn Error>> {
    run_with_solver(
        ospf_rust_core::solver::solvers::GurobiSolver::new(),
        demo5_input(),
    )
}

/// 无 solver feature 时给出明确提示 / Report an explicit hint without a solver feature.
#[cfg(not(any(
    feature = "backend-gurobi",
    feature = "demo5-gurobi-bp",
    feature = "demo5-scip-bp"
)))]
pub fn run() -> Result<(), Box<dyn Error>> {
    Err("demo5 requires --features demo5-gurobi-bp or demo5-scip-bp".into())
}

fn demo5_input() -> Demo5Input {
    Demo5Input {
        solomon_text: INLINE_SOLOMON.to_owned(),
        ..Default::default()
    }
}

fn run_with_solver<S>(solver: S, input: Demo5Input) -> Result<(), Box<dyn Error>>
where
    S: LinearSolver + 'static,
{
    let data = parse_solomon(&input.solomon_text)?;
    let instance = instance_from_solomon(&data)?;
    let parameter = SemanticParameter {
        time_limit: input.time_limit,
        node_limit: input.node_limit,
        relative_gap_tolerance: input.relative_gap_tolerance,
        max_cg_iterations_per_node: input.max_cg_iterations_per_node,
    };
    let result = solve_instance(instance.clone(), solver, parameter)?;

    println!("=== framework:demo5 ===");
    println!("problem_status: {:?}", result.problem_status);
    println!("termination_reason: {:?}", result.termination_reason);
    println!("lower_bound: {:?}", result.statistics.best_bound_value);
    println!("upper_bound: {:?}", result.trace.upper_bound);
    println!("relative_gap: {:?}", result.statistics.relative_gap);
    if let Some(solution) = result.incumbent() {
        for route in &solution.routes {
            RouteValidator::validate(
                &instance,
                route,
                &EuclideanDistanceCalculator,
                &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
                &DistanceArcCostCalculator,
                &FixedPlusArcCostPolicy,
                None,
            )?;
            println!("route {} cost={:?}", route.signature(), route.cost.value);
        }
    }
    Ok(())
}

/// 使用 demo5 的统一 wiring 求解适配后的实例 / Solve an adapted instance through demo5 wiring.
pub fn solve_instance<S>(
    instance: Arc<VrptwInstance<f64>>,
    solver: S,
    parameter: SemanticParameter,
) -> Result<
    SolveReport<ospf_rust_framework_network_scheduling::domain::vrp::VrptwSolution<f64>>,
    Box<dyn Error>,
>
where
    S: LinearSolver + 'static,
{
    let config = BranchNodeSolverConfig {
        distance_calculator: EuclideanDistanceCalculator,
        travel_time_calculator: DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
        arc_cost_calculator: DistanceArcCostCalculator,
        route_cost_policy: FixedPlusArcCostPolicy,
        arc_feasibility_policy: Arc::new(DefaultArcFeasibilityPolicy),
        dominance_policy: DefaultLabelDominancePolicy,
        column_selector: DefaultPricingColumnSelector,
        extensions: Vec::new(),
    };
    let provider = BranchNodeSolver::new(instance.clone(), Arc::new(solver), config)?;
    let mut branch_config = BranchAndPriceConfig::default();
    branch_config.time_limit = parameter.time_limit;
    branch_config.node_limit = parameter.node_limit;
    branch_config.relative_gap_tolerance = parameter.relative_gap_tolerance;
    branch_config.max_cg_iterations_per_node = parameter.max_cg_iterations_per_node;
    Ok(VrptwApplicationService::new(instance, provider, branch_config)?.solve()?)
}

#[cfg(all(test, feature = "demo5-gurobi-bp"))]
mod tests {
    use super::*;
    use crate::framework::demo5::direct_mip::{DirectMipPolicy, solve as solve_direct_mip};
    use crate::framework::demo5::infrastructure::{
        all100_instance, first25_instance, native_solver_available, proof100_instance,
    };
    use ospf_rust_core::solver::solvers::GurobiSolver;
    use ospf_rust_core::solver::solvers::gurobi::GurobiConfig;
    use ospf_rust_core::solver::{ProblemStatus, SolveReport, SolverStatus, TerminationReason};
    use std::time::Duration;

    #[test]
    #[ignore = "requires a native Gurobi installation; run with --include-ignored"]
    fn demo17_25_branch_and_price_matches_direct_mip_objective() {
        let instance = first25_instance().expect("first 25 Demo17 instance");
        let probe_solver = GurobiSolver::new();
        assert!(
            native_solver_available(&probe_solver),
            "Gurobi native environment is required for the 25-customer gate"
        );
        let direct_solver = GurobiSolver::with_config(
            GurobiConfig::new()
                .with_time_limit(60.0)
                .with_mip_gap(1e-4)
                .with_output(false),
        );
        let direct_policy = DirectMipPolicy {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
        };
        let direct =
            solve_direct_mip(&instance, &direct_solver, &direct_policy).expect("direct-MIP solve");
        assert!(direct.objective.is_finite());
        assert!(direct.best_bound.is_some_and(f64::is_finite));
        let branch_solver =
            GurobiSolver::with_config(GurobiConfig::new().with_time_limit(60.0).with_output(false));
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(branch_solver), config)
            .expect("branch-node provider");
        let result = VrptwApplicationService::new(
            instance.clone(),
            provider,
            BranchAndPriceConfig {
                time_limit: Some(Duration::from_secs(60)),
                ..BranchAndPriceConfig::default()
            },
        )
        .expect("application service")
        .solve()
        .expect("branch-and-price solve");
        assert_ne!(result.termination_reason, TerminationReason::Interrupted);
        let solution = result
            .incumbent()
            .expect("25-customer gate requires a Branch-and-Price incumbent");
        assert!(result.trace.upper_bound.is_some_and(f64::is_finite));
        let served = solution
            .routes
            .iter()
            .flat_map(|route| route.customer_ids())
            .collect::<std::collections::HashSet<_>>();
        let expected = instance
            .customers
            .iter()
            .map(|customer| customer.id.clone())
            .collect::<std::collections::HashSet<_>>();
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
            .expect("branch-and-price route validation");
        }
        assert!(
            result
                .trace
                .upper_bound
                .is_some_and(|upper| (upper - solution.total_cost.value).abs() <= 1e-6)
        );
        assert!(
            direct
                .best_bound
                .is_some_and(|best_bound| best_bound <= solution.total_cost.value + 1e-4)
        );
        if result.is_optimal() && direct.relative_gap.is_some_and(|gap| gap <= 1e-6) {
            assert!(
                (direct.objective - result.trace.upper_bound.expect("optimal upper bound")).abs()
                    <= 1e-4
            );
        } else {
            if let Some(lower_bound) = result.statistics.best_bound_value {
                assert!(lower_bound <= direct.objective + 1e-4);
            }
            if let Some(upper_bound) = result.trace.upper_bound {
                assert!(
                    direct
                        .best_bound
                        .is_none_or(|best_bound| best_bound <= upper_bound + 1e-4)
                );
            }
        }
        assert!(result.trace.total_iterations > 0);
    }

    #[test]
    #[ignore = "requires a native Gurobi installation; run with --include-ignored"]
    fn demo17_100_branch_and_price_smoke_respects_limits() {
        let instance = all100_instance().expect("all 100 Demo17 customers");
        let probe_solver = GurobiSolver::new();
        assert!(
            native_solver_available(&probe_solver),
            "Gurobi native environment is required for the 100-customer gate"
        );
        let branch_solver =
            GurobiSolver::with_config(GurobiConfig::new().with_time_limit(60.0).with_output(false));
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(branch_solver), config)
            .expect("branch-node provider");
        let result = VrptwApplicationService::new(
            instance.clone(),
            provider,
            BranchAndPriceConfig {
                time_limit: Some(Duration::from_secs(60)),
                node_limit: 50,
                ..BranchAndPriceConfig::default()
            },
        )
        .expect("application service")
        .solve()
        .expect("branch-and-price smoke solve");
        assert_ne!(result.termination_reason, TerminationReason::Interrupted);
        if let Some(solution) = result.incumbent() {
            let served = solution
                .routes
                .iter()
                .flat_map(|route| route.customer_ids())
                .collect::<std::collections::HashSet<_>>();
            let expected = instance
                .customers
                .iter()
                .map(|customer| customer.id.clone())
                .collect::<std::collections::HashSet<_>>();
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
                .expect("100-customer route validation");
            }
        }
    }

    #[test]
    #[ignore = "requires a native Gurobi installation; run with --include-ignored"]
    fn proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds() {
        let instance = proof100_instance().expect("100-customer proof fixture");
        let probe_solver = GurobiSolver::new();
        assert!(
            native_solver_available(&probe_solver),
            "Gurobi native environment is required for the strict proof gate"
        );
        let direct_solver = GurobiSolver::with_config(
            GurobiConfig::new()
                .with_time_limit(60.0)
                .with_mip_gap(0.0)
                .with_output(false),
        );
        let direct_policy = DirectMipPolicy {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
        };
        let direct = solve_direct_mip(&instance, &direct_solver, &direct_policy)
            .expect("strict direct-MIP proof");
        assert_eq!(direct.status, SolverStatus::Optimal);
        assert!((direct.objective - 100.0).abs() <= 1e-8);
        assert!(
            direct
                .best_bound
                .is_some_and(|bound| (bound - 100.0).abs() <= 1e-8)
        );
        assert!(direct.relative_gap.is_some_and(|gap| gap <= 1e-8));
        assert_eq!(direct.customer_sequences.len(), 100);
        assert!(
            direct
                .customer_sequences
                .iter()
                .all(|sequence| sequence.len() == 1)
        );

        let branch_solver = GurobiSolver::with_config(
            GurobiConfig::new()
                .with_time_limit(60.0)
                .with_mip_gap(0.0)
                .with_output(false),
        );
        let result = solve_instance(
            instance.clone(),
            branch_solver,
            SemanticParameter {
                time_limit: Some(Duration::from_secs(60)),
                node_limit: 10,
                relative_gap_tolerance: 0.0,
                ..SemanticParameter::default()
            },
        )
        .expect("strict 100-customer branch-and-price proof");
        assert!(result.is_optimal());
        assert!(
            result
                .statistics
                .best_bound_value
                .is_some_and(|bound| (bound - 100.0).abs() <= 1e-8)
        );
        assert!(
            result
                .trace
                .upper_bound
                .is_some_and(|bound| (bound - 100.0).abs() <= 1e-8)
        );
        assert!(
            result
                .statistics
                .relative_gap
                .is_some_and(|gap| gap <= 1e-8)
        );
        let solution = result.incumbent().expect("100-customer incumbent");
        assert!((solution.total_cost.value - 100.0).abs() <= 1e-8);
        assert_eq!(solution.routes.len(), 100);
        assert!(
            solution
                .routes
                .iter()
                .all(|route| route.customer_ids().len() == 1)
        );
    }
}

#[cfg(all(test, feature = "demo5-scip-bp"))]
mod scip_tests {
    use super::*;
    use crate::framework::demo5::infrastructure::{first25_instance, native_solver_available};
    use ospf_rust_core::solver::ProblemStatus;
    use ospf_rust_core::solver::solvers::scip::{SCIPConfig, SCIPSolver};
    use std::time::Duration;

    #[test]
    #[ignore = "requires a native SCIP installation; run with --include-ignored"]
    fn demo17_25_scip_branch_and_price_returns_legal_terminal() {
        let instance = first25_instance().expect("first 25 Demo17 customers");
        let solver = SCIPSolver::with_config(
            SCIPConfig::recommended_lp_subproblem_defaults()
                .with_time_limit(60.0)
                .with_output(false),
        );
        assert!(
            native_solver_available(&solver),
            "SCIP native environment is required for the SCIP gate"
        );
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(solver), config)
            .expect("branch-node provider");
        let result = VrptwApplicationService::new(
            instance.clone(),
            provider,
            BranchAndPriceConfig {
                time_limit: Some(Duration::from_secs(60)),
                node_limit: 50,
                ..BranchAndPriceConfig::default()
            },
        )
        .expect("application service")
        .solve()
        .expect("SCIP branch-and-price solve");
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
                .collect::<std::collections::HashSet<_>>();
            let expected = instance
                .customers
                .iter()
                .map(|customer| customer.id.clone())
                .collect::<std::collections::HashSet<_>>();
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
}
