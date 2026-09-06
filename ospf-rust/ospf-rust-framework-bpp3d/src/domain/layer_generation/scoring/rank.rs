fn score_layer_coverage<V, U>(
    request: &LayerGenerationRequest<V, U>,
    coverage: &[Bpp3dLayerDemandCoverage],
) -> (Option<V>, Option<f64>)
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    let mut shadow_sum = V::zero();
    let mut has_shadow = false;
    for entry in coverage {
        let key = DemandShadowPriceKey {
            mode: entry.mode,
            key: entry.key.clone(),
        };
        if let Some(price) = request.shadow_prices.get(&key) {
            shadow_sum = shadow_sum + *price * V::from(entry.coefficient).unwrap_or_else(V::zero);
            has_shadow = true;
        }
    }
    if has_shadow {
        (Some(shadow_sum), shadow_sum.to_f64())
    } else {
        (None, None)
    }
}

fn rank_layer_results<V, U>(results: &mut [LayerGenerationResult<V, U>])
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    results.sort_by(|lhs, rhs| {
        rhs.numeric_score
            .partial_cmp(&lhs.numeric_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| rhs.block_traces.len().cmp(&lhs.block_traces.len()))
            .then_with(|| rhs.placement_traces.len().cmp(&lhs.placement_traces.len()))
    });
}

