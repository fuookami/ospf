use std::error::Error;
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{Binary, VariableCombination2D};
use super::common::{read_solution_value, solve_typed};

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
    let x_shape = Shape::new([companies.len(), products.len()]);
    let x_vars: VariableCombination2D<Binary> =
        VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
            format!("{}_{}", vector[0], vector[1])
        });
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<Binary, _>(x_vars.iter().cloned())?,
    );

    let mut cost_terms = Vec::with_capacity(companies.len() * products.len());
    for (c, company) in companies.iter().enumerate() {
        for (p, _) in products.iter().enumerate() {
            cost_terms.push(LinearMonomial::new(
                company.cost_of(p),
                x_vars[&[c, p]].to_owned_symbol(),
            ));
        }
    }
    let cost = Linear::new(cost_terms, 0.0);
    let assignment_company =
        MultiArrayBuilder::new_by(Shape::<1>::new([companies.len()]), |_idx, vec| {
            let c = vec[0];
            Linear::new(
                products
                    .iter()
                    .enumerate()
                    .map(|(p, _)| LinearMonomial::new(1.0, x_vars[&[c, p]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        });
    let assignment_product =
        MultiArrayBuilder::new_by(Shape::<1>::new([products.len()]), |_idx, vec| {
            let p = vec[0];
            Linear::new(
                companies
                    .iter()
                    .enumerate()
                    .map(|(c, _)| LinearMonomial::new(1.0, x_vars[&[c, p]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        });

    model.set_math_linear_objective(cost, ObjectiveCategory::Minimum, "cost")?;

    for (c, _) in companies.iter().enumerate() {
        model.add_math_inequality(
            assignment_company[c].clone().le(1.0),
            &format!("company_{}", c),
        );
    }

    for (p, _) in products.iter().enumerate() {
        model.add_math_inequality(
            assignment_product[p].clone().eq_to(1.0),
            &format!("product_{}", p),
        );
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo2 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("total cost: {:.2}", obj);
    }
    for (c, company) in companies.iter().enumerate() {
        for (p, product) in products.iter().enumerate() {
            if read_solution_value(&solution, x_idx[&[c, p]]) > 0.5 {
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
