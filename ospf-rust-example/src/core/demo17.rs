use std::error::Error;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{

    Binary, UContinuous, VariableCombination2D, VariableCombination3D, VariableRange,
};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};

use super::common::{read_solution_value, solve_typed};

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
    let x_shape = Shape::new([nodes.len(), nodes.len(), data.vehicle_count]);
    let x_vars: VariableCombination3D<Binary> =
        VariableCombination3D::with_name_and_range_generator(
            x_shape.clone(),
            "x",
            |_index, vector| format!("{}_{}_{}", vector[0], vector[1], vector[2]),
            |_index, vector| {
                let n1 = vector[0];
                let n2 = vector[1];
                if nodes[n1].kind != NodeKind::End && nodes[n2].kind != NodeKind::Origin && n1 != n2
                {
                    VariableRange::bounded(0.0, 1.0)
                } else {
                    VariableRange::fixed(0.0)
                }
            },
        );
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<Binary, _>(x_vars.iter().cloned())?,
    );

    let s_shape = Shape::new([nodes.len(), data.vehicle_count]);
    let s_vars: VariableCombination2D<UContinuous> =
        VariableCombination2D::with_name_and_range_generator(
            s_shape.clone(),
            "s",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, vector| VariableRange::bounded(nodes[vector[0]].tw_lb, nodes[vector[0]].tw_ub),
        );
    model.register_variables::<UContinuous, _>(s_vars.iter().cloned())?;

    let mut vehicle_usage_cost_terms = Vec::new();
    for v in 0..data.vehicle_count {
        for n2 in 0..nodes.len() {
            vehicle_usage_cost_terms.push(LinearMonomial::new(
                data.fixed_used_cost,
                x_vars[&[0, n2, v]].to_owned_symbol(),
            ));
        }
    }
    let mut transportation_cost_terms = Vec::new();
    for n1 in 0..nodes.len() {
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                transportation_cost_terms.push(LinearMonomial::new(
                    dist(&nodes[n1], &nodes[n2]),
                    x_vars[&[n1, n2, v]].to_owned_symbol(),
                ));
            }
        }
    }
    let vehicle_usage_cost = Linear::new(vehicle_usage_cost_terms, 0.0);
    let transportation_cost = Linear::new(transportation_cost_terms, 0.0);
    let origin =
        MultiArrayBuilder::new_by(Shape::<1>::new([data.vehicle_count]), |_idx, vector| {
            let v = vector[0];
            Linear::new(
                (0..nodes.len())
                    .map(|n2| LinearMonomial::new(1.0, x_vars[&[0, n2, v]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        });
    let destination =
        MultiArrayBuilder::new_by(Shape::<1>::new([data.vehicle_count]), |_idx, vector| {
            let v = vector[0];
            Linear::new(
                (0..nodes.len())
                    .map(|n1| {
                        LinearMonomial::new(
                            1.0,
                            x_vars[&[n1, nodes.len() - 1, v]].to_owned_symbol(),
                        )
                    })
                    .collect(),
                0.0,
            )
        });
    let in_flow = MultiArrayBuilder::new_by(
        Shape::<2>::new([nodes.len(), data.vehicle_count]),
        |_idx, vector| {
            let n = vector[0];
            let v = vector[1];
            Linear::new(
                (0..nodes.len())
                    .map(|n1| LinearMonomial::new(1.0, x_vars[&[n1, n, v]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        },
    );
    let out_flow = MultiArrayBuilder::new_by(
        Shape::<2>::new([nodes.len(), data.vehicle_count]),
        |_idx, vector| {
            let n = vector[0];
            let v = vector[1];
            Linear::new(
                (0..nodes.len())
                    .map(|n2| LinearMonomial::new(1.0, x_vars[&[n, n2, v]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        },
    );
    let service = MultiArrayBuilder::new_by(Shape::<1>::new([nodes.len()]), |_idx, vector| {
        let n = vector[0];
        let mut terms = Vec::new();
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                terms.push(LinearMonomial::new(
                    1.0,
                    x_vars[&[n, n2, v]].to_owned_symbol(),
                ));
            }
        }
        Linear::new(terms, 0.0)
    });
    let capacity =
        MultiArrayBuilder::new_by(Shape::<1>::new([data.vehicle_count]), |_idx, vector| {
            let v = vector[0];
            let mut terms = Vec::new();
            for n2 in 0..nodes.len() {
                if nodes[n2].kind != NodeKind::Demand {
                    continue;
                }
                for n1 in 0..nodes.len() {
                    terms.push(LinearMonomial::new(
                        nodes[n2].demand,
                        x_vars[&[n1, n2, v]].to_owned_symbol(),
                    ));
                }
            }
            Linear::new(terms, 0.0)
        });

    model.set_math_linear_objective(
        vehicle_usage_cost + transportation_cost,
        ObjectiveCategory::Minimum,
        "cost",
    )?;

    for v in 0..data.vehicle_count {
        model.add_math_inequality(origin[v].clone().le(1.0), &format!("origin_{}", v));
        model.add_math_inequality(
            destination[v].clone().le(1.0),
            &format!("destination_{}", v),
        );
    }

    for (n, node) in nodes.iter().enumerate() {
        if node.kind != NodeKind::Demand {
            continue;
        }
        for v in 0..data.vehicle_count {
            model.add_math_inequality(
                (in_flow[&[n, v]].clone() - out_flow[&[n, v]].clone()).eq_to(0.0),
                &format!("flow_{}_{}", n, v),
            );
        }
    }

    for (n, node) in nodes.iter().enumerate() {
        if node.kind != NodeKind::Demand {
            continue;
        }
        model.add_math_inequality(service[n].clone().eq_to(1.0), &format!("service_{}", n));
    }

    for n1 in 0..nodes.len() {
        for n2 in 0..nodes.len() {
            for v in 0..data.vehicle_count {
                let time_link = Linear::new(
                    vec![
                        LinearMonomial::new(1.0, s_vars[&[n1, v]].to_owned_symbol()),
                        LinearMonomial::new(-1.0, s_vars[&[n2, v]].to_owned_symbol()),
                        LinearMonomial::new(data.big_m, x_vars[&[n1, n2, v]].to_owned_symbol()),
                    ],
                    0.0,
                );
                model.add_math_inequality(
                    time_link
                        .le(data.big_m - nodes[n1].service_time - dist(&nodes[n1], &nodes[n2])),
                    &format!("time_link_{}_{}_{}", n1, n2, v),
                );
            }
        }
    }

    for (n, node) in nodes.iter().enumerate() {
        for v in 0..data.vehicle_count {
            let service_time = Linear::new(
                vec![LinearMonomial::new(1.0, s_vars[&[n, v]].to_owned_symbol())],
                0.0,
            );
            model.add_math_inequality(
                service_time.clone().ge(node.tw_lb),
                &format!("time_lb_{}_{}", n, v),
            );
            model.add_math_inequality(service_time.le(node.tw_ub), &format!("time_ub_{}_{}", n, v));
        }
    }

    for v in 0..data.vehicle_count {
        model.add_math_inequality(
            capacity[v].clone().le(data.vehicle_capacity),
            &format!("capacity_{}", v),
        );
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo17 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for v in 0..data.vehicle_count {
        let mut used = false;
        for n2 in 0..nodes.len() {
            if read_solution_value(&solution, x_idx[&[0, n2, v]]) > 0.5 {
                used = true;
            }
        }
        if used {
            println!("vehicle {} used", v);
        }
        for n1 in 0..nodes.len() {
            for n2 in 0..nodes.len() {
                if read_solution_value(&solution, x_idx[&[n1, n2, v]]) > 0.5 {
                    println!("v{}: {} -> {}", v, n1, n2);
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
