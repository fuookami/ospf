//! 初始路线生成 / Initial route generation.

use std::collections::BTreeSet;
use std::sync::Arc;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use time::Duration;

use crate::domain::vrp::{
    ArcCostCalculator, ArcFeasibilityPolicy, BranchMask, DistanceCalculator, Route,
    RouteCostPolicy, RouteStop, RouteValidator, TravelTimeCalculator, VehicleType, VehicleTypeId,
    VrptwInstance, default_arc_id,
};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkArcId;

type ArcMetrics<V> = (Quantity<V, Unit>, Duration, Quantity<V, Unit>, NetworkArcId);

/// 为受限主问题生成单客户初始列 / Generate single-customer initial columns for an RMP.
pub struct InitialRouteGenerator<
    V,
    D = crate::domain::vrp::EuclideanDistanceCalculator,
    T = crate::domain::vrp::DistanceAsTravelTimeCalculator<D>,
    A = crate::domain::vrp::DistanceArcCostCalculator,
    C = crate::domain::vrp::FixedPlusArcCostPolicy,
> where
    V: SolveValue + UnitConversionValue,
{
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<V>>,
    /// 距离策略 / Distance policy.
    pub distance_calculator: D,
    /// 行驶时间策略 / Travel-time policy.
    pub travel_time_calculator: T,
    /// 弧成本策略 / Arc-cost policy.
    pub arc_cost_calculator: A,
    /// 路线成本策略 / Route-cost policy.
    pub route_cost_policy: C,
    /// 静态弧可行性策略 / Static arc-feasibility policy.
    pub arc_feasibility_policy: Arc<dyn ArcFeasibilityPolicy<V>>,
}

