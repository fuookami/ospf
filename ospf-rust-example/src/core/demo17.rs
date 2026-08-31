//! Demo17 模块 / Demo17 module
use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, SymbolCombination, flat_map1, flat_map1_indexed,
};
use ospf_rust_core::variable::{
    Binary, UContinuous, VariableCombination2D, VariableCombination3D, VariableRange,
};
use ospf_rust_multiarray::Shape;

use super::common::{extract_coeffs, read_solution_value, solve_typed};

/// 节点类型枚举 / Node kind enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    /// 起点 / Origin
    Origin,
    /// 需求点 / Demand point
    Demand,
    /// 终点 / End
    End,
}

/// 节点数据结构 / Node data structure
#[derive(Clone, Copy)]
struct Node {
    /// 节点类型 / Node kind
    kind: NodeKind,
    /// x 坐标 / x coordinate
    x: f64,
    /// y 坐标 / y coordinate
    y: f64,
    /// 需求量 / Demand
    demand: f64,
    /// 服务时间 / Service time
    service_time: f64,
    /// 时间窗下限 / Time window lower bound
    tw_lb: f64,
    /// 时间窗上限 / Time window upper bound
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

    /// 创建起点节点 / Create origin node
    fn origin(x: f64, y: f64, tw_lb: f64, tw_ub: f64) -> Self {
        Self::new(NodeKind::Origin, x, y, 0.0, 0.0, tw_lb, tw_ub)
    }

    /// 创建需求点节点 / Create demand node
    fn demand(x: f64, y: f64, demand: f64, service_time: f64, tw_lb: f64, tw_ub: f64) -> Self {
        Self::new(NodeKind::Demand, x, y, demand, service_time, tw_lb, tw_ub)
    }

    /// 创建终点节点 / Create end node
    fn end(x: f64, y: f64, tw_lb: f64, tw_ub: f64) -> Self {
        Self::new(NodeKind::End, x, y, 0.0, 0.0, tw_lb, tw_ub)
    }
}

/// VRP 数据结构 / VRP data structure
#[derive(Clone)]
struct VrpData {
    /// 节点列表 / Node list
    nodes: Vec<Node>,
    /// 车辆数量 / Vehicle count
    vehicle_count: usize,
    /// 车辆容量 / Vehicle capacity
    vehicle_capacity: f64,
    /// 固定使用成本 / Fixed used cost
    fixed_used_cost: f64,
    /// 大 M 常数 / Big M constant
    big_m: f64,
}

