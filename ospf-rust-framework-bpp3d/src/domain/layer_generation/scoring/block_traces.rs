fn append_block_traces<V, U>(
    request: &LayerGenerationRequest<V, U>,
    block: &Block<V, U>,
    indexed_items: &[(usize, ActualItem<V, U>)],
    origin: &MetricPoint3<V, U>,
    traces: &mut Vec<LayerBlockTrace<V, U>>,
) where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    match block {
        Block::Simple(simple) => {
            append_simple_block_trace(request, simple, indexed_items, origin, traces);
        }
        Block::Complex(complex) => {
            for placement in &complex.blocks {
                let Some(simple) = complex.sub_blocks.get(placement.block_index) else {
                    continue;
                };
                let sub_origin = MetricPoint3 {
                    x: Quantity::new_ct(origin.x.value + placement.position.x.value),
                    y: Quantity::new_ct(origin.y.value + placement.position.y.value),
                    z: Quantity::new_ct(origin.z.value + placement.position.z.value),
                };
                append_simple_block_trace(request, simple, indexed_items, &sub_origin, traces);
            }
        }
    }
}

fn append_simple_block_trace<V, U>(
    request: &LayerGenerationRequest<V, U>,
    simple: &crate::domain::block_loading::SimpleBlock<V, U>,
    indexed_items: &[(usize, ActualItem<V, U>)],
    origin: &MetricPoint3<V, U>,
    traces: &mut Vec<LayerBlockTrace<V, U>>,
) where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let original_index = indexed_items
        .get(simple.item_view.item_index)
        .map(|(index, _)| *index)
        .unwrap_or(simple.item_view.item_index);
    let item_id = request
        .items
        .get(original_index)
        .map(|item| item.id.clone())
        .unwrap_or_else(|| format!("item-{}", original_index).into());
    traces.push(LayerBlockTrace {
        block_index: traces.len(),
        item_index: original_index,
        item_id,
        orientation: simple.item_view.orientation,
        nx: simple.nx,
        ny: simple.ny,
        nz: simple.nz,
        item_count: simple.item_count,
        size: MetricSize3 {
            width: simple.width.clone(),
            height: simple.height.clone(),
            depth: simple.depth.clone(),
        },
        origin: origin.clone(),
    });
}

