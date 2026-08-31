use std::error::Error;

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::BinaryVariableItem;
use ospf_rust_math::symbol::{Linear, LinearMonomial};

use super::common::{read_solution_value, solve_typed};

#[derive(Debug, Clone)]
struct Cargo {
    name: String,
    weight: f64,
    value: f64,
}

impl Cargo {
    fn new(name: &str, weight: f64, value: f64) -> Self {
        Self {
            name: name.to_string(),
            weight,
            value,
        }
    }
}

fn build_cargos() -> Vec<Cargo> {
    vec![
        Cargo::new("C0", 2.0, 6.0),
        Cargo::new("C1", 2.0, 3.0),
        Cargo::new("C2", 6.0, 5.0),
        Cargo::new("C3", 5.0, 4.0),
        Cargo::new("C4", 4.0, 6.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let cargos = build_cargos();
    let max_weight = 10.0;

    let mut model = MetaModel::<f64>::new("demo5");
    let mut x_vars = Vec::with_capacity(cargos.len());
    let mut x_idx = vec![0usize; cargos.len()];

    for (i, _) in cargos.iter().enumerate() {
        let variable = BinaryVariableItem::auto(&format!("x_{}", i));
        x_idx[i] = model.register_variable(variable.clone())?;
        x_vars.push(variable);
    }

    let cargo_value = Linear::new(
        x_vars
            .iter()
            .zip(cargos.iter())
            .map(|(var, cargo)| LinearMonomial::new(cargo.value, var.to_owned_symbol()))
            .collect(),
        0.0,
    );
    let cargo_weight = Linear::new(
        x_vars
            .iter()
            .zip(cargos.iter())
            .map(|(var, cargo)| LinearMonomial::new(cargo.weight, var.to_owned_symbol()))
            .collect(),
        0.0,
    );

    model.set_math_linear_objective(cargo_value, ObjectiveCategory::Maximum, "value")?;
    model.add_math_inequality(cargo_weight.le(max_weight), "weight");

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo5 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("value: {:.2}", obj);
    }
    for (i, cargo) in cargos.iter().enumerate() {
        if read_solution_value(&solution, x_idx[i]) > 0.5 {
            println!("pick {}", cargo.name);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo5() {
        assert!(run().is_ok());
    }
}
