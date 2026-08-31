//! 普通散货目的地分配限制 / Normal bulk destination assignment limits
use std::error::Error;
use std::collections::BTreeMap;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 普通散货目的地分配限制: 同一舱位的货物必须来自同一目的地
/// 对齐 Kotlin NormalBulkDestinationAssignmentLimit
///
/// 简化实现: 对于每个舱位，来自不同目的地的货物不能同时装载
pub fn apply_normal_bulk_destination_assignment_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // 按目的地分组
    let mut cargos_by_destination: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for c in 0..context.request.cargos.len() {
        cargos_by_destination
            .entry(context.request.cargos[c].destination.clone())
            .or_default()
            .push(c);
    }

    let destinations: Vec<&String> = cargos_by_destination.keys().collect();

    // 对于每对不同目的地，它们的货物不能在同一舱位
    for i in 0..destinations.len() {
        for j in (i + 1)..destinations.len() {
            let dest_i = &cargos_by_destination[destinations[i]];
            let dest_j = &cargos_by_destination[destinations[j]];
            for &c1 in dest_i {
                for &c2 in dest_j {
                    for p in 0..context.request.positions.len() {
                        model.add_linear_constraint(
                            &[
                                (stowage_vars.stowage[c1][p], 1.0),
                                (stowage_vars.stowage[c2][p], 1.0),
                            ],
                            ConstraintRelation::LessEqual,
                            1.0,
                            &format!(
                                "stowage_dest_assignment_{}_{}_{}_{}_{}",
                                mode_name(context.mode),
                                destinations[i],
                                destinations[j],
                                c1,
                                p
                            ),
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
