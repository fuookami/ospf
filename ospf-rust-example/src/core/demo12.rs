use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol,
    BinaryzationFunction, MaxFunction, flat_map1_indexed,
};
use ospf_rust_core::variable::{UInteger, VariableCombination1D};

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

/// Portfolio optimization model matching Kotlin's 4-symbol structure.
///
/// Kotlin symbols (4): `assignment`, `premium`, `risk`, `yield`
///   - `assignment[i] = Binaryzation(x[i])`          -- binary indicator via Big-M
///   - `premium[i]   = max(rate*x[i], minPrem*a[i])` -- max of two expressions
///   - `risk  = sum(risk_rate_i / funds * x_i)`
///   - `yield = sum(yield_rate_i * x_i - premium_i)`
///
/// Rust symbols (4): assignment / premium via FunctionSymbol, risk / yield via LinearExpressionSymbol
///   - `assignment_fn`  : BinaryzationFunction (Big-M mechanism constraints auto-generated)
///   - `premium_fn`     : MaxFunction (max lower-bound mechanism constraints auto-generated)
///   - `risk_expr`      : LinearExpressionSymbol (same as Kotlin)
///   - `yield_expr`     : LinearExpressionSymbol (same as Kotlin)
struct PortfolioModel {
    x: VariableCombination1D<UInteger>,
    x_idx: MultiArray<usize, Shape<1>>,
    /// `assignment[i] = Binaryzation(x[i])` via Big-M
    assignment_fn: SymbolCombination<f64, BinaryzationFunction<f64>, Shape<1>>,
    /// `premium[i] = max(rate*x[i], minPrem*a[i])`
    premium_fn: SymbolCombination<f64, MaxFunction<f64>, Shape<1>>,
    /// sum(yield_rate_i * x_i - premium_i)
    yield_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// sum(x_i + premium_i) = funds (budget constraint)
    funds_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// sum(risk_rate_i / funds * x_i) <= max_risk
    risk_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl PortfolioModel {
    fn register(
        model: &mut MetaModel<f64>,
        products: &[InvestmentProduct],
        funds: f64,
    ) -> Result<Self, Box<dyn Error>> {
        let n = products.len();

        // Decision variables: x[i] = allocation amount for product i
        let x = VariableCombination1D::new(Shape::new([n]), "x");
        let x_idx = model.register_combination(&x)?;

        // Function symbol: assignment[i] = Binaryzation(x[i]) via Big-M
        // Mechanism constraints (x_i - M*a_i <= 0, x_i >= eps*a_i) are auto-generated.
        let assignment_fn = SymbolCombination::new(
            Shape::new([n]), "assignment",
            |i, _| {
                BinaryzationFunction::with_big_m(
                    i as u64 + 100,
                    &products[i].name,
                    ospf_rust_core::symbol::flatten::Linear::new(
                        vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[i])],
                        0.0,
                    ),
                    funds,
                )
            },
        );
        model.add_symbol_combination(&assignment_fn)?;

        // Function symbol: premium[i] = max(premium_rate_i * x_i, min_premium_i * a_i)
        // a_i is the binary result variable from assignment_fn[i].
        // MaxFunction lower-bound constraints are auto-generated.
        let premium_fn = SymbolCombination::new(
            Shape::new([n]), "premium",
            |i, _| {
                let assign_result_idx = assignment_fn.symbol_polynomial(i).monomials()[0].var_index();
                MaxFunction::new(
                    i as u64 + 200,
                    &products[i].name,
                    vec![
                        ospf_rust_core::symbol::flatten::Linear::new(
                            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                                products[i].premium_rate, x_idx[i],
                            )],
                            0.0,
                        ),
                        ospf_rust_core::symbol::flatten::Linear::new(
                            vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                                products[i].min_premium, assign_result_idx,
                            )],
                            0.0,
                        ),
                    ],
                    false,
                )
            },
        );
        model.add_symbol_combination(&premium_fn)?;

        // Compute result variable indices from function symbols
        let premium_fn_idx: Vec<usize> = (0..n)
            .map(|i| premium_fn.symbol_polynomial(i).monomials()[0].var_index())
            .collect();

        // Objective: yield = sum(yield_rate_i * x_i - premium_i)
        let yield_expr = flat_map1_indexed("yield", products, |i, product| {
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(product.yield_rate, x_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, premium_fn_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&yield_expr)?;

        // Constraint: funds = sum(x_i + premium_i)
        let funds_expr = flat_map1_indexed("funds", products, |i, _product| {
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, premium_fn_idx[i]),
                ],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&funds_expr)?;

        // Constraint: risk = sum(risk_rate_i / funds * x_i) <= max_risk
        let risk_expr = flat_map1_indexed("risk", products, |i, product| {
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    if funds.abs() < 1e-10 { 0.0 } else { product.risk_rate / funds }, x_idx[i],
                )],
                0.0,
            )
        }, |_, product| product.name.clone());
        model.add_symbol_combination(&risk_expr)?;

        Ok(PortfolioModel {
            x, x_idx,
            assignment_fn, premium_fn,
            yield_expr, funds_expr, risk_expr,
        })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        products: &[InvestmentProduct],
        funds: f64,
        max_risk: f64,
    ) -> Result<(), Box<dyn Error>> {
        // Objective: maximize yield = sum(yield_rate_i * x_i - premium_i)
        let mut yield_coeffs = Vec::new();
        for i in 0..products.len() {
            let poly = self.yield_expr.symbol_polynomial(i);
            for m in poly.monomials() {
                yield_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_objective(&yield_coeffs, "yield");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // Aggregate funds constraint: sum(x_i + premium_i) = funds
        let mut funds_coeffs = Vec::new();
        for i in 0..products.len() {
            let poly = self.funds_expr.symbol_polynomial(i);
            for m in poly.monomials() {
                funds_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_constraint(&funds_coeffs, ConstraintRelation::Equal, funds, "funds")?;

        // Aggregate risk constraint: sum(risk_rate_i/funds * x_i) <= max_risk
        let mut risk_coeffs = Vec::new();
        for i in 0..products.len() {
            let poly = self.risk_expr.symbol_polynomial(i);
            for m in poly.monomials() {
                risk_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_constraint(&risk_coeffs, ConstraintRelation::LessEqual, max_risk, "risk")?;

        // Binaryzation and Max mechanism constraints are auto-generated by
        // BinaryzationFunction and MaxFunction during model conversion.
        // No per-product hand-written constraints needed.

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
            let premium_result_idx = portfolio.premium_fn.symbol_polynomial(i).monomials()[0].var_index();
            let premium = read_solution_value(&solution, premium_result_idx);
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
