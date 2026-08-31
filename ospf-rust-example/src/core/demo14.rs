use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, LinearIntermediateSymbol, flat_map1,
};
use ospf_rust_core::variable::{UContinuous, VariableCombination2D, VariableRange};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// Node type enum
#[derive(Clone, Copy)]
enum NodeType {
    Product(f64),
    Sale(f64),
    Distribution,
}

/// Node data structure
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

/// Arc data structure
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

/// Build node list
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

/// Build arc list
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

/// Demo14 main function: Transshipment problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let nodes = build_nodes();
    let arcs = build_arcs();

    let mut model = MetaModel::<f64>::new("demo14");

    // Register 2D arc variables with bounds
    let x_shape = Shape::new([nodes.len(), nodes.len()]);
    let x_vars: VariableCombination2D<UContinuous> =
        VariableCombination2D::with_name_and_range_generator(
            x_shape,
            "x",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, vector| {
                if arcs.iter().any(|arc| arc.from == vector[0] && arc.to == vector[1]) {
                    VariableRange::with_lower(0.0)
                } else {
                    VariableRange::fixed(0.0)
                }
            },
        );
    let x_idx = model.register_combination(&x_vars)?;

    // Objective: minimize cost = sum(unit_cost * x[from][to]) over arcs
    let cost_expr = flat_map1("cost", &arcs, |arc| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                arc.unit_cost,
                x_idx[&[arc.from, arc.to]],
            )],
            0.0,
        )
    }, |_, arc| format!("{}_{}", arc.from, arc.to));
    model.add_symbol_combination(&cost_expr)?;

    let mut cost_coeffs = Vec::new();
    for i in 0..arcs.len() {
        let poly = cost_expr.symbol_polynomial(i);
        for m in poly.monomials() {
            cost_coeffs.push((m.var_index(), *m.coefficient()));
        }
    }
    model.add_linear_objective(&cost_coeffs, "cost");
    model.set_objective_category(ObjectiveCategory::Minimum);

    // Flow out from each node: sum_to x[node][to]
    let node_indices: Vec<usize> = (0..nodes.len()).collect();
    let trans_out_expr = flat_map1("trans_out", &node_indices, |&node| {
        let monomials: Vec<_> = (0..nodes.len())
            .map(|to| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[node, to]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, &node| format!("{}", node));
    model.add_symbol_combination(&trans_out_expr)?;

    // Flow in to each node: sum_from x[from][node]
    let trans_in_expr = flat_map1("trans_in", &node_indices, |&node| {
        let monomials: Vec<_> = (0..nodes.len())
            .map(|from| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[from, node]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, &node| format!("{}", node));
    model.add_symbol_combination(&trans_in_expr)?;

    // Node balance constraints
    for (node_idx, node) in nodes.iter().enumerate() {
        let out_coeffs = extract_coeffs(&trans_out_expr[node_idx]);
        let in_coeffs = extract_coeffs(&trans_in_expr[node_idx]);

        match node.kind {
            NodeType::Product(storage) => {
                // flow_out <= storage
                model.add_linear_constraint(
                    &out_coeffs,
                    ConstraintRelation::LessEqual,
                    storage,
                    &format!("product_out_{}", node_idx),
                )?;
            }
            NodeType::Sale(demand) => {
                // flow_in >= demand
                model.add_linear_constraint(
                    &in_coeffs,
                    ConstraintRelation::GreaterEqual,
                    demand,
                    &format!("sale_in_{}", node_idx),
                )?;
            }
            NodeType::Distribution => {
                // flow_out - flow_in = 0
                let mut coeffs = out_coeffs;
                for (idx, coeff) in in_coeffs {
                    coeffs.push((idx, -coeff));
                }
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::Equal,
                    0.0,
                    &format!("balance_{}", node_idx),
                )?;
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
