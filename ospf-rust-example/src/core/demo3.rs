use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UIntegerVariableItem;

use super::common::{read_solution_value, solve};

#[derive(Debug, Clone)]
struct Material {
    name: String,
    unit_cost: f64,
    yields: Vec<f64>,
}

impl Material {
    fn new(name: &str, unit_cost: f64, yields: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            unit_cost,
            yields,
        }
    }

    fn yield_of(&self, product_idx: usize) -> f64 {
        self.yields[product_idx]
    }
}

#[derive(Debug, Clone)]
struct ProductTarget {
    name: String,
    min_yield: f64,
}

impl ProductTarget {
    fn new(name: &str, min_yield: f64) -> Self {
        Self {
            name: name.to_string(),
            min_yield,
        }
    }
}

fn build_materials() -> Vec<Material> {
    vec![
        Material::new("M0", 115.0, vec![30.0, 10.0, 0.0]),
        Material::new("M1", 97.0, vec![15.0, 0.0, 20.0]),
        Material::new("M2", 82.0, vec![0.0, 25.0, 15.0]),
        Material::new("M3", 76.0, vec![15.0, 15.0, 15.0]),
    ]
}

fn build_product_targets() -> Vec<ProductTarget> {
    vec![
        ProductTarget::new("P0", 15000.0),
        ProductTarget::new("P1", 15000.0),
        ProductTarget::new("P2", 10000.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let materials = build_materials();
    let targets = build_product_targets();

    let mut model = MetaModel::<f64>::new("demo3");
    let mut x_idx = vec![0usize; materials.len()];

    for (m, _) in materials.iter().enumerate() {
        let variable = UIntegerVariableItem::auto(&format!("x_{}", m));
        x_idx[m] = model.register_variable(variable)?;
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for (m, material) in materials.iter().enumerate() {
        objective[x_idx[m]] = material.unit_cost;
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for (p, target) in targets.iter().enumerate() {
        let mut coefficients = Vec::with_capacity(materials.len());
        for (m, material) in materials.iter().enumerate() {
            coefficients.push((x_idx[m], material.yield_of(p)));
        }
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            target.min_yield,
            &format!("yield_{}", target.name),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo3 has no feasible solution"))?;

    println!("=== Demo3 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("total cost: {:.2}", obj);
    }
    for (m, material) in materials.iter().enumerate() {
        println!("{}: {:.2}", material.name, read_solution_value(&solution, x_idx[m]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo3() {
        assert!(run().is_ok());
    }
}
