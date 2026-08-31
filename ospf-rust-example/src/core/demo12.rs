use std::error::Error;

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem};
use ospf_rust_math::symbol::{Linear, LinearMonomial};

use super::common::{read_solution_value, solve_typed};

#[derive(Debug, Clone)]
struct InvestmentProduct {
    name: String,
    yield_rate: f64,
    risk_rate: f64,
    premium_rate: f64,
    min_premium: f64,
}

impl InvestmentProduct {
    fn new(
        name: &str,
        yield_rate: f64,
        risk_rate: f64,
        premium_rate: f64,
        min_premium: f64,
    ) -> Self {
        Self {
            name: name.to_string(),
            yield_rate,
            risk_rate,
            premium_rate,
            min_premium,
        }
    }
}

fn build_products() -> Vec<InvestmentProduct> {
    vec![
        InvestmentProduct::new("P0", 0.28, 0.04, 0.08, 103.0),
        InvestmentProduct::new("P1", 0.21, 0.015, 0.02, 198.0),
        InvestmentProduct::new("P2", 0.23, 0.05, 0.045, 52.0),
        InvestmentProduct::new("P3", 0.25, 0.026, 0.04, 40.0),
        InvestmentProduct::new("P4", 0.05, 0.0, 0.0, 0.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let products = build_products();
    let product_count = products.len();
    let funds = 1_000_000.0;
    let max_risk = 0.02;

    let mut model = MetaModel::<f64>::new("demo12");
    let mut x_vars = Vec::with_capacity(product_count);
    let mut assign_vars = Vec::with_capacity(product_count);
    let mut premium_vars = Vec::with_capacity(product_count);
    let mut x_idx = vec![0usize; product_count];
    let mut premium_idx = vec![0usize; product_count];

    for i in 0..product_count {
        let x_var = UContinuousVariableItem::auto(&format!("x_{}", i));
        let assign_var = BinaryVariableItem::auto(&format!("a_{}", i));
        let premium_var = UContinuousVariableItem::auto(&format!("premium_{}", i));
        x_idx[i] = model.register_variable(x_var.clone())?;
        model.register_variable(assign_var.clone())?;
        premium_idx[i] = model.register_variable(premium_var.clone())?;
        x_vars.push(x_var);
        assign_vars.push(assign_var);
        premium_vars.push(premium_var);
    }

    let mut yield_terms = Vec::with_capacity(product_count * 2);
    let mut funds_terms = Vec::with_capacity(product_count * 2);
    let mut risk_terms = Vec::with_capacity(product_count);
    for i in 0..product_count {
        yield_terms.push(LinearMonomial::new(
            products[i].yield_rate,
            x_vars[i].to_owned_symbol(),
        ));
        yield_terms.push(LinearMonomial::new(-1.0, premium_vars[i].to_owned_symbol()));
        funds_terms.push(LinearMonomial::new(1.0, x_vars[i].to_owned_symbol()));
        funds_terms.push(LinearMonomial::new(1.0, premium_vars[i].to_owned_symbol()));
        risk_terms.push(LinearMonomial::new(
            products[i].risk_rate / funds,
            x_vars[i].to_owned_symbol(),
        ));
    }
    let yield_expr = Linear::new(yield_terms, 0.0);
    let funds_expr = Linear::new(funds_terms, 0.0);
    let risk_expr = Linear::new(risk_terms, 0.0);

    model.set_math_linear_objective(yield_expr, ObjectiveCategory::Maximum, "yield")?;
    model.add_math_inequality(funds_expr.eq_to(funds), "funds");
    model.add_math_inequality(risk_expr.le(max_risk), "risk");

    for i in 0..product_count {
        let activation = Linear::new(
            vec![
                LinearMonomial::new(1.0, x_vars[i].to_owned_symbol()),
                LinearMonomial::new(-funds, assign_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        model.add_math_inequality(activation.le(0.0), &format!("activate_{}", i));

        let premium_rate = Linear::new(
            vec![
                LinearMonomial::new(1.0, premium_vars[i].to_owned_symbol()),
                LinearMonomial::new(-products[i].premium_rate, x_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        model.add_math_inequality(premium_rate.ge(0.0), &format!("premium_rate_{}", i));

        let premium_min = Linear::new(
            vec![
                LinearMonomial::new(1.0, premium_vars[i].to_owned_symbol()),
                LinearMonomial::new(-products[i].min_premium, assign_vars[i].to_owned_symbol()),
            ],
            0.0,
        );
        model.add_math_inequality(premium_min.ge(0.0), &format!("premium_min_{}", i));
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo12 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("yield: {:.4}", obj);
    }
    for i in 0..product_count {
        let amount = read_solution_value(&solution, x_idx[i]);
        if amount > 0.0 {
            let premium = read_solution_value(&solution, premium_idx[i]);
            println!(
                "product {} amount {:.2}, premium {:.2}",
                products[i].name, amount, premium
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo12() {
        assert!(run().is_ok());
    }
}
