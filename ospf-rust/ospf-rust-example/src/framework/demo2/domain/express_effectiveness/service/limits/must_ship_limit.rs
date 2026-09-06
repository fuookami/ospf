//! 必装限制 / Must-ship limits
use crate::framework::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 必须装载限制: 高优先级货物必须装载 / Must-ship limit: high-priority cargos must be loaded
/// 对齐 Kotlin MustShipLimit
pub fn apply_must_ship_limits(
    model: &mut MetaModel<f64>,
    context: &ExpressEffectivenessContext<'_>,
    aggregation: &ExpressEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    for c in &aggregation.must_ship_indices {
        let coefficients: Vec<(usize, f64)> = (0..context.request.positions.len())
            .map(|p| (context.x_idx[*c][p], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!("express_must_ship_{}_{}", mode_name(context.mode), c),
        )?;
    }
    Ok(())
}
