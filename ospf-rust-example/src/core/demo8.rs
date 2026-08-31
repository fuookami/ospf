use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UIntegerVariableItem;

use super::common::{read_solution_value, solve};

#[derive(Debug, Clone)]
struct Product {
    name: String,
    profit: f64,
}

impl Product {
    fn new(name: &str, profit: f64) -> Self {
        Self {
            name: name.to_string(),
            profit,
        }
    }
}

#[derive(Debug, Clone)]
struct Equipment {
    name: String,
    amount: f64,
    man_hours_by_product: Vec<f64>,
}

impl Equipment {
    fn new(name: &str, amount: f64, man_hours_by_product: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            amount,
            man_hours_by_product,
        }
    }

    fn man_hours_for(&self, product_idx: usize) -> f64 {
        self.man_hours_by_product[product_idx]
    }
}

fn build_products() -> Vec<Product> {
    vec![
        Product::new("P0", 123.0),
        Product::new("P1", 94.0),
        Product::new("P2", 105.0),
        Product::new("P3", 132.0),
        Product::new("P4", 118.0),
    ]
}

fn build_equipments() -> Vec<Equipment> {
    vec![
        Equipment::new("E0", 12.0, vec![0.23, 0.44, 0.17, 0.08, 0.36]),
        Equipment::new("E1", 14.0, vec![0.13, 0.00, 0.20, 0.37, 0.19]),
        Equipment::new("E2", 8.0, vec![0.00, 0.25, 0.34, 0.00, 0.18]),
        Equipment::new("E3", 6.0, vec![0.55, 0.72, 0.00, 0.61, 0.00]),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let products = build_products();
    let equipments = build_equipments();
    let max_man_hours = 2000.0;

    let mut model = MetaModel::<f64>::new("demo8");
    let mut x_idx = vec![0usize; products.len()];

    for (p, _) in products.iter().enumerate() {
        let variable = UIntegerVariableItem::auto(&format!("x_{}", p));
        x_idx[p] = model.register_variable(variable)?;
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for (p, product) in products.iter().enumerate() {
        objective[x_idx[p]] = product.profit;
    }
    model.set_linear_objective(objective, ObjectiveCategory::Maximum);

    for (e, equipment) in equipments.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = products
            .iter()
            .enumerate()
            .filter_map(|(p, _)| {
                let value = equipment.man_hours_for(p);
                if value == 0.0 {
                    None
                } else {
                    Some((x_idx[p], value))
                }
            })
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            equipment.amount * max_man_hours,
            &format!("equipment_{}_{}", e, equipment.name),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo8 has no feasible solution"))?;

    println!("=== Demo8 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("profit: {:.2}", obj);
    }
    for (p, product) in products.iter().enumerate() {
        println!("{}: {:.2}", product.name, read_solution_value(&solution, x_idx[p]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo8() {
        assert!(run().is_ok());
    }
}
