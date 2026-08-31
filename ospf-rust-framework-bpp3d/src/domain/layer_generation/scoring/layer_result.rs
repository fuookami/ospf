fn placement_set_to_layer_result<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    strategy: &str,
    bin: &BinType<V, U>,
    blocks: &[Block<V, U>],
    indexed_items: &[(usize, ActualItem<V, U>)],
    placements: Vec<BlockPlacement<V, U>>,
    coverage_coefficient: f64,
) -> Option<LayerGenerationResult<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let mut block_traces = Vec::new();
    for placement in placements {
        append_block_traces(
            request,
            blocks.get(placement.block_index)?,
            indexed_items,
            &placement.position,
            &mut block_traces,
        );
    }
    if block_traces.is_empty() {
        return None;
    }
    if !block_traces_allow_package_rules(request, bin, &block_traces) {
        return None;
    }
    let mut coverage_by_item = HashMap::<ItemId, f64>::new();
    let mut max_depth = V::zero();
    for trace in &block_traces {
        *coverage_by_item.entry(trace.item_id.clone()).or_default() +=
            trace.item_count.to_f64().unwrap_or(0.0) * coverage_coefficient;
        let depth = trace.origin.z.value + trace.size.depth.value;
        if depth > max_depth {
            max_depth = depth;
        }
    }
    let mut demand_coverage = coverage_by_item
        .into_iter()
        .map(|(id, coefficient)| {
            Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id },
                coefficient,
            )
        })
        .collect::<Vec<_>>();
    demand_coverage.sort_by(|lhs, rhs| format!("{:?}", lhs.key).cmp(&format!("{:?}", rhs.key)));
    let placement_traces = block_traces
        .iter()
        .map(|trace| LayerPlacementTrace {
            item_index: trace.item_index,
            item_id: trace.item_id.clone(),
            position: trace.origin.clone(),
            orientation: trace.orientation,
            amount: trace.item_count,
        })
        .collect::<Vec<_>>();
    let (score, numeric_score) = score_layer_coverage(request, &demand_coverage);
    let covered_amount = demand_coverage
        .iter()
        .map(|coverage| coverage.coefficient)
        .sum::<f64>();
    let placed_blocks = block_traces.len();
    Some(LayerGenerationResult {
        layer: BinLayer {
            iteration: request.iteration,
            from: generator_name.to_string(),
            bin: Some(bin.clone()),
            depth: Quantity::new_ct(max_depth),
            demand_coverage,
        },
        reduced_cost: None,
        score,
        numeric_score: Some(numeric_score.unwrap_or(0.0) + covered_amount),
        block_traces,
        placement_traces,
        diagnostics: vec![format!(
            "{} layer generated block-loading candidate: placed_blocks={}, covered_amount={}, has_bin=true",
            strategy,
            placed_blocks,
            covered_amount,
        )],
        source: generator_name.to_string(),
    })
}

