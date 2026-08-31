use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UContinuousVariableItem;

use super::common::{read_solution_value, solve};

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
    let mut arc_idx = vec![vec![None; node_count]; node_count];
    for arc in &data.capacities {
        let var = UContinuousVariableItem::auto(&format!("x_{}_{}", arc.from, arc.to));
        let idx = model.register_variable(var)?;
        arc_idx[arc.from][arc.to] = Some(idx);
        model.add_linear_constraint(
            &[(idx, 1.0)],
            ConstraintRelation::LessEqual,
            arc.capacity,
            &format!("cap_{}_{}", arc.from, arc.to),
        )?;
    }
    let flow_idx = model.register_variable(UContinuousVariableItem::auto("flow"))?;

    let mut objective = vec![0.0; model.num_tokens()];
    objective[flow_idx] = 1.0;
    model.set_linear_objective(objective, ObjectiveCategory::Maximum);

    for node in 0..node_count {
        let mut coeffs: Vec<(usize, f64)> = Vec::new();
        for j in 0..node_count {
            if let Some(idx) = arc_idx[node][j] {
                coeffs.push((idx, 1.0));
            }
        }
        for i in 0..node_count {
            if let Some(idx) = arc_idx[i][node] {
                coeffs.push((idx, -1.0));
            }
        }

        if node == data.root {
            coeffs.push((flow_idx, -1.0));
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 0.0, "root_balance")?;
        } else if node == data.end {
            coeffs.push((flow_idx, 1.0));
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 0.0, "end_balance")?;
        } else {
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::Equal,
                0.0,
                &format!("balance_{}", node),
            )?;
        }
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo11 has no feasible solution"))?;

    println!("=== Demo11 ===");
    println!("status: {:?}", output.status);
    println!("max flow: {:.2}", read_solution_value(&solution, flow_idx));
    for arc in &data.capacities {
        if let Some(idx) = arc_idx[arc.from][arc.to] {
            let value = read_solution_value(&solution, idx);
            if value > 0.0 {
                println!(
                    "{} -> {} = {:.2}",
                    data.nodes[arc.from].name, data.nodes[arc.to].name, value
                );
            }
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
