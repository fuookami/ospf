use std::error::Error;
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{UContinuous, VariableCombination2D};
use super::common::{read_solution_value, solve_typed};

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
    let x_shape = Shape::new([n, n]);
    let x_vars: VariableCombination2D<UContinuous> =
        VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
            format!("{}_{}", vector[0], vector[1])
        });
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<UContinuous, _>(x_vars.iter().cloned())?,
    );

    let mut delay_delivery_cost_terms = Vec::new();
    let mut storage_cost_terms = Vec::new();
    let mut produce_cost_terms = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            if i < j {
                storage_cost_terms.push(LinearMonomial::new(
                    (j - i) as f64 * storage_price,
                    x_vars[&[i, j]].to_owned_symbol(),
                ));
                delay_delivery_cost_terms.push(LinearMonomial::new(
                    ((j - i) * (j - i)) as f64 * delay_price,
                    x_vars[&[j, i]].to_owned_symbol(),
                ));
            }
            produce_cost_terms.push(LinearMonomial::new(
                product_price,
                x_vars[&[i, j]].to_owned_symbol(),
            ));
        }
    }
    let delay_delivery_cost = Linear::new(delay_delivery_cost_terms, 0.0);
    let storage_cost = Linear::new(storage_cost_terms, 0.0);
    let produce_cost = Linear::new(produce_cost_terms, 0.0);
    let total_cost = delay_delivery_cost + storage_cost + produce_cost;
    let produce = MultiArrayBuilder::new_by(Shape::<1>::new([n]), |_idx, vec| {
        let i = vec[0];
        Linear::new(
            (0..n)
                .map(|j| LinearMonomial::new(1.0, x_vars[&[i, j]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });
    let supply = MultiArrayBuilder::new_by(Shape::<1>::new([n]), |_idx, vec| {
        let j = vec[0];
        Linear::new(
            (0..n)
                .map(|i| LinearMonomial::new(1.0, x_vars[&[i, j]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });

    model.set_math_linear_objective(total_cost, ObjectiveCategory::Minimum, "cost")?;

    for j in 0..n {
        model.add_math_inequality(
            supply[j].clone().ge(plans[j].demand),
            &format!("demand_{}", plans[j].month),
        );
    }

    for i in 0..n {
        model.add_math_inequality(
            produce[i].clone().le(plans[i].productivity),
            &format!("productivity_{}", plans[i].month),
        );
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
