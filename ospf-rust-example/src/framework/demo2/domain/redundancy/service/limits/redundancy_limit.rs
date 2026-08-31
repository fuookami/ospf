use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackFunction;
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 冗余限制: 最小化冗余松弛变量
/// 对齐 Kotlin RedundancyLimit
///
/// Kotlin: model.minimize(coefficient * redundancy.redundancySlack)
/// 其中 redundancySlack = SlackFunction(mainDeckCapacity, actualLoad)
///
/// Rust 实现: 使用 SlackFunction 创建 slack = |capacity - load|，
/// 然后最小化 slack 的结果变量。
pub fn apply_redundancy_limits(
    model: &mut MetaModel<f64>,
    context: &RedundancyContext<'_>,
    _aggregation: &RedundancyAggregation,
) -> Result<(), Box<dyn Error>> {
    let pos_count = context.request.positions.len();
    let cargo_count = context.request.cargos.len();

    // 构建 capacity (left) 和 load (right) 的 Linear 表达式
    // capacity = sum(positions[p].max_weight for main_deck positions)
    // load = sum(cargo_weight * x[c][p] for main_deck positions)
    // Kotlin: redundancy = sum(-1 for reserved) + sum(-loaded[i] for optional)
    // 简化: slack = |sum(capacity_p) - sum(weight_c * x[c][p])|
    let mut capacity_monomials: Vec<LinearMonomial<f64>> = Vec::new();
    let mut load_monomials: Vec<LinearMonomial<f64>> = Vec::new();
    let mut total_capacity = 0.0_f64;

    for p in 0..pos_count {
        let capacity = context.request.positions[p].max_weight;
        total_capacity += capacity;
        for c in 0..cargo_count {
            load_monomials.push(LinearMonomial::new(
                context.request.cargos[c].weight,
                context.x_idx[c][p],
            ));
        }
    }

    // 使用 SlackFunction: slack = |capacity_linear - load_linear|
    // 对齐 Kotlin: redundancySlack = SlackFunction(redundancy, minRedundancy)
    let capacity_linear = Linear::new(vec![], total_capacity);
    let load_linear = Linear::new(load_monomials, 0.0);
    let slack_fn = SlackFunction::named(
        &format!("redundancy_slack_{}", mode_name(context.mode)),
        capacity_linear,
        load_linear,
    );
    let slack_idx = slack_fn.result_variable().index();
    model.add_symbol(Arc::new(slack_fn))?;

    // 最小化 redundancy_slack
    let obj = LinearObjectiveInput::minimize(
        &format!("redundancy_{}", mode_name(context.mode)),
    ).terms(std::iter::once((slack_idx, 1.0)));
    model.add_linear_objective_input(obj);

    Ok(())
}