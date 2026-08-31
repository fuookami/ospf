use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework_demo::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework_demo::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_balance_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    if let Some(z_idx) = context.z {
        for p in 0..context.request.positions.len() {
            let load_coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
                .map(|c| (context.x_idx[c][p], context.request.cargos[c].weight))
                .collect();

            let mut upper = load_coefficients.clone();
            upper.push((z_idx, -1.0));
            model.add_linear_constraint(
                &upper,
                ConstraintRelation::LessEqual,
                aggregation.target_balance,
                &format!("mac_balance_up_{}_{}", mode_name(context.mode), p),
            )?;

            let mut lower: Vec<(usize, f64)> = load_coefficients
                .iter()
                .map(|(idx, coef)| (*idx, -*coef))
                .collect();
            lower.push((z_idx, -1.0));
            model.add_linear_constraint(
                &lower,
                ConstraintRelation::LessEqual,
                -aggregation.target_balance,
                &format!("mac_balance_down_{}_{}", mode_name(context.mode), p),
            )?;
        }
    }
    Ok(())
}
