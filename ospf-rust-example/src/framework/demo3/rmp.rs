use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::variable::{UContinuousVariableItem, UIntegerVariableItem};

use crate::framework::demo3::domain::{CuttingPlan, Product};

pub struct LpResultData {
    pub objective: f64,
    pub shadow_prices: Vec<f64>,
}

pub struct Rmp {
    products: Vec<Product>,
    plans: Vec<CuttingPlan>,
}

impl Rmp {
    pub fn new(products: Vec<Product>, initial_plans: Vec<CuttingPlan>) -> Self {
        Self {
            products,
            plans: initial_plans,
        }
    }

    pub fn add_column_if_new(&mut self, plan: CuttingPlan) -> bool {
        if self.plans.iter().any(|existing| existing == &plan) {
            return false;
        }
        self.plans.push(plan);
        true
    }

    pub fn solve_lp(&self) -> Result<LpResultData, Box<dyn Error>> {
        let (model, _) = self.build_model(false)?;
        let solver = GurobiSolver::new();
        let output = model.solve(&solver)?;

        let objective = output.objective_value.unwrap_or(0.0);
        let dual = output.dual_solution.unwrap_or_default();
        let mut shadow_prices = vec![0.0; self.products.len()];
        for (i, slot) in shadow_prices.iter_mut().enumerate() {
            if let Some(value) = dual.get(i) {
                *slot = *value;
            }
        }

        Ok(LpResultData {
            objective,
            shadow_prices,
        })
    }

    pub fn solve_milp(&self) -> Result<Vec<(CuttingPlan, u64)>, Box<dyn Error>> {
        let (model, x_idx) = self.build_model(true)?;
        let solver = GurobiSolver::new();
        let output = model.solve(&solver)?;
        let solution = output
            .solution
            .ok_or_else(|| String::from("rmp milp has no feasible solution"))?;

        let mut ret = Vec::new();
        for (k, plan) in self.plans.iter().enumerate() {
            let value = solution.get(x_idx[k]).copied().unwrap_or(0.0);
            if value >= 1e-6 {
                ret.push((plan.clone(), value.round() as u64));
            }
        }
        Ok(ret)
    }

    fn build_model(&self, integral: bool) -> Result<(MetaModel<f64>, Vec<usize>), Box<dyn Error>> {
        let mut model = MetaModel::<f64>::new("framework_demo3_rmp");
        let mut x_idx = Vec::with_capacity(self.plans.len());
        for (k, _) in self.plans.iter().enumerate() {
            let idx = if integral {
                model.register_variable(UIntegerVariableItem::auto(&format!("x_{}", k)))?
            } else {
                model.register_variable(UContinuousVariableItem::auto(&format!("x_{}", k)))?
            };
            x_idx.push(idx);
        }

        let objective: Vec<f64> = {
            let mut coeff = vec![0.0; model.num_tokens()];
            for idx in &x_idx {
                coeff[*idx] = 1.0;
            }
            coeff
        };
        model.set_linear_objective(objective, ObjectiveCategory::Minimum);

        for (p, product) in self.products.iter().enumerate() {
            let coefficients: Vec<(usize, f64)> = self
                .plans
                .iter()
                .enumerate()
                .map(|(k, plan)| (x_idx[k], plan.amounts[p] as f64))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::GreaterEqual,
                product.demand as f64,
                &format!("demand_{}", p),
            )?;
        }

        Ok((model, x_idx))
    }
}
