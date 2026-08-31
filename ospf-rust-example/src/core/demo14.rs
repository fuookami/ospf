use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UContinuousVariableItem;

use super::common::{read_solution_value, solve};

#[derive(Clone, Copy)]
enum NodeType {
    Product(f64),
    Sale(f64),
    Distribution,
}

#[derive(Clone)]
struct Node {
    name: String,
    kind: NodeType,
}

impl Node {
    fn new(name: &str, kind: NodeType) -> Self {
        Self {
            name: name.to_string(),
            kind,
        }
    }
}

#[derive(Clone)]
struct ArcData {
    from: usize,
    to: usize,
    unit_cost: f64,
}

impl ArcData {
    fn new(from: usize, to: usize, unit_cost: f64) -> Self {
        Self {
            from,
            to,
            unit_cost,
        }
    }
}

fn build_nodes() -> Vec<Node> {
    vec![
        Node::new("Guangzhou", NodeType::Product(600.0)),
        Node::new("Dalian", NodeType::Product(400.0)),
        Node::new("Nanjing", NodeType::Sale(200.0)),
        Node::new("Jinan", NodeType::Sale(150.0)),
        Node::new("Nanchang", NodeType::Sale(350.0)),
        Node::new("Qingdao", NodeType::Sale(300.0)),
        Node::new("Shanghai", NodeType::Distribution),
        Node::new("Tianjin", NodeType::Distribution),
    ]
}

fn build_arcs() -> Vec<ArcData> {
    vec![
        ArcData::new(0, 6, 2.0),
        ArcData::new(0, 7, 3.0),
        ArcData::new(1, 5, 4.0),
        ArcData::new(1, 6, 3.0),
        ArcData::new(1, 7, 1.0),
        ArcData::new(6, 2, 2.0),
        ArcData::new(6, 3, 6.0),
        ArcData::new(6, 4, 3.0),
        ArcData::new(6, 5, 6.0),
        ArcData::new(7, 2, 4.0),
        ArcData::new(7, 3, 4.0),
        ArcData::new(7, 4, 6.0),
        ArcData::new(7, 5, 5.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let nodes = build_nodes();
    let arcs = build_arcs();

    let mut model = MetaModel::<f64>::new("demo14");
    let mut x_idx = vec![vec![None; nodes.len()]; nodes.len()];
    for arc in &arcs {
        let var = UContinuousVariableItem::auto(&format!("x_{}_{}", arc.from, arc.to));
        x_idx[arc.from][arc.to] = Some(model.register_variable(var)?);
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for arc in &arcs {
        if let Some(idx) = x_idx[arc.from][arc.to] {
            objective[idx] = arc.unit_cost;
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for (node_idx, node) in nodes.iter().enumerate() {
        match node.kind {
            NodeType::Product(storage) => {
                let out_coeffs: Vec<(usize, f64)> = (0..nodes.len())
                    .filter_map(|to| x_idx[node_idx][to].map(|idx| (idx, 1.0)))
                    .collect();
                model.add_linear_constraint(
                    &out_coeffs,
                    ConstraintRelation::LessEqual,
                    storage,
                    &format!("product_out_{}", node_idx),
                )?;
            }
            NodeType::Sale(demand) => {
                let in_coeffs: Vec<(usize, f64)> = (0..nodes.len())
                    .filter_map(|from| x_idx[from][node_idx].map(|idx| (idx, 1.0)))
                    .collect();
                model.add_linear_constraint(
                    &in_coeffs,
                    ConstraintRelation::GreaterEqual,
                    demand,
                    &format!("sale_in_{}", node_idx),
                )?;
            }
            NodeType::Distribution => {
                let mut coeffs: Vec<(usize, f64)> = (0..nodes.len())
                    .filter_map(|to| x_idx[node_idx][to].map(|idx| (idx, 1.0)))
                    .collect();
                coeffs.extend(
                    (0..nodes.len())
                        .filter_map(|from| x_idx[from][node_idx].map(|idx| (idx, -1.0))),
                );
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::Equal,
                    0.0,
                    &format!("balance_{}", node_idx),
                )?;
            }
        }
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo14 has no feasible solution"))?;

    println!("=== Demo14 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for arc in arcs {
        if let Some(idx) = x_idx[arc.from][arc.to] {
            let value = read_solution_value(&solution, idx);
            if value > 0.0 {
                println!("{} -> {} = {:.2}", nodes[arc.from].name, nodes[arc.to].name, value);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo14() {
        assert!(run().is_ok());
    }
}
