//! 物品优先级反向限制 / Item priority reverse limits
use crate::framework::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use std::error::Error;
use std::sync::Arc;

/// 物品优先级反转限制: 低优先级货物不能在高优先级之前装载 / Item priority reverse limit: lower-priority cargos cannot load before higher-priority ones
/// 对齐 Kotlin ItemPriorityReverseLimit
///
/// Kotlin 使用 model.minimize(sum(unloading.itemPriorityReverse)) 最小化优先级反转。
///
/// 简化实现: 使用 LinearExpressionSymbol 创建每个货物的装载状态符号，
/// 然后添加约束: 如果高优先级货物未装载，低优先级也不能装载。
/// 使用跨所有位置的总和约束，而非逐位置约束。
pub fn apply_item_priority_reverse_limits(
    model: &mut MetaModel<f64>,
    context: &ExpressEffectivenessContext<'_>,
    aggregation: &ExpressEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    let pos_count = context.request.positions.len();
    let mut next_id = 70000u64;

    // 为每个货物创建 loaded_count 中间符号: loaded_count[c] = sum_p x[c][p]
    let mut loaded_count_idx: Vec<usize> = Vec::with_capacity(context.request.cargos.len());
    for c in 0..context.request.cargos.len() {
        let monomials: Vec<LinearMonomial<f64>> = (0..pos_count)
            .map(|p| LinearMonomial::new(1.0, context.x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("priority_reverse_loaded_{}_{}", mode_name(context.mode), c),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        loaded_count_idx.push(next_id as usize);
        next_id += 1;
    }

    // 对于非 must_ship 的货物，如果同目的地的 must_ship 货物未装载，则也不能装载
    // 使用跨所有位置的总和: sum_p x[c_low][p] <= N * sum_p x[c_high][p]
    // 当 c_high 未装载时 (sum=0)，c_low 也不能装载 (sum=0)
    let non_must_ship: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| !aggregation.must_ship_indices.contains(c))
        .collect();

    let n = pos_count as f64;

    for &c_low in &non_must_ship {
        for &c_high in &aggregation.must_ship_indices {
            if context.request.cargos[c_low].destination
                == context.request.cargos[c_high].destination
            {
                // sum_p x[c_low][p] - N * sum_p x[c_high][p] <= 0
                let mut coefficients: Vec<(usize, f64)> = (0..pos_count)
                    .map(|p| (context.x_idx[c_low][p], 1.0))
                    .collect();
                coefficients.push((loaded_count_idx[c_high], -n));

                model.add_linear_constraint(
                    &coefficients,
                    ConstraintRelation::LessEqual,
                    0.0,
                    &format!(
                        "express_priority_reverse_{}_{}",
                        mode_name(context.mode),
                        c_low,
                    ),
                )?;
            }
        }
    }

    Ok(())
}
