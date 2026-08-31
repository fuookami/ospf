use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, LinearIntermediateSymbol, flat_map1,
};
use ospf_rust_core::variable::{UContinuous, VariableCombination2D};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// Monthly plan data structure
#[derive(Debug, Clone)]
struct MonthPlan {
    month: i32,
    productivity: f64,
    demand: f64,
}

impl MonthPlan {
    fn new(month: i32, productivity: f64, demand: f64) -> Self {
        Self {
            month,
            productivity,
            demand,
        }
    }
}

/// Build monthly plan list
fn build_month_plans() -> Vec<MonthPlan> {
    vec![
        MonthPlan::new(3, 50.0, 100.0),
        MonthPlan::new(4, 180.0, 200.0),
        MonthPlan::new(5, 280.0, 180.0),
        MonthPlan::new(6, 270.0, 300.0),
    ]
}

/// Demo16 main function: Production planning problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let plans = build_month_plans();
    let product_price = 40.0;
    let delay_price = 2.0;
    let storage_price = 0.5;

    let n = plans.len();
    let mut model = MetaModel::<f64>::new("demo16");

    // Register 2D variable combination x[i][j]: produce in month i for demand in month j
    let x_shape = Shape::new([n, n]);
    let x_vars: VariableCombination2D<UContinuous> =
        VariableCombination2D::with_name_generator(x_shape, "x", |_index, vector| {
            format!("{}_{}", vector[0], vector[1])
        });
    let x_idx = model.register_combination(&x_vars)?;

    // Objective components: produce cost, storage cost, delay delivery cost
    // produce cost = product_price * sum(x[i][j])
    let produce_cost_expr = flat_map1("produce_cost", &plans, |plan| {
        let i = plans.iter().position(|p| p.month == plan.month).unwrap();
        let monomials: Vec<_> = (0..n)
            .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                product_price,
                x_idx[&[i, j]],
            ))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, plan| format!("{}", plan.month));
    model.add_symbol_combination(&produce_cost_expr)?;

    // storage cost = sum((j-i) * storage_price * x[i][j]) for i < j
    let storage_cost_expr = flat_map1("storage_cost", &plans, |plan| {
        let i = plans.iter().position(|p| p.month == plan.month).unwrap();
        let monomials: Vec<_> = (0..n)
            .filter(|&j| i < j)
            .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                (j - i) as f64 * storage_price,
                x_idx[&[i, j]],
            ))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, plan| format!("{}", plan.month));
    model.add_symbol_combination(&storage_cost_expr)?;

    // delay delivery cost = sum((j-i)^2 * delay_price * x[j][i]) for i < j
    // Note: x[j][i] means produce in month j (later) for demand in month i (earlier)
    let delay_cost_expr = flat_map1("delay_cost", &plans, |plan| {
        let i = plans.iter().position(|p| p.month == plan.month).unwrap();
        let monomials: Vec<_> = (0..n)
            .filter(|&j| i < j)
            .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                ((j - i) * (j - i)) as f64 * delay_price,
                x_idx[&[j, i]],
            ))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, plan| format!("{}", plan.month));
    model.add_symbol_combination(&delay_cost_expr)?;

    // Aggregate total cost objective
    let mut total_cost_coeffs = Vec::new();
    for i in 0..n {
        for expr in [&produce_cost_expr, &storage_cost_expr, &delay_cost_expr] {
            let poly = expr.symbol_polynomial(i);
            for m in poly.monomials() {
                total_cost_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
    }
    model.add_linear_objective(&total_cost_coeffs, "cost");
    model.set_objective_category(ObjectiveCategory::Minimum);

    // Supply constraints per demand month: sum_i x[i][j] >= demand[j]
    let supply_expr = flat_map1("supply", &plans, |plan| {
        let j = plans.iter().position(|p| p.month == plan.month).unwrap();
        let monomials: Vec<_> = (0..n)
            .map(|i| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, plan| format!("{}", plan.month));
    model.add_symbol_combination(&supply_expr)?;

    for j in 0..n {
        let coeffs = extract_coeffs(&supply_expr[j]);
        model.add_linear_constraint(
            &coeffs,
            ConstraintRelation::GreaterEqual,
            plans[j].demand,
            &format!("demand_{}", plans[j].month),
        )?;
    }

    // Productivity constraints per produce month: sum_j x[i][j] <= productivity[i]
    let produce_expr = flat_map1("produce", &plans, |plan| {
        let i = plans.iter().position(|p| p.month == plan.month).unwrap();
        let monomials: Vec<_> = (0..n)
            .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, plan| format!("{}", plan.month));
    model.add_symbol_combination(&produce_expr)?;

    for i in 0..n {
        let coeffs = extract_coeffs(&produce_expr[i]);
        model.add_linear_constraint(
            &coeffs,
            ConstraintRelation::LessEqual,
            plans[i].productivity,
            &format!("productivity_{}", plans[i].month),
        )?;
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo16 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for i in 0..n {
        for j in 0..n {
            let value = read_solution_value(&solution, x_idx[&[i, j]]);
            if value > 0.0 {
                println!(
                    "produce month {} -> demand month {}: {:.2}",
                    plans[i].month, plans[j].month, value
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo16() {
        assert!(run().is_ok());
    }
}
