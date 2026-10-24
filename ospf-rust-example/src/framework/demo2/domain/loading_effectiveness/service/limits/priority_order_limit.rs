//! 优先级排序限制 / Priority order limits
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 优先级顺序限制: 高优先级货物应在低优先级之前装载 / Priority order limit: higher-priority cargos should load before lower-priority ones
/// 对齐 Kotlin PriorityOrderLimit
pub fn apply_priority_order_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    for i in 0..context.request.cargos.len() {
        for j in 0..context.request.cargos.len() {
            let first = context.request.loading.cargos.get(&context.request.cargos[i].name).and_then(|policy| policy.order);
            let second = context.request.loading.cargos.get(&context.request.cargos[j].name).and_then(|policy| policy.order);
            if match (first, second) { (Some(first), Some(second)) => first >= second,
                _ => context.request.cargos[i].priority <= context.request.cargos[j].priority } {
                continue;
            }
            let scopes = context.request.loading.positions.values().map(|input| &input.loading_order_scope).collect::<std::collections::BTreeSet<_>>();
            for scope in scopes {
                let mut coefficients = Vec::new();
                let scoped = context.request.positions.iter().enumerate().filter(|(_, position)| &context.request.loading.positions[&position.name].loading_order_scope == scope).collect::<Vec<_>>();
                let min_order = scoped.iter().map(|(_, position)| context.request.loading.positions[&position.name].loading_order).min().unwrap_or(0) as f64;
                let max_order = scoped.iter().map(|(_, position)| context.request.loading.positions[&position.name].loading_order).max().unwrap_or(0) as f64;
                let big_m = (max_order - min_order).max(1.0);
                for (p, position) in scoped {
                    let order = context.request.loading.positions[&position.name].loading_order as f64 - min_order;
                    coefficients.push((context.x_idx[i][p], order + big_m));
                    coefficients.push((context.x_idx[j][p], -order + big_m));
                }
                model.add_linear_constraint(&coefficients, ConstraintRelation::LessEqual, 2.0 * big_m,
                    &format!("loading_priority_order_{}_{}_{}_{}", mode_name(context.mode), i, j, scope))?;
            }
        }
    }
    Ok(())
}
