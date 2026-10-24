//! 冗余范围限制 / Redundancy range limits
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::flatten::LinearMonomial;
use std::error::Error;

/// 冗余限制: 最小化冗余松弛变量 / Redundancy limit: minimize redundancy slack variable
/// 对齐 Kotlin RedundancyLimit
///
/// Kotlin: model.minimize(coefficient * redundancy.redundancySlack)
/// 其中 redundancySlack = SlackFunction(mainDeckCapacity, actualLoad)
///
/// 使用有物理边界的 epigraph 最小化绝对偏差。 / Minimize absolute deviation through a physically bounded epigraph.
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

    let terms = load_monomials.iter().map(|term| (term.var_index(), *term.coefficient())).collect::<Vec<_>>();
    let slack_idx = crate::framework::demo2::domain::shared::deviation::register(
        model, &terms, total_capacity, &format!("redundancy_slack_{}", mode_name(context.mode)))?;

    // 最小化 redundancy_slack
    let obj = LinearObjectiveInput::minimize(&format!("redundancy_{}", mode_name(context.mode)))
        .terms(std::iter::once((slack_idx, 1.0)));
    model.add_linear_objective_input(obj);

    Ok(())
}
