use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 冗余限制: 每个舱位预留一定冗余空间
/// 对齐 Kotlin RedundancyLimit
pub fn apply_redundancy_limits(
    model: &mut MetaModel<f64>,
    context: &RedundancyContext<'_>,
    _aggregation: &RedundancyAggregation,
) -> Result<(), Box<dyn Error>> {
    // 冗余约束: 每个舱位的装载量 <= max_weight * (1 - redundancy_ratio)
    // redundancy_ratio 默认为 0.1 (10% 冗余)
    let redundancy_ratio = 0.1;
    for p in 0..context.request.positions.len() {
        let max_with_redundancy = context.request.positions[p].max_weight * (1.0 - redundancy_ratio);
        let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
            .map(|c| (context.x_idx[c][p], context.request.cargos[c].weight))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            max_with_redundancy,
            &format!("redundancy_limit_{}_{}", mode_name(context.mode), p),
        )?;
    }
    Ok(())
}
