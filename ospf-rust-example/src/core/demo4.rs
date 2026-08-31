use std::error::Error;

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableRange};
use ospf_rust_math::symbol::{Linear, LinearMonomial};

use super::common::{read_solution_value, solve_typed};

#[derive(Debug, Clone)]
struct Material {
    name: String,
    available: f64,
}

impl Material {
    fn new(name: &str, available: f64) -> Self {
        Self {
            name: name.to_string(),
            available,
        }
    }
}

#[derive(Debug, Clone)]
struct Product {
    name: String,
    max_yield: f64,
    profit: f64,
    usage_by_material: Vec<f64>,
}

impl Product {
    fn new(name: &str, max_yield: f64, profit: f64, usage_by_material: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            max_yield,
            profit,
            usage_by_material,
        }
    }

    fn usage_of(&self, material_idx: usize) -> f64 {
        self.usage_by_material[material_idx]
    }
}

fn build_materials() -> Vec<Material> {
    vec![Material::new("M0", 24.0), Material::new("M1", 8.0)]
}

fn build_products() -> Vec<Product> {
    vec![
        Product::new("P0", 3.0, 5.0, vec![6.0, 1.0]),
        Product::new("P1", 2.0, 4.0, vec![4.0, 2.0]),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let materials = build_materials();
    let products = build_products();

    let mut model = MetaModel::<f64>::new("demo4");
    let mut x_vars = Vec::with_capacity(products.len());
    let mut x_idx = vec![0usize; products.len()];

    for (p, product) in products.iter().enumerate() {
        let variable = ContinuousVariableItem::auto_with_range(
            &format!("x_{}", p),
            VariableRange::bounded(0.0, product.max_yield),
        );
        x_idx[p] = model.register_variable(variable.clone())?;
        x_vars.push(variable);
    }

    let profit = Linear::new(
        x_vars
            .iter()
            .zip(products.iter())
            .map(|(var, product)| LinearMonomial::new(product.profit, var.to_owned_symbol()))
            .collect(),
        0.0,
    );
    let usage: Vec<Linear<f64>> = materials
        .iter()
        .enumerate()
        .map(|(m, _)| {
            Linear::new(
                x_vars
                    .iter()
                    .zip(products.iter())
                    .map(|(var, product)| {
                        LinearMonomial::new(product.usage_of(m), var.to_owned_symbol())
                    })
                    .collect(),
                0.0,
            )
        })
        .collect();

    for (m, material) in materials.iter().enumerate() {
        model.add_math_inequality(
            usage[m].clone().le(material.available),
            &format!("material_{}_{}", m, material.name),
        );
    }

    model.set_math_linear_objective(profit, ObjectiveCategory::Maximum, "profit")?;

    for p1 in 0..products.len() {
        for p2 in 0..products.len() {
            if p1 == p2 {
                continue;
            }
            let difference = x_vars[p1].clone() - x_vars[p2].clone();
            model.add_math_inequality(difference.le(1.0), &format!("diff_{}_{}", p1, p2));
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo4 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("profit: {:.2}", obj);
    }
    for (p, product) in products.iter().enumerate() {
        println!(
            "{}: {:.4}",
            product.name,
            read_solution_value(&solution, x_idx[p])
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo4() {
        assert!(run().is_ok());
    }
}
