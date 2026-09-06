fn select_mixed_pattern_pile_with_size<V, U>(
    selected_items: &[PatternSelectedItem<V>],
    package_attributes: &HashMap<ItemId, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    bin: &BinType<V, U>,
    rest_capacity: &V,
    size: usize,
) -> Option<PatternMixedPile<V>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Debug + Clone + Send + Sync,
{
    let mut best = None::<PatternMixedPile<V>>;
    let mut best_slack = None::<V>;
    let mut current = Vec::<PatternSelectedItem<V>>::new();
    select_mixed_pattern_pile_recursive(
        selected_items,
        package_attributes,
        package_rule_policy,
        remaining,
        bin,
        rest_capacity,
        size,
        0,
        &mut current,
        &mut best,
        &mut best_slack,
    );
    if size == 3 {
        select_mixed_pattern_pile_with_repeated_unit(
            selected_items,
            package_attributes,
            package_rule_policy,
            remaining,
            bin,
            rest_capacity,
            &mut best,
            &mut best_slack,
        );
    }
    best
}

fn select_mixed_pattern_pile_recursive<V, U>(
    selected_items: &[PatternSelectedItem<V>],
    package_attributes: &HashMap<ItemId, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    bin: &BinType<V, U>,
    rest_capacity: &V,
    size: usize,
    start: usize,
    current: &mut Vec<PatternSelectedItem<V>>,
    best: &mut Option<PatternMixedPile<V>>,
    best_slack: &mut Option<V>,
)
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Debug + Clone + Send + Sync,
{
    if current.len() == size {
        let Some(pile) = build_mixed_pattern_pile(
            current,
            package_attributes,
            package_rule_policy,
            remaining,
            bin,
            rest_capacity,
        ) else {
            return;
        };
        let slack = bin.height.value - pile.height;
        let should_replace = best_slack.map(|best| slack < best).unwrap_or(true);
        if should_replace {
            *best = Some(pile);
            *best_slack = Some(slack);
        }
        return;
    }
    for index in start..selected_items.len() {
        current.push(selected_items[index].clone());
        select_mixed_pattern_pile_recursive(
            selected_items,
            package_attributes,
            package_rule_policy,
            remaining,
            bin,
            rest_capacity,
            size,
            index + 1,
            current,
            best,
            best_slack,
        );
        current.pop();
    }
}

fn select_mixed_pattern_pile_with_repeated_unit<V, U>(
    selected_items: &[PatternSelectedItem<V>],
    package_attributes: &HashMap<ItemId, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    bin: &BinType<V, U>,
    rest_capacity: &V,
    best: &mut Option<PatternMixedPile<V>>,
    best_slack: &mut Option<V>,
)
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Debug + Clone + Send + Sync,
{
    for repeated in selected_items {
        if remaining.get(&repeated.item_index).copied().unwrap_or(0) < 2 {
            continue;
        }
        for other in selected_items {
            if other.item_index == repeated.item_index || other.height == repeated.height {
                continue;
            }
            let units = vec![repeated.clone(), repeated.clone(), other.clone()];
            let Some(pile) = build_mixed_pattern_pile(
                &units,
                package_attributes,
                package_rule_policy,
                remaining,
                bin,
                rest_capacity,
            ) else {
                continue;
            };
            let slack = bin.height.value - pile.height;
            let should_replace = best_slack.map(|best| slack < best).unwrap_or(true);
            if should_replace {
                *best = Some(pile);
                *best_slack = Some(slack);
            }
        }
    }
}

