use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 实验性纵向平衡限制: 纵向力矩偏差约束
/// 对齐 Kotlin ExperimentalLongitudinalBalanceLimit
pub fn apply_experimental_longitudinal_balance_limits(
    model: &mut MetaModel<f64>,
    context: &RedundancyContext<'_>,
    _aggregation: &RedundancyAggregation,
) -> Result<(), Box<dyn Error>> {
    // 实验性纵向平衡: 纵向力矩偏差不超过目标值
    let target = context.request.target_longitudinal_moment;
    let max_deviation = context.request.max_longitudinal_moment_deviation;

    // 上限: sum(weight * longArm * x[c][p]) <= target + max_deviation
    let mut upper_coefficients: Vec<(usize, f64)> = Vec::new();
    let mut lower_coefficients: Vec<(usize, f64)> = Vec::new();
    for p in 0..context.request.positions.len() {
        for c in 0..context.request.cargos.len() {
            let coeff = context.request.cargos[c].weight
                * context.request.positions[p].longitudinal_arm;
            upper_coefficients.push((context.x_idx[c][p], coeff));
            lower_coefficients.push((context.x_idx[c][p], -coeff));
        }
    }

    model.add_linear_constraint(
        &upper_coefficients,
        ConstraintRelation::LessEqual,
        target + max_deviation,
        &format!(
            "redundancy_exp_long_upper_{}",
            mode_name(context.mode)
        ),
    )?;
    model.add_linear_constraint(
        &lower_coefficients,
        ConstraintRelation::LessEqual,
        -target + max_deviation,
        &format!(
            "redundancy_exp_long_lower_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
