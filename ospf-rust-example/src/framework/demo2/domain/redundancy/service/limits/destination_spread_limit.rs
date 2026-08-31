//! 目的地分散限制 / Destination spread limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::redundancy::aggregation::RedundancyAggregation;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 目的地分散限制 / Destination spread limit
///
/// 同一目的地的货物应分散到不同舱位，每个舱位最多装载同一目的地的 n-1 件货物。
/// Cargos with the same destination should be spread across positions; each position can hold at most n-1 cargos of the same destination.
pub fn apply_destination_spread_limits(
    model: &mut MetaModel<f64>,
    context: &RedundancyContext<'_>,
    aggregation: &RedundancyAggregation,
) -> Result<(), Box<dyn Error>> {
    for (destination, cargos) in &aggregation.cargos_by_destination {
        if cargos.len() <= 1 {
            continue;
        }
        for p in 0..context.request.positions.len() {
            let coefficients: Vec<(usize, f64)> =
                cargos.iter().map(|c| (context.x_idx[*c][p], 1.0)).collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                (cargos.len() - 1) as f64,
                &format!(
                    "redundancy_destination_spread_{}_{}_{}",
                    mode_name(context.mode),
                    destination,
                    p
                ),
            )?;
        }
    }
    Ok(())
}
