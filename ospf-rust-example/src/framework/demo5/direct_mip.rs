//! Demo5 direct-MIP oracle / Demo5 direct-MIP oracle.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::{LinearSolver, SolverStatus};
use ospf_rust_core::variable::{Binary, UContinuous, VariableRange};
use ospf_rust_framework_network_scheduling::domain::vrp::{
    ArcCostCalculator, CustomerId, DistanceCalculator, TravelTimeCalculator, VrptwInstance,
};

/// Direct-MIP policy bundle / direct-MIP 策略配置。
pub struct DirectMipPolicy<D, T, A> {
    /// Distance policy / 距离策略。
    pub distance_calculator: D,
    /// Travel-time policy / 行驶时间策略。
    pub travel_time_calculator: T,
    /// Arc-cost policy / 弧成本策略。
    pub arc_cost_calculator: A,
}

/// Direct-MIP oracle result / direct-MIP oracle 结果。
#[derive(Debug, Clone)]
pub struct DirectMipResult {
    /// Solver terminal status / 求解器终态。
    pub status: SolverStatus,
    /// Objective value / 目标值。
    pub objective: f64,
    /// Solver best bound / 求解器最佳下界。
    pub best_bound: Option<f64>,
    /// Solver relative gap / 求解器相对 gap。
    pub relative_gap: Option<f64>,
    /// Customer sequences by route / 每条路线的客户序列。
    pub customer_sequences: Vec<Vec<CustomerId>>,
}

struct ArcData {
    from: usize,
    to: usize,
    token_index: usize,
    travel_time: f64,
    cost: f64,
}

