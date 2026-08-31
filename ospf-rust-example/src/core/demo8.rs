use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1,
};
use ospf_rust_core::variable::{UInteger, VariableCombination1D};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// Product data structure
#[derive(Debug, Clone)]
struct Product {
    name: String,
    profit: f64,
}

impl Product {
    fn new(name: &str, profit: f64) -> Self {
        Self { name: name.to_string(), profit }
    }
}

/// Equipment data structure
#[derive(Debug, Clone)]
struct Equipment {
    name: String,
    amount: f64,
    man_hours_by_product: Vec<f64>,
}

impl Equipment {
    fn new(name: &str, amount: f64, man_hours_by_product: Vec<f64>) -> Self {
        Self { name: name.to_string(), amount, man_hours_by_product }
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

/// Equipment allocation model using VariableCombination + SymbolCombination
struct EquipmentModel {
    x: VariableCombination1D<UInteger>,
    x_idx: MultiArray<usize, Shape<1>>,
    profit_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    man_hours_exprs: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl EquipmentModel {
    fn register(
        model: &mut MetaModel<f64>,
        products: &[Product],
        equipments: &[Equipment],
    ) -> Result<Self, Box<dyn Error>> {
        let x = VariableCombination1D::new(Shape::new([products.len()]), "x");
        let x_idx = model.register_combination(&x)?;

        // Objective: profit = sum(profit_i * x_i)
        let profit_expr = flat_map1("profit", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(product.profit, x_idx[i])],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&profit_expr)?;

        // Constraints: man_hours per equipment
        let man_hours_exprs = flat_map1("man_hours", equipments, |equipment| {
            let monomials: Vec<_> = products.iter().enumerate()
                .filter_map(|(p, product)| {
                    let value = equipment.man_hours_by_product[p];
                    (value != 0.0).then(|| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(value, x_idx[p])
                    })
                })
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, equipment| equipment.name.clone());
        model.add_symbol_combination(&man_hours_exprs)?;

        Ok(EquipmentModel { x, x_idx, profit_expr, man_hours_exprs })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        equipments: &[Equipment],
        max_man_hours: f64,
    ) -> Result<(), Box<dyn Error>> {
        // Objective: maximize profit
        let profit_coeffs = extract_coeffs(&self.profit_expr[0]);
        model.add_linear_objective(&profit_coeffs, "profit");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // Constraints: man_hours_i <= amount_i * max_man_hours
        for (e, equipment) in equipments.iter().enumerate() {
            let coeffs = extract_coeffs(&self.man_hours_exprs[e]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                equipment.amount * max_man_hours,
                &format!("equipment_{}_{}", e, equipment.name),
            )?;
        }

        Ok(())
    }
}

/// Demo8 main function: Equipment allocation problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let products = build_products();
    let equipments = build_equipments();
    let max_man_hours = 2000.0;

    let mut model = MetaModel::<f64>::new("demo8");
    let em = EquipmentModel::register(&mut model, &products, &equipments)?;
    em.add_constraints(&mut model, &equipments, max_man_hours)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo8 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("profit: {:.2}", obj);
    }
    for (p, product) in products.iter().enumerate() {
        println!(
            "{}: {:.2}",
            product.name,
            read_solution_value(&solution, em.x_idx[p])
        );
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
