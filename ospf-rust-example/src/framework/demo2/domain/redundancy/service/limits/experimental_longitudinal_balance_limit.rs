//! 实验性纵向平衡限制 / Experimental longitudinal balance limits
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::flatten::LinearMonomial;
use std::error::Error;

/// 实验性纵向平衡限制: 最小化纵向力矩偏差松弛 / Experimental longitudinal balance limit: minimize longitudinal torque deviation slack
/// 对齐 Kotlin ExperimentalLongitudinalBalanceLimit
///
/// Kotlin: model.minimize(coefficient * longitudinalBalance.longitudinalTorqueSlack)
/// 其中 longitudinalTorqueSlack = SlackFunction(mainActualLongitudinalTorque, predicateLongitudinalTorque)
///
/// 使用有物理边界的 epigraph 最小化力矩偏差。 / Minimize moment deviation through a physically bounded epigraph.
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
            let coeff =
                context.request.cargos[c].weight * context.request.positions[p].longitudinal_arm;
            moment_monomials.push(LinearMonomial::new(coeff, context.x_idx[c][p]));
        }
    }

    let terms = moment_monomials.iter().map(|term| (term.var_index(), *term.coefficient())).collect::<Vec<_>>();
    let slack_idx = crate::framework::demo2::domain::shared::deviation::register(
        model, &terms, target, &format!("exp_long_slack_{}", mode_name(context.mode)))?;

    // 最小化 exp_long_slack
    let obj = LinearObjectiveInput::minimize(&format!(
        "experimental_longitudinal_balance_{}",
        mode_name(context.mode)
    ))
    .terms(std::iter::once((slack_idx, 1.0)));
    model.add_linear_objective_input(obj);

    Ok(())
}
