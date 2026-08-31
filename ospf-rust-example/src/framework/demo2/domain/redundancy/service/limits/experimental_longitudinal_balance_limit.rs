use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackFunction;
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 实验性纵向平衡限制: 最小化纵向力矩偏差松弛
/// 对齐 Kotlin ExperimentalLongitudinalBalanceLimit
///
/// Kotlin: model.minimize(coefficient * longitudinalBalance.longitudinalTorqueSlack)
/// 其中 longitudinalTorqueSlack = SlackFunction(mainActualLongitudinalTorque, predicateLongitudinalTorque)
///
/// Rust 实现: 使用 SlackFunction 创建 slack = |moment - target|，
/// 然后最小化 slack 的结果变量。
pub fn apply_experimental_longitudinal_balance_limits(
    model: &mut MetaModel<f64>,
    context: &RedundancyContext<'_>,
    _aggregation: &RedundancyAggregation,
) -> Result<(), Box<dyn Error>> {
    let target = context.request.target_longitudinal_moment;

    // 构建 long_moment 线性表达式
    let mut moment_monomials: Vec<LinearMonomial<f64>> = Vec::new();
    for p in 0..context.request.positions.len() {
        for c in 0..context.request.cargos.len() {
            let coeff = context.request.cargos[c].weight
                * context.request.positions[p].longitudinal_arm;
            moment_monomials.push(LinearMonomial::new(coeff, context.x_idx[c][p]));
        }
    }

    // 使用 SlackFunction: slack = |moment - target|
    // 对齐 Kotlin: longitudinalTorqueSlack = SlackFunction(mainActualLongitudinalTorque, predicateLongitudinalTorque)
    let moment_linear = Linear::new(moment_monomials, 0.0);
    let target_linear = Linear::new(vec![], target);
    let slack_fn = SlackFunction::named(
        &format!("exp_long_slack_{}", mode_name(context.mode)),
        moment_linear,
        target_linear,
    );
    let slack_idx = slack_fn.result_variable().index();
    model.add_symbol(Arc::new(slack_fn))?;

    // 最小化 exp_long_slack
    let obj = LinearObjectiveInput::minimize(
        &format!("experimental_longitudinal_balance_{}", mode_name(context.mode)),
    ).terms(std::iter::once((slack_idx, 1.0)));
    model.add_linear_objective_input(obj);

    Ok(())
}