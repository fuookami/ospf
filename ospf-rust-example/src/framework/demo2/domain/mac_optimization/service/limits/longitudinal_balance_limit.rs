use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 纵向平衡限制 / Longitudinal balance limit
/// 对齐 Kotlin LongitudinalBalanceLimit
///
/// 包含两种约束:
/// 1. 载荷偏差约束 (Predistribution/WeightRecommendation): |load - target| <= z
/// 2. 纵向力矩约束 (All modes): long_moment 在 target ± deviation 范围内
pub fn apply_longitudinal_balance_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    // 纵向力矩约束
    let mut long_upper = aggregation.long_moment.clone();
    let mut long_lower = aggregation.neg_long_moment.clone();
    if let Some(z_idx) = context.z {
        long_upper.push((z_idx, -aggregation.max_arm_abs));
        long_lower.push((z_idx, -aggregation.max_arm_abs));
    }
    model.add_linear_constraint(
        &long_upper,
        ConstraintRelation::LessEqual,
        context.request.target_longitudinal_moment
            + context.request.max_longitudinal_moment_deviation,
        &format!("mac_longitudinal_upper_{}", mode_name(context.mode)),
    )?;
    model.add_linear_constraint(
        &long_lower,
        ConstraintRelation::LessEqual,
        -context.request.target_longitudinal_moment
            + context.request.max_longitudinal_moment_deviation,
        &format!("mac_longitudinal_lower_{}", mode_name(context.mode)),
    )?;

    // 载荷偏差约束 (仅 Predistribution/WeightRecommendation)
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
