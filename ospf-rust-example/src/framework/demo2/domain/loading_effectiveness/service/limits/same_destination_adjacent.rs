//! 同目的地邻接限制 / Same destination adjacent limits
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;

/// 同目的地邻接限制: 最大化同目的地货物的邻接性 / Same-destination adjacent limit: maximize same-destination cargo adjacency
///
/// Aligned with Kotlin SameDestinationAdjacent + TransferAdjacentLoading.
/// For each destination and each adjacent position pair, creates an IfFunction
/// indicator that equals 1 when the combined load amount of that destination's
/// cargo at both positions is nonzero (condition != 0). The sum of all indicators
/// is added as a maximization objective to encourage same-destination cargo
/// to be placed at adjacent positions.
pub fn apply_same_destination_adjacent_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    let adjacent_positions = &context.request.adjacent_positions;
    if adjacent_positions.is_empty() {
        return Ok(());
    }

    // Group cargos by destination (aligned with Kotlin TransferAdjacentLoading.destinations)
    let mut cargos_by_destination: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for c in 0..context.request.cargos.len() {
        cargos_by_destination
            .entry(context.request.cargos[c].destination.clone())
            .or_default()
            .push(c);
    }

    let mut next_id = 50000u64;
    let mut objective_terms: Vec<(usize, f64)> = Vec::new();

    for (_dest, cargos) in &cargos_by_destination {
        if cargos.len() <= 1 {
            continue;
        }

        for pair in adjacent_positions.iter() {
            // loadAmount1 = sum(x_idx[c][pair.first] for c in destination group)
            let mut load_amount_1_monomials: Vec<LinearMonomial<f64>> = Vec::new();
            for &c in cargos {
                if pair.first < context.x_idx[c].len() {
                    load_amount_1_monomials
                        .push(LinearMonomial::new(1.0, context.x_idx[c][pair.first]));
                }
            }

            // loadAmount2 = sum(x_idx[c][pair.second] for c in destination group)
            let mut load_amount_2_monomials: Vec<LinearMonomial<f64>> = Vec::new();
            for &c in cargos {
                if pair.second < context.x_idx[c].len() {
                    load_amount_2_monomials
                        .push(LinearMonomial::new(1.0, context.x_idx[c][pair.second]));
                }
            }

            // condition = loadAmount1 + loadAmount2 - 2
            // Aligned with Kotlin: IfFunction(condition = loadAmount1 + loadAmount2 - Flt64.two)
            let mut condition_monomials = load_amount_1_monomials;
            condition_monomials.extend(load_amount_2_monomials);

            let condition = Linear::new(condition_monomials, -2.0);
            let then_expr = Linear::new(Vec::new(), 1.0);
            let else_expr = Linear::new(Vec::new(), 0.0);

            let if_fn = IfFunction::new(
                next_id,
                &format!(
                    "same_destination_adjacent_{}_{}_{}",
                    _dest, pair.first, pair.second
                ),
                condition,
                then_expr,
                else_expr,
            );
            let result_idx = if_fn.result_variable().index();
            model.add_symbol(Arc::new(if_fn))?;
            objective_terms.push((result_idx, 1.0));
            next_id += 1;
        }
    }

    if !objective_terms.is_empty() {
        model.add_linear_objective(&objective_terms, "same_destination_adjacent");
    }

    Ok(())
}
