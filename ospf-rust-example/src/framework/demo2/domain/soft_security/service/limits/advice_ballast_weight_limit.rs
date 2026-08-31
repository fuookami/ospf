use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 建议压舱物重量限制
/// 对齐 Kotlin AdviceBallastWeightLimit
///
/// 简化实现: 使用总装载重量作为压舱物重量的代理
pub fn apply_advice_ballast_weight_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    // 压舱物重量建议约束
    // 简化实现: 总装载重量不超过 max_weight * 0.8
    for p in 0..context.request.positions.len() {
        let advice_weight = context.request.positions[p].max_weight * 0.8;
        let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
            .map(|c| (context.x_idx[c][p], context.request.cargos[c].weight))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            advice_weight,
            &format!(
                "soft_security_advice_ballast_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
