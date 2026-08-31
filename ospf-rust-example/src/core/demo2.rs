use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use super::common::{
    add_constraint_with_metadata, linear_expr_from_indices, linear_expr_from_sparse_terms,
    read_solution_value, register_binary_matrix, solve,
};

#[derive(Debug, Clone)]
struct Product {
    name: String,
}

impl Product {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Company {
    name: String,
    costs: Vec<f64>,
}

impl Company {
    fn new(name: &str, costs: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            costs,
        }
    }

    fn cost_of(&self, product_idx: usize) -> f64 {
        self.costs[product_idx]
    }
}

fn build_products() -> Vec<Product> {
    vec![
        Product::new("P0"),
        Product::new("P1"),
        Product::new("P2"),
        Product::new("P3"),
    ]
}

fn build_companies() -> Vec<Company> {
    vec![
        Company::new("C0", vec![920.0, 480.0, 650.0, 340.0]),
        Company::new("C1", vec![870.0, 510.0, 700.0, 350.0]),
        Company::new("C2", vec![880.0, 500.0, 720.0, 400.0]),
        Company::new("C3", vec![930.0, 490.0, 680.0, 410.0]),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let companies = build_companies();
    let products = build_products();

    let mut model = MetaModel::<f64>::new("demo2");
    let x_idx = register_binary_matrix(&mut model, companies.len(), products.len(), "x")?;
    let group = model.create_constraint_group(2002, "demo2_assignment")?;

    let mut objective = vec![0.0; model.num_tokens()];
    for (c, company) in companies.iter().enumerate() {
        for (p, _) in products.iter().enumerate() {
            objective[x_idx[c][p]] = company.cost_of(p);
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for (c, _) in companies.iter().enumerate() {
        let indices: Vec<usize> = products.iter().enumerate().map(|(p, _)| x_idx[c][p]).collect();
        let coefficients = linear_expr_from_indices(&indices, 1.0);
        add_constraint_with_metadata(
            &mut model,
            &coefficients,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("company_{}", c),
            Some(group.clone()),
            false,
            1,
            Some(String::from("{\"kind\":\"company-capacity\"}")),
        )?;
    }

    for (p, _) in products.iter().enumerate() {
        let raw_terms: Vec<(usize, f64)> = companies
            .iter()
            .enumerate()
            .map(|(c, _)| (x_idx[c][p], 1.0))
            .collect();
        let coefficients = linear_expr_from_sparse_terms(&raw_terms);
        add_constraint_with_metadata(
            &mut model,
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!("product_{}", p),
            Some(group.clone()),
            false,
            1,
            Some(String::from("{\"kind\":\"product-partition\"}")),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo2 has no feasible solution"))?;

    println!("=== Demo2 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("total cost: {:.2}", obj);
    }
    for (c, company) in companies.iter().enumerate() {
        for (p, product) in products.iter().enumerate() {
            if read_solution_value(&solution, x_idx[c][p]) > 0.5 {
                println!("assign {} -> {}", product.name, company.name);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo2() {
        assert!(run().is_ok());
    }
}
