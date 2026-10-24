//! 纵向平衡限制 / Longitudinal balance limits
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, LinearObjectiveInput, MetaModel};
use std::error::Error;

/// 纵向平衡限制 / Longitudinal balance limit
/// 对齐 Kotlin LongitudinalBalanceLimit
///
/// Kotlin: model.minimize(sum(coefficient(range) * slack.value))
/// 其中 slack 是每个 MAC 范围类型的 SlackFunction 结果变量。
///
/// 有界 epigraph 最小化绝对力矩偏差，无任意 Big-M。 / Bounded epigraph minimizes absolute moment deviation without arbitrary Big-M.
pub fn apply_longitudinal_balance_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    let target = context.request.target_longitudinal_moment;

    let slack_idx = crate::framework::demo2::domain::shared::deviation::register(
        model, &aggregation.long_moment, target, &format!("long_balance_slack_{}", mode_name(context.mode)))?;

    // 最小化 long_balance_slack
    let obj = LinearObjectiveInput::minimize(&format!(
        "longitudinal_balance_{}",
        mode_name(context.mode)
    ))
    .terms(std::iter::once((slack_idx, 1.0)));
    model.add_linear_objective_input(obj);

    // 载荷偏差约束 (仅 Predistribution/WeightRecommendation)
    if let Some(z_idx) = context.z {
        for p in 0..context.request.positions.len() {
            let load_coefficients = vec![(context.weights[p], 1.0)];

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
