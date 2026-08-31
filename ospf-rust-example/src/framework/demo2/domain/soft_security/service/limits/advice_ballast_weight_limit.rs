use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackFunction;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 建议压舱物重量限制
/// 对齐 Kotlin AdviceBallastWeightLimit
///
/// Kotlin 使用 exampleThresholdSlack(ballastWeight, adviceBallastWeight) 创建松弛函数，
/// 然后 model.minimize(slack) 最小化偏差。
///
/// 简化实现: 使用 SlackFunction 计算实际装载重量与建议压舱物重量之间的偏差，
/// 然后添加最小化目标。
pub fn apply_advice_ballast_weight_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    // 计算实际总装载重量: sum(cargos[c].weight * x[c][p]) for all c, p
    let load_weight_monomials: Vec<LinearMonomial<f64>> = (0..context.request.cargos.len())
        .flat_map(|c| {
            (0..context.request.positions.len()).map(move |p| {
                LinearMonomial::new(context.request.cargos[c].weight, context.x_idx[c][p])
            })
        })
        .collect();
    let load_weight = Linear::new(load_weight_monomials, 0.0);

    // 建议压舱物重量阈值: 使用 payload_upper_bound 作为代理
    let advice_weight = context.request.payload_upper_bound;

    // 创建 SlackFunction: slack = |load_weight - advice_weight|
    // 对齐 Kotlin exampleThresholdSlack
    let slack = SlackFunction::named(
        &format!(
            "soft_security_advice_ballast_weight_slack_{}",
            mode_name(context.mode)
        ),
        load_weight,
        Linear::new(vec![], advice_weight),
    );
    let slack_idx = slack.result_variable().index();
    model.add_symbol(Arc::new(slack))?;

    // 最小化松弛值: model.minimize(slack, "advice ballast weight")
    let obj_input = LinearObjectiveInput::minimize(&format!(
        "soft_security_advice_ballast_weight_{}",
        mode_name(context.mode)
    ))
    .term(slack_idx, 1.0);
    model.add_linear_objective_input(obj_input);

    Ok(())
}
