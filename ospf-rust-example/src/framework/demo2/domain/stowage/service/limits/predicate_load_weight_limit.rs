use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::load::LoadVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 预测装载重量限制: y[j] <= position.max_weight
/// 对齐 Kotlin PredicateLoadWeightLimit
pub fn apply_predicate_load_weight_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    load_vars: &LoadVariables,
) -> Result<(), Box<dyn Error>> {
    // 预测装载重量不能超过舱位最大重量
    for p in 0..context.request.positions.len() {
        model.add_linear_constraint(
            &[(load_vars.y[p], 1.0)],
            ConstraintRelation::LessEqual,
            context.request.positions[p].max_weight,
            &format!(
                "stowage_predicate_load_weight_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
