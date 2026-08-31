use std::error::Error;
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{UContinuous, VariableCombination2D, VariableRange};
use super::common::{read_solution_value, solve_typed};

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
    let x_shape = Shape::new([nodes.len(), nodes.len()]);
    let x_vars: VariableCombination2D<UContinuous> =
        VariableCombination2D::with_name_and_range_generator(
            x_shape.clone(),
            "x",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, vector| {
                if arcs
                    .iter()
                    .any(|arc| arc.from == vector[0] && arc.to == vector[1])
                {
                    VariableRange::with_lower(0.0)
                } else {
                    VariableRange::fixed(0.0)
                }
            },
        );
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<UContinuous, _>(x_vars.iter().cloned())?,
    );

    let mut cost_terms = Vec::with_capacity(arcs.len());
    for arc in &arcs {
        cost_terms.push(LinearMonomial::new(
            arc.unit_cost,
            x_vars[&[arc.from, arc.to]].to_owned_symbol(),
        ));
    }
    let cost = Linear::new(cost_terms, 0.0);
    let trans_out = MultiArrayBuilder::new_by(Shape::<1>::new([nodes.len()]), |_idx, vector| {
        let node = vector[0];
        Linear::new(
            (0..nodes.len())
                .map(|to| LinearMonomial::new(1.0, x_vars[&[node, to]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });
    let trans_in = MultiArrayBuilder::new_by(Shape::<1>::new([nodes.len()]), |_idx, vector| {
        let node = vector[0];
        Linear::new(
            (0..nodes.len())
                .map(|from| LinearMonomial::new(1.0, x_vars[&[from, node]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });

    model.set_math_linear_objective(cost, ObjectiveCategory::Minimum, "cost")?;

    for (node_idx, node) in nodes.iter().enumerate() {
        match node.kind {
            NodeType::Product(storage) => {
                model.add_math_inequality(
                    trans_out[node_idx].clone().le(storage),
                    &format!("product_out_{}", node_idx),
                );
            }
            NodeType::Sale(demand) => {
                model.add_math_inequality(
                    trans_in[node_idx].clone().ge(demand),
                    &format!("sale_in_{}", node_idx),
                );
            }
            NodeType::Distribution => {
                model.add_math_inequality(
                    (trans_out[node_idx].clone() - trans_in[node_idx].clone()).eq_to(0.0),
                    &format!("balance_{}", node_idx),
                );
            }
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo14 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for arc in arcs {
        let value = read_solution_value(&solution, x_idx[&[arc.from, arc.to]]);
        if value > 0.0 {
            println!(
                "{} -> {} = {:.2}",
                nodes[arc.from].name, nodes[arc.to].name, value
            );
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
