use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::variable::UIntegerVariableItem;

use crate::framework::demo3::domain::{CuttingPlan, Product};

pub struct Sp;

impl Sp {
    pub fn new() -> Self {
        Self
    }

    pub fn solve(
        &self,
        stock_length: u64,
        products: &[Product],
        shadow_prices: &[f64],
    ) -> Result<(CuttingPlan, f64), Box<dyn Error>> {
        let mut model = MetaModel::<f64>::new("framework_demo3_sp");
        let mut y_idx = vec![0usize; products.len()];
        for (p, _) in products.iter().enumerate() {
            y_idx[p] = model.register_variable(UIntegerVariableItem::auto(&format!("y_{}", p)))?;
        }

        let mut objective = vec![0.0; model.num_tokens()];
        for (p, idx) in y_idx.iter().enumerate() {
            objective[*idx] = shadow_prices.get(p).copied().unwrap_or(0.0);
        }
        model.set_linear_objective(objective, ObjectiveCategory::Maximum);

        let length_coefficients: Vec<(usize, f64)> = products
            .iter()
            .enumerate()
            .map(|(p, product)| (y_idx[p], product.length as f64))
            .collect();
        model.add_linear_constraint(
            &length_coefficients,
            ConstraintRelation::LessEqual,
            stock_length as f64,
            "length",
        )?;

        let solver = GurobiSolver::new();
        let output = model.solve(&solver)?;
        let solution = output
            .solution
            .ok_or_else(|| String::from("sp has no feasible solution"))?;

        let amounts: Vec<u64> = y_idx
            .iter()
            .map(|idx| solution.get(*idx).copied().unwrap_or(0.0).round() as u64)
            .collect();
        let score = output.objective_value.unwrap_or(0.0);
        let reduced_cost = 1.0 - score;
        Ok((CuttingPlan::new(amounts), reduced_cost))
    }
}