fn build_mixed_pattern_pile<V, U>(
    units: &[PatternSelectedItem<V>],
    package_attributes: &HashMap<ItemId, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    remaining: &HashMap<usize, u64>,
    bin: &BinType<V, U>,
    rest_capacity: &V,
) -> Option<PatternMixedPile<V>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Debug + Clone + Send + Sync,
{
    let mut amount_by_item = HashMap::<usize, u64>::new();
    for unit in units {
        *amount_by_item.entry(unit.item_index).or_default() += 1;
        if amount_by_item[&unit.item_index] > remaining.get(&unit.item_index).copied().unwrap_or(0) {
            return None;
        }
    }
    let mut units = units.to_vec();
    units.sort_by(|lhs, rhs| {
        rhs.weight
            .partial_cmp(&lhs.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if !mixed_pattern_pile_allows_stacking(&units, package_attributes, package_rule_policy, bin) {
        return None;
    }
    let height = units
        .iter()
        .fold(V::zero(), |acc, unit| acc + unit.height);
    if height > bin.height.value {
        return None;
    }
    let total_weight = units
        .iter()
        .fold(V::zero(), |acc, unit| acc + unit.weight);
    if total_weight > *rest_capacity {
        return None;
    }
    let width = units
        .iter()
        .map(|unit| unit.width)
        .fold(V::zero(), |acc, value| if value > acc { value } else { acc });
    let depth = units
        .iter()
        .map(|unit| unit.depth)
        .fold(V::zero(), |acc, value| if value > acc { value } else { acc });
    Some(PatternMixedPile {
        units,
        width,
        depth,
        height,
    })
}

fn mixed_pattern_pile_allows_stacking<V, U>(
    units: &[PatternSelectedItem<V>],
    package_attributes: &HashMap<ItemId, PackageAttribute>,
    package_rule_policy: &dyn LayerGenerationPackageRulePolicy<V, U>,
    bin: &BinType<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Debug + Clone + Send + Sync,
{
    let Some(first) = units.first() else {
        return true;
    };
    let policy_units = units
        .iter()
        .map(|unit| LayerGenerationStackingUnit {
            item_index: unit.item_index,
            item_id: unit.item_id.clone(),
            orientation: unit.orientation,
            orientation_enabled: unit.orientation_enabled,
            width: unit.width.clone(),
            depth: unit.depth.clone(),
            height: unit.height.clone(),
            weight: unit.weight.clone(),
        })
        .collect::<Vec<_>>();
    if units.len() > 1 {
        let distinct_item_ids = units
            .iter()
            .map(|unit| unit.item_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if distinct_item_ids.len() > 1
            && units.iter().any(|unit| {
                package_attributes
                    .get(&unit.item_id)
                    .is_some_and(|attribute| attribute.allow_mixed_loading == Some(false))
            })
        {
            return false;
        }
    }
    for (index, unit) in units.iter().enumerate() {
        if let Some(attribute) = package_attributes.get(&unit.item_id) {
            if let Some(max_stack_layers) = attribute.max_stack_layers {
                if (index as u64) + 1 > max_stack_layers {
                    return false;
                }
            }
        }
    }
    let pile_space_height = bin.height.value.to_f64().unwrap_or(f64::INFINITY);
    let mut cumulative_height = first.height.to_f64().unwrap_or(0.0);
    for top_index in 1..units.len() {
        let top = &units[top_index];
        let bottom = &units[top_index - 1];
        let top_height = top.height.to_f64().unwrap_or(0.0);
        let top_width = top.width.to_f64().unwrap_or(0.0);
        let top_depth = top.depth.to_f64().unwrap_or(0.0);
        let bottom_width = bottom.width.to_f64().unwrap_or(0.0);
        let bottom_depth = bottom.depth.to_f64().unwrap_or(0.0);
        let item_weight = top.weight.to_f64().unwrap_or(0.0);
        let bottom_weight = bottom.weight.to_f64().unwrap_or(0.0);
        let direct_bottom_attributes = package_attributes
            .get(&bottom.item_id)
            .into_iter()
            .collect::<Vec<_>>();
        let indirect_bottom_attributes = units
            .iter()
            .take(top_index.saturating_sub(1))
            .filter_map(|unit| package_attributes.get(&unit.item_id))
            .collect::<Vec<_>>();
        if let Some(top_attribute) = package_attributes.get(&top.item_id) {
            let direct_bottom_attribute = direct_bottom_attributes.first().copied();
            let input = PackageStackingInput {
                item: top_attribute,
                bottom_item: direct_bottom_attribute,
                layer: top_index as u64,
                height: cumulative_height,
                item_width: top_width,
                item_height: top_height,
                item_depth: top_depth,
                item_weight,
                bottom_width,
                bottom_depth,
                bottom_weight,
                item_orientation: top.orientation,
                bottom_orientation: bottom.orientation,
                item_orientation_enabled: top.orientation_enabled,
                item_orientation_enabled_at_space: true,
                bottom_orientation_enabled: direct_bottom_attribute
                    .map(|attribute| attribute.enabled_orientation_by_rule(&PackageOrientationRuleInput {
                        orientation: bottom.orientation,
                        space_width: bin.width.value.to_f64().unwrap_or(f64::INFINITY),
                        space_height: pile_space_height,
                        space_depth: bin.depth.value.to_f64().unwrap_or(f64::INFINITY),
                    }))
                    .unwrap_or(true),
                space_width: bin.width.value.to_f64().unwrap_or(f64::INFINITY),
                space_height: pile_space_height,
                space_depth: bin.depth.value.to_f64().unwrap_or(f64::INFINITY),
            };
            let footprint_area = top_width * top_depth;
            let bottom_support_area = top_width.min(bottom_width) * top_depth.min(bottom_depth);
            let footprint_min_span = top_width.min(top_depth);
            if !top_attribute.enabled_stacking_on_support(
                item_weight,
                footprint_area,
                footprint_min_span,
                bottom_support_area,
                bottom_weight,
            ) {
                return false;
            }
            if !top_attribute.enabled_stacking_on(&input) {
                return false;
            }
            let placement_input = PackagePlacementStackingInput {
                item: top_attribute,
                item_orientation: top.orientation,
                item_orientation_enabled_at_space: true,
                item_orientation_enabled: top.orientation_enabled,
                direct_bottom_items: direct_bottom_attributes,
                direct_bottom_contexts: direct_bottom_attribute
                    .map(|attribute| {
                        vec![PackagePlacementBottomContext {
                            item: attribute,
                            orientation: bottom.orientation,
                            orientation_enabled: bottom.orientation_enabled,
                        }]
                    })
                    .unwrap_or_default(),
                indirect_bottom_items: indirect_bottom_attributes,
                indirect_bottom_contexts: Vec::new(),
                layer: top_index as u64,
            };
            if !top_attribute.enabled_placement_stacking(&placement_input) {
                return false;
            }
            if !package_rule_policy.allows_placement_stacking(
                &policy_units,
                top_index,
                &placement_input,
            ) {
                return false;
            }
        }
        cumulative_height += top_height;
    }
    true
}