/// Solve the single-vehicle-type direct-MIP oracle / 求解单一车辆类型 direct-MIP oracle。
pub fn solve<S, D, T, A>(
    instance: &VrptwInstance<f64>,
    solver: &S,
    policy: &DirectMipPolicy<D, T, A>,
) -> Result<DirectMipResult, String>
where
    S: LinearSolver,
    D: DistanceCalculator<f64> + Sync,
    T: TravelTimeCalculator<f64> + Sync,
    A: ArcCostCalculator<f64> + Sync,
{
    if instance.vehicle_types.len() != 1 {
        return Err(
            "direct-MIP 目前要求单一车辆类型 / direct-MIP currently requires one vehicle type"
                .to_owned(),
        );
    }
    let vehicle = &instance.vehicle_types[0];
    let nodes = std::iter::once(&instance.start_depot.node)
        .chain(instance.customers.iter().map(|customer| &customer.node))
        .chain(std::iter::once(&instance.end_depot.node))
        .collect::<Vec<_>>();
    let start = 0usize;
    let end = nodes.len() - 1;
    let ready_times = std::iter::once(instance.start_depot.time_window.ready_time)
        .chain(
            instance
                .customers
                .iter()
                .map(|customer| customer.time_window.ready_time),
        )
        .chain(std::iter::once(instance.end_depot.time_window.ready_time))
        .map(|instant| instance.scheduling_window.value_of_instant(instant))
        .collect::<Vec<_>>();
    let due_times = std::iter::once(instance.start_depot.time_window.due_time)
        .chain(
            instance
                .customers
                .iter()
                .map(|customer| customer.time_window.due_time),
        )
        .chain(std::iter::once(instance.end_depot.time_window.due_time))
        .map(|instant| instance.scheduling_window.value_of_instant(instant))
        .collect::<Vec<_>>();
    let service_times = std::iter::once(0.0)
        .chain(instance.customers.iter().map(|customer| {
            instance
                .scheduling_window
                .value_of_duration(customer.service_time)
        }))
        .chain(std::iter::once(0.0))
        .collect::<Vec<_>>();
    let capacity = vehicle
        .capacity
        .to_unit(&instance.units.load_unit)
        .map_err(|error| error.to_string())?
        .value;
    let demands = instance
        .customers
        .iter()
        .map(|customer| {
            customer
                .demand
                .to_unit(&instance.units.load_unit)
                .map(|quantity| quantity.value)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let fixed_cost = vehicle
        .fixed_cost
        .to_unit(&instance.units.cost_unit)
        .map_err(|error| error.to_string())?
        .value;

    let mut model = MetaModel::<f64>::new("demo5_direct_mip");
    let mut x_indices = vec![vec![None; nodes.len()]; nodes.len()];
    let mut arcs = Vec::new();
    for from in 0..nodes.len() {
        for to in 0..nodes.len() {
            if from == end || to == start || from == to || (from == start && to == end) {
                continue;
            }
            let distance = policy
                .distance_calculator
                .distance(&nodes[from], &nodes[to], &instance.units.distance_unit)
                .map_err(|error| error.to_string())?;
            let travel_time = policy
                .travel_time_calculator
                .travel_time(&nodes[from], &nodes[to], vehicle, instance)
                .map_err(|error| error.to_string())?;
            let arc_cost = policy
                .arc_cost_calculator
                .cost(
                    &nodes[from],
                    &nodes[to],
                    &distance,
                    travel_time,
                    vehicle,
                    instance,
                )
                .map_err(|error| error.to_string())?
                .to_unit(&instance.units.cost_unit)
                .map_err(|error| error.to_string())?
                .value;
            let token_index = model
                .register_auto_variable::<Binary>(&format!("x_{from}_{to}"))
                .map_err(|error| error.to_string())?;
            x_indices[from][to] = Some(token_index);
            arcs.push(ArcData {
                from,
                to,
                token_index,
                travel_time: instance.scheduling_window.value_of_duration(travel_time),
                cost: arc_cost,
            });
        }
    }

    let service_start = ready_times
        .iter()
        .zip(due_times.iter())
        .enumerate()
        .map(|(index, (&ready, &due))| {
            model
                .register_auto_variable_with_range::<UContinuous>(
                    &format!("service_start_{index}"),
                    VariableRange::bounded(ready, due),
                )
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let load = demands
        .iter()
        .enumerate()
        .map(|(index, &demand)| {
            model
                .register_auto_variable_with_range::<UContinuous>(
                    &format!("load_{}", index + 1),
                    VariableRange::bounded(demand, capacity),
                )
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut objective = arcs
        .iter()
        .map(|arc| {
            (
                arc.token_index,
                arc.cost + if arc.from == start { fixed_cost } else { 0.0 },
            )
        })
        .collect::<Vec<_>>();
    objective.sort_unstable_by_key(|(index, _)| *index);
    model.add_linear_objective(&objective, "direct_mip_cost");
    model.set_objective_category(ObjectiveCategory::Minimum);

    let start_outgoing = arcs
        .iter()
        .filter(|arc| arc.from == start)
        .map(|arc| (arc.token_index, 1.0))
        .collect::<Vec<_>>();
    let end_incoming = arcs
        .iter()
        .filter(|arc| arc.to == end)
        .map(|arc| (arc.token_index, 1.0))
        .collect::<Vec<_>>();
    let depot_balance = difference(&start_outgoing, &end_incoming);
    add_constraint(
        &mut model,
        &depot_balance,
        ConstraintRelation::Equal,
        0.0,
        "depot_route_balance",
    )?;
    add_constraint(
        &mut model,
        &start_outgoing,
        ConstraintRelation::LessEqual,
        vehicle.amount as f64,
        "fleet_size",
    )?;

    for customer in 1..end {
        let incoming = arcs
            .iter()
            .filter(|arc| arc.to == customer)
            .map(|arc| (arc.token_index, 1.0))
            .collect::<Vec<_>>();
        let outgoing = arcs
            .iter()
            .filter(|arc| arc.from == customer)
            .map(|arc| (arc.token_index, 1.0))
            .collect::<Vec<_>>();
        add_constraint(
            &mut model,
            &incoming,
            ConstraintRelation::Equal,
            1.0,
            &format!("customer_{customer}_in"),
        )?;
        add_constraint(
            &mut model,
            &outgoing,
            ConstraintRelation::Equal,
            1.0,
            &format!("customer_{customer}_out"),
        )?;
    }

    let maximum_travel_time = arcs.iter().map(|arc| arc.travel_time).fold(0.0, f64::max);
    let maximum_service_time = service_times.iter().copied().fold(0.0, f64::max);
    let minimum_ready = ready_times.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum_due = due_times.iter().copied().fold(0.0, f64::max);
    let time_big_m = maximum_due - minimum_ready + maximum_travel_time + maximum_service_time + 1.0;
    for arc in &arcs {
        add_constraint(
            &mut model,
            &[
                (service_start[arc.from], 1.0),
                (service_start[arc.to], -1.0),
                (arc.token_index, time_big_m),
            ],
            ConstraintRelation::LessEqual,
            time_big_m - service_times[arc.from] - arc.travel_time,
            &format!("time_propagation_{}_{}", arc.from, arc.to),
        )?;
    }

    for (customer, &demand) in demands.iter().enumerate() {
        let node = customer + 1;
        add_constraint(
            &mut model,
            &[(load[customer], -1.0)],
            ConstraintRelation::LessEqual,
            -demand,
            &format!("load_lower_{node}"),
        )?;
        let from_depot = x_indices[start][node].ok_or_else(|| {
            format!(
                "客户 {} 缺少 depot 弧 / customer {} is missing a depot arc",
                node, node
            )
        })?;
        add_constraint(
            &mut model,
            &[(load[customer], -1.0), (from_depot, capacity)],
            ConstraintRelation::LessEqual,
            capacity - demand,
            &format!("load_from_depot_{node}"),
        )?;
        for other in 1..end {
            if node == other {
                continue;
            }
            let arc = x_indices[node][other].ok_or_else(|| {
                format!(
                    "客户弧缺失：{} -> {} / customer arc is missing: {} -> {}",
                    node, other, node, other
                )
            })?;
            add_constraint(
                &mut model,
                &[
                    (load[customer], 1.0),
                    (load[other - 1], -1.0),
                    (arc, capacity),
                ],
                ConstraintRelation::LessEqual,
                capacity - demands[other - 1],
                &format!("load_propagation_{}_{}", node, other),
            )?;
        }
    }

    let linear_model = model
        .try_to_linear_triad_model()
        .map_err(|error| error.to_string())?;
    let output = solver
        .solve_linear(&linear_model)
        .map_err(|error| error.to_string())?;
    if !output.status.is_feasible() {
        return Err(format!(
            "direct-MIP 求解失败：{:?} / direct-MIP solve failed: {:?}",
            output.status, output.status
        ));
    }
    let objective = output.objective_value.ok_or_else(|| {
        "direct-MIP 缺少目标值 / direct-MIP output has no objective value".to_owned()
    })?;
    let solution = output.solution.as_deref().ok_or_else(|| {
        "direct-MIP 缺少解向量 / direct-MIP output has no solution vector".to_owned()
    })?;
    let customer_sequences = extract_sequences(instance, &model, solution, &arcs, start, end)?;
    Ok(DirectMipResult {
        status: output.status,
        objective,
        best_bound: output.best_bound,
        relative_gap: output.mip_gap,
        customer_sequences,
    })
}

fn add_constraint(
    model: &mut MetaModel<f64>,
    coefficients: &[(usize, f64)],
    relation: ConstraintRelation,
    rhs: f64,
    name: &str,
) -> Result<(), String> {
    model
        .add_linear_constraint(coefficients, relation, rhs, name)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn difference(left: &[(usize, f64)], right: &[(usize, f64)]) -> Vec<(usize, f64)> {
    let mut coefficients = HashMap::<usize, f64>::new();
    for &(index, value) in left {
        *coefficients.entry(index).or_default() += value;
    }
    for &(index, value) in right {
        *coefficients.entry(index).or_default() -= value;
    }
    let mut coefficients = coefficients
        .into_iter()
        .filter(|(_, value)| *value != 0.0)
        .collect::<Vec<_>>();
    coefficients.sort_unstable_by_key(|(index, _)| *index);
    coefficients
}

fn extract_sequences(
    instance: &VrptwInstance<f64>,
    model: &MetaModel<f64>,
    solution: &[f64],
    arcs: &[ArcData],
    start: usize,
    end: usize,
) -> Result<Vec<Vec<CustomerId>>, String> {
    let value_for_token = |token_index: usize| {
        model
            .tokens()
            .get(token_index)
            .and_then(|token| solution.get(token.solver_index))
            .copied()
            .ok_or_else(|| {
                format!(
                    "direct-MIP 解向量缺少变量 {} / direct-MIP solution is missing variable {}",
                    token_index, token_index
                )
            })
    };
    let mut successors = HashMap::<usize, usize>::new();
    let mut start_targets = Vec::new();
    for arc in arcs {
        let value = value_for_token(arc.token_index)?;
        if !value.is_finite() || (value - value.round()).abs() > 1e-5 {
            return Err(format!(
                "direct-MIP 弧变量不是整数：{} -> {} = {} / direct-MIP arc variable is fractional: {} -> {} = {}",
                arc.from, arc.to, value, arc.from, arc.to, value
            ));
        }
        if value >= 0.5 {
            if arc.from == start {
                start_targets.push(arc.to);
            } else if successors.insert(arc.from, arc.to).is_some() {
                return Err(format!(
                    "direct-MIP 节点存在多条出弧：{} / direct-MIP node has multiple outgoing arcs: {}",
                    arc.from, arc.from
                ));
            }
        }
    }
    let mut sequences = Vec::new();
    let mut covered = HashSet::new();
    for first_customer in start_targets {
        let mut current = first_customer;
        let mut visited = HashSet::new();
        let mut sequence = Vec::new();
        while current != end {
            if current == start || current == 0 || current >= end || !visited.insert(current) {
                return Err(
                    "direct-MIP 路线包含非法节点或子环 / direct-MIP route contains an invalid node or subtour"
                        .to_owned(),
                );
            }
            let customer = instance.customers.get(current - 1).ok_or_else(|| {
                format!(
                    "direct-MIP 客户索引无效：{} / direct-MIP customer index is invalid: {}",
                    current, current
                )
            })?;
            covered.insert(customer.id.clone());
            sequence.push(customer.id.clone());
            current = successors.get(&current).copied().ok_or_else(|| {
                format!(
                    "direct-MIP 路线在客户 {} 后中断 / direct-MIP route stops after customer {}",
                    customer.id, customer.id
                )
            })?;
        }
        sequences.push(sequence);
    }
    let expected = instance
        .customers
        .iter()
        .map(|customer| customer.id.clone())
        .collect::<HashSet<_>>();
    if covered != expected {
        return Err(
            "direct-MIP 路线未完整覆盖客户 / direct-MIP routes do not cover all customers"
                .to_owned(),
        );
    }
    Ok(sequences)
}

#[cfg(all(test, feature = "demo5-gurobi-bp"))]
mod tests {
    use super::{DirectMipPolicy, solve};
    use crate::framework::demo5::infrastructure::{first25_instance, native_solver_available};
    use ospf_rust_core::solver::solvers::GurobiSolver;
    use ospf_rust_framework_network_scheduling::domain::vrp::{
        DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator,
    };
    use std::collections::HashSet;

    #[test]
    #[ignore = "requires a native Gurobi installation; run with --include-ignored"]
    fn demo17_25_direct_mip_returns_a_complete_route_cover() {
        let instance = first25_instance().expect("first 25 Demo17 instance");
        let solver = GurobiSolver::new();
        assert!(
            native_solver_available(&solver),
            "Gurobi native environment is required for the direct-MIP gate"
        );
        let policy = DirectMipPolicy {
            distance_calculator: EuclideanDistanceCalculator,
            travel_time_calculator: DistanceAsTravelTimeCalculator::new(
                EuclideanDistanceCalculator,
            ),
            arc_cost_calculator: DistanceArcCostCalculator,
        };
        let result = solve(&instance, &solver, &policy).expect("direct-MIP solve");
        let served = result
            .customer_sequences
            .iter()
            .flatten()
            .cloned()
            .collect::<HashSet<_>>();
        assert_eq!(served.len(), 25);
        assert!(result.objective.is_finite());
        assert!(result.best_bound.is_some_and(f64::is_finite));
    }
}