fn block_traces_allow_package_rules<V, U>(
    request: &LayerGenerationRequest<V, U>,
    bin: &BinType<V, U>,
    traces: &[LayerBlockTrace<V, U>],
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    if !traces.iter().all(|trace| block_trace_allows_internal_stacking(request, bin, trace)) {
        return false;
    }
    let units = traces
        .iter()
        .map(|trace| LayerGenerationStackingUnit {
            item_index: trace.item_index,
            item_id: trace.item_id.clone(),
            orientation: trace.orientation,
            orientation_enabled: request
                .items
                .get(trace.item_index)
                .map(|item| {
                    item.enabled_orientations.is_empty()
                        || item.enabled_orientations.contains(&trace.orientation)
                })
                .unwrap_or(true),
            width: trace.size.width.value,
            depth: trace.size.depth.value,
            height: trace.size.height.value,
            weight: request
                .items
                .get(trace.item_index)
                .map(|item| item.weight.value * V::from(trace.item_count).unwrap_or_else(V::zero))
                .unwrap_or_else(V::zero),
        })
        .collect::<Vec<_>>();
    for (top_index, top) in traces.iter().enumerate() {
        let Some(top_attribute) = request.package_attributes.get(&top.item_id) else {
            continue;
        };
        let direct_bottom_indices = traces
            .iter()
            .enumerate()
            .filter_map(|(bottom_index, bottom)| {
                (bottom_index != top_index
                    && traces_touch_vertically(top, bottom)
                    && traces_overlap_footprint(top, bottom))
                    .then_some(bottom_index)
            })
            .collect::<Vec<_>>();
        if direct_bottom_indices.is_empty() {
            continue;
        }
        let indirect_bottom_indices = traces
            .iter()
            .enumerate()
            .filter_map(|(bottom_index, bottom)| {
                (bottom_index != top_index
                    && bottom.origin.y.value + bottom.size.height.value < top.origin.y.value
                    && traces_overlap_footprint(top, bottom))
                    .then_some(bottom_index)
            })
            .collect::<Vec<_>>();
        let direct_bottom_items = direct_bottom_indices
            .iter()
            .filter_map(|index| request.package_attributes.get(&traces[*index].item_id))
            .collect::<Vec<_>>();
        let indirect_bottom_items = indirect_bottom_indices
            .iter()
            .filter_map(|index| request.package_attributes.get(&traces[*index].item_id))
            .collect::<Vec<_>>();
        let direct_bottom_contexts = direct_bottom_indices
            .iter()
            .filter_map(|index| {
                let bottom = &traces[*index];
                request.package_attributes.get(&bottom.item_id).map(|attribute| {
                    PackagePlacementBottomContext {
                        item: attribute,
                        orientation: bottom.orientation,
                        orientation_enabled: request
                            .items
                            .get(bottom.item_index)
                            .map(|item| {
                                item.enabled_orientations.is_empty()
                                    || item.enabled_orientations.contains(&bottom.orientation)
                            })
                            .unwrap_or(true),
                    }
                })
            })
            .collect::<Vec<_>>();
        let indirect_bottom_contexts = indirect_bottom_indices
            .iter()
            .filter_map(|index| {
                let bottom = &traces[*index];
                request.package_attributes.get(&bottom.item_id).map(|attribute| {
                    PackagePlacementBottomContext {
                        item: attribute,
                        orientation: bottom.orientation,
                        orientation_enabled: request
                            .items
                            .get(bottom.item_index)
                            .map(|item| {
                                item.enabled_orientations.is_empty()
                                    || item.enabled_orientations.contains(&bottom.orientation)
                            })
                            .unwrap_or(true),
                    }
                })
            })
            .collect::<Vec<_>>();
        let layer = direct_bottom_indices
            .iter()
            .filter_map(|index| traces.get(*index))
            .map(|bottom| {
                let top_y = top.origin.y.value;
                let bottom_y = bottom.origin.y.value;
                if bottom.size.height.value > V::zero() {
                    ((top_y - bottom_y) / bottom.size.height.value)
                        .floor()
                        .to_u64()
                        .unwrap_or(1)
                } else {
                    1
                }
            })
            .max()
            .unwrap_or(1);
        let placement_input = PackagePlacementStackingInput {
            item: top_attribute,
            item_orientation: top.orientation,
            item_orientation_enabled_at_space: true,
            item_orientation_enabled: request
                .items
                .get(top.item_index)
                .map(|item| {
                    item.enabled_orientations.is_empty()
                        || item.enabled_orientations.contains(&top.orientation)
                })
                .unwrap_or(true),
            direct_bottom_items,
            direct_bottom_contexts,
            indirect_bottom_items,
            indirect_bottom_contexts,
            layer,
        };
        if !top_attribute.enabled_placement_stacking(&placement_input)
            || !request.package_rule_policy.as_ref().allows_placement_stacking(
                &units,
                top_index,
                &placement_input,
            )
        {
            return false;
        }
    }
    true
}

fn block_trace_allows_internal_stacking<V, U>(
    request: &LayerGenerationRequest<V, U>,
    bin: &BinType<V, U>,
    trace: &LayerBlockTrace<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    if trace.ny <= 1 {
        return true;
    }
    let Some(attribute) = request.package_attributes.get(&trace.item_id) else {
        return true;
    };
    let Some(item) = request.items.get(trace.item_index) else {
        return false;
    };
    let unit_width = trace.size.width.value / V::from(trace.nx).unwrap_or_else(V::one);
    let unit_height = trace.size.height.value / V::from(trace.ny).unwrap_or_else(V::one);
    let unit_depth = trace.size.depth.value / V::from(trace.nz).unwrap_or_else(V::one);
    same_item_pile_allows_stacking(
        attribute,
        item,
        trace.orientation,
        &unit_width,
        &unit_height,
        &unit_depth,
        bin,
        trace.ny,
        trace.item_index,
        request.package_rule_policy.as_ref(),
    )
}

fn traces_touch_vertically<V, U>(
    top: &LayerBlockTrace<V, U>,
    bottom: &LayerBlockTrace<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let bottom_top = bottom.origin.y.value + bottom.size.height.value;
    (top.origin.y.value - bottom_top).abs() <= V::epsilon()
}

fn traces_overlap_footprint<V, U>(
    lhs: &LayerBlockTrace<V, U>,
    rhs: &LayerBlockTrace<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let lhs_max_x = lhs.origin.x.value + lhs.size.width.value;
    let rhs_max_x = rhs.origin.x.value + rhs.size.width.value;
    let lhs_max_z = lhs.origin.z.value + lhs.size.depth.value;
    let rhs_max_z = rhs.origin.z.value + rhs.size.depth.value;
    lhs.origin.x.value < rhs_max_x
        && rhs.origin.x.value < lhs_max_x
        && lhs.origin.z.value < rhs_max_z
        && rhs.origin.z.value < lhs_max_z
}

