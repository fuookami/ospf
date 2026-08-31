use std::error::Error;

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{IntegerVariableItem, UContinuousVariableItem, VariableRange};
use ospf_rust_math::symbol::{Linear, LinearMonomial};

use super::common::{read_solution_value, solve_typed};

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

    let x_var = IntegerVariableItem::auto_with_range("x", VariableRange::bounded(-100.0, 100.0));
    let y_var = IntegerVariableItem::auto_with_range("y", VariableRange::bounded(-100.0, 100.0));
    let x_idx = model.register_variable(x_var.clone())?;
    let y_idx = model.register_variable(y_var.clone())?;

    let mut dx_vars = Vec::with_capacity(settlements.len());
    let mut dy_vars = Vec::with_capacity(settlements.len());
    let mut dx_idx = vec![0usize; settlements.len()];
    let mut dy_idx = vec![0usize; settlements.len()];
    for i in 0..settlements.len() {
        let dx_var = UContinuousVariableItem::auto(&format!("dx_{}", i));
        let dy_var = UContinuousVariableItem::auto(&format!("dy_{}", i));
        dx_idx[i] = model.register_variable(dx_var.clone())?;
        dy_idx[i] = model.register_variable(dy_var.clone())?;
        dx_vars.push(dx_var);
        dy_vars.push(dy_var);
    }

    let mut distance_terms = Vec::with_capacity(settlements.len() * 2);
    for i in 0..settlements.len() {
        distance_terms.push(LinearMonomial::new(1.0, dx_vars[i].to_owned_symbol()));
        distance_terms.push(LinearMonomial::new(1.0, dy_vars[i].to_owned_symbol()));
    }
    let distance = Linear::new(distance_terms, 0.0);
    model.set_math_linear_objective(distance, ObjectiveCategory::Minimum, "distance")?;

    for (i, settlement) in settlements.iter().enumerate() {
        let dx_lower = Linear::new(
            vec![
                LinearMonomial::new(-1.0, x_var.to_owned_symbol()),
                LinearMonomial::new(1.0, dx_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        let dx_upper = Linear::new(
            vec![
                LinearMonomial::new(1.0, x_var.to_owned_symbol()),
                LinearMonomial::new(1.0, dx_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        let dy_lower = Linear::new(
            vec![
                LinearMonomial::new(-1.0, y_var.to_owned_symbol()),
                LinearMonomial::new(1.0, dy_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        let dy_upper = Linear::new(
            vec![
                LinearMonomial::new(1.0, y_var.to_owned_symbol()),
                LinearMonomial::new(1.0, dy_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        model.add_math_inequality(dx_lower.ge(-settlement.x), &format!("dx_lb1_{}", i));
        model.add_math_inequality(dx_upper.ge(settlement.x), &format!("dx_lb2_{}", i));
        model.add_math_inequality(dy_lower.ge(-settlement.y), &format!("dy_lb1_{}", i));
        model.add_math_inequality(dy_upper.ge(settlement.y), &format!("dy_lb2_{}", i));
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

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
