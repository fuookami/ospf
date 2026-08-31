use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{UContinuousVariableItem, UIntegerVariableItem};
use ospf_rust_framework::solver::{

    ColumnGenerationSolver, FrameworkSolveOptions, GurobiColumnGenerationSolver,
};

use crate::framework::demo3::domain::{CuttingPlan, Product};

/// LP 结果数据 / LP result data
pub struct LpResultData {
    pub objective: f64,
    pub shadow_prices: Vec<f64>,
}

/// 限制性主问题 / Restricted master problem
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
        let linear_model = model.try_to_linear_triad_model()?;
        let solver = GurobiColumnGenerationSolver::new();
        let output = solver.solve_lp_with_options(
            &linear_model,
            FrameworkSolveOptions::new().with_name("framework_demo3_rmp_lp"),
        )?;

        let objective = output.result.obj;
        let dual = output.dual_solution.constraints;
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
        let linear_model = model.try_to_linear_triad_model()?;
        let solver = GurobiColumnGenerationSolver::new();
        let output = solver.solve_milp_with_options(
            &linear_model,
            FrameworkSolveOptions::new().with_name("framework_demo3_rmp_milp"),
        )?;
        let solution = output.solution;

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

        let objective = MetaModel::linear_expression_builder()
            .terms(x_idx.iter().copied().map(|index| (index, 1.0)))
            .minimize("rmp_min_stock_used");
        model.set_linear_objective_input(objective);

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
