//! 纵向平衡限制 / Longitudinal balance limits
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{ConstraintRelation, LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackFunction;
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 纵向平衡限制 / Longitudinal balance limit
/// 对齐 Kotlin LongitudinalBalanceLimit
///
/// Kotlin: model.minimize(sum(coefficient(range) * slack.value))
/// 其中 slack 是每个 MAC 范围类型的 SlackFunction 结果变量。
///
/// Rust 实现: 使用 SlackFunction 创建 slack = |long_moment - target|，
/// 然后最小化 slack 的结果变量。
pub fn apply_longitudinal_balance_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    let target = context.request.target_longitudinal_moment;

    // 构建 long_moment 线性表达式
    // 优先使用已注册中间符号，回退到裸系数
    let moment_linear = if let Some(ref sym) = aggregation.long_moment_symbol {
        // 使用已注册的 long_moment 符号构建 Linear
        use ospf_rust_core::symbol::LinearIntermediateSymbol;
        sym.to_linear_polynomial()
    } else {
        // 回退: 从裸系数构建
        let monomials: Vec<LinearMonomial<f64>> = aggregation.long_moment
            .iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        Linear::new(monomials, 0.0)
    };

    // 使用 SlackFunction: slack = |long_moment - target|
    // 对齐 Kotlin: slack = SlackFunction for each MAC range type
    let target_linear = Linear::new(vec![], target);
    let slack_fn = SlackFunction::named(
        &format!("long_balance_slack_{}", mode_name(context.mode)),
        moment_linear,
        target_linear,
    );
    let slack_idx = slack_fn.result_variable().index();
    model.add_symbol(Arc::new(slack_fn))?;

    // 最小化 long_balance_slack
    let obj = LinearObjectiveInput::minimize(
        &format!("longitudinal_balance_{}", mode_name(context.mode)),
    ).terms(std::iter::once((slack_idx, 1.0)));
    model.add_linear_objective_input(obj);

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