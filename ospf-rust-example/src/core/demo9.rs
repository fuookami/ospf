use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{IntegerVariableItem, UContinuousVariableItem, VariableRange};

use super::common::{read_solution_value, solve};

#[derive(Debug, Clone)]
struct Settlement {
    name: String,
    x: f64,
    y: f64,
}

impl Settlement {
    fn new(name: &str, x: f64, y: f64) -> Self {
        Self {
            name: name.to_string(),
            x,
            y,
        }
    }
}

fn build_settlements() -> Vec<Settlement> {
    vec![
        Settlement::new("S0", 9.0, 2.0),
        Settlement::new("S1", 2.0, 1.0),
        Settlement::new("S2", 3.0, 8.0),
        Settlement::new("S3", 3.0, -2.0),
        Settlement::new("S4", 5.0, 9.0),
        Settlement::new("S5", 4.0, -2.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let settlements = build_settlements();

    let mut model = MetaModel::<f64>::new("demo9");

    let x_idx = model.register_variable(IntegerVariableItem::auto_with_range(
        "x",
        VariableRange::bounded(-100.0, 100.0),
    ))?;
    let y_idx = model.register_variable(IntegerVariableItem::auto_with_range(
        "y",
        VariableRange::bounded(-100.0, 100.0),
    ))?;

    let mut dx_idx = vec![0usize; settlements.len()];
    let mut dy_idx = vec![0usize; settlements.len()];
    for i in 0..settlements.len() {
        dx_idx[i] = model.register_variable(UContinuousVariableItem::auto(&format!("dx_{}", i)))?;
        dy_idx[i] = model.register_variable(UContinuousVariableItem::auto(&format!("dy_{}", i)))?;
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for i in 0..settlements.len() {
        objective[dx_idx[i]] = 1.0;
        objective[dy_idx[i]] = 1.0;
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for (i, settlement) in settlements.iter().enumerate() {
        model.add_linear_constraint(
            &[(x_idx, -1.0), (dx_idx[i], 1.0)],
            ConstraintRelation::GreaterEqual,
            -settlement.x,
            &format!("dx_lb1_{}", i),
        )?;
        model.add_linear_constraint(
            &[(x_idx, 1.0), (dx_idx[i], 1.0)],
            ConstraintRelation::GreaterEqual,
            settlement.x,
            &format!("dx_lb2_{}", i),
        )?;

        model.add_linear_constraint(
            &[(y_idx, -1.0), (dy_idx[i], 1.0)],
            ConstraintRelation::GreaterEqual,
            -settlement.y,
            &format!("dy_lb1_{}", i),
        )?;
        model.add_linear_constraint(
            &[(y_idx, 1.0), (dy_idx[i], 1.0)],
            ConstraintRelation::GreaterEqual,
            settlement.y,
            &format!("dy_lb2_{}", i),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo9 has no feasible solution"))?;

    println!("=== Demo9 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("distance sum: {:.2}", obj);
    }
    println!(
        "position: ({:.2}, {:.2})",
        read_solution_value(&solution, x_idx),
        read_solution_value(&solution, y_idx)
    );
    for (i, settlement) in settlements.iter().enumerate() {
        println!(
            "{} ({:.2},{:.2}) distance components ({:.2},{:.2})",
            settlement.name,
            settlement.x,
            settlement.y,
            read_solution_value(&solution, dx_idx[i]),
            read_solution_value(&solution, dy_idx[i])
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo9() {
        assert!(run().is_ok());
    }
}
