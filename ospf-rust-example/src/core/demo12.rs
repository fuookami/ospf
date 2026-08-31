use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1,
};
use ospf_rust_core::variable::{Binary, UContinuous, VariableCombination1D};

use super::common::{read_solution_value, solve_typed};

/// Investment product data structure
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

/// Portfolio optimization model using VariableCombination + SymbolCombination
struct PortfolioModel {
    x: VariableCombination1D<UContinuous>,
    assign: VariableCombination1D<Binary>,
    premium: VariableCombination1D<UContinuous>,
    x_idx: MultiArray<usize, Shape<1>>,
    premium_idx: MultiArray<usize, Shape<1>>,
    assign_idx: MultiArray<usize, Shape<1>>,
    yield_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    funds_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    risk_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    activation_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    premium_rate_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    premium_min_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl PortfolioModel {
    fn register(
        model: &mut MetaModel<f64>,
        products: &[InvestmentProduct],
        funds: f64,
    ) -> Result<Self, Box<dyn Error>> {
        let n = products.len();

        let x = VariableCombination1D::new(Shape::new([n]), "x");
        let assign = VariableCombination1D::new(Shape::new([n]), "a");
        let premium = VariableCombination1D::new(Shape::new([n]), "premium");
        let x_idx = model.register_combination(&x)?;
        let assign_idx = model.register_combination(&assign)?;
        let premium_idx = model.register_combination(&premium)?;

        // Objective: yield = sum(yield_rate_i * x_i - premium_i)
        let yield_expr = flat_map1("yield", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(product.yield_rate, x_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, premium_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&yield_expr)?;

        // Constraint: funds = sum(x_i + premium_i)
        let funds_expr = flat_map1("funds", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, premium_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&funds_expr)?;

        // Constraint: risk = sum(risk_rate_i / funds * x_i) <= max_risk
        let risk_expr = flat_map1("risk", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    product.risk_rate / funds, x_idx[i],
                )],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&risk_expr)?;

        // Per-product: activation constraint  x_i - funds * a_i <= 0
        let activation_expr = flat_map1("activate", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-funds, assign_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&activation_expr)?;

        // Per-product: premium rate  premium_i - premium_rate_i * x_i >= 0
        let premium_rate_expr = flat_map1("premium_rate", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, premium_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-product.premium_rate, x_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&premium_rate_expr)?;

        // Per-product: premium minimum  premium_i - min_premium_i * a_i >= 0
        let premium_min_expr = flat_map1("premium_min", products, |product| {
            let i = products.iter().position(|p| p.name == product.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, premium_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-product.min_premium, assign_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&premium_min_expr)?;

        Ok(PortfolioModel {
            x, assign, premium,
            x_idx, premium_idx, assign_idx,
            yield_expr, funds_expr, risk_expr,
            activation_expr, premium_rate_expr, premium_min_expr,
        })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        products: &[InvestmentProduct],
        funds: f64,
        max_risk: f64,
    ) -> Result<(), Box<dyn Error>> {
        // Objective: maximize yield
        let yield_poly = self.yield_expr[0].to_linear_polynomial();
        let yield_coeffs: Vec<_> = yield_poly.monomials().iter()
            .map(|m| (m.var_index(), *m.coefficient())).collect();
        model.add_linear_objective(&yield_coeffs, "yield");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // Aggregate funds constraint: sum(x_i + premium_i) = funds
        let mut funds_coeffs = Vec::new();
        for i in 0..products.len() {
            let poly = self.funds_expr[i].to_linear_polynomial();
            for m in poly.monomials() {
                funds_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_constraint(&funds_coeffs, ConstraintRelation::Equal, funds, "funds")?;

        // Aggregate risk constraint: sum(risk_rate_i/funds * x_i) <= max_risk
        let mut risk_coeffs = Vec::new();
        for i in 0..products.len() {
            let poly = self.risk_expr[i].to_linear_polynomial();
            for m in poly.monomials() {
                risk_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_constraint(&risk_coeffs, ConstraintRelation::LessEqual, max_risk, "risk")?;

        // Per-product constraints
        for i in 0..products.len() {
            // x_i - funds * a_i <= 0
            let act_poly = self.activation_expr[i].to_linear_polynomial();
            let act_coeffs: Vec<_> = act_poly.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &act_coeffs, ConstraintRelation::LessEqual, 0.0,
                &format!("activate_{}", i),
            )?;

            // premium_i - premium_rate_i * x_i >= 0
            let pr_poly = self.premium_rate_expr[i].to_linear_polynomial();
            let pr_coeffs: Vec<_> = pr_poly.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &pr_coeffs, ConstraintRelation::GreaterEqual, 0.0,
                &format!("premium_rate_{}", i),
            )?;

            // premium_i - min_premium_i * a_i >= 0
            let pm_poly = self.premium_min_expr[i].to_linear_polynomial();
            let pm_coeffs: Vec<_> = pm_poly.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &pm_coeffs, ConstraintRelation::GreaterEqual, 0.0,
                &format!("premium_min_{}", i),
            )?;
        }

        Ok(())
    }
}

/// Demo12 main function: Portfolio optimization problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let products = build_products();
    let product_count = products.len();
    let funds = 1_000_000.0;
    let max_risk = 0.02;

    let mut model = MetaModel::<f64>::new("demo12");
    let portfolio = PortfolioModel::register(&mut model, &products, funds)?;
    portfolio.add_constraints(&mut model, &products, funds, max_risk)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo12 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("yield: {:.4}", obj);
    }
    for i in 0..product_count {
        let amount = read_solution_value(&solution, portfolio.x_idx[i]);
        if amount > 0.0 {
            let premium = read_solution_value(&solution, portfolio.premium_idx[i]);
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
