fn select_pattern_item<'a, V, U>(
    indexed_items: &'a [(usize, ActualItem<V, U>)],
    package_attributes: &HashMap<String, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    step: &PatternStep,
    bin: &BinType<V, U>,
    config: &DeferredLayerGeneratorConfig,
) -> Option<(usize, &'a ActualItem<V, U>, Orientation, V, V, V)>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    indexed_items.iter().find_map(|(item_index, item)| {
        if remaining.get(item_index).copied().unwrap_or(0) == 0 {
            return None;
        }
        // 底面范围过滤（Kotlin Bottom.length/width 语义，默认朝向下 depth/width）
        // Bottom range filtering (Kotlin Bottom.length/width semantics, depth/width under default orientation)
        let bottom_depth = item.depth.value.to_f64().unwrap_or(0.0);
        let bottom_width = item.width.value.to_f64().unwrap_or(0.0);
        if !config.pattern.accepts_bottom_dimensions(bottom_depth, bottom_width) {
            return None;
        }
        let attribute = package_attributes.get(&item.id);
        let orientations = item.enabled_orientations_at_bin(attribute, bin);
        orientations.into_iter().find_map(|orientation| {
            let orientation_input = PackageOrientationRuleInput {
                orientation,
                space_width: bin.width.value.to_f64().unwrap_or(f64::INFINITY),
                space_height: bin.height.value.to_f64().unwrap_or(f64::INFINITY),
                space_depth: bin.depth.value.to_f64().unwrap_or(f64::INFINITY),
            };
            if !package_rule_policy.allows_orientation(
                item,
                attribute,
                orientation,
                &orientation_input,
            ) {
                return None;
            }
            let (width, height, depth) = item.oriented_dimensions(orientation);
            let accepted = pattern_step_accepts_dimensions(step, &width, &depth);
            accepted.then_some((*item_index, item, orientation, width, depth, height))
        })
    })
}

fn select_mixed_pattern_pile<V, U>(
    indexed_items: &[(usize, ActualItem<V, U>)],
    package_attributes: &HashMap<String, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    step: &PatternStep,
    bin: &BinType<V, U>,
    loaded_weight: V,
    config: &DeferredLayerGeneratorConfig,
) -> Option<PatternMixedPile<V>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let rest_capacity = bin.capacity.value - loaded_weight;
    if rest_capacity <= V::zero() {
        return None;
    }
    let selected_items = pattern_selectable_items(
        indexed_items,
        package_attributes,
        package_rule_policy,
        remaining,
        step,
        bin,
        config,
    );
    if selected_items.len() < 2 {
        return None;
    }
    if config.pattern.enables_three_sum() {
        if let Some(pile) = select_mixed_pattern_pile_with_size(
            &selected_items,
            package_attributes,
            package_rule_policy,
            remaining,
            bin,
            &rest_capacity,
            3,
        ) {
            return Some(pile);
        }
    }
    if config.pattern.enables_two_sum() {
        select_mixed_pattern_pile_with_size(
            &selected_items,
            package_attributes,
            package_rule_policy,
            remaining,
            bin,
            &rest_capacity,
            2,
        )
    } else {
        None
    }
}

fn pattern_selectable_items<V, U>(
    indexed_items: &[(usize, ActualItem<V, U>)],
    package_attributes: &HashMap<String, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    step: &PatternStep,
    bin: &BinType<V, U>,
    config: &DeferredLayerGeneratorConfig,
) -> Vec<PatternSelectedItem<V>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    indexed_items
        .iter()
        .flat_map(|(item_index, item)| {
            if remaining.get(item_index).copied().unwrap_or(0) == 0 {
                return Vec::new();
            }
            // 底面范围过滤（Kotlin Bottom.length/width 语义，默认朝向下 depth/width）
            // Bottom range filtering (Kotlin Bottom.length/width semantics, depth/width under default orientation)
            let bottom_depth = item.depth.value.to_f64().unwrap_or(0.0);
            let bottom_width = item.width.value.to_f64().unwrap_or(0.0);
            if !config.pattern.accepts_bottom_dimensions(bottom_depth, bottom_width) {
                return Vec::new();
            }
            let attribute = package_attributes.get(&item.id);
            let orientations = item.enabled_orientations_at_bin(attribute, bin);
            orientations
                .into_iter()
                .filter_map(|orientation| {
                    let orientation_input = PackageOrientationRuleInput {
                        orientation,
                        space_width: bin.width.value.to_f64().unwrap_or(f64::INFINITY),
                        space_height: bin.height.value.to_f64().unwrap_or(f64::INFINITY),
                        space_depth: bin.depth.value.to_f64().unwrap_or(f64::INFINITY),
                    };
                    if !package_rule_policy.allows_orientation(
                        item,
                        attribute,
                        orientation,
                        &orientation_input,
                    ) {
                        return None;
                    }
                    let (width, height, depth) = item.oriented_dimensions(orientation);
                    if !pattern_step_accepts_dimensions(step, &width, &depth) {
                        return None;
                    }
                    Some(PatternSelectedItem {
                        item_index: *item_index,
                        item_id: item.id.clone(),
                        orientation,
                        orientation_enabled: item.enabled_orientations.is_empty()
                            || item.enabled_orientations.contains(&orientation),
                        width,
                        depth,
                        height,
                        weight: item.weight.value,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

