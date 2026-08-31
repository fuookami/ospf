use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UIntegerVariableItem;

use super::common::{read_solution_value, solve};

#[derive(Debug, Clone)]
struct Cargo {
    name: String,
    weight: f64,
    value: f64,
    max_amount: f64,
}

impl Cargo {
    fn new(name: &str, weight: f64, value: f64, max_amount: f64) -> Self {
        Self {
            name: name.to_string(),
            weight,
            value,
            max_amount,
        }
    }
}

fn build_cargos() -> Vec<Cargo> {
    vec![
        Cargo::new("C0", 1.0, 6.0, 10.0),
        Cargo::new("C1", 2.0, 10.0, 5.0),
        Cargo::new("C2", 2.0, 20.0, 2.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let cargos = build_cargos();
    let max_weight = 8.0;

    let mut model = MetaModel::<f64>::new("demo6");
    let mut x_idx = vec![0usize; cargos.len()];

    for (i, _) in cargos.iter().enumerate() {
        let variable = UIntegerVariableItem::auto(&format!("x_{}", i));
        x_idx[i] = model.register_variable(variable)?;
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for (i, cargo) in cargos.iter().enumerate() {
        objective[x_idx[i]] = cargo.value;
    }
    model.set_linear_objective(objective, ObjectiveCategory::Maximum);

    let weight_coefficients: Vec<(usize, f64)> = x_idx
        .iter()
        .enumerate()
        .map(|(i, idx)| (*idx, cargos[i].weight))
        .collect();
    model.add_linear_constraint(
        &weight_coefficients,
        ConstraintRelation::LessEqual,
        max_weight,
        "weight",
    )?;

    for (i, cargo) in cargos.iter().enumerate() {
        model.add_linear_constraint(
            &[(x_idx[i], 1.0)],
            ConstraintRelation::LessEqual,
            cargo.max_amount,
            &format!("upper_{}", i),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo6 has no feasible solution"))?;

    println!("=== Demo6 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("value: {:.2}", obj);
    }
    for (i, cargo) in cargos.iter().enumerate() {
        println!("{}: {:.2}", cargo.name, read_solution_value(&solution, x_idx[i]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo6() {
        assert!(run().is_ok());
    }
}
