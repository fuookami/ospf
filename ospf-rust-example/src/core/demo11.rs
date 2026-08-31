use std::error::Error;

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{
    UContinuous, UContinuousVariableItem, VariableCombination2D, VariableRange,
};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};

use super::common::{read_solution_value, solve_typed};

#[derive(Debug, Clone)]
struct Node {
    name: String,
}

impl Node {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct ArcCapacity {
    from: usize,
    to: usize,
    capacity: f64,
}

impl ArcCapacity {
    fn new(from: usize, to: usize, capacity: f64) -> Self {
        Self { from, to, capacity }
    }
}

#[derive(Debug, Clone)]
struct MaxFlowData {
    nodes: Vec<Node>,
    root: usize,
    end: usize,
    capacities: Vec<ArcCapacity>,
}

impl MaxFlowData {
    fn sample() -> Self {
        Self {
            nodes: (0..9).map(|i| Node::new(&format!("N{}", i))).collect(),
            root: 0,
            end: 8,
            capacities: vec![
                ArcCapacity::new(0, 1, 15.0),
                ArcCapacity::new(0, 2, 10.0),
                ArcCapacity::new(0, 3, 40.0),
                ArcCapacity::new(1, 4, 15.0),
                ArcCapacity::new(2, 5, 10.0),
                ArcCapacity::new(2, 6, 35.0),
                ArcCapacity::new(3, 6, 30.0),
                ArcCapacity::new(3, 7, 20.0),
                ArcCapacity::new(4, 6, 10.0),
                ArcCapacity::new(5, 8, 10.0),
                ArcCapacity::new(6, 7, 10.0),
                ArcCapacity::new(7, 8, 45.0),
            ],
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let data = MaxFlowData::sample();
    let node_count = data.nodes.len();

    let mut model = MetaModel::<f64>::new("demo11");
    let arc_shape = Shape::new([node_count, node_count]);
    let arc_vars: VariableCombination2D<UContinuous> =
        VariableCombination2D::with_name_and_range_generator(
            arc_shape.clone(),
            "x",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, vector| {
                data.capacities
                    .iter()
                    .find(|arc| arc.from == vector[0] && arc.to == vector[1])
                    .map(|arc| VariableRange::bounded(0.0, arc.capacity))
                    .unwrap_or_else(|| VariableRange::fixed(0.0))
            },
        );
    let arc_idx = MultiArrayBuilder::from_list(
        arc_shape,
        model.register_variables::<UContinuous, _>(arc_vars.iter().cloned())?,
    );
    let flow_var = UContinuousVariableItem::auto("flow");
    let flow_idx = model.register_variable(flow_var.clone())?;

    let flow = Linear::new(
        vec![LinearMonomial::new(1.0, flow_var.to_owned_symbol())],
        0.0,
    );
    let flow_out = MultiArrayBuilder::new_by(Shape::<1>::new([node_count]), |_idx, vector| {
        let node = vector[0];
        Linear::new(
            (0..node_count)
                .map(|j| LinearMonomial::new(1.0, arc_vars[&[node, j]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });
    let flow_in = MultiArrayBuilder::new_by(Shape::<1>::new([node_count]), |_idx, vector| {
        let node = vector[0];
        Linear::new(
            (0..node_count)
                .map(|i| LinearMonomial::new(1.0, arc_vars[&[i, node]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });

    model.set_math_linear_objective(flow.clone(), ObjectiveCategory::Maximum, "flow")?;

    for node in 0..node_count {
        let balance = flow_out[node].clone() - flow_in[node].clone();
        if node == data.root {
            model.add_math_inequality((balance - flow.clone()).eq_to(0.0), "root_balance");
        } else if node == data.end {
            model.add_math_inequality((balance + flow.clone()).eq_to(0.0), "end_balance");
        } else {
            model.add_math_inequality(balance.eq_to(0.0), &format!("balance_{}", node));
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo11 ===");
    println!("status: {:?}", output.status);
    println!("max flow: {:.2}", read_solution_value(&solution, flow_idx));
    for arc in &data.capacities {
        let value = read_solution_value(&solution, arc_idx[&[arc.from, arc.to]]);
        if value > 0.0 {
            println!(
                "{} -> {} = {:.2}",
                data.nodes[arc.from].name, data.nodes[arc.to].name, value
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo11() {
        assert!(run().is_ok());
    }
}
