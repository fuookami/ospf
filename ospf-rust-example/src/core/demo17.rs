use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem};

use super::common::{read_solution_value, solve};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Origin,
    Demand,
    End,
}

#[derive(Clone, Copy)]
struct Node {
    kind: NodeKind,
    x: f64,
    y: f64,
    demand: f64,
    service_time: f64,
    tw_lb: f64,
    tw_ub: f64,
}

impl Node {
    fn new(
        kind: NodeKind,
        x: f64,
        y: f64,
        demand: f64,
        service_time: f64,
        tw_lb: f64,
        tw_ub: f64,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            demand,
            service_time,
            tw_lb,
            tw_ub,
        }
    }

    fn origin(x: f64, y: f64, tw_lb: f64, tw_ub: f64) -> Self {
        Self::new(NodeKind::Origin, x, y, 0.0, 0.0, tw_lb, tw_ub)
    }

    fn demand(x: f64, y: f64, demand: f64, service_time: f64, tw_lb: f64, tw_ub: f64) -> Self {
        Self::new(NodeKind::Demand, x, y, demand, service_time, tw_lb, tw_ub)
    }

    fn end(x: f64, y: f64, tw_lb: f64, tw_ub: f64) -> Self {
        Self::new(NodeKind::End, x, y, 0.0, 0.0, tw_lb, tw_ub)
    }
}

#[derive(Clone)]
struct VrpData {
    nodes: Vec<Node>,
    vehicle_count: usize,
    vehicle_capacity: f64,
    fixed_used_cost: f64,
    big_m: f64,
}

impl VrpData {
    fn sample() -> Self {
        Self {
            nodes: vec![
                Node::origin(40.0, 50.0, 0.0, 400.0),
                Node::demand(45.0, 68.0, 10.0, 15.0, 30.0, 130.0),
                Node::demand(42.0, 66.0, 12.0, 12.0, 60.0, 180.0),
                Node::demand(35.0, 69.0, 15.0, 20.0, 90.0, 220.0),
                Node::demand(30.0, 52.0, 9.0, 10.0, 120.0, 260.0),
                Node::end(40.0, 50.0, 0.0, 400.0),
            ],
            vehicle_count: 3,
            vehicle_capacity: 25.0,
            fixed_used_cost: 100.0,
            big_m: 500.0,
        }
    }
}

