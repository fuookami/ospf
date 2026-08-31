fn pattern_step_accepts_dimensions<V>(
    step: &PatternStep,
    width: &V,
    depth: &V,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd,
{
    match step.length_orientation {
        PatternProjectionOrientation::Front => depth >= width,
        PatternProjectionOrientation::Side => depth <= width,
    }
}

fn pattern_step_pile_amount<V, U>(
    item_index: usize,
    item: &ActualItem<V, U>,
    package_attribute: Option<&PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    orientation: Orientation,
    item_width: &V,
    item_height: &V,
    item_depth: &V,
    bin: &BinType<V, U>,
    remaining: u64,
    loaded_weight: V,
    config: &DeferredLayerGeneratorConfig,
) -> u64
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    if remaining == 0 || *item_height <= V::zero() {
        return 0;
    }
    let rest_capacity = bin.capacity.value - loaded_weight;
    if rest_capacity <= V::zero() {
        return 0;
    }
    let max_by_height = (bin.height.value / *item_height).floor().to_u64().unwrap_or(0);
    let max_by_weight = if item.weight.value > V::zero() {
        (rest_capacity / item.weight.value).floor().to_u64().unwrap_or(0)
    } else {
        remaining
    };
    let max_by_piling = package_attribute
        .and_then(|attribute| {
            let orientation_enabled = item.enabled_orientations.is_empty()
                || item.enabled_orientations.contains(&orientation);
            attribute.max_layer_for_orientation(orientation, orientation_enabled)
        })
        .unwrap_or(config.pattern.single_pile_limit().unwrap_or(u64::MAX));
    let mut amount = remaining
        .min(max_by_height)
        .min(max_by_weight)
        .min(max_by_piling);
    if let Some(attribute) = package_attribute {
        while amount > 0
            && !same_item_pile_allows_stacking(
                attribute,
                item,
                orientation,
                item_width,
                item_height,
                item_depth,
                bin,
                amount,
                item_index,
                package_rule_policy,
            )
        {
            amount -= 1;
        }
    }
    amount
}

fn same_item_pile_allows_stacking<V, U>(
    attribute: &PackageAttribute,
    item: &ActualItem<V, U>,
    orientation: Orientation,
    item_width: &V,
    item_height: &V,
    item_depth: &V,
    bin: &BinType<V, U>,
    amount: u64,
    item_index: usize,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let orientation_enabled = item.enabled_orientations.is_empty()
        || item.enabled_orientations.contains(&orientation);
    let policy_units = (0..amount)
        .map(|_| LayerGenerationStackingUnit {
            item_index,
            item_id: item.id.clone(),
            orientation,
            orientation_enabled,
            width: *item_width,
            depth: *item_depth,
            height: *item_height,
            weight: item.weight.value,
        })
        .collect::<Vec<_>>();
    for layer in 0..amount {
        let bottom_item = (layer > 0).then_some(attribute);
        let input = PackageStackingInput {
            item: attribute,
            bottom_item,
            layer,
            height: (*item_height * V::from(layer).unwrap_or_else(V::zero))
                .to_f64()
                .unwrap_or(0.0),
            item_width: item_width.to_f64().unwrap_or(0.0),
            item_height: item_height.to_f64().unwrap_or(0.0),
            item_depth: item_depth.to_f64().unwrap_or(0.0),
            item_weight: item.weight.value.to_f64().unwrap_or(0.0),
            bottom_width: item_width.to_f64().unwrap_or(0.0),
            bottom_depth: item_depth.to_f64().unwrap_or(0.0),
            bottom_weight: item.weight.value.to_f64().unwrap_or(0.0),
            item_orientation: orientation,
            bottom_orientation: orientation,
            item_orientation_enabled: orientation_enabled,
            item_orientation_enabled_at_space: true,
            bottom_orientation_enabled: orientation_enabled,
            space_width: bin.width.value.to_f64().unwrap_or(0.0),
            space_height: bin.height.value.to_f64().unwrap_or(0.0),
            space_depth: bin.depth.value.to_f64().unwrap_or(0.0),
        };
        if !attribute.enabled_stacking_on(&input) {
            return false;
        }
        if layer > 0 {
            let placement_input = PackagePlacementStackingInput {
                item: attribute,
                item_orientation: orientation,
                item_orientation_enabled_at_space: true,
                item_orientation_enabled: orientation_enabled,
                direct_bottom_items: vec![attribute],
                direct_bottom_contexts: vec![PackagePlacementBottomContext {
                    item: attribute,
                    orientation,
                    orientation_enabled,
                }],
                indirect_bottom_items: (0..layer.saturating_sub(1))
                    .map(|_| attribute)
                    .collect::<Vec<_>>(),
                indirect_bottom_contexts: (0..layer.saturating_sub(1))
                    .map(|_| PackagePlacementBottomContext {
                        item: attribute,
                        orientation,
                        orientation_enabled,
                    })
                    .collect::<Vec<_>>(),
                layer,
            };
            if !attribute.enabled_placement_stacking(&placement_input)
                || !package_rule_policy.allows_placement_stacking(
                    &policy_units,
                    layer as usize,
                    &placement_input,
                )
            {
                return false;
            }
        }
    }
    true
}

fn next_pattern_point<V>(
    step: &PatternStep,
    placements: &[PatternPlanePlacement<V>],
    current_depth: &V,
) -> Option<(V, V)>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd,
{
    if placements.is_empty() {
        return Some((V::zero(), V::zero()));
    }
    match step.next_point_policy.unwrap_or(PatternNextPointPolicy::RightBottom) {
        PatternNextPointPolicy::RightBottom => {
            let x = placements
                .iter()
                .filter(|placement| placement.z == V::zero())
                .map(|placement| placement.x + placement.width)
                .fold(V::zero(), |acc, value| if value > acc { value } else { acc });
            Some((x, V::zero()))
        }
        PatternNextPointPolicy::LeftUpper => {
            let z = placements
                .iter()
                .filter(|placement| placement.z == V::zero())
                .map(|placement| placement.z + placement.depth)
                .fold(V::zero(), |acc, value| if value > acc { value } else { acc });
            let max_z = placements
                .iter()
                .map(|placement| placement.z + placement.depth)
                .fold(z + *current_depth, |acc, value| if value > acc { value } else { acc });
            let x = placements.iter().fold(V::zero(), |acc, placement| {
                let placement_max_z = placement.z + placement.depth;
                if placement_max_z <= z || placement.z >= max_z {
                    acc
                } else {
                    let placement_max_x = placement.x + placement.width;
                    if placement_max_x > acc { placement_max_x } else { acc }
                }
            });
            Some((x, z))
        }
    }
}

