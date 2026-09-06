fn pile_layer_candidates<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    config: &DeferredLayerGeneratorConfig,
) -> Vec<LayerGenerationResult<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let Some(bin) = request.bin.clone() else {
        return Vec::new();
    };
    let mut results = request
        .items
        .iter()
        .enumerate()
        .filter_map(|(item_index, item)| {
            let package_attribute = request.package_attributes.get(&item.id);
            if let Some(PackageShapeSpec::Cylinder { axis, .. }) = &item.shape_spec_override {
                if *axis != Axis3::Y {
                    return None;
                }
            }
            if item.height.value <= V::zero() {
                return None;
            }
            if !item.enabled_orientations_at_bin(package_attribute, &bin).contains(&Orientation::Upright)
                || !package_rule_policy_allows_orientation(
                    request,
                    item,
                    package_attribute,
                    Orientation::Upright,
                    &bin,
                )
            {
                return None;
            }
            let remaining = item_remaining_amount(request, item_index, &item.id);
            let max_by_height = (bin.height.value / item.height.value)
                .floor()
                .to_u64()
                .unwrap_or(0);
            let max_by_piling = package_attribute
                .and_then(|attribute| attribute.max_stack_layers)
                .unwrap_or(config.pattern.single_pile_limit().unwrap_or(u64::MAX));
            let mut stack_count = remaining.min(max_by_height).min(max_by_piling);
            if let Some(attribute) = package_attribute {
                while stack_count > 0
                    && !same_item_pile_allows_stacking(
                        attribute,
                        item,
                        Orientation::Upright,
                        &item.width.value,
                        &item.height.value,
                        &item.depth.value,
                        &bin,
                        stack_count,
                        item_index,
                        request.package_rule_policy.as_ref(),
                    )
                {
                    stack_count -= 1;
                }
            }
            if stack_count <= 1 {
                return None;
            }
            let demand_key = Bpp3dDemandKey::Item {
                id: item.id.clone(),
            };
            let coverage = vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                demand_key,
                stack_count.to_f64().unwrap_or(0.0) * config.coverage_coefficient,
            )];
            let placement_traces = (0..stack_count)
                .map(|layer| LayerPlacementTrace {
                    item_index,
                    item_id: item.id.clone(),
                    position: MetricPoint3 {
                        x: Quantity::new_ct(V::zero()),
                        y: Quantity::new_ct(item.height.value * V::from(layer).unwrap_or_else(V::zero)),
                        z: Quantity::new_ct(V::zero()),
                    },
                    orientation: Orientation::Upright,
                    amount: 1,
                })
                .collect::<Vec<_>>();
            let (score, numeric_score) = score_layer_coverage(request, &coverage);
            Some(LayerGenerationResult {
                layer: BinLayer {
                    iteration: request.iteration,
                    from: generator_name.to_string(),
                    bin: Some(bin.clone()),
                    depth: item.depth.clone(),
                    demand_coverage: coverage,
                },
                reduced_cost: None,
                score,
                numeric_score: Some(numeric_score.unwrap_or(0.0) + stack_count.to_f64().unwrap_or(0.0)),
                block_traces: Vec::new(),
                placement_traces,
                diagnostics: vec![format!(
                    "pile layer generated vertical stacking candidate: item_id={}, stack_count={}, max_by_height={}",
                    item.id,
                    stack_count,
                    max_by_height,
                )],
                source: generator_name.to_string(),
            })
        })
        .collect::<Vec<_>>();
    rank_layer_results(&mut results);
    results.truncate(request.max_candidates.min(config.max_candidates.max(1)));
    results
}