fn dist(a: &Node, b: &Node) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let data = VrpData::sample();
    let nodes = &data.nodes;

    let mut model = MetaModel::<f64>::new("demo17");
    let mut x_idx = vec![vec![vec![None; data.vehicle_count]; nodes.len()]; nodes.len()];
    let mut s_idx = vec![vec![0usize; data.vehicle_count]; nodes.len()];

    for n1 in 0..nodes.len() {
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                if nodes[n1].kind != NodeKind::End && nodes[n2].kind != NodeKind::Origin && n1 != n2
                {
                    let var = BinaryVariableItem::auto(&format!("x_{}_{}_{}", n1, n2, v));
                    x_idx[n1][n2][v] = Some(model.register_variable(var)?);
                }
            }
        }
    }

    for n in 0..nodes.len() {
        for v in 0..data.vehicle_count {
            let var = UContinuousVariableItem::auto(&format!("s_{}_{}", n, v));
            s_idx[n][v] = model.register_variable(var)?;
        }
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for v in 0..data.vehicle_count {
        let origin_flow: Vec<usize> = (0..nodes.len()).filter_map(|n2| x_idx[0][n2][v]).collect();
        for idx in origin_flow {
            objective[idx] += data.fixed_used_cost;
        }
    }
    for n1 in 0..nodes.len() {
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                if let Some(idx) = x_idx[n1][n2][v] {
                    objective[idx] += dist(&nodes[n1], &nodes[n2]);
                }
            }
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for v in 0..data.vehicle_count {
        let origin_coeffs: Vec<(usize, f64)> = (0..nodes.len())
            .filter_map(|n2| x_idx[0][n2][v].map(|idx| (idx, 1.0)))
            .collect();
        model.add_linear_constraint(
            &origin_coeffs,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("origin_{}", v),
        )?;

        let destination_coeffs: Vec<(usize, f64)> = (0..nodes.len())
            .filter_map(|n1| x_idx[n1][nodes.len() - 1][v].map(|idx| (idx, 1.0)))
            .collect();
        model.add_linear_constraint(
            &destination_coeffs,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("destination_{}", v),
        )?;
    }

    for (n, node) in nodes.iter().enumerate() {
        if node.kind != NodeKind::Demand {
            continue;
        }
        for v in 0..data.vehicle_count {
            let in_coeffs: Vec<(usize, f64)> = (0..nodes.len())
                .filter_map(|n1| x_idx[n1][n][v].map(|idx| (idx, 1.0)))
                .collect();
            let out_coeffs: Vec<(usize, f64)> = (0..nodes.len())
                .filter_map(|n2| x_idx[n][n2][v].map(|idx| (idx, -1.0)))
                .collect();
            let mut coeffs = in_coeffs;
            coeffs.extend(out_coeffs);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::Equal,
                0.0,
                &format!("flow_{}_{}", n, v),
            )?;
        }
    }

    for (n, node) in nodes.iter().enumerate() {
        if node.kind != NodeKind::Demand {
            continue;
        }
        let mut coefficients: Vec<(usize, f64)> = Vec::new();
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                if let Some(idx) = x_idx[n][n2][v] {
                    coefficients.push((idx, 1.0));
                }
            }
        }
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!("service_{}", n),
        )?;
    }

    for n1 in 0..nodes.len() {
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                if let Some(x) = x_idx[n1][n2][v] {
                    model.add_linear_constraint(
                        &[(s_idx[n1][v], 1.0), (s_idx[n2][v], -1.0), (x, data.big_m)],
                        ConstraintRelation::LessEqual,
                        data.big_m - nodes[n1].service_time - dist(&nodes[n1], &nodes[n2]),
                        &format!("time_link_{}_{}_{}", n1, n2, v),
                    )?;
                }
            }
        }
    }

    for (n, node) in nodes.iter().enumerate() {
        for v in 0..data.vehicle_count {
            model.add_linear_constraint(
                &[(s_idx[n][v], 1.0)],
                ConstraintRelation::GreaterEqual,
                node.tw_lb,
                &format!("time_lb_{}_{}", n, v),
            )?;
            model.add_linear_constraint(
                &[(s_idx[n][v], 1.0)],
                ConstraintRelation::LessEqual,
                node.tw_ub,
                &format!("time_ub_{}_{}", n, v),
            )?;
        }
    }

    for v in 0..data.vehicle_count {
        let mut coefficients: Vec<(usize, f64)> = Vec::new();
        for n2 in 0..nodes.len() {
            if nodes[n2].kind != NodeKind::Demand {
                continue;
            }
            for n1 in 0..nodes.len() {
                if let Some(idx) = x_idx[n1][n2][v] {
                    coefficients.push((idx, nodes[n2].demand));
                }
            }
        }
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            data.vehicle_capacity,
            &format!("capacity_{}", v),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo17 has no feasible solution"))?;

    println!("=== Demo17 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for v in 0..data.vehicle_count {
        let mut used = false;
        for n2 in 0..nodes.len() {
            if let Some(idx) = x_idx[0][n2][v] {
                if read_solution_value(&solution, idx) > 0.5 {
                    used = true;
                }
            }
        }
        if used {
            println!("vehicle {} used", v);
        }
        for n1 in 0..nodes.len() {
            for n2 in 0..nodes.len() {
                if let Some(idx) = x_idx[n1][n2][v] {
                    if read_solution_value(&solution, idx) > 0.5 {
                        println!("v{}: {} -> {}", v, n1, n2);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo17() {
        assert!(run().is_ok());
    }
}
