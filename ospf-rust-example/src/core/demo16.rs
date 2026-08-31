use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UContinuousVariableItem;

use super::common::{read_solution_value, solve};

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

fn build_month_plans() -> Vec<MonthPlan> {
    vec![
        MonthPlan::new(3, 50.0, 100.0),
        MonthPlan::new(4, 180.0, 200.0),
        MonthPlan::new(5, 280.0, 180.0),
        MonthPlan::new(6, 270.0, 300.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let plans = build_month_plans();
    let product_price = 40.0;
    let delay_price = 2.0;
    let storage_price = 0.5;

    let n = plans.len();
    let mut model = MetaModel::<f64>::new("demo16");
    let mut x_idx = vec![vec![0usize; n]; n];
    for i in 0..n {
        for j in 0..n {
            x_idx[i][j] = model
                .register_variable(UContinuousVariableItem::auto(&format!("x_{}_{}", i, j)))?;
        }
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for i in 0..n {
        for j in 0..n {
            if i < j {
                objective[x_idx[i][j]] += (j - i) as f64 * storage_price;
                objective[x_idx[j][i]] += ((j - i) * (j - i)) as f64 * delay_price;
            }
            objective[x_idx[i][j]] += product_price;
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for j in 0..n {
        let coefficients: Vec<(usize, f64)> = (0..n).map(|i| (x_idx[i][j], 1.0)).collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            plans[j].demand,
            &format!("demand_{}", plans[j].month),
        )?;
    }

    for i in 0..n {
        let coefficients: Vec<(usize, f64)> = (0..n).map(|j| (x_idx[i][j], 1.0)).collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            plans[i].productivity,
            &format!("productivity_{}", plans[i].month),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo16 has no feasible solution"))?;

    println!("=== Demo16 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for i in 0..n {
        for j in 0..n {
            let value = read_solution_value(&solution, x_idx[i][j]);
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
