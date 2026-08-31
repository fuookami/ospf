//! 通用网络流与 VRPTW 分支定价框架 / Generic network-flow and VRPTW branch-and-price framework.
//!
//! 本 crate 将通用 graph/flow 原语与 VRPTW、ESPPRC、受限主问题和
//! Branch-and-Price 组织在一个有明确依赖方向的领域框架中。
//! This crate organizes generic graph/flow primitives, VRPTW, ESPPRC,
//! restricted masters, and Branch-and-Price in one layered domain framework.

pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use error::{NetworkSchedulingError, Result};

#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)]

    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;
    use std::time::Instant;

    use ospf_rust_core::model::MetaModel;
    #[cfg(any(
        feature = "gurobi10",
        feature = "gurobi11",
        feature = "gurobi12",
        feature = "scip"
    ))]
    use ospf_rust_core::solver::LinearSolver;
    use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
    use ospf_rust_core::solver::{
        ProblemStatus, SolveFingerprints, SolveProof, SolveReport, SolveSolution,
        TerminationReason, linear_model_fingerprint,
    };
    use ospf_rust_framework_gantt_scheduling::infrastructure::{TimeRange, TimeWindow};
    use ospf_rust_quantities::Quantity;
    use ospf_rust_quantities::unit::{CTUnit, Kilogram, Kilometer, Meter};
    use time::{Duration as TimeDuration, OffsetDateTime};

    use crate::application::{
        BranchAndPriceConfig, BranchDecision, BranchNode, BranchNodeSolveContext,
        BranchNodeSolveResult, BranchNodeSolveStatus, BranchNodeSolver, BranchNodeSolverConfig,
        BranchNodeSolverProvider, BranchPath, LinearProgrammingRequest, LinearProgrammingResult,
        LinearProgrammingSolver, LinearProgrammingStatus, select_branch_decision,
    };
    use crate::domain::flow::{
        FlowContext, FlowContextExtension, FlowGraph, FlowNode, FlowUnits, SupplyDemand,
    };
    use crate::domain::route_compilation::{RouteCompilationContext, RouteCompilationExtension};
    use crate::domain::route_generation::{
        CancellationToken, DefaultLabelDominancePolicy, DefaultPricingColumnSelector, EspprcPricer,
        InitialRouteGenerator, PricingGraph, PricingRequest, RouteGraphBuilder,
    };
    use crate::domain::vrp::{
        BranchMask, Coordinate, Customer, CustomerId, Depot, DistanceArcCostCalculator,
        DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator, FixedPlusArcCostPolicy,
        PricingDuals, PricingPhase, ResourceArc, Route, RouteStop, RouteValidator,
        ServiceTimeWindow, TravelTime, TravelTimeCalculator, VehicleType, VehicleTypeId, VrptwArc,
        VrptwInstance, VrptwUnits, coordinate_node, default_arc_id,
    };
    use crate::infrastructure::{
        CapacityBounds, NetworkArc, NetworkArcId, NetworkCost, NetworkGraph, NetworkNode,
        NetworkNodeId,
    };

    fn test_instance() -> Arc<VrptwInstance<f64>> {
        let start = OffsetDateTime::UNIX_EPOCH;
        let end = start + TimeDuration::hours(24);
        let scheduling_window = TimeWindow::seconds(TimeRange::new(start, end), 0.0, false, 1.0);
        let units = VrptwUnits::default();
        let service_window = ServiceTimeWindow::new(start, end).expect("valid test window");
        test_instance_with_customer_window(scheduling_window, service_window, units)
    }

    fn test_instance_with_customer_window(
        scheduling_window: TimeWindow<f64>,
        customer_window: ServiceTimeWindow,
        units: VrptwUnits,
    ) -> Arc<VrptwInstance<f64>> {
        let start = scheduling_window.window.start;
        let end = scheduling_window.window.end;
        let start_node = coordinate_node("start", 0.0, 0.0, Meter::INSTANT.clone())
            .expect("valid start coordinate");
        let end_node =
            coordinate_node("end", 0.0, 0.0, Meter::INSTANT.clone()).expect("valid end coordinate");
        let customer_node = coordinate_node("c1-node", 1.0, 0.0, Meter::INSTANT.clone())
            .expect("valid customer coordinate");
        let customer = Customer::new(
            CustomerId::from("c1"),
            customer_node,
            Quantity::new(1.0, Kilogram::INSTANT.clone()),
            customer_window,
            TimeDuration::ZERO,
        )
        .expect("valid customer");
        let vehicle = VehicleType::new(
            VehicleTypeId::from("v1"),
            Quantity::new(10.0, Kilogram::INSTANT.clone()),
            Quantity::new(1.0, units.cost_unit.clone()),
            1,
        )
        .expect("valid vehicle");
        Arc::new(
            VrptwInstance::new(
                "test",
                Depot {
                    node: start_node,
                    time_window: ServiceTimeWindow::new(start, end).expect("valid start window"),
                },
                Depot {
                    node: end_node,
                    time_window: ServiceTimeWindow::new(start, end).expect("valid end window"),
                },
                vec![customer],
                vec![vehicle],
                scheduling_window,
                units,
                Default::default(),
            )
            .expect("valid test instance"),
        )
    }

    fn multi_customer_instance(customer_count: usize) -> Arc<VrptwInstance<f64>> {
        assert!(customer_count > 0);
        let start = OffsetDateTime::UNIX_EPOCH;
        let end = start + TimeDuration::hours(24);
        let scheduling_window = TimeWindow::seconds(TimeRange::new(start, end), 0.0, false, 1.0);
        let service_window = ServiceTimeWindow::new(start, end).expect("valid test window");
        let customers = (0..customer_count)
            .map(|index| {
                let node = coordinate_node(
                    format!("c{}-node", index + 1),
                    index as f64 + 1.0,
                    0.0,
                    Meter::INSTANT.clone(),
                )
                .expect("valid customer coordinate");
                Customer::new(
                    CustomerId::from(format!("c{}", index + 1)),
                    node,
                    Quantity::new(1.0, Kilogram::INSTANT.clone()),
                    service_window,
                    TimeDuration::ZERO,
                )
                .expect("valid customer")
            })
            .collect::<Vec<_>>();
        let vehicle = VehicleType::new(
            VehicleTypeId::from("v1"),
            Quantity::new(20.0, Kilogram::INSTANT.clone()),
            Quantity::new(1.0, VrptwUnits::default().cost_unit),
            2,
        )
        .expect("valid vehicle");
        Arc::new(
            VrptwInstance::new(
                "multi-customer-test",
                Depot {
                    node: coordinate_node("start", 0.0, 0.0, Meter::INSTANT.clone())
                        .expect("valid start coordinate"),
                    time_window: service_window,
                },
                Depot {
                    node: coordinate_node("end", 0.0, 0.0, Meter::INSTANT.clone())
                        .expect("valid end coordinate"),
                    time_window: service_window,
                },
                customers,
                vec![vehicle],
                scheduling_window,
                VrptwUnits::default(),
                Default::default(),
            )
            .expect("valid multi-customer instance"),
        )
    }

    fn multi_vehicle_type_instance() -> Arc<VrptwInstance<f64>> {
        let base = multi_customer_instance(2);
        let small = VehicleType::new(
            "small",
            Quantity::new(1.0, Kilogram::INSTANT.clone()),
            Quantity::new(1.0, base.units.cost_unit.clone()),
            2,
        )
        .expect("valid small vehicle type");
        let large = VehicleType::new(
            "large",
            Quantity::new(2.0, Kilogram::INSTANT.clone()),
            Quantity::new(10.0, base.units.cost_unit.clone()),
            1,
        )
        .expect("valid large vehicle type");
        Arc::new(
            VrptwInstance::new(
                "multi-vehicle-type-test",
                base.start_depot.clone(),
                base.end_depot.clone(),
                base.customers.clone(),
                vec![small, large],
                base.scheduling_window.clone(),
                base.units.clone(),
                base.tolerances,
            )
            .expect("valid multi-vehicle-type instance"),
        )
    }

    #[derive(Debug)]
    struct FlowFixtureExtension;

    impl FlowContextExtension<f64> for FlowFixtureExtension {
        fn register(
            &self,
            model: &mut MetaModel<f64>,
            _aggregation: &crate::domain::flow::FlowAggregation<f64>,
        ) -> crate::Result<()> {
            let index = model
                .register_auto_variable_with_range::<ospf_rust_core::variable::Continuous>(
                    "flow_extra_variable",
                    ospf_rust_core::variable::VariableRange::bounded(0.0, 1.0),
                )
                .map_err(|error| crate::NetworkSchedulingError::model(error.to_string()))?;
            model
                .add_linear_constraint(
                    &[(index, 1.0)],
                    ospf_rust_core::model::ConstraintRelation::LessEqual,
                    1.0,
                    "flow_extra_constraint",
                )
                .map_err(|error| crate::NetworkSchedulingError::model(error.to_string()))?;
            model.add_linear_objective(&[(index, 2.0)], "flow_extra_objective");
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingFlowFixtureExtension;

    impl FlowContextExtension<f64> for FailingFlowFixtureExtension {
        fn register(
            &self,
            model: &mut MetaModel<f64>,
            _aggregation: &crate::domain::flow::FlowAggregation<f64>,
        ) -> crate::Result<()> {
            model
                .register_auto_variable::<ospf_rust_core::variable::Continuous>(
                    "failing_flow_extension_variable",
                )
                .map_err(|error| crate::NetworkSchedulingError::model(error.to_string()))?;
            Err(crate::NetworkSchedulingError::contract(
                "注入的 flow 扩展注册失败 / injected flow extension registration failed",
            ))
        }
    }

    fn combined_two_customer_route(instance: &Arc<VrptwInstance<f64>>) -> Route<f64> {
        let start = instance.scheduling_window.window.start;
        let instant = |seconds: i64| start + TimeDuration::seconds(seconds);
        let load = |value: f64| Quantity::new(value, instance.units.load_unit.clone());
        let stops = vec![
            RouteStop {
                node_id: instance.start_depot.node.id.clone(),
                customer_id: None,
                arrival: instant(0),
                service_start: instant(0),
                departure: instant(0),
                accumulated_load: load(0.0),
            },
            RouteStop {
                node_id: instance.customers[0].node.id.clone(),
                customer_id: Some(instance.customers[0].id.clone()),
                arrival: instant(1),
                service_start: instant(1),
                departure: instant(1),
                accumulated_load: load(1.0),
            },
            RouteStop {
                node_id: instance.customers[1].node.id.clone(),
                customer_id: Some(instance.customers[1].id.clone()),
                arrival: instant(2),
                service_start: instant(2),
                departure: instant(2),
                accumulated_load: load(2.0),
            },
            RouteStop {
                node_id: instance.end_depot.node.id.clone(),
                customer_id: None,
                arrival: instant(4),
                service_start: instant(4),
                departure: instant(4),
                accumulated_load: load(2.0),
            },
        ];
        Route::new(
            VehicleTypeId::from("v1"),
            stops,
            Quantity::new(4.0, instance.units.distance_unit.clone()),
            Quantity::new(5.0, instance.units.cost_unit.clone()),
        )
        .expect("valid combined route")
    }

    #[derive(Debug, Clone)]
    struct ExhaustivePricingPath {
        arc_ids: Vec<NetworkArcId>,
        reduced_cost: f64,
    }

    fn exhaustive_pricing_paths(
        graph: &PricingGraph,
        instance: &VrptwInstance<f64>,
        max_depth: usize,
    ) -> Vec<ExhaustivePricingPath> {
        let mut paths = Vec::new();
        let mut visited = vec![false; instance.customers.len()];
        enumerate_pricing_paths(
            graph,
            instance,
            graph.start_index(),
            graph.nodes[graph.start_index()].ready_time,
            0.0,
            0.0,
            max_depth,
            &mut visited,
            &mut Vec::new(),
            &mut paths,
        );
        paths
    }

    #[allow(clippy::too_many_arguments)]
    fn enumerate_pricing_paths(
        graph: &PricingGraph,
        instance: &VrptwInstance<f64>,
        current_node: usize,
        current_time: f64,
        current_load: f64,
        reduced_cost: f64,
        max_depth: usize,
        visited: &mut [bool],
        arc_ids: &mut Vec<NetworkArcId>,
        paths: &mut Vec<ExhaustivePricingPath>,
    ) {
        let feasibility_tolerance = instance.tolerances.feasibility;
        let end_index = graph.end_index();
        for arc_index in &graph.outgoing[current_node] {
            let arc = &graph.arcs[*arc_index];
            let node = &graph.nodes[arc.to];
            let customer_index = node.customer_index;
            if customer_index.is_some_and(|index| {
                visited[index] || visited.iter().filter(|visited| **visited).count() >= max_depth
            }) {
                continue;
            }
            let travel = instance
                .scheduling_window
                .value_of_duration(arc.travel_time)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .expect("duration converts to f64");
            let arrival = current_time + travel;
            let service_start = arrival.max(node.ready_time);
            if service_start > node.due_time + feasibility_tolerance {
                continue;
            }
            let next_load = current_load + node.demand;
            if next_load > graph.vehicle_capacity + feasibility_tolerance {
                continue;
            }
            if let Some(index) = customer_index {
                visited[index] = true;
            }
            arc_ids.push(arc.arc_id.clone());
            let next_reduced_cost = reduced_cost + arc.reduced_cost;
            let visited_count = visited.iter().filter(|visited| **visited).count();
            if arc.to == end_index && visited_count > 0 {
                paths.push(ExhaustivePricingPath {
                    arc_ids: arc_ids.clone(),
                    reduced_cost: next_reduced_cost,
                });
            } else if arc.to != end_index {
                enumerate_pricing_paths(
                    graph,
                    instance,
                    arc.to,
                    service_start + node.service_time,
                    next_load,
                    next_reduced_cost,
                    max_depth,
                    visited,
                    arc_ids,
                    paths,
                );
            }
            arc_ids.pop();
            if let Some(index) = customer_index {
                visited[index] = false;
            }
        }
    }

    #[test]
    fn network_graph_rejects_duplicate_ids_and_keeps_parallel_endpoints() {
        let nodes = vec![NetworkNode::new("a", ()), NetworkNode::new("b", ())];
        let graph = NetworkGraph::new(
            nodes.clone(),
            vec![
                NetworkArc::new("ab-1", "a", "b", ()),
                NetworkArc::new("ab-2", "a", "b", ()),
            ],
        )
        .expect("parallel arcs are valid");
        assert_eq!(graph.outgoing(&NetworkNodeId::from("a")).count(), 2);
        assert!(
            NetworkGraph::new(
                nodes,
                vec![
                    NetworkArc::new("ab", "a", "b", ()),
                    NetworkArc::new("ab", "a", "b", ())
                ]
            )
            .is_err()
        );
    }

    #[test]
    fn flow_context_registers_shared_capacity_and_balance_constraints() {
        let unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let arc = crate::domain::flow::FlowArc::new(
            "ab",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(2.0, unit.clone()),
                Quantity::new(5.0, unit.clone()),
            )
            .expect("valid capacity"),
            NetworkCost::new(Quantity::new(2.0, cost_unit)),
        );
        let commodity = crate::domain::flow::SupplyDemand::new("m1")
            .with_balance("a", Quantity::new(5.0, unit.clone()))
            .with_balance("b", Quantity::new(-5.0, unit));
        let graph = crate::domain::flow::FlowGraph::new(
            vec![
                crate::domain::flow::FlowNode::new("a"),
                crate::domain::flow::FlowNode::new("b"),
            ],
            vec![arc],
            vec![commodity],
        )
        .expect("valid flow graph");
        let mut context = crate::domain::flow::FlowContext::new(graph);
        let mut model = MetaModel::<f64>::new("flow-test");
        context.register(&mut model).expect("flow registers");
        assert_eq!(model.tokens().len(), 1);
        assert_eq!(model.as_basic().constraints().len(), 4);
        let lower = model
            .as_basic()
            .constraints()
            .iter()
            .find(|constraint| constraint.name == "flow_capacity_ab_lower")
            .expect("non-zero lower-bound row");
        assert_eq!(
            lower.inequality.relation,
            ospf_rust_core::model::ConstraintRelation::GreaterEqual
        );
        assert_eq!(lower.inequality.rhs, 2.0);
        let linear = model
            .try_to_linear_triad_model()
            .expect("flow model flattens");
        let lower_row = linear
            .basic
            .constraint_names
            .iter()
            .position(|name| name == "flow_capacity_ab_lower")
            .expect("flattened lower-bound row");
        assert_eq!(linear.basic.b[lower_row], -2.0);
    }

    #[test]
    fn flow_context_binding_survives_model_move() {
        let unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let arc = crate::domain::flow::FlowArc::new(
            "ab",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(0.0, unit.clone()),
                Quantity::new(1.0, unit.clone()),
            )
            .expect("valid capacity"),
            NetworkCost::new(Quantity::new(1.0, cost_unit.clone())),
        );
        let graph = FlowGraph::new_with_units(
            vec![FlowNode::new("a"), FlowNode::new("b")],
            vec![arc],
            vec![
                SupplyDemand::new("m1")
                    .with_balance("a", Quantity::new(1.0, unit.clone()))
                    .with_balance("b", Quantity::new(-1.0, unit)),
            ],
            FlowUnits::new(Kilogram::INSTANT.clone(), cost_unit).expect("valid flow units"),
        )
        .expect("valid flow graph");
        let mut context = FlowContext::new(graph);
        let mut model_holder = Vec::with_capacity(1);
        model_holder.push(MetaModel::<f64>::new("flow-move-stable-model"));
        let model_identity = model_holder[0].model_identity();
        context
            .register(&mut model_holder[0])
            .expect("register flow context");
        model_holder.reserve(1);
        model_holder.push(MetaModel::<f64>::new("flow-move-padding"));
        assert_eq!(model_holder[0].model_identity(), model_identity);
        context
            .refresh_shadow_price(&model_holder[0], &[])
            .expect("moved model remains bound");
    }

    #[test]
    fn flow_value_objects_reject_non_comparable_values() {
        assert!(
            CapacityBounds::try_new(
                Quantity::new(f64::NAN, Kilogram::INSTANT.clone()),
                Quantity::new(1.0, Kilogram::INSTANT.clone()),
            )
            .is_err()
        );
        assert!(
            crate::infrastructure::Flow::try_new(Quantity::new(
                f64::NAN,
                Kilogram::INSTANT.clone(),
            ))
            .is_err()
        );
        assert!(
            CapacityBounds::try_new(
                Quantity::new(-1.0, Kilogram::INSTANT.clone()),
                Quantity::new(1.0, Kilogram::INSTANT.clone()),
            )
            .is_err(),
            "capacity lower bounds must be non-negative"
        );
    }

    #[test]
    fn model_lifecycle_bindings_survive_move_without_using_addresses() {
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);
        let mut compilation = RouteCompilationContext::new(instance);
        let mut model_holder = Vec::with_capacity(1);
        model_holder.push(MetaModel::<f64>::new("move-stable-model"));
        let model_identity = model_holder[0].model_identity();
        compilation
            .register(&mut model_holder[0])
            .expect("register compilation");
        model_holder.reserve(1);
        model_holder.push(MetaModel::<f64>::new("move-padding-model"));
        assert_eq!(model_holder[0].model_identity(), model_identity);
        compilation
            .add_columns(0, [route], &mut model_holder[0])
            .expect("moved model remains bound");
    }

    #[test]
    fn flow_supports_coordinate_free_bop_shared_capacity_and_mrp_shapes() {
        let flow_unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let shared_arc = crate::domain::flow::FlowArc::new(
            "shared",
            "source",
            "sink",
            CapacityBounds::try_new(
                Quantity::new(0.0, flow_unit.clone()),
                Quantity::new(5.0, flow_unit.clone()),
            )
            .expect("valid shared capacity"),
            NetworkCost::new(Quantity::new(-2.0, cost_unit.clone())),
        );
        let graph = FlowGraph::new_with_units(
            vec![FlowNode::new("source"), FlowNode::new("sink")],
            vec![shared_arc],
            vec![
                SupplyDemand::new("commodity-a")
                    .with_balance("source", Quantity::new(3.0, flow_unit.clone()))
                    .with_balance("sink", Quantity::new(-3.0, flow_unit.clone())),
                SupplyDemand::new("commodity-b")
                    .with_balance("source", Quantity::new(2.0, flow_unit.clone()))
                    .with_balance("sink", Quantity::new(-2.0, flow_unit.clone())),
            ],
            FlowUnits::new(flow_unit.clone(), cost_unit.clone()).expect("valid flow units"),
        )
        .expect("coordinate-free BOP flow graph");
        let mut context = FlowContext::new(graph);
        let mut model = MetaModel::<f64>::new("bop-mrp-flow-shapes");
        context.register(&mut model).expect("register shared flow");
        assert_eq!(model.tokens().len(), 2);
        let capacity_rows = model
            .as_basic()
            .constraints()
            .iter()
            .filter(|constraint| constraint.name.contains("flow_capacity_shared"))
            .collect::<Vec<_>>();
        assert_eq!(capacity_rows.len(), 2);
        assert!(
            capacity_rows
                .iter()
                .all(|constraint| { constraint.inequality.polynomial.monomials().len() == 2 })
        );

        let inventory_graph = FlowGraph::new_with_units(
            vec![
                FlowNode::new("raw@0"),
                FlowNode::new("raw@1"),
                FlowNode::new("finished@1"),
            ],
            vec![
                crate::domain::flow::FlowArc::new(
                    "inventory",
                    "raw@0",
                    "raw@1",
                    CapacityBounds::try_new(
                        Quantity::new(0.0, flow_unit.clone()),
                        Quantity::new(4.0, flow_unit.clone()),
                    )
                    .expect("valid inventory capacity"),
                    NetworkCost::new(Quantity::new(0.0, cost_unit.clone())),
                ),
                crate::domain::flow::FlowArc::new(
                    "production",
                    "raw@1",
                    "finished@1",
                    CapacityBounds::try_new(
                        Quantity::new(0.0, flow_unit.clone()),
                        Quantity::new(4.0, flow_unit.clone()),
                    )
                    .expect("valid production capacity"),
                    NetworkCost::new(Quantity::new(1.0, cost_unit)),
                ),
            ],
            vec![
                SupplyDemand::new("material")
                    .with_balance("raw@0", Quantity::new(4.0, flow_unit.clone()))
                    .with_balance("finished@1", Quantity::new(-4.0, flow_unit)),
            ],
            FlowUnits::new(Kilogram::INSTANT.clone(), VrptwUnits::default().cost_unit)
                .expect("valid inventory units"),
        )
        .expect("MRP-shaped flow graph");
        assert_eq!(inventory_graph.nodes.len(), 3);
        assert_eq!(inventory_graph.arcs.len(), 2);
    }

    #[test]
    fn mrp_shape_has_a_complete_bounded_integral_flow_oracle_certificate() {
        let flow_unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let graph = FlowGraph::new_with_units(
            vec![
                FlowNode::new("raw@0"),
                FlowNode::new("raw@1"),
                FlowNode::new("finished@1"),
            ],
            vec![
                crate::domain::flow::FlowArc::new(
                    "inventory",
                    "raw@0",
                    "raw@1",
                    CapacityBounds::try_new(
                        Quantity::new(0.0, flow_unit.clone()),
                        Quantity::new(4.0, flow_unit.clone()),
                    )
                    .expect("valid inventory capacity"),
                    NetworkCost::new(Quantity::new(0.0, cost_unit.clone())),
                ),
                crate::domain::flow::FlowArc::new(
                    "production",
                    "raw@1",
                    "finished@1",
                    CapacityBounds::try_new(
                        Quantity::new(0.0, flow_unit.clone()),
                        Quantity::new(4.0, flow_unit.clone()),
                    )
                    .expect("valid production capacity"),
                    NetworkCost::new(Quantity::new(1.0, cost_unit.clone())),
                ),
            ],
            vec![
                SupplyDemand::new("material")
                    .with_balance("raw@0", Quantity::new(4.0, flow_unit.clone()))
                    .with_balance("finished@1", Quantity::new(-4.0, flow_unit)),
            ],
            FlowUnits::new(Kilogram::INSTANT.clone(), cost_unit).expect("valid flow units"),
        )
        .expect("valid MRP-shaped graph");

        let result = crate::domain::flow::solve_bounded_integral_min_cost_flow(&graph)
            .expect("MRP oracle succeeds")
            .expect("MRP fixture is feasible");
        assert_eq!(result.objective, 4.0);
        assert_eq!(result.flows[&("material".into(), "inventory".into())], 4);
        assert_eq!(result.flows[&("material".into(), "production".into())], 4);
    }

    #[cfg(any(
        feature = "gurobi10",
        feature = "gurobi11",
        feature = "gurobi12",
        feature = "scip"
    ))]
    fn nonzero_lower_bound_flow_model() -> ospf_rust_core::model::intermediate::LinearTriadModel {
        let flow_unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let graph = FlowGraph::new_with_units(
            vec![FlowNode::new("source"), FlowNode::new("sink")],
            vec![crate::domain::flow::FlowArc::new(
                "negative",
                "source",
                "sink",
                CapacityBounds::try_new(
                    Quantity::new(2.0, flow_unit.clone()),
                    Quantity::new(5.0, flow_unit.clone()),
                )
                .expect("valid negative-cost capacity"),
                NetworkCost::new(Quantity::new(-2.0, cost_unit.clone())),
            )],
            vec![
                SupplyDemand::new("m1")
                    .with_balance("source", Quantity::new(5.0, flow_unit.clone()))
                    .with_balance("sink", Quantity::new(-5.0, flow_unit)),
            ],
            FlowUnits::new(Kilogram::INSTANT.clone(), cost_unit).expect("valid MCMF units"),
        )
        .expect("valid MCMF graph");
        let mut context = FlowContext::new(graph);
        let mut model = MetaModel::<f64>::new("negative-cost-mcmf");
        context.register(&mut model).expect("register MCMF model");
        model
            .try_to_linear_triad_model()
            .expect("linear MCMF model")
    }

    #[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
    #[test]
    fn negative_cost_mcmf_has_the_known_gurobi_lp_optimum() {
        let linear = nonzero_lower_bound_flow_model();
        let output = LinearSolver::solve_linear(
            &ospf_rust_core::solver::solvers::GurobiSolver::new(),
            &linear,
        )
        .expect("Gurobi MCMF solve");
        assert!(output.status.is_feasible());
        assert!((output.objective_value.expect("MCMF objective") + 10.0).abs() <= 1e-7);
    }

    #[cfg(feature = "scip")]
    #[test]
    fn nonzero_lower_bound_mcmf_has_a_valid_scip_solution() {
        let linear = nonzero_lower_bound_flow_model();
        let output = LinearSolver::solve_linear(
            &ospf_rust_core::solver::solvers::SCIPSolver::new(),
            &linear,
        )
        .expect("SCIP MCMF solve");
        assert!(output.status.is_feasible());
        assert!((output.objective_value.expect("SCIP MCMF objective") + 10.0).abs() <= 1e-7);
    }

    #[cfg(any(
        feature = "gurobi10",
        feature = "gurobi11",
        feature = "gurobi12",
        feature = "scip"
    ))]
    fn native_infeasible_column_generation_model() -> MetaModel<f64> {
        let mut model = MetaModel::<f64>::new("native-infeasible-column-generation");
        let index = model
            .register_auto_variable::<ospf_rust_core::variable::Continuous>("x")
            .expect("register native infeasible LP variable");
        model
            .add_linear_constraint(
                &[(index, 1.0)],
                ospf_rust_core::model::ConstraintRelation::LessEqual,
                0.0,
                "x_upper",
            )
            .expect("add native infeasible LP upper bound");
        model
            .add_linear_constraint(
                &[(index, -1.0)],
                ospf_rust_core::model::ConstraintRelation::LessEqual,
                -1.0,
                "x_lower",
            )
            .expect("add native infeasible LP lower bound");
        model
    }

    #[cfg(any(
        feature = "gurobi10",
        feature = "gurobi11",
        feature = "gurobi12",
        feature = "scip"
    ))]
    fn assert_native_infeasible_column_generation_report<S>(solver: &S)
    where
        S: LinearProgrammingSolver,
    {
        let model = native_infeasible_column_generation_model();
        let linear_model = model
            .try_to_linear_triad_model()
            .expect("flatten native infeasible LP");
        let fingerprint = linear_model_fingerprint(&linear_model).expect("LP fingerprint");
        for (stage, phase, iteration) in [
            ("phase-one", PricingPhase::PhaseOne, 1),
            ("phase-two", PricingPhase::PhaseTwo, 1),
            ("final-lp", PricingPhase::PhaseTwo, 2),
        ] {
            let request = LinearProgrammingRequest {
                node_id: 0,
                phase,
                iteration,
                model: &model,
                deadline: None,
                cancellation: None,
            };
            let result = solver
                .solve_lp(&request)
                .expect("native infeasible LP should return a report");
            assert_eq!(result.status, LinearProgrammingStatus::Infeasible);
            let report = result.report.as_ref().unwrap_or_else(|| {
                panic!("{stage} infeasible LP should retain its unified report")
            });
            ospf_rust_core::solver::require_infeasibility_certificate_for_model(
                report,
                &fingerprint,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{stage} native infeasible LP report should pass the certificate gate: {error}"
                )
            });
            assert!(result.solution.is_none());
            assert!(result.dual_solution.is_none());
        }
    }

    #[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
    #[test]
    fn gurobi_column_generation_keeps_verified_infeasible_phase_and_final_reports() {
        assert_native_infeasible_column_generation_report(
            &ospf_rust_core::solver::solvers::GurobiSolver::new(),
        );
    }

    #[cfg(feature = "scip")]
    #[test]
    fn scip_column_generation_keeps_verified_infeasible_phase_and_final_reports() {
        assert_native_infeasible_column_generation_report(
            &ospf_rust_core::solver::solvers::SCIPSolver::new(),
        );
    }

    #[test]
    fn flow_context_extension_registers_through_the_standard_lifecycle() {
        let unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let arc = crate::domain::flow::FlowArc::new(
            "ab",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(0.0, unit.clone()),
                Quantity::new(5.0, unit.clone()),
            )
            .expect("valid capacity"),
            NetworkCost::new(Quantity::new(2.0, cost_unit)),
        );
        let graph = FlowGraph::new(
            vec![FlowNode::new("a"), FlowNode::new("b")],
            vec![arc],
            vec![
                SupplyDemand::new("m1")
                    .with_balance("a", Quantity::new(5.0, unit.clone()))
                    .with_balance("b", Quantity::new(-5.0, unit)),
            ],
        )
        .expect("valid extension graph");
        let mut context = FlowContext::new(graph);
        context
            .add_extra_pipeline(Arc::new(FlowFixtureExtension))
            .expect("add flow extra pipeline");
        let mut model = MetaModel::<f64>::new("flow-extension");
        context
            .register(&mut model)
            .expect("register flow extension");
        context
            .refresh_shadow_price(&model, &[])
            .expect("refresh flow extension");
        context
            .extract_solution(&model)
            .expect("extract flow extension");
        assert!(
            model
                .as_basic()
                .constraints()
                .iter()
                .any(|constraint| constraint.name == "flow_extra_constraint")
        );
        assert!(
            model
                .objective()
                .sub_objectives
                .iter()
                .any(|objective| objective.name == "flow_extra_objective")
        );
        let mut other_model = MetaModel::<f64>::new("flow-extension-other");
        assert!(context.register(&mut other_model).is_err());
        assert!(
            context
                .add_extra_pipeline(Arc::new(FlowFixtureExtension))
                .is_err()
        );
    }

    #[test]
    fn flow_context_register_rolls_back_model_and_binding_on_extension_failure() {
        let unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let arc = crate::domain::flow::FlowArc::new(
            "ab",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(0.0, unit.clone()),
                Quantity::new(5.0, unit.clone()),
            )
            .expect("valid capacity"),
            NetworkCost::new(Quantity::new(2.0, cost_unit)),
        );
        let graph = FlowGraph::new(
            vec![FlowNode::new("a"), FlowNode::new("b")],
            vec![arc],
            vec![
                SupplyDemand::new("m1")
                    .with_balance("a", Quantity::new(5.0, unit.clone()))
                    .with_balance("b", Quantity::new(-5.0, unit)),
            ],
        )
        .expect("valid rollback graph");
        let mut context =
            FlowContext::with_extensions(graph, vec![Arc::new(FailingFlowFixtureExtension)]);
        let mut first_model = MetaModel::<f64>::new("flow-rollback-first");
        assert!(context.register(&mut first_model).is_err());
        assert!(first_model.tokens().is_empty());
        assert!(first_model.as_basic().constraints().is_empty());
        assert!(first_model.objective().sub_objectives.is_empty());

        let mut second_model = MetaModel::<f64>::new("flow-rollback-second");
        context
            .register_variables(&mut second_model)
            .expect("failed registration must not retain the first model binding");
        assert_eq!(second_model.tokens().len(), 1);
        assert!(second_model.as_basic().constraints().is_empty());

        assert!(context.register(&mut second_model).is_err());
        assert_eq!(second_model.tokens().len(), 1);
        assert!(second_model.as_basic().constraints().is_empty());
        assert!(second_model.objective().sub_objectives.is_empty());
    }

    #[test]
    fn flow_context_register_variables_rolls_back_partial_aggregation_registration() {
        let flow_unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let finite_arc = crate::domain::flow::FlowArc::new(
            "finite",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(0.0, flow_unit.clone()),
                Quantity::new(5.0, flow_unit.clone()),
            )
            .expect("valid finite capacity"),
            NetworkCost::new(Quantity::new(1.0, cost_unit.clone())),
        );
        let non_finite_arc = crate::domain::flow::FlowArc::new(
            "non-finite",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(0.0, flow_unit.clone()),
                Quantity::new(f64::INFINITY, flow_unit),
            )
            .expect("capacity bounds preserve the conversion failure input"),
            NetworkCost::new(Quantity::new(1.0, cost_unit)),
        );
        let graph = FlowGraph::new(
            vec![FlowNode::new("a"), FlowNode::new("b")],
            vec![finite_arc, non_finite_arc],
            vec![SupplyDemand::new("m1")],
        )
        .expect("valid partial-registration graph");
        let mut context = FlowContext::new(graph);
        let mut model = MetaModel::<f64>::new("flow-partial-registration-rollback");

        assert!(context.register_variables(&mut model).is_err());
        assert!(model.tokens().is_empty());
        assert!(context.aggregation.variable_indices().is_empty());

        let retry_unit = context.aggregation.graph.units.flow_unit.clone();
        context.aggregation.graph.arcs[1].capacity.upper = Quantity::new(5.0, retry_unit);
        let mut retry_model = MetaModel::<f64>::new("flow-partial-registration-retry");
        context
            .register_variables(&mut retry_model)
            .expect("failed registration must restore the model binding");
        assert_eq!(retry_model.tokens().len(), 2);
        assert_eq!(context.aggregation.variable_indices().len(), 2);
    }

    #[test]
    fn initial_route_is_valid_and_branch_mask_filters_it() {
        let instance = test_instance();
        let generator = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let routes = generator.generate(None).expect("initial route generation");
        assert_eq!(routes.len(), 1);
        RouteValidator::validate(
            &instance,
            &routes[0],
            &EuclideanDistanceCalculator,
            &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            &DistanceArcCostCalculator,
            &FixedPlusArcCostPolicy,
            None,
        )
        .expect("generated route is valid");
        let mask = BranchMask::new(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
            [(
                VehicleTypeId::from("v1"),
                [instance.customers[0].node.id.clone()]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
        .expect("valid branch mask");
        assert!(
            generator
                .generate(Some(&mask))
                .expect("masked generation")
                .is_empty()
        );
    }

    #[test]
    fn minute_scale_time_window_preserves_waiting_in_initial_route() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let scheduling_window = TimeWindow::seconds(
            TimeRange::new(start, start + TimeDuration::hours(1)),
            0.0,
            false,
            1.0,
        );
        let customer_window = ServiceTimeWindow::new(
            start + TimeDuration::minutes(2),
            start + TimeDuration::minutes(3),
        )
        .expect("valid customer window");
        let instance = test_instance_with_customer_window(
            scheduling_window,
            customer_window,
            VrptwUnits::default(),
        );
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route generation")
        .remove(0);

        assert_eq!(route.stops[1].arrival, start + TimeDuration::seconds(1));
        assert_eq!(
            route.stops[1].service_start,
            start + TimeDuration::minutes(2)
        );
        assert_eq!(route.stops[1].departure, start + TimeDuration::minutes(2));
        RouteValidator::validate(
            &instance,
            &route,
            &EuclideanDistanceCalculator,
            &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            &DistanceArcCostCalculator,
            &FixedPlusArcCostPolicy,
            None,
        )
        .expect("waiting route is valid");
    }

    #[test]
    fn espprc_pricing_reconstructs_waiting_route() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let scheduling_window = TimeWindow::seconds(
            TimeRange::new(start, start + TimeDuration::hours(1)),
            0.0,
            false,
            1.0,
        );
        let customer_window = ServiceTimeWindow::new(
            start + TimeDuration::minutes(2),
            start + TimeDuration::minutes(3),
        )
        .expect("valid customer window");
        let instance = test_instance_with_customer_window(
            scheduling_window,
            customer_window,
            VrptwUnits::default(),
        );
        let vehicle_type_id = VehicleTypeId::from("v1");
        let duals = PricingDuals::new(PricingPhase::PhaseTwo, [(CustomerId::from("c1"), 10.0)], []);
        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let pricer = EspprcPricer::new(builder);
        let request = PricingRequest::new(instance, duals, vehicle_type_id);
        let result = pricer.price_request(&request).expect("pricing succeeds");
        assert!(result.exact_pricing_complete);
        assert_eq!(result.routes.len(), 1);
        assert_eq!(
            result.routes[0].stops[1].arrival,
            start + TimeDuration::seconds(1)
        );
        assert_eq!(
            result.routes[0].stops[1].service_start,
            start + TimeDuration::minutes(2)
        );
    }

    #[test]
    fn espprc_matches_exhaustive_oracle_and_reports_minimum_reduced_cost() {
        let instance = multi_customer_instance(3);
        let vehicle_type_id = VehicleTypeId::from("v1");
        let duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [
                (CustomerId::from("c1"), 8.0),
                (CustomerId::from("c2"), 8.0),
                (CustomerId::from("c3"), 8.0),
            ],
            [(vehicle_type_id.clone(), 4.0)],
        );
        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let graph = builder
            .build(&vehicle_type_id, &duals, None)
            .expect("pricing graph");
        let oracle = exhaustive_pricing_paths(&graph, &instance, instance.customers.len());
        assert!(!oracle.is_empty(), "oracle should find feasible routes");
        let oracle_minimum = oracle
            .iter()
            .map(|path| path.reduced_cost)
            .fold(f64::INFINITY, f64::min);

        let mut request = PricingRequest::new(instance.clone(), duals, vehicle_type_id.clone());
        request.max_columns_per_pricing = usize::MAX;
        let result = EspprcPricer::new(builder)
            .price_request(&request)
            .expect("pricing succeeds");
        assert!(result.exact_pricing_complete);
        assert!((result.min_reduced_cost - oracle_minimum).abs() <= 1e-8);
        assert!(
            result
                .routes
                .iter()
                .all(|route| route.cost.value >= 0.0 && !route.customer_ids().is_empty())
        );

        let oracle_arc_paths = oracle
            .iter()
            .map(|path| path.arc_ids.clone())
            .collect::<BTreeSet<_>>();
        assert!(
            result
                .routes
                .iter()
                .all(|route| oracle_arc_paths.contains(&route.effective_arc_ids()))
        );
        let negative_count = oracle
            .iter()
            .filter(|path| path.reduced_cost < -request.pricing_tolerance)
            .count();
        assert!(result.routes.len() <= negative_count);

        let zero_duals = PricingDuals::new(PricingPhase::PhaseTwo, [], []);
        let positive_graph = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .build(&vehicle_type_id, &zero_duals, None)
        .expect("positive-cost pricing graph");
        let positive_oracle =
            exhaustive_pricing_paths(&positive_graph, &instance, instance.customers.len());
        let positive_minimum = positive_oracle
            .iter()
            .map(|path| path.reduced_cost)
            .fold(f64::INFINITY, f64::min);
        let positive_request = PricingRequest::new(instance, zero_duals, vehicle_type_id);
        let positive_result = EspprcPricer::new(RouteGraphBuilder::new(
            positive_request.instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&positive_request)
        .expect("positive pricing succeeds");
        assert!(positive_result.routes.is_empty());
        assert!((positive_result.min_reduced_cost - positive_minimum).abs() <= 1e-8);
    }

    #[test]
    fn espprc_enforces_resource_boundaries_and_reports_truncation() {
        let instance = multi_customer_instance(3);
        let vehicle_type_id = VehicleTypeId::from("v1");
        let duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [
                (CustomerId::from("c1"), 8.0),
                (CustomerId::from("c2"), 8.0),
                (CustomerId::from("c3"), 8.0),
            ],
            [(vehicle_type_id.clone(), 4.0)],
        );
        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let full_request =
            PricingRequest::new(instance.clone(), duals.clone(), vehicle_type_id.clone());
        let full_result = EspprcPricer::new(builder)
            .price_request(&full_request)
            .expect("full pricing");
        assert!(full_result.routes.len() >= 2);

        let mut columns_request = full_request.clone();
        columns_request.max_columns_per_pricing = 1;
        let columns_result = EspprcPricer::new(RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&columns_request)
        .expect("column-truncated pricing");
        assert_eq!(columns_result.routes.len(), 1);
        assert!(columns_result.truncated);
        assert_eq!(
            columns_result.truncation_reason,
            Some(crate::domain::route_generation::TruncationReason::MaxColumns)
        );
        assert!(columns_result.exact_pricing_complete);
        assert_eq!(
            columns_result.min_reduced_cost,
            full_result.min_reduced_cost
        );

        let mut labels_request = full_request.clone();
        labels_request.max_labels = Some(2);
        let labels_result = EspprcPricer::new(RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&labels_request)
        .expect("label-truncated pricing");
        assert!(labels_result.truncated);
        assert!(!labels_result.exact_pricing_complete);
        assert_eq!(
            labels_result.truncation_reason,
            Some(crate::domain::route_generation::TruncationReason::MaxLabels)
        );

        let mut depth_request = full_request.clone();
        depth_request.max_depth = Some(1);
        let depth_result = EspprcPricer::new(RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&depth_request)
        .expect("depth-truncated pricing");
        assert!(depth_result.truncated);
        assert!(!depth_result.exact_pricing_complete);
        assert_eq!(
            depth_result.truncation_reason,
            Some(crate::domain::route_generation::TruncationReason::MaxDepth)
        );
        assert!(depth_result.routes.iter().all(|route| {
            route
                .stops
                .iter()
                .filter(|stop| stop.customer_id.is_some())
                .count()
                <= 1
        }));

        let required_arc = ResourceArc::new(
            vehicle_type_id.clone(),
            default_arc_id(
                &instance.start_depot.node.id,
                &instance.customers[0].node.id,
            ),
            instance.start_depot.node.id.clone(),
            instance.customers[0].node.id.clone(),
        );
        let required_mask = BranchMask::new(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            [required_arc].into_iter().collect(),
        )
        .expect("required start arc mask");
        let mut required_request = full_request.clone();
        required_request.branch_mask = Some(required_mask.clone());
        let required_result = EspprcPricer::new(RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&required_request)
        .expect("required arc pricing");
        assert!(!required_result.routes.is_empty());
        assert!(required_result.routes.iter().all(|route| {
            required_mask.is_route_compatible(
                &route.vehicle_type_id,
                &route
                    .stops
                    .iter()
                    .map(|stop| stop.node_id.clone())
                    .collect::<Vec<_>>(),
                &route.effective_arc_ids(),
            ) && (!route
                .stops
                .iter()
                .any(|stop| stop.node_id == instance.customers[0].node.id)
                || route.uses_arc(&default_arc_id(
                    &instance.start_depot.node.id,
                    &instance.customers[0].node.id,
                )))
        }));

        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let mut cancelled_request = full_request.clone();
        cancelled_request.cancellation = Some(cancellation);
        let cancelled_result = EspprcPricer::new(RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&cancelled_request)
        .expect("cancelled pricing");
        assert!(cancelled_result.interrupted);
        assert!(!cancelled_result.exact_pricing_complete);
        assert!(cancelled_result.routes.is_empty());

        let mut deadline_request = full_request;
        deadline_request.deadline = Some(Instant::now() - std::time::Duration::from_millis(1));
        let deadline_result = EspprcPricer::new(RouteGraphBuilder::new(
            instance,
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&deadline_request)
        .expect("deadline pricing");
        assert!(deadline_result.interrupted);
        assert!(!deadline_result.exact_pricing_complete);

        let mut exact_due = (*test_instance()).clone();
        let due_start = exact_due.scheduling_window.window.start;
        exact_due.customers[0].time_window =
            ServiceTimeWindow::new(due_start, due_start + TimeDuration::seconds(1))
                .expect("closed due window");
        let exact_due = Arc::new(exact_due);
        let exact_due_id = VehicleTypeId::from("v1");
        let exact_due_duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [(CustomerId::from("c1"), 20.0)],
            [(exact_due_id.clone(), 0.0)],
        );
        let exact_due_builder = RouteGraphBuilder::new(
            exact_due.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let exact_due_result = EspprcPricer::new(exact_due_builder).price_request(
            &PricingRequest::new(exact_due, exact_due_duals, exact_due_id),
        );
        assert!(
            exact_due_result
                .expect("exact due pricing")
                .routes
                .iter()
                .any(|route| route.customer_ids().contains(&CustomerId::from("c1")))
        );

        let mut exact_capacity = (*test_instance()).clone();
        exact_capacity.vehicle_types[0].capacity = Quantity::new(1.0, Kilogram::INSTANT.clone());
        let exact_capacity = Arc::new(exact_capacity);
        let exact_capacity_id = VehicleTypeId::from("v1");
        let exact_capacity_duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [(CustomerId::from("c1"), 20.0)],
            [(exact_capacity_id.clone(), 0.0)],
        );
        let exact_capacity_builder = RouteGraphBuilder::new(
            exact_capacity.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let exact_capacity_result = EspprcPricer::new(exact_capacity_builder).price_request(
            &PricingRequest::new(exact_capacity, exact_capacity_duals, exact_capacity_id),
        );
        assert!(
            exact_capacity_result
                .expect("exact capacity pricing")
                .routes
                .iter()
                .any(|route| route.customer_ids().contains(&CustomerId::from("c1")))
        );

        let start = OffsetDateTime::UNIX_EPOCH;
        let short_window = TimeWindow::seconds(
            TimeRange::new(start, start + TimeDuration::hours(1)),
            0.0,
            false,
            1.0,
        );
        let unreachable = test_instance_with_customer_window(
            short_window,
            ServiceTimeWindow::new(start, start).expect("unreachable window"),
            VrptwUnits::default(),
        );
        let unreachable_id = VehicleTypeId::from("v1");
        let unreachable_duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [(CustomerId::from("c1"), 20.0)],
            [(unreachable_id.clone(), 0.0)],
        );
        let unreachable_builder = RouteGraphBuilder::new(
            unreachable.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let unreachable_result = EspprcPricer::new(unreachable_builder).price_request(
            &PricingRequest::new(unreachable, unreachable_duals, unreachable_id),
        );
        assert!(
            unreachable_result
                .expect("unreachable pricing")
                .routes
                .is_empty()
        );
    }

    #[test]
    fn espprc_rejects_stale_graph_snapshots_and_corrupted_reduced_cost() {
        let instance = test_instance();
        let vehicle_type_id = VehicleTypeId::from("v1");
        let duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [(CustomerId::from("c1"), 10.0)],
            [(vehicle_type_id.clone(), 1.0)],
        );
        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let graph = builder
            .build(&vehicle_type_id, &duals, None)
            .expect("pricing graph");
        let pricer = EspprcPricer::new(builder);
        let request = PricingRequest::new(instance.clone(), duals.clone(), vehicle_type_id.clone());

        let mut branch_request = request.clone();
        branch_request.branch_mask = Some(
            BranchMask::empty(
                instance.start_depot.node.id.clone(),
                instance.end_depot.node.id.clone(),
            )
            .expect("empty branch mask"),
        );
        assert!(pricer.price(&graph, &branch_request).is_err());

        let mut dual_request = request.clone();
        dual_request
            .duals
            .customer
            .insert(CustomerId::from("c1"), 11.0);
        assert!(pricer.price(&graph, &dual_request).is_err());

        let other_instance = test_instance();
        let other_request = PricingRequest::new(other_instance, duals, vehicle_type_id);
        assert!(pricer.price(&graph, &other_request).is_err());

        let mut corrupted_graph = graph.clone();
        corrupted_graph.arcs[0].reduced_cost += 1.0;
        assert!(pricer.price(&corrupted_graph, &request).is_err());

        let mut corrupted_outgoing = graph.clone();
        corrupted_outgoing.outgoing[graph.start_index()].clear();
        assert!(pricer.price(&corrupted_outgoing, &request).is_err());

        let mut corrupted_cost_snapshot = graph.clone();
        corrupted_cost_snapshot.arcs[0].route_cost += 1.0;
        corrupted_cost_snapshot.arcs[0].reduced_cost += 1.0;
        assert!(pricer.price(&corrupted_cost_snapshot, &request).is_err());
    }

    #[test]
    fn route_compilation_deduplicates_columns_and_refreshes_indices() {
        let instance = test_instance();
        let generator = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let route = generator.generate(None).expect("initial route").remove(0);
        let mut model = MetaModel::<f64>::new("compilation-test");
        let mut context = RouteCompilationContext::new(instance);
        context.register(&mut model).expect("register compilation");
        assert_eq!(
            context
                .add_columns(0, [route.clone(), route.clone()], &mut model)
                .expect("add columns")
                .len(),
            1
        );
        context
            .remove_columns([route], &mut model)
            .expect("remove column");
        assert!(context.aggregation.routes().is_empty());
        assert!(context.aggregation.route_variable_indices().is_empty());
    }

    #[test]
    fn route_compilation_canonicalizes_unique_explicit_arcs_and_rejects_ambiguous_implicit_arcs() {
        let base = test_instance();
        let cost_unit = base.units.cost_unit.clone();
        let travel = TravelTime::new(TimeDuration::seconds(1)).expect("valid travel time");
        let arcs = vec![
            VrptwArc::new(
                "start-c1",
                base.start_depot.node.id.clone(),
                base.customers[0].node.id.clone(),
                Quantity::new(1.0, Meter::INSTANT.clone()),
                travel,
                Quantity::new(1.0, cost_unit.clone()),
            )
            .expect("valid start arc"),
            VrptwArc::new(
                "c1-end",
                base.customers[0].node.id.clone(),
                base.end_depot.node.id.clone(),
                Quantity::new(1.0, Meter::INSTANT.clone()),
                travel,
                Quantity::new(1.0, cost_unit),
            )
            .expect("valid end arc"),
        ];
        let instance = Arc::new(
            VrptwInstance::new_with_arcs(
                base.name.clone(),
                base.start_depot.clone(),
                base.end_depot.clone(),
                base.customers.clone(),
                base.vehicle_types.clone(),
                base.scheduling_window.clone(),
                base.units.clone(),
                base.tolerances,
                arcs.clone(),
            )
            .expect("explicit-arc instance"),
        );
        let explicit = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("explicit initial route")
        .remove(0);
        assert_eq!(explicit.arc_ids.len(), 2);
        let implicit = Route::new(
            explicit.vehicle_type_id.clone(),
            explicit.stops.clone(),
            explicit.distance.clone(),
            explicit.cost.clone(),
        )
        .expect("implicit route");
        let mut model = MetaModel::<f64>::new("explicit-arc-canonicalization");
        let mut context = RouteCompilationContext::new(instance);
        context
            .register(&mut model)
            .expect("register explicit master");
        let added = context
            .add_columns(0, [explicit.clone(), implicit], &mut model)
            .expect("canonicalize route columns");
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].effective_arc_ids(), explicit.effective_arc_ids());

        let parallel_instance = Arc::new(
            VrptwInstance::new_with_arcs(
                base.name.clone(),
                base.start_depot.clone(),
                base.end_depot.clone(),
                base.customers.clone(),
                base.vehicle_types.clone(),
                base.scheduling_window.clone(),
                base.units.clone(),
                base.tolerances,
                vec![
                    arcs[0].clone(),
                    VrptwArc::new(
                        "start-c1-parallel",
                        base.start_depot.node.id.clone(),
                        base.customers[0].node.id.clone(),
                        Quantity::new(2.0, Meter::INSTANT.clone()),
                        travel,
                        Quantity::new(2.0, base.units.cost_unit.clone()),
                    )
                    .expect("valid parallel start arc"),
                    arcs[1].clone(),
                ],
            )
            .expect("parallel explicit-arc instance"),
        );
        let mut parallel_model = MetaModel::<f64>::new("parallel-arc-canonicalization");
        let mut parallel_context = RouteCompilationContext::new(parallel_instance);
        parallel_context
            .register(&mut parallel_model)
            .expect("register parallel master");
        let implicit = Route::new(
            explicit.vehicle_type_id,
            explicit.stops,
            explicit.distance,
            explicit.cost,
        )
        .expect("parallel implicit route");
        assert!(
            parallel_context
                .add_columns(0, [implicit], &mut parallel_model)
                .is_err()
        );
    }

    #[test]
    fn implicit_graph_rejects_fake_arc_ids_before_mutating_the_model() {
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);
        let fake = Route::with_arc_ids(
            route.vehicle_type_id.clone(),
            route.stops.clone(),
            vec![
                NetworkArcId::from("fake-arc"),
                NetworkArcId::from("fake-arc-2"),
            ],
            route.distance.clone(),
            route.cost.clone(),
        )
        .expect("fake route shape");
        let mut model = MetaModel::<f64>::new("implicit-fake-arc");
        let mut context = RouteCompilationContext::new(instance);
        context
            .register(&mut model)
            .expect("register implicit master");
        let token_count = model.tokens().len();
        assert!(context.add_columns(0, [fake], &mut model).is_err());
        assert_eq!(model.tokens().len(), token_count);
        assert!(context.aggregation.routes().is_empty());
        assert!(context.aggregation.route_variable_indices().is_empty());
    }

    #[test]
    fn route_compilation_matches_two_customer_master_oracle_and_phase_contracts() {
        let instance = multi_customer_instance(2);
        let generator = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let singles = generator.generate(None).expect("single-customer routes");
        assert_eq!(singles.len(), 2);
        let combined = combined_two_customer_route(&instance);
        RouteValidator::validate(
            &instance,
            &combined,
            &EuclideanDistanceCalculator,
            &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            &DistanceArcCostCalculator,
            &FixedPlusArcCostPolicy,
            None,
        )
        .expect("combined route is valid");

        let mut model = MetaModel::<f64>::new("master-oracle");
        let mut context = RouteCompilationContext::new(instance.clone());
        context.register(&mut model).expect("register master");
        assert_eq!(context.aggregation.artificial_coverage.len(), 2);
        assert!(
            context
                .aggregation
                .artificial_coverage
                .variable_indices
                .iter()
                .all(|index| model.variable_range_by_index(*index).is_some())
        );

        let constraints = model.as_basic().constraints();
        let coverage = constraints
            .iter()
            .filter(|constraint| constraint.name.starts_with("customer_coverage_"))
            .collect::<Vec<_>>();
        let fleet = constraints
            .iter()
            .filter(|constraint| constraint.name.starts_with("fleet_size_"))
            .collect::<Vec<_>>();
        assert_eq!(coverage.len(), 2);
        assert!(coverage.iter().all(|constraint| {
            constraint.inequality.relation == ospf_rust_core::model::ConstraintRelation::Equal
                && constraint.inequality.rhs == 1.0
        }));
        assert_eq!(fleet.len(), 1);
        assert_eq!(
            fleet[0].inequality.relation,
            ospf_rust_core::model::ConstraintRelation::LessEqual
        );
        assert_eq!(fleet[0].inequality.rhs, 2.0);
        assert_eq!(
            model.objective().sub_objectives[0].name,
            "phase_one_artificial_coverage"
        );
        assert_eq!(
            model.objective().sub_objectives[0]
                .polynomial
                .monomials()
                .len(),
            context.aggregation.artificial_coverage.len()
        );

        context
            .add_columns(
                0,
                [singles[0].clone(), singles[1].clone(), combined.clone()],
                &mut model,
            )
            .expect("add all master routes");
        assert_eq!(context.aggregation.routes().len(), 3);
        assert_eq!(context.aggregation.route_variable_indices().len(), 3);
        assert_eq!(
            model.objective().sub_objectives[0].name,
            "phase_one_artificial_coverage"
        );
        context
            .switch_to_phase_two(&mut model)
            .expect("switch to phase two");
        assert_eq!(
            model.objective().sub_objectives[0].name,
            "route_cost_minimization"
        );
        assert_eq!(
            model.objective().sub_objectives[0]
                .polynomial
                .monomials()
                .len(),
            3
        );
        assert!(
            context
                .aggregation
                .artificial_coverage
                .variable_indices
                .iter()
                .all(|index| model
                    .variable_range_by_index(*index)
                    .is_some_and(
                        |range| range.lower_bound == Some(0.0) && range.upper_bound == Some(0.0)
                    ))
        );

        let routes = [singles[0].clone(), singles[1].clone(), combined.clone()];
        let mut oracle_best = f64::INFINITY;
        let mut oracle_selected = None;
        for mask in 0usize..(1usize << routes.len()) {
            let selected = routes
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1usize << index) != 0)
                .map(|(_, route)| route)
                .collect::<Vec<_>>();
            if selected.len() > 2 {
                continue;
            }
            let mut covered = BTreeSet::new();
            for route in &selected {
                for customer in route.customer_ids() {
                    if !covered.insert(customer) {
                        covered.clear();
                        break;
                    }
                }
            }
            if covered.len() != instance.customers.len() {
                continue;
            }
            let cost = selected.iter().map(|route| route.cost.value).sum::<f64>();
            if cost < oracle_best {
                oracle_best = cost;
                oracle_selected = Some(
                    selected
                        .iter()
                        .map(|route| route.signature())
                        .collect::<BTreeSet<_>>(),
                );
            }
        }
        assert_eq!(oracle_best, 5.0);
        assert_eq!(
            oracle_selected,
            Some([combined.signature()].into_iter().collect())
        );

        let mut solution = vec![0.0; model.tokens_in_solver_order().len()];
        let combined_index = context
            .aggregation
            .route_variable_indices()
            .get(&combined.signature())
            .expect("combined route index")
            .model_index;
        let combined_token = &model.tokens()[combined_index];
        solution[combined_token.solver_index] = 1.0;
        model.set_solution(&solution);
        let extracted = context
            .extract_solution(&model, 1e-7)
            .expect("extract master solution");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].signature(), combined.signature());
        let finalized = context
            .finalize(&model, 1e-7)
            .expect("finalize master solution");
        assert_eq!(finalized.total_cost.value, 5.0);
        assert_eq!(finalized.total_distance.value, 4.0);
    }

    #[test]
    fn flow_normalizes_units_rejects_duplicate_commodities_and_binds_context() {
        let flow_unit = Meter::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let arc = crate::domain::flow::FlowArc::new(
            "ab",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(1.0, Kilometer::INSTANT.clone()),
                Quantity::new(2.0, Kilometer::INSTANT.clone()),
            )
            .expect("valid kilometer capacity"),
            NetworkCost::new(Quantity::new(3.0, cost_unit.clone())),
        );
        let commodity = SupplyDemand::new("m1")
            .with_balance("a", Quantity::new(1000.0, flow_unit.clone()))
            .with_balance("b", Quantity::new(-1000.0, flow_unit.clone()));
        let graph = FlowGraph::new_with_units(
            vec![FlowNode::new("a"), FlowNode::new("b")],
            vec![arc],
            vec![commodity.clone()],
            FlowUnits::new(flow_unit.clone(), cost_unit.clone()).expect("valid flow units"),
        )
        .expect("flow units normalize");
        assert_eq!(graph.arcs[0].capacity.lower.value, 1000.0);
        assert_eq!(graph.arcs[0].capacity.upper.value, 2000.0);
        assert_eq!(
            graph.commodities[0].balances[&NetworkNodeId::from("a")].value,
            1000.0
        );

        let duplicate = FlowGraph::new_with_units(
            vec![FlowNode::new("a"), FlowNode::new("b")],
            graph.arcs.clone(),
            vec![commodity.clone(), commodity],
            FlowUnits::new(flow_unit, cost_unit).expect("valid flow units"),
        );
        assert!(
            duplicate.is_err(),
            "duplicate commodity IDs must be rejected"
        );

        let mut context = FlowContext::new(graph);
        let mut first_model = MetaModel::<f64>::new("flow-model-1");
        context
            .register(&mut first_model)
            .expect("register first model");
        let mut second_model = MetaModel::<f64>::new("flow-model-2");
        assert!(context.register(&mut second_model).is_err());
    }

    #[test]
    fn flow_rejects_mismatched_flow_dimension_and_self_loop_has_zero_balance_coefficient() {
        let cost_unit = VrptwUnits::default().cost_unit;
        let mismatched_arc = crate::domain::flow::FlowArc::new(
            "ab",
            "a",
            "b",
            CapacityBounds::try_new(
                Quantity::new(0.0, Kilogram::INSTANT.clone()),
                Quantity::new(1.0, Kilogram::INSTANT.clone()),
            )
            .expect("valid mass capacity"),
            NetworkCost::new(Quantity::new(1.0, cost_unit.clone())),
        );
        assert!(
            FlowGraph::new_with_units(
                vec![FlowNode::new("a"), FlowNode::new("b")],
                vec![mismatched_arc],
                vec![SupplyDemand::new("m1")],
                FlowUnits::new(Meter::INSTANT.clone(), cost_unit.clone())
                    .expect("valid flow units"),
            )
            .is_err()
        );

        let self_loop = crate::domain::flow::FlowArc::new(
            "aa",
            "a",
            "a",
            CapacityBounds::try_new(
                Quantity::new(0.0, Kilogram::INSTANT.clone()),
                Quantity::new(5.0, Kilogram::INSTANT.clone()),
            )
            .expect("valid self-loop capacity"),
            NetworkCost::new(Quantity::new(1.0, cost_unit.clone())),
        );
        let graph = FlowGraph::new_with_units(
            vec![FlowNode::new("a")],
            vec![self_loop],
            vec![SupplyDemand::new("m1")],
            FlowUnits::new(Kilogram::INSTANT.clone(), cost_unit).expect("valid flow units"),
        )
        .expect("self-loop flow graph");
        let mut context = FlowContext::new(graph);
        let mut model = MetaModel::<f64>::new("self-loop-flow");
        context
            .register(&mut model)
            .expect("register self-loop flow");
        let conservation = model
            .as_basic()
            .constraints()
            .iter()
            .find(|constraint| constraint.name.starts_with("flow_conservation_"))
            .expect("conservation row");
        assert!(conservation.inequality.polynomial.monomials().is_empty());
        assert_eq!(*conservation.inequality.polynomial.constant_term(), 0.0);
    }

    #[test]
    fn require_arc_only_restricts_routes_touching_customer_endpoints() {
        let vehicle = VehicleTypeId::from("v1");
        let start = NetworkNodeId::from("start");
        let end = NetworkNodeId::from("end");
        let required = ResourceArc::new(
            vehicle.clone(),
            NetworkArcId::from("start-a"),
            start.clone(),
            NetworkNodeId::from("a"),
        );
        let mask = BranchMask::new(
            start.clone(),
            end.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            [required].into_iter().collect(),
        )
        .expect("valid required-arc mask");
        assert!(mask.is_route_compatible(
            &vehicle,
            &[start.clone(), NetworkNodeId::from("b"), end.clone()],
            &[NetworkArcId::from("start-b"), NetworkArcId::from("b-end")],
        ));
        assert!(mask.is_route_compatible(
            &vehicle,
            &[
                start.clone(),
                NetworkNodeId::from("a"),
                NetworkNodeId::from("b"),
                end.clone()
            ],
            &[
                NetworkArcId::from("start-a"),
                NetworkArcId::from("a-b"),
                NetworkArcId::from("b-end")
            ],
        ));
        assert!(!mask.is_route_compatible(
            &vehicle,
            &[
                start.clone(),
                NetworkNodeId::from("b"),
                NetworkNodeId::from("a"),
                end.clone()
            ],
            &[
                NetworkArcId::from("start-b"),
                NetworkArcId::from("b-a"),
                NetworkArcId::from("a-end")
            ],
        ));
        assert!(mask.is_route_compatible(
            &vehicle,
            &[start.clone(), NetworkNodeId::from("a"), end],
            &[NetworkArcId::from("start-a"), NetworkArcId::from("a-end")],
        ));

        let required_end = ResourceArc::new(
            vehicle.clone(),
            NetworkArcId::from("a-end"),
            NetworkNodeId::from("a"),
            NetworkNodeId::from("end"),
        );
        let end_mask = BranchMask::new(
            start.clone(),
            NetworkNodeId::from("end"),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            [required_end].into_iter().collect(),
        )
        .expect("valid required end-arc mask");
        assert!(!end_mask.is_route_compatible(
            &vehicle,
            &[
                start.clone(),
                NetworkNodeId::from("a"),
                NetworkNodeId::from("b"),
                NetworkNodeId::from("end")
            ],
            &[
                NetworkArcId::from("start-a"),
                NetworkArcId::from("a-b"),
                NetworkArcId::from("b-end"),
            ],
        ));
        assert!(end_mask.is_route_compatible(
            &vehicle,
            &[start, NetworkNodeId::from("a"), NetworkNodeId::from("end")],
            &[NetworkArcId::from("start-a"), NetworkArcId::from("a-end")],
        ));
    }

    fn bare_route(vehicle_type_id: &str, nodes: &[&str], arc_ids: &[&str]) -> Route<f64> {
        let epoch = OffsetDateTime::UNIX_EPOCH;
        let stops = nodes
            .iter()
            .map(|node_id| RouteStop {
                node_id: NetworkNodeId::from(*node_id),
                customer_id: None,
                arrival: epoch,
                service_start: epoch,
                departure: epoch,
                accumulated_load: Quantity::new(0.0, Kilogram::INSTANT.clone()),
            })
            .collect();
        Route::with_arc_ids(
            vehicle_type_id,
            stops,
            arc_ids.iter().map(|id| NetworkArcId::from(*id)).collect(),
            Quantity::new(0.0, Meter::INSTANT.clone()),
            Quantity::new(0.0, VrptwUnits::default().cost_unit),
        )
        .expect("valid signature fixture route")
    }

    #[test]
    fn route_signatures_are_unambiguous_and_default_arcs_are_canonical() {
        let first = bare_route("v", &["start", "end"], &["a|b"]);
        let second = bare_route("v", &["start", "mid", "end"], &["a", "b"]);
        assert_ne!(first.signature(), second.signature());

        let implicit = bare_route("v", &["start", "end"], &[]);
        let default_id = default_arc_id(&NetworkNodeId::from("start"), &NetworkNodeId::from("end"));
        let explicit = bare_route("v", &["start", "end"], &[default_id.as_str()]);
        assert_eq!(implicit.signature(), explicit.signature());
        assert!(implicit.uses_arc(&default_id));
        assert_ne!(
            default_arc_id(&NetworkNodeId::from("a->b"), &NetworkNodeId::from("c")),
            default_arc_id(&NetworkNodeId::from("a"), &NetworkNodeId::from("b->c")),
        );

        let graph = NetworkGraph::new(
            vec![NetworkNode::new("start", ()), NetworkNode::new("end", ())],
            vec![NetworkArc::new("a|b", "start", "end", ())],
        )
        .expect("signature graph");
        assert!(
            graph
                .arc_signature([&NetworkArcId::from("a|b")])
                .contains("3:a|b")
        );
    }

    #[test]
    fn pricing_graph_preserves_parallel_arc_identity_and_feasibility() {
        let base = test_instance();
        let cost_unit = base.units.cost_unit.clone();
        let travel = TravelTime::new(TimeDuration::seconds(1)).expect("valid travel time");
        let invalid = VrptwArc::new(
            "start-c1-invalid",
            base.start_depot.node.id.clone(),
            base.customers[0].node.id.clone(),
            Quantity::new(1.0, Meter::INSTANT.clone()),
            travel,
            Quantity::new(8.0, cost_unit.clone()),
        )
        .expect("valid arc")
        .with_feasibility(false);
        let arcs = vec![
            VrptwArc::new(
                "start-c1-a",
                base.start_depot.node.id.clone(),
                base.customers[0].node.id.clone(),
                Quantity::new(1.0, Meter::INSTANT.clone()),
                travel,
                Quantity::new(2.0, cost_unit.clone()),
            )
            .expect("valid parallel arc"),
            VrptwArc::new(
                "start-c1-b",
                base.start_depot.node.id.clone(),
                base.customers[0].node.id.clone(),
                Quantity::new(3.0, Meter::INSTANT.clone()),
                TravelTime::new(TimeDuration::seconds(3)).expect("valid travel time"),
                Quantity::new(5.0, cost_unit.clone()),
            )
            .expect("valid parallel arc"),
            invalid,
            VrptwArc::new(
                "c1-end",
                base.customers[0].node.id.clone(),
                base.end_depot.node.id.clone(),
                Quantity::new(1.0, Meter::INSTANT.clone()),
                travel,
                Quantity::new(2.0, cost_unit),
            )
            .expect("valid return arc"),
        ];
        let instance = Arc::new(
            VrptwInstance::new_with_arcs(
                base.name.clone(),
                base.start_depot.clone(),
                base.end_depot.clone(),
                base.customers.clone(),
                base.vehicle_types.clone(),
                base.scheduling_window.clone(),
                base.units.clone(),
                base.tolerances,
                arcs,
            )
            .expect("parallel-arc instance"),
        );
        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let graph = builder
            .build(
                &VehicleTypeId::from("v1"),
                &PricingDuals::new(PricingPhase::PhaseTwo, [], []),
                None,
            )
            .expect("parallel pricing graph");
        let parallel = graph
            .arcs
            .iter()
            .filter(|arc| arc.from == graph.start_index() && arc.to != graph.end_index())
            .collect::<Vec<_>>();
        assert_eq!(parallel.len(), 2);
        assert!(
            parallel
                .iter()
                .any(|arc| arc.arc_id.as_str() == "start-c1-a" && arc.arc_cost == 2.0)
        );
        assert!(
            parallel
                .iter()
                .any(|arc| arc.arc_id.as_str() == "start-c1-b"
                    && arc.travel_time == TimeDuration::seconds(3))
        );
        assert!(
            !graph
                .arcs
                .iter()
                .any(|arc| arc.arc_id.as_str() == "start-c1-invalid")
        );
    }

    #[test]
    fn negative_travel_time_and_arc_distance_are_rejected() {
        assert!(TravelTime::new(-TimeDuration::seconds(1)).is_err());
        assert!(
            VrptwArc::new(
                "negative-distance",
                "start",
                "end",
                Quantity::new(-1.0, Meter::INSTANT.clone()),
                TravelTime::new(TimeDuration::ZERO).expect("zero travel time"),
                Quantity::new(0.0, VrptwUnits::default().cost_unit),
            )
            .is_err()
        );
    }

    #[derive(Debug, Clone, Copy)]
    struct NegativeTravelTimeCalculator;

    impl TravelTimeCalculator<f64> for NegativeTravelTimeCalculator {
        fn travel_time(
            &self,
            _from: &crate::domain::vrp::VrpNode<f64>,
            _to: &crate::domain::vrp::VrpNode<f64>,
            _vehicle_type: &VehicleType<f64>,
            _instance: &VrptwInstance<f64>,
        ) -> crate::Result<TimeDuration> {
            Ok(-TimeDuration::seconds(1))
        }
    }

    #[test]
    fn route_generation_rejects_negative_policy_travel_time() {
        let instance = test_instance();
        let generator = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            NegativeTravelTimeCalculator,
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        assert!(generator.generate(None).is_err());

        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            NegativeTravelTimeCalculator,
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        assert!(
            builder
                .build(
                    &VehicleTypeId::from("v1"),
                    &PricingDuals::new(PricingPhase::PhaseTwo, [], []),
                    None,
                )
                .is_err()
        );
    }

    #[test]
    fn vrptw_rejects_mismatched_end_depot_coordinate_axes() {
        let instance = test_instance();
        let mismatched_end = Depot {
            node: crate::infrastructure::NetworkNode::new(
                "end",
                Coordinate::new([
                    ("x", Quantity::new(0.0, Meter::INSTANT.clone())),
                    ("z", Quantity::new(0.0, Meter::INSTANT.clone())),
                ])
                .expect("valid mismatched coordinate axes"),
            ),
            time_window: instance.end_depot.time_window,
        };
        assert!(
            VrptwInstance::new(
                instance.name.clone(),
                instance.start_depot.clone(),
                mismatched_end,
                instance.customers.clone(),
                instance.vehicle_types.clone(),
                instance.scheduling_window.clone(),
                instance.units.clone(),
                instance.tolerances,
            )
            .is_err()
        );
    }

    fn verified_optimal_lp_report(
        request: &LinearProgrammingRequest<'_>,
        objective: f64,
        solution: Vec<f64>,
        dual_solution: Vec<f64>,
    ) -> SolveReport<f64> {
        let linear_model = request
            .model
            .try_to_linear_triad_model()
            .expect("test LP model should flatten");
        let model_fingerprint = linear_model_fingerprint(&linear_model)
            .expect("test LP model should have a fingerprint");
        let expanded_dual_solution = request
            .model
            .as_basic()
            .constraints()
            .iter()
            .zip(dual_solution)
            .flat_map(|(constraint, dual)| match constraint.inequality.relation {
                ospf_rust_core::model::ConstraintRelation::LessEqual => vec![dual],
                ospf_rust_core::model::ConstraintRelation::GreaterEqual => vec![-dual],
                ospf_rust_core::model::ConstraintRelation::Equal => vec![dual, 0.0],
            })
            .collect();
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: None,
                values: solution,
                stable_values: BTreeMap::new(),
                objective: Some(objective),
                objective_value: Some(objective),
                dual_solution: Some(expanded_dual_solution),
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(model_fingerprint),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("test LP report should satisfy the unified certificate contract")
    }

    fn verified_infeasible_lp_report(request: &LinearProgrammingRequest<'_>) -> SolveReport<f64> {
        let linear_model = request
            .model
            .try_to_linear_triad_model()
            .expect("test infeasible LP should flatten");
        let model_fingerprint = linear_model_fingerprint(&linear_model)
            .expect("test infeasible LP should have a fingerprint");
        SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .proof(SolveProof::infeasibility())
            .fingerprints(SolveFingerprints {
                model: Some(model_fingerprint),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("test infeasible report should satisfy the unified certificate contract")
    }

    struct FakeLpSolver;

    impl LinearProgrammingSolver for FakeLpSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            let mut solution = vec![0.0; request.model.tokens_in_solver_order().len()];
            if let Some(value) = solution.last_mut() {
                *value = 1.0;
            }
            let objective = if request.phase == crate::domain::vrp::PricingPhase::PhaseOne {
                0.0
            } else {
                3.0
            };
            let dual_solution: Vec<f64> = request
                .model
                .as_basic()
                .constraints()
                .iter()
                .map(|constraint| {
                    if request.phase == crate::domain::vrp::PricingPhase::PhaseTwo
                        && constraint.name.starts_with("customer_coverage_")
                    {
                        3.0
                    } else {
                        0.0
                    }
                })
                .collect();
            let report = verified_optimal_lp_report(
                request,
                objective,
                solution.clone(),
                dual_solution.clone(),
            );
            Ok(LinearProgrammingResult {
                status: LinearProgrammingStatus::Optimal,
                objective: Some(objective),
                solution: Some(solution),
                dual_solution: Some(dual_solution),
                report: Some(report),
            })
        }
    }

    struct UncertifiedOptimalLpSolver;

    impl LinearProgrammingSolver for UncertifiedOptimalLpSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            let mut result =
                <FakeLpSolver as LinearProgrammingSolver>::solve_lp(&FakeLpSolver, request)?;
            result.report = None;
            Ok(result)
        }
    }

    struct PhaseOneInfeasibleLpSolver;

    impl LinearProgrammingSolver for PhaseOneInfeasibleLpSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            Ok(LinearProgrammingResult {
                status: LinearProgrammingStatus::Infeasible,
                objective: None,
                solution: None,
                dual_solution: None,
                report: Some(verified_infeasible_lp_report(request)),
            })
        }
    }

    struct PhaseTwoInfeasibleLpSolver;

    impl LinearProgrammingSolver for PhaseTwoInfeasibleLpSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            if request.phase == PricingPhase::PhaseOne {
                return <FakeLpSolver as LinearProgrammingSolver>::solve_lp(&FakeLpSolver, request);
            }
            Ok(LinearProgrammingResult {
                status: LinearProgrammingStatus::Infeasible,
                objective: None,
                solution: None,
                dual_solution: None,
                report: Some(verified_infeasible_lp_report(request)),
            })
        }
    }

    struct FinalLpInfeasibleSolver;

    impl LinearProgrammingSolver for FinalLpInfeasibleSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            if request.phase == PricingPhase::PhaseOne {
                return <FakeLpSolver as LinearProgrammingSolver>::solve_lp(&FakeLpSolver, request);
            }
            if request.iteration == 1 {
                return <NonConvergingLpSolver as LinearProgrammingSolver>::solve_lp(
                    &NonConvergingLpSolver,
                    request,
                );
            }
            Ok(LinearProgrammingResult {
                status: LinearProgrammingStatus::Infeasible,
                objective: None,
                solution: None,
                dual_solution: None,
                report: Some(verified_infeasible_lp_report(request)),
            })
        }
    }

    struct OptimalThenTimeLimitSolver;

    impl LinearProgrammingSolver for OptimalThenTimeLimitSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            let mut solution = vec![0.0; request.model.tokens_in_solver_order().len()];
            if let Some(value) = solution.last_mut() {
                *value = 1.0;
            }
            if request.phase == PricingPhase::PhaseOne {
                let dual_solution = vec![0.0; request.model.as_basic().constraints().len()];
                let report = verified_optimal_lp_report(
                    request,
                    0.0,
                    solution.clone(),
                    dual_solution.clone(),
                );
                return Ok(LinearProgrammingResult {
                    status: LinearProgrammingStatus::Optimal,
                    objective: Some(0.0),
                    solution: Some(solution),
                    dual_solution: Some(dual_solution),
                    report: Some(report),
                });
            }
            Ok(LinearProgrammingResult {
                status: LinearProgrammingStatus::TimeLimit,
                objective: Some(3.0),
                solution: Some(solution),
                dual_solution: None,
                report: None,
            })
        }
    }

    struct FailingLpSolver;

    impl LinearProgrammingSolver for FailingLpSolver {
        fn solve_lp(
            &self,
            _request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            Err(crate::NetworkSchedulingError::solver(
                "injected LP failure / 注入的 LP 失败",
            ))
        }
    }

    struct NonConvergingLpSolver;

    impl LinearProgrammingSolver for NonConvergingLpSolver {
        fn solve_lp(
            &self,
            request: &LinearProgrammingRequest<'_>,
        ) -> crate::Result<LinearProgrammingResult> {
            let solution = vec![0.0; request.model.tokens_in_solver_order().len()];
            let dual_solution: Vec<f64> = request
                .model
                .as_basic()
                .constraints()
                .iter()
                .map(|constraint| {
                    if request.phase == PricingPhase::PhaseTwo
                        && constraint.name.starts_with("customer_coverage_")
                    {
                        100.0
                    } else {
                        0.0
                    }
                })
                .collect();
            let objective = if request.phase == PricingPhase::PhaseOne {
                0.0
            } else {
                3.0
            };
            let report = verified_optimal_lp_report(
                request,
                objective,
                solution.clone(),
                dual_solution.clone(),
            );
            Ok(LinearProgrammingResult {
                status: LinearProgrammingStatus::Optimal,
                objective: Some(objective),
                solution: Some(solution),
                dual_solution: Some(dual_solution),
                report: Some(report),
            })
        }
    }

    #[derive(Clone)]
    struct FixedNodeProvider {
        result: BranchNodeSolveResult<f64>,
    }

    impl BranchNodeSolverProvider<f64> for FixedNodeProvider {
        fn solve_node(
            &self,
            _node: &BranchNode,
            _inherited_columns: &[Route<f64>],
            _context: &BranchNodeSolveContext,
        ) -> crate::Result<BranchNodeSolveResult<f64>> {
            Ok(self.result.clone())
        }
    }

    #[derive(Clone)]
    struct RecordingExtension {
        events: Arc<std::sync::Mutex<Vec<&'static str>>>,
    }

    impl RouteCompilationExtension<f64> for RecordingExtension {
        fn register(
            &self,
            _model: &mut MetaModel<f64>,
            _compilation: &crate::domain::route_compilation::RouteCompilationAggregation<f64>,
        ) -> crate::Result<()> {
            self.events
                .lock()
                .expect("extension events")
                .push("register");
            Ok(())
        }

        fn add_columns(
            &self,
            _routes: &[Route<f64>],
            _model: &mut MetaModel<f64>,
        ) -> crate::Result<()> {
            self.events.lock().expect("extension events").push("add");
            Ok(())
        }

        fn remove_columns(
            &self,
            _routes: &[Route<f64>],
            _model: &mut MetaModel<f64>,
        ) -> crate::Result<()> {
            self.events.lock().expect("extension events").push("remove");
            Ok(())
        }

        fn refresh_shadow_price(
            &self,
            _model: &MetaModel<f64>,
            _dual_solution: &[f64],
            _prices: &mut crate::domain::vrp::VrpShadowPriceMap,
        ) -> crate::Result<()> {
            self.events.lock().expect("extension events").push("dual");
            Ok(())
        }

        fn extract_solution(&self, _routes: &mut Vec<Route<f64>>) -> crate::Result<()> {
            self.events
                .lock()
                .expect("extension events")
                .push("extract");
            Ok(())
        }
    }

    struct FailingSolutionExtension;

    impl RouteCompilationExtension<f64> for FailingSolutionExtension {
        fn extract_solution(&self, _routes: &mut Vec<Route<f64>>) -> crate::Result<()> {
            Err(crate::NetworkSchedulingError::contract(
                "injected solution enrichment failure / 注入的解 enrich 失败",
            ))
        }
    }

    struct FailingModelMutationExtension;

    impl RouteCompilationExtension<f64> for FailingModelMutationExtension {
        fn register(
            &self,
            model: &mut MetaModel<f64>,
            _compilation: &crate::domain::route_compilation::RouteCompilationAggregation<f64>,
        ) -> crate::Result<()> {
            model
                .register_auto_variable::<ospf_rust_core::variable::Continuous>(
                    "failing_extension_register_variable",
                )
                .map_err(|error| crate::NetworkSchedulingError::model(error.to_string()))?;
            Err(crate::NetworkSchedulingError::contract(
                "injected extension registration failure / 注入的扩展注册失败",
            ))
        }

        fn add_columns(
            &self,
            _routes: &[Route<f64>],
            model: &mut MetaModel<f64>,
        ) -> crate::Result<()> {
            model
                .register_auto_variable::<ospf_rust_core::variable::Continuous>(
                    "failing_extension_column_variable",
                )
                .map_err(|error| crate::NetworkSchedulingError::model(error.to_string()))?;
            Err(crate::NetworkSchedulingError::contract(
                "injected extension column failure / 注入的扩展加列失败",
            ))
        }
    }

    #[test]
    fn route_compilation_rolls_back_model_and_indices_when_extension_fails() {
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);

        let mut register_context = RouteCompilationContext::with_extensions(
            instance.clone(),
            vec![Arc::new(FailingModelMutationExtension)],
        );
        let mut register_model = MetaModel::<f64>::new("failing-register-model");
        assert!(register_context.register(&mut register_model).is_err());
        assert_eq!(register_model.tokens().len(), 0);
        assert_eq!(register_model.as_basic().constraints().len(), 0);
        assert!(register_context.aggregation.artificial_coverage.is_empty());

        let mut add_context = RouteCompilationContext::new(instance);
        add_context
            .register(&mut register_model)
            .expect("base register");
        let token_count = register_model.tokens().len();
        let constraint_count = register_model.as_basic().constraints().len();
        assert!(
            add_context.extensions.is_empty(),
            "base context has no extensions"
        );

        let mut failing_add_context = RouteCompilationContext::with_extensions(
            add_context.aggregation.instance.clone(),
            vec![Arc::new(FailingModelMutationExtension)],
        );
        failing_add_context
            .register(&mut register_model)
            .expect_err("extension registration should fail");

        let mut successful_context =
            RouteCompilationContext::new(add_context.aggregation.instance.clone());
        successful_context
            .register(&mut register_model)
            .expect_err("a context cannot reuse the failed extension model");
        assert_eq!(register_model.tokens().len(), token_count);
        assert_eq!(
            register_model.as_basic().constraints().len(),
            constraint_count
        );

        let mut fresh_model = MetaModel::<f64>::new("failing-add-model");
        let mut failing_add_context = RouteCompilationContext::with_extensions(
            add_context.aggregation.instance.clone(),
            vec![Arc::new(FailingModelMutationExtension)],
        );
        failing_add_context
            .aggregation
            .register(&mut fresh_model)
            .expect("base aggregation register");
        let token_count = fresh_model.tokens().len();
        let constraint_count = fresh_model.as_basic().constraints().len();
        assert!(
            failing_add_context
                .add_columns(0, [route], &mut fresh_model)
                .is_err()
        );
        assert_eq!(fresh_model.tokens().len(), token_count);
        assert_eq!(fresh_model.as_basic().constraints().len(), constraint_count);
        assert!(failing_add_context.aggregation.routes().is_empty());
        assert!(
            failing_add_context
                .aggregation
                .route_variable_indices()
                .is_empty()
        );
    }

    #[test]
    fn public_route_aggregation_rolls_back_register_remove_and_phase_two_failures() {
        let instance = test_instance();
        let mut model = MetaModel::<f64>::new("public-aggregation-rollback");
        let mut first =
            crate::domain::route_compilation::RouteCompilationAggregation::new(instance.clone());
        first
            .register(&mut model)
            .expect("first aggregation register");
        let token_count = model.tokens().len();
        let constraint_count = model.as_basic().constraints().len();

        let mut conflicting =
            crate::domain::route_compilation::RouteCompilationAggregation::new(instance.clone());
        assert!(conflicting.register(&mut model).is_err());
        assert!(conflicting.artificial_coverage.is_empty());
        assert!(conflicting.route_variable_indices().is_empty());
        assert_eq!(model.tokens().len(), token_count);
        assert_eq!(model.as_basic().constraints().len(), constraint_count);

        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);
        first
            .add_columns(0, [route.clone()], &mut model)
            .expect("add route column");
        first
            .switch_to_phase_two(&mut model)
            .expect("switch to phase two");
        let unindexed = bare_route("v1", &["start", "end"], &["unindexed"]);
        first.column_pool.add_columns([unindexed]);
        let route_count = first.routes().len();
        let route_index = first
            .route_variable_indices()
            .get(&route.signature())
            .copied()
            .expect("route index");
        let route_range = model
            .variable_range_by_index(route_index.model_index)
            .expect("route range");
        assert!(first.remove_columns([route.clone()], &mut model).is_err());
        assert_eq!(first.routes().len(), route_count);
        assert_eq!(
            first
                .route_variable_indices()
                .get(&route.signature())
                .copied(),
            Some(route_index)
        );
        assert_eq!(
            model.variable_range_by_index(route_index.model_index),
            Some(route_range)
        );

        let mut phase_model = MetaModel::<f64>::new("public-phase-two-rollback");
        let mut phase_aggregation =
            crate::domain::route_compilation::RouteCompilationAggregation::new(instance);
        phase_aggregation
            .register(&mut phase_model)
            .expect("phase aggregation register");
        phase_aggregation
            .artificial_coverage
            .variable_indices
            .push(usize::MAX);
        assert!(
            phase_aggregation
                .switch_to_phase_two(&mut phase_model)
                .is_err()
        );
        assert_eq!(phase_aggregation.phase, PricingPhase::PhaseOne);
        assert!(!phase_aggregation.artificial_coverage.is_phase_two);
    }

    #[test]
    fn route_compilation_extension_and_model_binding_cover_full_lifecycle() {
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let extension = Arc::new(RecordingExtension {
            events: events.clone(),
        });
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);
        let mut context = RouteCompilationContext::with_extensions(instance, vec![extension]);
        let mut model = MetaModel::<f64>::new("extension-model");
        context.register(&mut model).expect("register extension");
        context
            .add_columns(0, [route.clone()], &mut model)
            .expect("add extension column");
        let duals = vec![0.0; model.as_basic().constraints().len()];
        context
            .extract_pricing_duals(&model, &duals)
            .expect("refresh extension dual");
        context
            .remove_columns([route], &mut model)
            .expect("remove extension column");
        context
            .extract_solution(&model, 1e-7)
            .expect("extract extension solution");
        let events = events.lock().expect("extension events");
        assert!(events.contains(&"register"));
        assert!(events.contains(&"add"));
        assert!(events.contains(&"dual"));
        assert!(events.contains(&"remove"));
        assert!(events.contains(&"extract"));

        let mut other_model = MetaModel::<f64>::new("other-extension-model");
        assert!(context.add_columns(1, [], &mut other_model).is_err());
    }

    #[test]
    fn application_rejects_illegal_provider_routes_and_keeps_valid_incumbent_on_limit() {
        let instance = test_instance();
        let invalid = bare_route("v1", &["start", "end"], &[]);
        let invalid_provider = FixedNodeProvider {
            result: BranchNodeSolveResult {
                status: BranchNodeSolveStatus::Optimal,
                pricing_complete: true,
                is_feasible: true,
                is_integer: true,
                lower_bound: 0.0,
                lp_objective: 0.0,
                route_values: vec![(invalid.clone(), 1.0)],
                columns: vec![invalid],
                iterations: 1,
                pricing_calls: 0,
                generated_columns: 0,
                interrupted: false,
                solver_report: None,
                solver_model_fingerprint: None,
            },
        };
        let invalid_service = crate::application::VrptwApplicationService::new(
            instance.clone(),
            invalid_provider,
            BranchAndPriceConfig::default(),
        )
        .expect("invalid provider service");
        assert!(invalid_service.solve().is_err());

        let valid = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("valid initial route")
        .remove(0);
        let mut invalid_resources = valid.clone();
        invalid_resources.stops[1].departure += TimeDuration::seconds(1);
        let invalid_resource_provider = FixedNodeProvider {
            result: BranchNodeSolveResult {
                status: BranchNodeSolveStatus::Optimal,
                pricing_complete: true,
                is_feasible: true,
                is_integer: true,
                lower_bound: 3.0,
                lp_objective: 3.0,
                route_values: vec![(invalid_resources.clone(), 1.0)],
                columns: vec![invalid_resources],
                iterations: 1,
                pricing_calls: 0,
                generated_columns: 0,
                interrupted: false,
                solver_report: None,
                solver_model_fingerprint: None,
            },
        };
        assert!(
            crate::application::VrptwApplicationService::new(
                instance.clone(),
                invalid_resource_provider,
                BranchAndPriceConfig::default(),
            )
            .expect("invalid resource service")
            .solve()
            .is_err()
        );
        let mut invalid_cost = valid.clone();
        invalid_cost.cost.value = 99.0;
        let invalid_cost_provider = FixedNodeProvider {
            result: BranchNodeSolveResult {
                status: BranchNodeSolveStatus::Optimal,
                pricing_complete: true,
                is_feasible: true,
                is_integer: true,
                lower_bound: 0.0,
                lp_objective: 0.0,
                route_values: vec![(invalid_cost.clone(), 1.0)],
                columns: vec![invalid_cost],
                iterations: 1,
                pricing_calls: 0,
                generated_columns: 0,
                interrupted: false,
                solver_report: None,
                solver_model_fingerprint: None,
            },
        };
        assert!(
            crate::application::VrptwApplicationService::new(
                instance.clone(),
                invalid_cost_provider,
                BranchAndPriceConfig::default(),
            )
            .expect("invalid cost service")
            .solve()
            .is_err()
        );
        let interrupted_provider = FixedNodeProvider {
            result: BranchNodeSolveResult {
                status: BranchNodeSolveStatus::TimeLimit,
                pricing_complete: false,
                is_feasible: true,
                is_integer: true,
                lower_bound: f64::NEG_INFINITY,
                lp_objective: 3.0,
                route_values: vec![(valid.clone(), 1.0)],
                columns: vec![valid],
                iterations: 1,
                pricing_calls: 1,
                generated_columns: 1,
                interrupted: true,
                solver_report: None,
                solver_model_fingerprint: None,
            },
        };
        let result = crate::application::VrptwApplicationService::new(
            instance,
            interrupted_provider,
            BranchAndPriceConfig::default(),
        )
        .expect("interrupted provider service")
        .solve()
        .expect("time-limit result");
        assert_eq!(result.problem_status, ProblemStatus::Feasible);
        assert_eq!(result.termination_reason, TerminationReason::TimeLimit);
        assert!(result.has_incumbent());
        assert!(result.trace.upper_bound.is_some());
        assert!(result.statistics.best_bound_value.is_none());
        assert!(result.trace.global_lower_bound.is_none());
    }

    #[test]
    fn application_rejects_inconsistent_provider_bounds() {
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("valid initial route")
        .remove(0);
        let result = BranchNodeSolveResult {
            status: BranchNodeSolveStatus::Optimal,
            pricing_complete: true,
            is_feasible: true,
            is_integer: true,
            lower_bound: 1_000_000_000.0,
            lp_objective: 1_000_000_000.0,
            route_values: vec![(route.clone(), 1.0)],
            columns: vec![route],
            iterations: 1,
            pricing_calls: 0,
            generated_columns: 0,
            interrupted: false,
            solver_report: None,
            solver_model_fingerprint: None,
        };
        let service = crate::application::VrptwApplicationService::new(
            instance,
            FixedNodeProvider { result },
            BranchAndPriceConfig::default(),
        )
        .expect("inconsistent-bound service");
        assert!(service.solve().is_err());
    }

    #[test]
    fn application_reports_all_six_terminal_states_with_honest_bounds() {
        let instance = test_instance();
        let valid = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("valid initial route")
        .remove(0);
        let integer_result =
            |status: BranchNodeSolveStatus, lower_bound: f64| BranchNodeSolveResult {
                status,
                pricing_complete: true,
                is_feasible: true,
                is_integer: true,
                lower_bound,
                lp_objective: lower_bound,
                route_values: vec![(valid.clone(), 1.0)],
                columns: vec![valid.clone()],
                iterations: 2,
                pricing_calls: 1,
                generated_columns: 1,
                interrupted: false,
                solver_report: None,
                solver_model_fingerprint: None,
            };
        let interrupted_result = |status: BranchNodeSolveStatus| BranchNodeSolveResult {
            status,
            pricing_complete: false,
            is_feasible: false,
            is_integer: false,
            lower_bound: f64::NEG_INFINITY,
            lp_objective: f64::INFINITY,
            route_values: Vec::new(),
            columns: Vec::new(),
            iterations: 1,
            pricing_calls: 1,
            generated_columns: 0,
            interrupted: true,
            solver_report: None,
            solver_model_fingerprint: None,
        };
        let solve = |result: BranchNodeSolveResult<f64>, config: BranchAndPriceConfig| {
            crate::application::VrptwApplicationService::new(
                instance.clone(),
                FixedNodeProvider { result },
                config,
            )
            .expect("terminal-state service")
            .solve()
            .expect("terminal state is a normal result")
        };

        let optimal = solve(
            integer_result(BranchNodeSolveStatus::Optimal, 3.0),
            BranchAndPriceConfig::default(),
        );
        assert_eq!(optimal.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            optimal.solution_presence,
            ospf_rust_core::solver::SolutionPresence::Optimal
        );
        assert_eq!(optimal.statistics.best_bound_value, Some(3.0));
        assert_eq!(optimal.trace.upper_bound, Some(3.0));
        assert_eq!(optimal.statistics.relative_gap, Some(0.0));

        let feasible = solve(
            integer_result(BranchNodeSolveStatus::Optimal, 0.0),
            BranchAndPriceConfig::default(),
        );
        assert_eq!(feasible.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            feasible.solution_presence,
            ospf_rust_core::solver::SolutionPresence::Incumbent
        );
        assert_eq!(feasible.statistics.best_bound_value, Some(0.0));
        assert_eq!(feasible.trace.upper_bound, Some(3.0));
        assert!(
            feasible
                .statistics
                .relative_gap
                .is_some_and(|gap| gap > 0.0)
        );

        let infeasible = solve(
            BranchNodeSolveResult::infeasible(),
            BranchAndPriceConfig::default(),
        );
        assert_eq!(infeasible.problem_status, ProblemStatus::Unknown);
        assert!(!infeasible.has_incumbent());
        assert!(infeasible.statistics.best_bound_value.is_none());
        assert!(infeasible.trace.upper_bound.is_none());

        let claimed_report =
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof {
                    status: ospf_rust_core::solver::ProofStatus::Claimed,
                    reliability: ospf_rust_core::solver::ProofReliability::Unknown,
                    completeness: ospf_rust_core::solver::ProofCompleteness::Partial,
                    ..SolveProof::infeasibility()
                })
                .build()
                .expect("claimed node report should remain representable");
        let mut claimed_result = BranchNodeSolveResult::infeasible();
        claimed_result.solver_report = Some(claimed_report);
        let claimed = solve(claimed_result, BranchAndPriceConfig::default());
        assert_eq!(claimed.problem_status, ProblemStatus::Unknown);
        assert!(!claimed.has_incumbent());

        let mut verified_result = BranchNodeSolveResult::infeasible();
        verified_result.solver_report = Some(
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof::infeasibility())
                .build()
                .expect("verified node report should build"),
        );
        let verified = solve(verified_result, BranchAndPriceConfig::default());
        assert_eq!(verified.problem_status, ProblemStatus::Unknown);
        assert!(!verified.has_incumbent());
        assert!(verified.proof.is_none());

        let time_limit = solve(
            interrupted_result(BranchNodeSolveStatus::TimeLimit),
            BranchAndPriceConfig::default(),
        );
        assert_eq!(time_limit.problem_status, ProblemStatus::Unknown);
        assert_eq!(time_limit.termination_reason, TerminationReason::TimeLimit);
        assert!(!time_limit.has_incumbent());
        assert!(time_limit.statistics.best_bound_value.is_none());
        assert!(time_limit.trace.upper_bound.is_none());

        let node_limit = solve(
            interrupted_result(BranchNodeSolveStatus::TimeLimit),
            BranchAndPriceConfig {
                node_limit: 0,
                ..BranchAndPriceConfig::default()
            },
        );
        assert_eq!(node_limit.problem_status, ProblemStatus::Unknown);
        assert_eq!(node_limit.termination_reason, TerminationReason::NodeLimit);
        assert!(!node_limit.has_incumbent());
        assert!(node_limit.statistics.best_bound_value.is_none());

        let solver_stopped = solve(
            interrupted_result(BranchNodeSolveStatus::SolverStopped),
            BranchAndPriceConfig::default(),
        );
        assert_eq!(solver_stopped.problem_status, ProblemStatus::Unknown);
        assert_eq!(
            solver_stopped.termination_reason,
            TerminationReason::Interrupted
        );
        assert!(!solver_stopped.has_incumbent());
        assert!(solver_stopped.statistics.best_bound_value.is_none());
        assert!(solver_stopped.trace.upper_bound.is_none());
    }

    #[test]
    fn application_accepts_only_model_bound_phase_infeasibility_reports() {
        let instance = test_instance();
        let config = || BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let phase_one_provider = BranchNodeSolver::new(
            instance.clone(),
            Arc::new(PhaseOneInfeasibleLpSolver),
            config(),
        )
        .expect("phase-one provider construction");
        let phase_one = crate::application::VrptwApplicationService::new(
            instance.clone(),
            phase_one_provider,
            BranchAndPriceConfig::default(),
        )
        .expect("phase-one service construction")
        .solve()
        .expect("phase-one infeasible node is a normal terminal state");
        assert_eq!(phase_one.problem_status, ProblemStatus::Infeasible);
        assert!(phase_one.proof.is_some());

        let phase_two_provider = BranchNodeSolver::new(
            instance.clone(),
            Arc::new(PhaseTwoInfeasibleLpSolver),
            config(),
        )
        .expect("phase-two provider construction");
        let phase_two = crate::application::VrptwApplicationService::new(
            instance,
            phase_two_provider,
            BranchAndPriceConfig::default(),
        )
        .expect("phase-two service construction")
        .solve()
        .expect("phase-two infeasible node is a normal terminal state");
        assert_eq!(phase_two.problem_status, ProblemStatus::Infeasible);
        assert!(phase_two.proof.is_some());
    }

    #[test]
    fn branch_node_solver_accepts_verified_infeasibility_from_final_lp() {
        let instance = multi_customer_instance(2);
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider =
            BranchNodeSolver::new(instance.clone(), Arc::new(FinalLpInfeasibleSolver), config)
                .expect("final-LP provider construction");
        let node = BranchNode::root(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
        )
        .expect("root node");
        let result = provider
            .solve_node(
                &node,
                &[],
                &BranchNodeSolveContext {
                    max_cg_iterations: 3,
                    max_columns_per_pricing: usize::MAX,
                    max_labels: None,
                    deadline: None,
                    cancellation: None,
                },
            )
            .expect("final-LP infeasibility is a normal node terminal");
        assert_eq!(result.status, BranchNodeSolveStatus::Infeasible);
        assert!(result.pricing_complete);
        assert!(result.solver_report.is_some());
        assert!(result.solver_model_fingerprint.is_some());
        assert!(result.generated_columns > 0);

        let service = crate::application::VrptwApplicationService::new(
            instance,
            BranchNodeSolver::new(
                multi_customer_instance(2),
                Arc::new(FinalLpInfeasibleSolver),
                BranchNodeSolverConfig {
                    distance_calculator: EuclideanDistanceCalculator,
                    travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                        EuclideanDistanceCalculator,
                    ),
                    arc_cost_calculator: DistanceArcCostCalculator,
                    route_cost_policy: FixedPlusArcCostPolicy,
                    arc_feasibility_policy: Arc::new(
                        crate::domain::vrp::DefaultArcFeasibilityPolicy,
                    ),
                    dominance_policy: DefaultLabelDominancePolicy,
                    column_selector: DefaultPricingColumnSelector,
                    extensions: Vec::new(),
                },
            )
            .expect("application provider construction"),
            BranchAndPriceConfig::default(),
        )
        .expect("application construction");
        let report = service
            .solve()
            .expect("verified final-LP infeasibility should reach the application");
        assert_eq!(report.problem_status, ProblemStatus::Infeasible);
        assert!(report.proof.is_some());
    }

    #[test]
    fn branch_node_solver_retains_an_integer_rmp_solution_on_lp_time_limit() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(
            instance.clone(),
            Arc::new(OptimalThenTimeLimitSolver),
            config,
        )
        .expect("provider construction");
        let node = BranchNode::root(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
        )
        .expect("root node");
        let result = provider
            .solve_node(
                &node,
                &[],
                &BranchNodeSolveContext {
                    max_cg_iterations: 2,
                    max_columns_per_pricing: usize::MAX,
                    max_labels: None,
                    deadline: None,
                    cancellation: None,
                },
            )
            .expect("node solve");
        assert_eq!(result.status, BranchNodeSolveStatus::TimeLimit);
        assert!(!result.pricing_complete);
        assert_eq!(result.route_values.len(), 1);
        assert_eq!(result.route_values[0].1, 1.0);
    }

    #[test]
    fn branch_node_solver_propagates_solver_call_failure() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(FailingLpSolver), config)
            .expect("provider construction");
        let node = BranchNode::root(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
        )
        .expect("root node");
        let error = provider
            .solve_node(
                &node,
                &[],
                &BranchNodeSolveContext {
                    max_cg_iterations: 1,
                    max_columns_per_pricing: usize::MAX,
                    max_labels: None,
                    deadline: None,
                    cancellation: None,
                },
            )
            .expect_err("solver failure must propagate");
        assert!(matches!(
            error,
            crate::NetworkSchedulingError::Solver { .. }
        ));
    }

    #[test]
    fn branch_node_solver_rejects_optimal_status_without_verified_report() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(
            instance.clone(),
            Arc::new(UncertifiedOptimalLpSolver),
            config,
        )
        .expect("provider construction");
        let node = BranchNode::root(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
        )
        .expect("root node");
        let error = provider
            .solve_node(
                &node,
                &[],
                &BranchNodeSolveContext {
                    max_cg_iterations: 1,
                    max_columns_per_pricing: usize::MAX,
                    max_labels: None,
                    deadline: None,
                    cancellation: None,
                },
            )
            .expect_err("an uncertified optimal status must not enter pricing");
        assert!(matches!(
            error,
            crate::NetworkSchedulingError::Solver { .. }
        ));
    }

    #[test]
    fn branch_node_solver_reports_pricing_iteration_exhaustion() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider =
            BranchNodeSolver::new(instance.clone(), Arc::new(NonConvergingLpSolver), config)
                .expect("provider construction");
        let node = BranchNode::root(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
        )
        .expect("root node");
        let error = provider
            .solve_node(
                &node,
                &[],
                &BranchNodeSolveContext {
                    max_cg_iterations: 1,
                    max_columns_per_pricing: usize::MAX,
                    max_labels: None,
                    deadline: None,
                    cancellation: None,
                },
            )
            .expect_err("pricing iteration exhaustion must be reported");
        assert!(matches!(
            error,
            crate::NetworkSchedulingError::Pricing { .. }
        ));
    }

    #[test]
    fn application_propagates_pre_cancelled_token_as_solver_stopped() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(FakeLpSolver), config)
            .expect("provider construction");
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let result = crate::application::VrptwApplicationService::new(
            instance,
            provider,
            BranchAndPriceConfig {
                cancellation: Some(cancellation),
                ..BranchAndPriceConfig::default()
            },
        )
        .expect("service construction")
        .solve()
        .expect("cancellation is a normal terminal state");
        assert_eq!(result.problem_status, ProblemStatus::Unknown);
        assert_eq!(result.termination_reason, TerminationReason::Cancelled);
        assert!(!result.has_incumbent());
        assert!(result.statistics.best_bound_value.is_none());
    }

    #[test]
    fn application_propagates_trace_listener_failure() {
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);
        let provider = FixedNodeProvider {
            result: BranchNodeSolveResult {
                status: BranchNodeSolveStatus::Optimal,
                pricing_complete: true,
                is_feasible: true,
                is_integer: true,
                lower_bound: route.cost.value,
                lp_objective: route.cost.value,
                route_values: vec![(route.clone(), 1.0)],
                columns: vec![route],
                iterations: 1,
                pricing_calls: 1,
                generated_columns: 1,
                interrupted: false,
                solver_report: None,
                solver_model_fingerprint: None,
            },
        };
        let algorithm = crate::application::BranchAndPriceAlgorithm::new(
            instance,
            provider,
            BranchAndPriceConfig::default(),
        )
        .expect("algorithm construction")
        .with_trace_listener(Arc::new(
            |_: &crate::application::BranchAndPriceTrace| -> crate::Result<()> {
                Err(crate::NetworkSchedulingError::contract(
                    "injected trace failure / 注入的 trace 失败",
                ))
            },
        ));
        let error = algorithm.solve().expect_err("trace failure must propagate");
        assert!(matches!(
            error,
            crate::NetworkSchedulingError::Contract { .. }
        ));
    }

    #[test]
    fn application_propagates_fake_solution_enricher_failure() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: vec![Arc::new(FailingSolutionExtension)],
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(FakeLpSolver), config)
            .expect("provider construction");
        let error = crate::application::VrptwApplicationService::new(
            instance,
            provider,
            BranchAndPriceConfig::default(),
        )
        .expect("service construction")
        .solve()
        .expect_err("solution enrichment failure must propagate");
        assert!(matches!(
            error,
            crate::NetworkSchedulingError::Contract { .. }
        ));
    }

    #[test]
    fn application_runs_real_provider_through_phase_one_and_phase_two() {
        let instance = test_instance();
        let config = BranchNodeSolverConfig {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
            route_cost_policy: FixedPlusArcCostPolicy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
            dominance_policy: DefaultLabelDominancePolicy,
            column_selector: DefaultPricingColumnSelector,
            extensions: Vec::new(),
        };
        let provider = BranchNodeSolver::new(instance.clone(), Arc::new(FakeLpSolver), config)
            .expect("provider construction");
        let service = crate::application::VrptwApplicationService::new(
            instance,
            provider,
            BranchAndPriceConfig::default(),
        )
        .expect("service construction");
        let result = service.solve().expect("branch-and-price solve");
        assert_eq!(result.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            result.solution_presence,
            ospf_rust_core::solver::SolutionPresence::Optimal
        );
        assert!(result.has_incumbent());
        assert_eq!(result.trace.total_iterations, 2);
        assert!(result.trace.pricing_calls >= 1);
    }

    #[test]
    fn assignment_branch_precedes_edge_branch() {
        let instance = test_instance();
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("initial route")
        .remove(0);
        let decision = select_branch_decision(&[(route, 0.5)], &instance, 1e-7)
            .expect("branch selection")
            .expect("fractional assignment branch");
        assert!(matches!(
            decision,
            crate::application::BranchDecision::ForbidVehicleType { .. }
        ));
    }

    #[test]
    fn multi_vehicle_type_assignment_and_arc_branches_filter_both_sides() {
        let instance = multi_vehicle_type_instance();
        let start = instance.start_depot.node.id.clone();
        let end = instance.end_depot.node.id.clone();
        let customer = instance.customers[0].id.clone();
        let customer_node = instance.customers[0].node.id.clone();
        let small = VehicleTypeId::from("small");
        let large = VehicleTypeId::from("large");
        let root = BranchPath::root(start.clone(), end.clone()).expect("root branch path");
        let assignment_decision = BranchDecision::ForbidVehicleType {
            vehicle_type_id: small.clone(),
            customer_id: customer.clone(),
            customer_node_id: customer_node.clone(),
        };
        let forbid_small = root
            .with_decision(assignment_decision.clone())
            .expect("forbid-small branch");
        let require_small = root
            .with_decision(assignment_decision.complementary())
            .expect("require-small branch");

        let duals = PricingDuals::new(PricingPhase::PhaseTwo, [], []);
        let builder = RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        );
        let forbid_small_graph = builder
            .build(&small, &duals, Some(&forbid_small.mask))
            .expect("forbid-small pricing graph");
        let forbid_small_large_graph = builder
            .build(&large, &duals, Some(&forbid_small.mask))
            .expect("large graph on forbid-small branch");
        assert!(
            !forbid_small_graph
                .nodes
                .iter()
                .any(|node| node.node_id == customer_node)
        );
        assert!(
            forbid_small_large_graph
                .nodes
                .iter()
                .any(|node| node.node_id == customer_node)
        );

        let require_small_graph = builder
            .build(&small, &duals, Some(&require_small.mask))
            .expect("require-small pricing graph");
        let require_small_large_graph = builder
            .build(&large, &duals, Some(&require_small.mask))
            .expect("large graph on require-small branch");
        assert!(
            require_small_graph
                .nodes
                .iter()
                .any(|node| node.node_id == customer_node)
        );
        assert!(
            !require_small_large_graph
                .nodes
                .iter()
                .any(|node| node.node_id == customer_node)
        );

        let start_to_customer = default_arc_id(&start, &customer_node);
        let arc_decision = BranchDecision::ForbidArc {
            vehicle_type_id: large.clone(),
            arc_id: start_to_customer.clone(),
            from: start.clone(),
            to: customer_node.clone(),
        };
        let forbid_arc = root
            .with_decision(arc_decision.clone())
            .expect("forbid-arc branch");
        let require_arc = root
            .with_decision(arc_decision.complementary())
            .expect("require-arc branch");
        let forbid_arc_graph = builder
            .build(&large, &duals, Some(&forbid_arc.mask))
            .expect("forbid-arc graph");
        assert!(
            !forbid_arc_graph
                .arcs
                .iter()
                .any(|arc| arc.arc_id == start_to_customer)
        );

        let require_arc_graph = builder
            .build(&large, &duals, Some(&require_arc.mask))
            .expect("require-arc graph");
        assert!(
            require_arc_graph
                .arcs
                .iter()
                .any(|arc| arc.arc_id == start_to_customer)
        );
        assert!(
            require_arc_graph
                .arcs
                .iter()
                .any(|arc| arc.from == require_arc_graph.start_index()
                    && arc.arc_id != start_to_customer)
        );
        assert!(require_arc.mask.is_route_compatible(
            &large,
            &[start, customer_node, end],
            &[
                start_to_customer,
                default_arc_id(&instance.customers[0].node.id, &instance.end_depot.node.id)
            ],
        ));
    }

    #[cfg(feature = "big-decimal")]
    #[test]
    fn big_decimal_values_cross_the_domain_solver_boundary() {
        use crate::infrastructure::NetworkSchedulingSolverValueAdapter;
        use bigdecimal::BigDecimal;
        use std::str::FromStr;

        let unit = Kilogram::INSTANT.clone();
        let bounds = CapacityBounds::try_new(
            Quantity::new(BigDecimal::from(1), unit.clone()),
            Quantity::new(BigDecimal::from(2), unit.clone()),
        )
        .expect("valid decimal capacity");
        assert_eq!(bounds.upper.value, BigDecimal::from(2));

        let imbalanced = FlowGraph::new_with_units(
            vec![FlowNode::new("n")],
            Vec::new(),
            vec![SupplyDemand::new("m").with_balance(
                "n",
                Quantity::new(
                    BigDecimal::from_str("0.00000001").expect("high-precision imbalance"),
                    unit.clone(),
                ),
            )],
            FlowUnits::new(unit.clone(), VrptwUnits::default().cost_unit)
                .expect("valid decimal flow units"),
        );
        assert!(imbalanced.is_err());

        let adapter = NetworkSchedulingSolverValueAdapter::<BigDecimal>::default();
        let value = BigDecimal::from(3) / BigDecimal::from(2);
        assert_eq!(adapter.to_solver(&value).expect("decimal conversion"), 1.5);

        let start = OffsetDateTime::UNIX_EPOCH;
        let end = start + TimeDuration::hours(1);
        let scheduling_window = TimeWindow::seconds(
            TimeRange::new(start, end),
            BigDecimal::from(0),
            false,
            BigDecimal::from(1),
        );
        let service_window = ServiceTimeWindow::new(start, end).expect("decimal test window");
        let instance = Arc::new(
            VrptwInstance::new(
                "big-decimal-vrptw",
                Depot {
                    node: coordinate_node(
                        "start",
                        BigDecimal::from(0),
                        BigDecimal::from(0),
                        Meter::INSTANT.clone(),
                    )
                    .expect("decimal start coordinate"),
                    time_window: service_window,
                },
                Depot {
                    node: coordinate_node(
                        "end",
                        BigDecimal::from(0),
                        BigDecimal::from(0),
                        Meter::INSTANT.clone(),
                    )
                    .expect("decimal end coordinate"),
                    time_window: service_window,
                },
                vec![
                    Customer::new(
                        "c1",
                        coordinate_node(
                            "c1-node",
                            BigDecimal::from(1),
                            BigDecimal::from(0),
                            Meter::INSTANT.clone(),
                        )
                        .expect("decimal customer coordinate"),
                        Quantity::new(BigDecimal::from(1), Kilogram::INSTANT.clone()),
                        service_window,
                        TimeDuration::ZERO,
                    )
                    .expect("decimal customer"),
                ],
                vec![
                    VehicleType::new(
                        "v1",
                        Quantity::new(BigDecimal::from(2), Kilogram::INSTANT.clone()),
                        Quantity::new(BigDecimal::from(1), VrptwUnits::default().cost_unit),
                        1,
                    )
                    .expect("decimal vehicle"),
                ],
                scheduling_window,
                VrptwUnits::default(),
                Default::default(),
            )
            .expect("decimal instance"),
        );
        let route = InitialRouteGenerator::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        )
        .generate(None)
        .expect("decimal initial route")
        .remove(0);
        RouteValidator::validate(
            &instance,
            &route,
            &EuclideanDistanceCalculator,
            &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            &DistanceArcCostCalculator,
            &FixedPlusArcCostPolicy,
            None,
        )
        .expect("decimal route validation");
        let vehicle_type_id = VehicleTypeId::from("v1");
        let duals = PricingDuals::new(
            PricingPhase::PhaseTwo,
            [(CustomerId::from("c1"), 20.0)],
            [(vehicle_type_id.clone(), 0.0)],
        );
        let pricing = EspprcPricer::new(RouteGraphBuilder::new(
            instance.clone(),
            EuclideanDistanceCalculator,
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            DistanceArcCostCalculator,
            FixedPlusArcCostPolicy,
        ))
        .price_request(&PricingRequest::new(instance, duals, vehicle_type_id))
        .expect("decimal pricing");
        assert!(!pricing.routes.is_empty());
    }
}
