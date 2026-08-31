fn pattern_placements_to_layer_result<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    group_key: &str,
    bin: &BinType<V, U>,
    placements: Vec<PatternPlanePlacement<V>>,
    config: &DeferredLayerGeneratorConfig,
) -> Option<LayerGenerationResult<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let mut coverage_by_item = HashMap::<String, f64>::new();
    let mut max_depth = V::zero();
    let mut max_height = V::zero();
    for placement in &placements {
        *coverage_by_item.entry(placement.item_id.clone()).or_default() +=
            placement.amount.to_f64().unwrap_or(0.0) * config.coverage_coefficient;
        let depth = placement.z + placement.depth;
        if depth > max_depth {
            max_depth = depth;
        }
        if placement.height > max_height {
            max_height = placement.height;
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
    let placement_traces = placements
        .iter()
        .map(|placement| LayerPlacementTrace {
            item_index: placement.item_index,
            item_id: placement.item_id.clone(),
            position: MetricPoint3 {
                x: Quantity::new_ct(placement.x),
                y: Quantity::new_ct(placement.y),
                z: Quantity::new_ct(placement.z),
            },
            orientation: placement.orientation,
            amount: placement.amount,
        })
        .collect::<Vec<_>>();
    let block_traces = placements
        .iter()
        .enumerate()
        .map(|(index, placement)| LayerBlockTrace {
            block_index: index,
            item_index: placement.item_index,
            item_id: placement.item_id.clone(),
            orientation: placement.orientation,
            nx: 1,
            ny: placement.amount,
            nz: 1,
            item_count: placement.amount,
            size: MetricSize3 {
                width: Quantity::new_ct(placement.width),
                height: Quantity::new_ct(placement.height),
                depth: Quantity::new_ct(placement.depth),
            },
            origin: MetricPoint3 {
                x: Quantity::new_ct(placement.x),
                y: Quantity::new_ct(placement.y),
                z: Quantity::new_ct(placement.z),
            },
        })
        .collect::<Vec<_>>();
    let (score, numeric_score) = score_layer_coverage(request, &demand_coverage);
    let covered_amount = demand_coverage
        .iter()
        .map(|coverage| coverage.coefficient)
        .sum::<f64>();
    let placed_amount = placements
        .iter()
        .map(|placement| placement.amount)
        .sum::<u64>();
    let placed_piles = placements
        .iter()
        .map(|placement| placement.pile_index)
        .collect::<std::collections::HashSet<_>>()
        .len();
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
        diagnostics: vec![
            format!(
                "pattern layer generated step placement candidate: group={}, placed_piles={}, placed_items={}, max_height={:?}",
                group_key,
                placed_piles,
                placed_amount,
                max_height,
            ),
            format!(
                "pattern config: with_piling={:?}, with_remainder={}, two_sum={}, three_sum={}",
                config.pattern.with_piling,
                config.pattern.with_remainder,
                config.pattern.enables_two_sum(),
                config.pattern.enables_three_sum(),
            ),
        ],
        source: generator_name.to_string(),
    })
}