impl VrpData {
    /// 示例数据 / Sample data
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

/// 计算两点间欧几里得距离 / Calculate Euclidean distance between two points
fn dist(a: &Node, b: &Node) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// 带时间窗的车辆路径问题模型 / VRPTW model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct VehicleRoutingModel {
    /// 路径决策变量 / Route decision variables
    x_vars: VariableCombination3D<Binary>,
    /// 路径模型索引数组 / Route model index array
    x_idx: ospf_rust_multiarray::MultiArray<usize, Shape<3>>,
    /// 时间决策变量 / Time decision variables
    s_vars: VariableCombination2D<UContinuous>,
    /// 时间模型索引数组 / Time model index array
    s_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 车辆使用成本符号 / Vehicle usage cost symbol
    vehicle_usage_cost: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 运输成本符号 / Transportation cost symbol
    transportation_cost: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 起点流符号 / Origin flow symbol
    origin: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 终点流符号 / Destination flow symbol
    destination: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 流入符号 / In-flow symbol
    in_flow: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 流出符号 / Out-flow symbol
    out_flow: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 服务符号 / Service symbol
    service: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 容量符号 / Capacity symbol
    capacity: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl VehicleRoutingModel {
    /// 注册模型 / Register model
    fn register(model: &mut MetaModel<f64>, data: &VrpData) -> Result<Self, Box<dyn Error>> {
        let nodes = &data.nodes;
        let node_count = nodes.len();
        let vc = data.vehicle_count;

        // Find origin/end node indices by kind (replaces hardcoded index assumptions)
        let origin_idx = nodes
            .iter()
            .position(|n| n.kind == NodeKind::Origin)
            .expect("origin node must exist");
        let end_idx = nodes
            .iter()
            .position(|n| n.kind == NodeKind::End)
            .expect("end node must exist");

        // 1. 注册 x 变量组合 (node1 x node2 x vehicle)
        let x_shape = Shape::new([node_count, node_count, vc]);
        let x_vars: VariableCombination3D<Binary> =
            VariableCombination3D::with_name_and_range_generator(
                x_shape.clone(),
                "x",
                |_index, vector| format!("{}_{}_{}", vector[0], vector[1], vector[2]),
                |_index, vector| {
                    let n1 = vector[0];
                    let n2 = vector[1];
                    if nodes[n1].kind != NodeKind::End
                        && nodes[n2].kind != NodeKind::Origin
                        && n1 != n2
                    {
                        VariableRange::bounded(0.0, 1.0)
                    } else {
                        VariableRange::fixed(0.0)
                    }
                },
            );
        let x_idx = model.register_combination(&x_vars)?;

        // 2. 注册 s 变量组合 (node x vehicle)
        let s_shape = Shape::new([node_count, vc]);
        let s_vars: VariableCombination2D<UContinuous> =
            VariableCombination2D::with_name_and_range_generator(
                s_shape.clone(),
                "s",
                |_index, vector| format!("{}_{}", vector[0], vector[1]),
                |_index, vector| {
                    VariableRange::bounded(nodes[vector[0]].tw_lb, nodes[vector[0]].tw_ub)
                },
            );
        let s_idx = model.register_combination(&s_vars)?;

        // 3. 构建车辆使用成本符号 / Vehicle usage cost symbol
        let vehicle_usage_cost = flat_map1(
            "vehicle_usage_cost",
            &(0..vc).collect::<Vec<_>>(),
            |&v| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|n2| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            data.fixed_used_cost,
                            x_idx[&[origin_idx, n2, v]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, v| format!("{}", v),
        );
        model.add_symbol_combination(&vehicle_usage_cost)?;

        // 4. 构建运输成本符号 / Transportation cost symbol
        let x_idx_ref = &x_idx;
        let transportation_cost = flat_map1(
            "transportation_cost",
            &(0..vc).collect::<Vec<_>>(),
            |&v| {
                let monomials: Vec<_> = (0..node_count)
                    .flat_map(|n1| {
                        (0..node_count).map(move |n2| {
                            ospf_rust_core::symbol::flatten::LinearMonomial::new(
                                dist(&nodes[n1], &nodes[n2]),
                                x_idx_ref[&[n1, n2, v]],
                            )
                        })
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, v| format!("{}", v),
        );
        model.add_symbol_combination(&transportation_cost)?;

        // 5. 构建起点/终点流符号 / Origin/destination flow symbols
        // origin[v] = sum over nodes where kind == OriginNode of x[origin, n2, v]
        let origin = flat_map1(
            "origin",
            &(0..vc).collect::<Vec<_>>(),
            |&v| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|n2| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            1.0,
                            x_idx[&[origin_idx, n2, v]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, v| format!("{}", v),
        );
        model.add_symbol_combination(&origin)?;

        // destination[v] = sum over nodes where kind == EndNode of x[n1, end, v]
        let destination = flat_map1(
            "destination",
            &(0..vc).collect::<Vec<_>>(),
            |&v| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|n1| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            1.0,
                            x_idx[&[n1, end_idx, v]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, v| format!("{}", v),
        );
        model.add_symbol_combination(&destination)?;

        // 6. 构建流入/流出流符号 / In-flow/out-flow symbols
        // Kotlin: inFlow/outFlow only for non-Origin, non-End nodes (filterIsNotInstance)
        // 只为普通节点构建 in_flow/out_flow，排除 Origin 和 End 节点
        let regular_indices: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind != NodeKind::Origin && n.kind != NodeKind::End)
            .map(|(i, _)| i)
            .collect();
        let in_flow_keys: Vec<(usize, usize)> = regular_indices
            .iter()
            .flat_map(|&n| (0..vc).map(move |v| (n, v)))
            .collect();
        let in_flow = flat_map1_indexed(
            "in_flow",
            &in_flow_keys,
            |_, &(n, v)| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|n1| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            1.0,
                            x_idx_ref[&[n1, n, v]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, &(n, v)| format!("{}_{}", n, v),
        );
        model.add_symbol_combination(&in_flow)?;

        let out_flow_keys: Vec<(usize, usize)> = regular_indices
            .iter()
            .flat_map(|&n| (0..vc).map(move |v| (n, v)))
            .collect();
        let out_flow = flat_map1_indexed(
            "out_flow",
            &out_flow_keys,
            |_, &(n, v)| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|n2| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            1.0,
                            x_idx_ref[&[n, n2, v]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, &(n, v)| format!("{}_{}", n, v),
        );
        model.add_symbol_combination(&out_flow)?;

        // 7. 构建服务符号 / Service symbol
        // service[n] = sum over (n2, v) of x[n, n2, v] for nodes where kind != OriginNode
        // (matches Kotlin: filterIsInstance excluding OriginNode)
        let non_origin_indices: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind != NodeKind::Origin)
            .map(|(i, _)| i)
            .collect();
        let service = flat_map1(
            "service",
            &non_origin_indices,
            |&n| {
                let monomials: Vec<_> = (0..node_count)
                    .flat_map(|n2| {
                        (0..vc).map(move |v| {
                            ospf_rust_core::symbol::flatten::LinearMonomial::new(
                                1.0,
                                x_idx_ref[&[n, n2, v]],
                            )
                        })
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, &n| format!("{:?}", nodes[n].kind),
        );
        model.add_symbol_combination(&service)?;

        // 8. 构建容量符号 / Capacity symbol
        let capacity = flat_map1(
            "capacity",
            &(0..vc).collect::<Vec<_>>(),
            |&v| {
                let mut monomials = Vec::new();
                for n2 in 0..node_count {
                    if nodes[n2].kind != NodeKind::Demand {
                        continue;
                    }
                    for n1 in 0..node_count {
                        monomials.push(ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            nodes[n2].demand,
                            x_idx_ref[&[n1, n2, v]],
                        ));
                    }
                }
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, v| format!("{}", v),
        );
        model.add_symbol_combination(&capacity)?;

        Ok(VehicleRoutingModel {
            x_vars,
            x_idx,
            s_vars,
            s_idx,
            vehicle_usage_cost,
            transportation_cost,
            origin,
            destination,
            in_flow,
            out_flow,
            service,
            capacity,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        data: &VrpData,
    ) -> Result<(), Box<dyn Error>> {
        let nodes = &data.nodes;
        let node_count = nodes.len();
        let vc = data.vehicle_count;

        // 9. 目标: 最小化车辆使用成本 + 运输成本
        let mut obj_coeffs: Vec<(usize, f64)> = Vec::new();
        for v in 0..vc {
            let vuc_poly = self.vehicle_usage_cost.symbol_polynomial(v);
            for m in vuc_poly.monomials() {
                obj_coeffs.push((m.var_index(), *m.coefficient()));
            }
            let tc_poly = self.transportation_cost.symbol_polynomial(v);
            for m in tc_poly.monomials() {
                obj_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_objective(&obj_coeffs, "cost");
        model.set_objective_category(ObjectiveCategory::Minimum);

        // 10. 起点约束: origin[v] <= 1
        for v in 0..vc {
            let coeffs = extract_coeffs(&self.origin[v]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("origin_{}", v),
            )?;
        }

        // 11. 终点约束: destination[v] <= 1
        for v in 0..vc {
            let coeffs = extract_coeffs(&self.destination[v]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("destination_{}", v),
            )?;
        }

        // 12. 流量守恒约束: in_flow[n,v] - out_flow[n,v] = 0
        // in_flow/out_flow indexed by position in regular_indices * vc
        let regular_indices: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind != NodeKind::Origin && n.kind != NodeKind::End)
            .map(|(i, _)| i)
            .collect();
        for (flow_idx, &n) in regular_indices.iter().enumerate() {
            if nodes[n].kind != NodeKind::Demand {
                continue;
            }
            for v in 0..vc {
                let key_idx = flow_idx * vc + v;
                let in_coeffs = extract_coeffs(&self.in_flow[key_idx]);
                let out_coeffs = extract_coeffs(&self.out_flow[key_idx]);
                let mut coeffs = in_coeffs;
                for (var_idx, coeff) in out_coeffs {
                    coeffs.push((var_idx, -coeff));
                }
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::Equal,
                    0.0,
                    &format!("flow_{}_{}", n, v),
                )?;
            }
        }

        // 13. 服务约束: service[n] = 1 (仅需求点)
        // service symbol is indexed over non-origin nodes; iterate to find demand nodes
        let non_origin_indices: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind != NodeKind::Origin)
            .map(|(i, _)| i)
            .collect();
        for (svc_idx, &node_idx) in non_origin_indices.iter().enumerate() {
            if nodes[node_idx].kind != NodeKind::Demand {
                continue;
            }
            let coeffs = extract_coeffs(&self.service[svc_idx]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::Equal,
                1.0,
                &format!("service_{}", node_idx),
            )?;
        }

        // 14. 时间链接约束: s[n1,v] - s[n2,v] + M*x[n1,n2,v] <= M - service_time - dist
        for n1 in 0..node_count {
            for n2 in 0..node_count {
                for v in 0..vc {
                    let coeffs = vec![
                        (self.s_idx[&[n1, v]], 1.0),
                        (self.s_idx[&[n2, v]], -1.0),
                        (self.x_idx[&[n1, n2, v]], data.big_m),
                    ];
                    let rhs = data.big_m - nodes[n1].service_time - dist(&nodes[n1], &nodes[n2]);
                    model.add_linear_constraint(
                        &coeffs,
                        ConstraintRelation::LessEqual,
                        rhs,
                        &format!("time_link_{}_{}_{}", n1, n2, v),
                    )?;
                }
            }
        }

        // 15. 时间窗约束: s[n,v] >= tw_lb, s[n,v] <= tw_ub
        for (n, node) in nodes.iter().enumerate() {
            for v in 0..vc {
                model.add_linear_constraint(
                    &[(self.s_idx[&[n, v]], 1.0)],
                    ConstraintRelation::GreaterEqual,
                    node.tw_lb,
                    &format!("time_lb_{}_{}", n, v),
                )?;
                model.add_linear_constraint(
                    &[(self.s_idx[&[n, v]], 1.0)],
                    ConstraintRelation::LessEqual,
                    node.tw_ub,
                    &format!("time_ub_{}_{}", n, v),
                )?;
            }
        }

        // 16. 容量约束: capacity[v] <= vehicle_capacity
        for v in 0..vc {
            let coeffs = extract_coeffs(&self.capacity[v]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                data.vehicle_capacity,
                &format!("capacity_{}", v),
            )?;
        }

        Ok(())
    }
}

/// Demo17 主函数：带时间窗的车辆路径问题（VRPTW）
/// Demo17 main function: Vehicle Routing Problem with Time Windows (VRPTW)
pub fn run() -> Result<(), Box<dyn Error>> {
    let data = VrpData::sample();
    let nodes = &data.nodes;
    let node_count = nodes.len();
    let vc = data.vehicle_count;

    let mut model = MetaModel::<f64>::new("demo17");
    let routing = VehicleRoutingModel::register(&mut model, &data)?;

    routing.add_constraints(&mut model, &data)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    // Find origin node index by kind for output
    let origin_idx = nodes
        .iter()
        .position(|n| n.kind == NodeKind::Origin)
        .expect("origin node must exist");

    println!("=== Demo17 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for v in 0..vc {
        let mut used = false;
        for n2 in 0..node_count {
            if read_solution_value(&solution, routing.x_idx[&[origin_idx, n2, v]]) > 0.5 {
                used = true;
            }
        }
        if used {
            println!("vehicle {} used", v);
        }
        for n1 in 0..node_count {
            for n2 in 0..node_count {
                if read_solution_value(&solution, routing.x_idx[&[n1, n2, v]]) > 0.5 {
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
