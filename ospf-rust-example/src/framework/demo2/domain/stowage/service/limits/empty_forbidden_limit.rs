use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 空禁限制: 禁止放空的舱位必须装载至少一个货物
/// 对齐 Kotlin EmptyForbiddenLimit
pub fn apply_empty_forbidden_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // 在 FullLoad 模式下，所有舱位必须装载至少一个货物
    if context.mode == crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode::FullLoad {
        for p in 0..context.request.positions.len() {
            let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
                .map(|c| (stowage_vars.stowage[c][p], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::GreaterEqual,
                1.0,
                &format!("stowage_empty_forbidden_{}_{}", mode_name(context.mode), p),
            )?;
        }
    }
    Ok(())
}
