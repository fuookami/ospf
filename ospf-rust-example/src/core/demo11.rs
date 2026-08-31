use std::cell::Cell;
use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, LinearIntermediateSymbol, flat_map1,
};
use ospf_rust_core::variable::{
    UContinuous, VariableCombination1D, VariableCombination2D, VariableRange,
};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// Node data structure
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

/// Arc capacity data structure
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

/// Max flow data structure
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

/// Demo11 main function: Maximum flow problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let data = MaxFlowData::sample();
    let node_count = data.nodes.len();

    let mut model = MetaModel::<f64>::new("demo11");

    // Register 2D arc variables with capacity bounds
    let arc_vars = VariableCombination2D::<UContinuous>::with_name_and_range_generator(
        Shape::new([node_count, node_count]),
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
    let arc_idx = model.register_combination(&arc_vars)?;

    // Register 1D flow variable
    let flow_vars = VariableCombination1D::<UContinuous>::new(Shape::new([1]), "flow");
    let flow_idx = model.register_combination(&flow_vars)?;

    // Objective: maximize flow
    let flow_obj = flat_map1("flow_obj", &data.nodes, |_node| {
        ospf_rust_core::symbol::flatten::Linear::new(
            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, flow_idx[0])],
            0.0,
        )
    }, |i, _| format!("{}", i));
    model.add_symbol_combination(&flow_obj)?;

    let obj_poly = flow_obj.symbol_polynomial(0);
    let obj_coeffs: Vec<_> = obj_poly.monomials().iter()
        .map(|m| (m.var_index(), *m.coefficient())).collect();
    model.add_linear_objective(&obj_coeffs, "flow");
    model.set_objective_category(ObjectiveCategory::Maximum);

    // Flow out from each node: sum_j x[node][j]
    let counter = Cell::new(0usize);
    let flow_out_expr = flat_map1("flow_out", &data.nodes, |_node| {
        let node = counter.get();
        counter.set(node + 1);
        let monomials: Vec<_> = (0..node_count)
            .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, arc_idx[&[node, j]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |i, _| format!("{}", i));
    model.add_symbol_combination(&flow_out_expr)?;

    // Flow in to each node: sum_i x[i][node]
    let counter = Cell::new(0usize);
    let flow_in_expr = flat_map1("flow_in", &data.nodes, |_node| {
        let node = counter.get();
        counter.set(node + 1);
        let monomials: Vec<_> = (0..node_count)
            .map(|i| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, arc_idx[&[i, node]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |i, _| format!("{}", i));
    model.add_symbol_combination(&flow_in_expr)?;

    // Flow balance constraints per node
    for node in 0..node_count {
        let out_coeffs = extract_coeffs(&flow_out_expr[node]);
        let in_coeffs = extract_coeffs(&flow_in_expr[node]);

        // Combine: flow_out - flow_in [+/- flow] = 0
        let mut coeffs = out_coeffs;
        for (idx, coeff) in in_coeffs {
            coeffs.push((idx, -coeff));
        }

        if node == data.root {
            coeffs.push((flow_idx[0], -1.0));
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 0.0, "root_balance")?;
        } else if node == data.end {
            coeffs.push((flow_idx[0], 1.0));
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 0.0, "end_balance")?;
        } else {
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 0.0, &format!("balance_{}", node))?;
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo11 ===");
    println!("status: {:?}", output.status);
    println!("max flow: {:.2}", read_solution_value(&solution, flow_idx[0]));
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