impl<V, D, T, A, C> InitialRouteGenerator<V, D, T, A, C>
where
    V: SolveValue + UnitConversionValue,
    D: DistanceCalculator<V>,
    T: TravelTimeCalculator<V>,
    A: ArcCostCalculator<V>,
    C: RouteCostPolicy<V>,
{
    /// 创建初始路线生成器 / Create an initial-route generator.
    pub fn new(
        instance: Arc<VrptwInstance<V>>,
        distance_calculator: D,
        travel_time_calculator: T,
        arc_cost_calculator: A,
        route_cost_policy: C,
    ) -> Self {
        Self {
            instance,
            distance_calculator,
            travel_time_calculator,
            arc_cost_calculator,
            route_cost_policy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
        }
    }

    /// 设置静态弧可行性策略 / Set the static arc-feasibility policy.
    pub fn with_arc_feasibility_policy(mut self, policy: Arc<dyn ArcFeasibilityPolicy<V>>) -> Self {
        self.arc_feasibility_policy = policy;
        self
    }

    /// 生成单客户可行路线；不可行启发式候选会被跳过 / Generate feasible single-customer routes; infeasible heuristic candidates are skipped.
    pub fn generate(
        &self,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<Vec<Route<V>>> {
        let mut routes = Vec::new();
        let mut signatures = BTreeSet::new();
        for customer in &self.instance.customers {
            for vehicle_type in &self.instance.vehicle_types {
                if branch_mask
                    .is_some_and(|mask| !mask.allows_node(&vehicle_type.id, &customer.node.id))
                {
                    continue;
                }
                let Some(route) = self.build_single_customer_route(customer, vehicle_type)? else {
                    continue;
                };
                if branch_mask.is_some_and(|mask| {
                    !mask.is_route_compatible(
                        &vehicle_type.id,
                        &route
                            .stops
                            .iter()
                            .map(|stop| stop.node_id.clone())
                            .collect::<Vec<_>>(),
                        &route.effective_arc_ids(),
                    )
                }) {
                    continue;
                }
                if signatures.insert(route.signature()) {
                    routes.push(route);
                }
            }
        }
        Ok(routes)
    }

    /// 生成所有单客户初始列的别名 / Alias for generating all single-customer initial columns.
    pub fn generate_routes(
        &self,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<Vec<Route<V>>> {
        self.generate(branch_mask)
    }

    fn build_single_customer_route(
        &self,
        customer: &crate::domain::vrp::Customer<V>,
        vehicle_type: &VehicleType<V>,
    ) -> Result<Option<Route<V>>> {
        let start = &self.instance.start_depot.node;
        let end = &self.instance.end_depot.node;
        if !self
            .arc_feasibility_policy
            .is_feasible(start, &customer.node, vehicle_type)?
            || !self
                .arc_feasibility_policy
                .is_feasible(&customer.node, end, vehicle_type)?
        {
            return Ok(None);
        }
        let Some((distance1, travel1, arc_cost1, arc_id1)) =
            self.arc_metrics(start, &customer.node, vehicle_type)?
        else {
            return Ok(None);
        };
        let Some((distance2, travel2, arc_cost2, arc_id2)) =
            self.arc_metrics(&customer.node, end, vehicle_type)?
        else {
            return Ok(None);
        };

        let window = &self.instance.scheduling_window;
        let to_value = |instant| {
            window
                .value_of_instant(instant)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })
        };
        let start_departure = to_value(self.instance.start_depot.time_window.ready_time)?;
        let arrival_customer = start_departure
            + window
                .value_of_duration(travel1)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
        let customer_ready = to_value(customer.time_window.ready_time)?;
        let customer_due = to_value(customer.time_window.due_time)?;
        let service_start_customer = arrival_customer.max(customer_ready);
        if service_start_customer > customer_due + self.instance.tolerances.feasibility {
            return Ok(None);
        }
        let departure_customer = service_start_customer
            + window
                .value_of_duration(customer.service_time)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
        let arrival_end = departure_customer
            + window
                .value_of_duration(travel2)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
        let end_ready = to_value(self.instance.end_depot.time_window.ready_time)?;
        let end_due = to_value(self.instance.end_depot.time_window.due_time)?;
        let service_start_end = arrival_end.max(end_ready);
        if service_start_end > end_due + self.instance.tolerances.feasibility {
            return Ok(None);
        }

        let demand = customer
            .demand
            .to_unit(&self.instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let capacity = vehicle_type
            .capacity
            .to_unit(&self.instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let demand_f64 = demand
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let capacity_f64 = capacity
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if demand_f64 > capacity_f64 + self.instance.tolerances.feasibility {
            return Ok(None);
        }
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let load_quantity = |value: f64| {
            V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding)
                .map(|converted| Quantity::new(converted, self.instance.units.load_unit.clone()))
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })
        };
        let stops = vec![
            RouteStop {
                node_id: start.id.clone(),
                customer_id: None,
                arrival: window.instant_of(
                    V::from_f64_with_policy(
                        start_departure,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                service_start: window.instant_of(
                    V::from_f64_with_policy(
                        start_departure,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                departure: window.instant_of(
                    V::from_f64_with_policy(
                        start_departure,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                accumulated_load: Quantity::new(zero, self.instance.units.load_unit.clone()),
            },
            RouteStop {
                node_id: customer.node.id.clone(),
                customer_id: Some(customer.id.clone()),
                arrival: window.instant_of(
                    V::from_f64_with_policy(
                        arrival_customer,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                service_start: window.instant_of(
                    V::from_f64_with_policy(
                        service_start_customer,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                departure: window.instant_of(
                    V::from_f64_with_policy(
                        departure_customer,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                accumulated_load: load_quantity(demand_f64)?,
            },
            RouteStop {
                node_id: end.id.clone(),
                customer_id: None,
                arrival: window.instant_of(
                    V::from_f64_with_policy(arrival_end, SolveValueConversionPolicy::AllowRounding)
                        .map_err(|error| NetworkSchedulingError::Conversion {
                            message: error.to_string(),
                        })?,
                ),
                service_start: window.instant_of(
                    V::from_f64_with_policy(
                        service_start_end,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                departure: window.instant_of(
                    V::from_f64_with_policy(
                        service_start_end,
                        SolveValueConversionPolicy::AllowRounding,
                    )
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                ),
                accumulated_load: load_quantity(demand_f64)?,
            },
        ];
        let total_distance = distance1
            .to_unit(&self.instance.units.distance_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            + distance2
                .to_unit(&self.instance.units.distance_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .value;
        let cost = self
            .route_cost_policy
            .cost(vehicle_type, &[arc_cost1, arc_cost2], &self.instance)?
            .to_unit(&self.instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let route = Route::with_arc_ids(
            vehicle_type.id.clone(),
            stops,
            vec![arc_id1, arc_id2],
            Quantity::new(total_distance, self.instance.units.distance_unit.clone()),
            cost,
        )?;
        if RouteValidator::validate(
            &self.instance,
            &route,
            &self.distance_calculator,
            &self.travel_time_calculator,
            &self.arc_cost_calculator,
            &self.route_cost_policy,
            None,
        )
        .is_err()
        {
            return Ok(None);
        }
        Ok(Some(route))
    }

    fn arc_metrics(
        &self,
        from: &crate::domain::vrp::VrpNode<V>,
        to: &crate::domain::vrp::VrpNode<V>,
        vehicle_type: &VehicleType<V>,
    ) -> Result<Option<ArcMetrics<V>>> {
        let default_id = default_arc_id(&from.id, &to.id);
        if !self.instance.arcs.is_empty() {
            let Some(arc) = self.instance.arc_for_route(&default_id, &from.id, &to.id) else {
                return Ok(None);
            };
            if !arc.feasible {
                return Ok(None);
            }
            let distance = arc
                .distance
                .to_unit(&self.instance.units.distance_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let cost = arc
                .cost
                .to_unit(&self.instance.units.cost_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            if arc.travel_time.duration < Duration::ZERO {
                return Err(NetworkSchedulingError::validation(
                    "行驶时间不能为负 / travel time cannot be negative",
                ));
            }
            return Ok(Some((
                distance,
                arc.travel_time.duration,
                cost,
                arc.id.clone(),
            )));
        }
        let distance =
            self.distance_calculator
                .distance(from, to, &self.instance.units.distance_unit)?;
        let travel_time =
            self.travel_time_calculator
                .travel_time(from, to, vehicle_type, &self.instance)?;
        if travel_time < Duration::ZERO {
            return Err(NetworkSchedulingError::validation(
                "行驶时间不能为负 / travel time cannot be negative",
            ));
        }
        let cost = self.arc_cost_calculator.cost(
            from,
            to,
            &distance,
            travel_time,
            vehicle_type,
            &self.instance,
        )?;
        Ok(Some((distance, travel_time, cost, default_id)))
    }
}
