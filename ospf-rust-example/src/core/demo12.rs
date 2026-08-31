use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem};

use super::common::{read_solution_value, solve};

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
    let mut x_idx = vec![0usize; product_count];
    let mut assign_idx = vec![0usize; product_count];
    let mut premium_idx = vec![0usize; product_count];

    for i in 0..product_count {
        x_idx[i] = model.register_variable(UContinuousVariableItem::auto(&format!("x_{}", i)))?;
        assign_idx[i] = model.register_variable(BinaryVariableItem::auto(&format!("a_{}", i)))?;
        premium_idx[i] =
            model.register_variable(UContinuousVariableItem::auto(&format!("premium_{}", i)))?;
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for i in 0..product_count {
        objective[x_idx[i]] = products[i].yield_rate;
        objective[premium_idx[i]] = -1.0;
    }
    model.set_linear_objective(objective, ObjectiveCategory::Maximum);

    let mut fund_coefficients = Vec::with_capacity(product_count * 2);
    for i in 0..product_count {
        fund_coefficients.push((x_idx[i], 1.0));
        fund_coefficients.push((premium_idx[i], 1.0));
    }
    model.add_linear_constraint(
        &fund_coefficients,
        ConstraintRelation::Equal,
        funds,
        "funds",
    )?;

    let risk_coefficients: Vec<(usize, f64)> = (0..product_count)
        .map(|i| (x_idx[i], products[i].risk_rate / funds))
        .collect();
    model.add_linear_constraint(
        &risk_coefficients,
        ConstraintRelation::LessEqual,
        max_risk,
        "risk",
    )?;

    for i in 0..product_count {
        model.add_linear_constraint(
            &[(x_idx[i], 1.0), (assign_idx[i], -funds)],
            ConstraintRelation::LessEqual,
            0.0,
            &format!("activate_{}", i),
        )?;

        model.add_linear_constraint(
            &[(premium_idx[i], 1.0), (x_idx[i], -products[i].premium_rate)],
            ConstraintRelation::GreaterEqual,
            0.0,
            &format!("premium_rate_{}", i),
        )?;

        model.add_linear_constraint(
            &[(premium_idx[i], 1.0), (assign_idx[i], -products[i].min_premium)],
            ConstraintRelation::GreaterEqual,
            0.0,
            &format!("premium_min_{}", i),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo12 has no feasible solution"))?;

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
