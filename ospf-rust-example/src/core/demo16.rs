use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, flat_map1_indexed,
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

/// 生产计划问题模型 / Production planning problem model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct ProductionModel {
    /// 生产量决策变量 / Production decision variables
    x_vars: VariableCombination2D<UContinuous>,
    /// 生产量模型索引数组 / Production model index array
    x_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 生产成本符号 / Production cost symbol
    produce: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 供应符号 / Supply symbol
    supply: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 生产成本符号 / Produce cost symbol
    produce_cost: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 存储成本符号 / Storage cost symbol
    storage_cost: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 延迟交付成本符号 / Delay delivery cost symbol
    delay_cost: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl ProductionModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        plans: &[MonthPlan],
        product_price: f64,
        delay_price: f64,
        storage_price: f64,
    ) -> Result<Self, Box<dyn Error>> {
        let n = plans.len();

        // Register 2D variable combination x[i][j]: produce in month i for demand in month j
        let x_shape = Shape::new([n, n]);
        let x_vars: VariableCombination2D<UContinuous> =
            VariableCombination2D::with_name_generator(x_shape, "x", |_index, vector| {
                format!("{}_{}", vector[0], vector[1])
            });
        let x_idx = model.register_combination(&x_vars)?;

        // produce cost = product_price * sum(x[i][j])
        let produce_cost = flat_map1_indexed("produce_cost", plans, |i, _plan| {
            let monomials: Vec<_> = (0..n)
                .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    product_price,
                    x_idx[&[i, j]],
                ))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, plan| format!("{}", plan.month));
        model.add_symbol_combination(&produce_cost)?;

        // storage cost = sum((j-i) * storage_price * x[i][j]) for i < j
        let storage_cost = flat_map1_indexed("storage_cost", plans, |i, _plan| {
            let monomials: Vec<_> = (0..n)
                .filter(|&j| i < j)
                .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    (j - i) as f64 * storage_price,
                    x_idx[&[i, j]],
                ))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, plan| format!("{}", plan.month));
        model.add_symbol_combination(&storage_cost)?;

        // delay delivery cost = sum((j-i)^2 * delay_price * x[j][i]) for i < j
        // Note: x[j][i] means produce in month j (later) for demand in month i (earlier)
        let delay_cost = flat_map1_indexed("delay_cost", plans, |i, _plan| {
            let monomials: Vec<_> = (0..n)
                .filter(|&j| i < j)
                .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    ((j - i) * (j - i)) as f64 * delay_price,
                    x_idx[&[j, i]],
                ))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, plan| format!("{}", plan.month));
        model.add_symbol_combination(&delay_cost)?;

        // Supply constraints per demand month: sum_i x[i][j] >= demand[j]
        let supply = flat_map1_indexed("supply", plans, |j, _plan| {
            let monomials: Vec<_> = (0..n)
                .map(|i| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]]))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, plan| format!("{}", plan.month));
        model.add_symbol_combination(&supply)?;

        // Productivity constraints per produce month: sum_j x[i][j] <= productivity[i]
        let produce = flat_map1_indexed("produce", plans, |i, _plan| {
            let monomials: Vec<_> = (0..n)
                .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]]))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, plan| format!("{}", plan.month));
        model.add_symbol_combination(&produce)?;

        Ok(ProductionModel {
            x_vars,
            x_idx,
            produce,
            supply,
            produce_cost,
            storage_cost,
            delay_cost,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        plans: &[MonthPlan],
    ) -> Result<(), Box<dyn Error>> {
        let n = plans.len();

        // Aggregate total cost objective
        let mut total_cost_coeffs = Vec::new();
        for i in 0..n {
            for expr in [&self.produce_cost, &self.storage_cost, &self.delay_cost] {
                let poly = expr.symbol_polynomial(i);
                for m in poly.monomials() {
                    total_cost_coeffs.push((m.var_index(), *m.coefficient()));
                }
            }
        }
        model.add_linear_objective(&total_cost_coeffs, "cost");
        model.set_objective_category(ObjectiveCategory::Minimum);

        // Supply constraints per demand month: sum_i x[i][j] >= demand[j]
        for j in 0..n {
            let coeffs = extract_coeffs(&self.supply[j]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::GreaterEqual,
                plans[j].demand,
                &format!("demand_{}", plans[j].month),
            )?;
        }

        // Productivity constraints per produce month: sum_j x[i][j] <= productivity[i]
        for i in 0..n {
            let coeffs = extract_coeffs(&self.produce[i]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                plans[i].productivity,
                &format!("productivity_{}", plans[i].month),
            )?;
        }

        Ok(())
    }
}

/// Demo16 main function: Production planning problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let plans = build_month_plans();
    let product_price = 40.0;
    let delay_price = 2.0;
    let storage_price = 0.5;

    let mut model = MetaModel::<f64>::new("demo16");
    let production = ProductionModel::register(&mut model, &plans, product_price, delay_price, storage_price)?;

    production.add_constraints(&mut model, &plans)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo16 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for i in 0..plans.len() {
        for j in 0..plans.len() {
            let value = read_solution_value(&solution, production.x_idx[&[i, j]]);
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
