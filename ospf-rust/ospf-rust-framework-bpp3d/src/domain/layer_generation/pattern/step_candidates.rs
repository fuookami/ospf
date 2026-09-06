fn pattern_step_layer_candidates<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    group_key: &str,
    indexed_items: &[(usize, ActualItem<V, U>)],
    config: &DeferredLayerGeneratorConfig,
) -> Vec<LayerGenerationResult<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let Some(bin) = request.bin.clone() else {
        return Vec::new();
    };
    let mut results = Vec::new();
    let mut seen_placement_signatures = HashSet::<String>::new();
    let max_pattern_candidates = request.max_candidates.min(config.max_candidates.max(1));
    for pattern in config.pattern.effective_patterns() {
        if pattern.is_empty() {
            continue;
        }
        let mut remaining = indexed_items
            .iter()
            .map(|(index, item)| (*index, item_remaining_amount(request, *index, &item.id)))
            .collect::<HashMap<_, _>>();
        let mut produced_for_pattern = 0usize;
        while produced_for_pattern < max_pattern_candidates {
            let mut layer_remaining = remaining.clone();
            let mut placements = Vec::<PatternPlanePlacement<V>>::new();
            let mut failed_full_pattern = false;
            for (step_index, step) in pattern.iter().enumerate() {
                let loaded_weight = placements
                    .iter()
                    .fold(V::zero(), |acc, placement| acc + placement.total_weight);
                if let Some(mixed_pile) = select_mixed_pattern_pile(
                    indexed_items,
                    &request.package_attributes,
                    request.package_rule_policy.as_ref(),
                    &layer_remaining,
                    step,
                    &bin,
                    loaded_weight,
                    config,
                ) {
                    let Some((x, z)) = next_pattern_point(step, &placements, &mixed_pile.depth) else {
                        failed_full_pattern = true;
                        break;
                    };
                    if x + mixed_pile.width > bin.width.value
                        || z + mixed_pile.depth > bin.depth.value
                        || mixed_pile.height > bin.height.value
                    {
                        failed_full_pattern = true;
                        break;
                    }
                    let pile_index = placements.len();
                    let mut y = V::zero();
                    for unit in mixed_pile.units {
                        placements.push(PatternPlanePlacement {
                            pile_index,
                            item_index: unit.item_index,
                            item_id: unit.item_id.clone(),
                            orientation: unit.orientation,
                            x,
                            y,
                            z,
                            width: unit.width,
                            depth: unit.depth,
                            height: unit.height,
                            amount: 1,
                            total_weight: unit.weight,
                        });
                        y = y + placements.last().map(|placement| placement.height).unwrap_or_else(V::zero);
                        if let Some(amount) = layer_remaining.get_mut(&unit.item_index) {
                            *amount = amount.saturating_sub(1);
                        }
                    }
                    if step_index + 1 == pattern.len() {
                        failed_full_pattern = false;
                    }
                    continue;
                }
                let Some((item_index, item, orientation, width, depth, item_height)) =
                    select_pattern_item(
                        indexed_items,
                        &request.package_attributes,
                        request.package_rule_policy.as_ref(),
                        &layer_remaining,
                        step,
                        &bin,
                        config,
                    )
                else {
                    failed_full_pattern = true;
                    break;
                };
                let Some((x, z)) = next_pattern_point(step, &placements, &depth) else {
                    failed_full_pattern = true;
                    break;
                };
                let amount = pattern_step_pile_amount(
                    item_index,
                    item,
                    request.package_attributes.get(&item.id),
                    request.package_rule_policy.as_ref(),
                    orientation,
                    &width,
                    &item_height,
                    &depth,
                    &bin,
                    layer_remaining.get(&item_index).copied().unwrap_or(0),
                    loaded_weight,
                    config,
                );
                if amount == 0 {
                    failed_full_pattern = true;
                    break;
                }
                let height = item_height * V::from(amount).unwrap_or_else(V::zero);
                if x + width > bin.width.value || z + depth > bin.depth.value || height > bin.height.value {
                    failed_full_pattern = true;
                    break;
                }
                placements.push(PatternPlanePlacement {
                    pile_index: placements.len(),
                    item_index,
                    item_id: item.id.clone(),
                    orientation,
                    x,
                    y: V::zero(),
                    z,
                    width,
                    depth,
                    height,
                    amount,
                    total_weight: item.weight.value * V::from(amount).unwrap_or_else(V::zero),
                });
                if let Some(amount) = layer_remaining.get_mut(&item_index) {
                    *amount = amount.saturating_sub(placements.last().map(|placement| placement.amount).unwrap_or(0));
                }
                if step_index + 1 == pattern.len() {
                    failed_full_pattern = false;
                }
            }
            if placements.is_empty() || (!config.pattern.with_remainder && failed_full_pattern) {
                break;
            }
            let signature = pattern_placement_signature(&placements);
            if let Some(result) = pattern_placements_to_layer_result(
                request,
                generator_name,
                group_key,
                &bin,
                placements,
                config,
            ) {
                if seen_placement_signatures.insert(signature) {
                    results.push(result);
                    produced_for_pattern += 1;
                }
                remaining = layer_remaining;
            } else {
                break;
            }
            if remaining.values().all(|amount| *amount == 0)
                || (config.pattern.with_remainder && failed_full_pattern)
            {
                break;
            }
        }
    }
    rank_layer_results(&mut results);
    results.truncate(request.max_candidates.min(config.max_candidates.max(1)));
    results
}

fn pattern_placement_signature<V>(placements: &[PatternPlanePlacement<V>]) -> String
where
    V: Debug + Clone + Send + Sync,
{
    placements
        .iter()
        .map(|placement| {
            format!(
                "{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}",
                placement.item_id,
                placement.orientation,
                placement.x,
                placement.y,
                placement.z,
                placement.width,
                placement.depth,
                placement.height,
                placement.amount,
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

