use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework_demo::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework_demo::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_source_early_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    for (source, cargos) in &aggregation.cargos_by_source {
        if cargos.len() <= 1 {
            continue;
        }
        let coefficients: Vec<(usize, f64)> = cargos
            .iter()
            .flat_map(|c| (0..=aggregation.early_end).map(move |p| (context.x_idx[*c][p], 1.0)))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            1.0,
            &format!(
                "loading_source_early_{}_{}",
                mode_name(context.mode),
                source
            ),
        )?;
    }
    Ok(())
}
