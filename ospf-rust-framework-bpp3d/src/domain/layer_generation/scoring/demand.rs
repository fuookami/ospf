fn item_remaining_amount<V, U>(
    request: &LayerGenerationRequest<V, U>,
    item_index: usize,
    item_id: &str,
) -> u64
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    request
        .demand_entries
        .iter()
        .find_map(|entry| match &entry.key {
            Bpp3dDemandKey::Item { id } if id == item_id => {
                Some(entry.remaining().ceil().to_u64().unwrap_or(1).max(1))
            }
            _ => None,
        })
        .unwrap_or_else(|| if item_index < request.items.len() { 1 } else { 0 })
}

fn pattern_item_priority_score<V, U>(
    request: &LayerGenerationRequest<V, U>,
    item_index: usize,
    item: &ActualItem<V, U>,
) -> f64
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let bin_height = request
        .bin
        .as_ref()
        .map(|bin| bin.height.value.to_f64().unwrap_or(f64::INFINITY))
        .unwrap_or(f64::INFINITY);
    let item_height = item.height.value.to_f64().unwrap_or(f64::INFINITY);
    let item_weight = item.weight.value.to_f64().unwrap_or(f64::INFINITY);
    let attribute = request.package_attributes.get(&item.id);
    let max_layer = attribute
        .and_then(PackageAttribute::max_layer)
        .unwrap_or(u64::MAX);
    let max_height = attribute
        .and_then(|attribute| attribute.max_height)
        .unwrap_or(f64::INFINITY);
    let remaining = item_remaining_amount(request, item_index, &item.id) as f64;
    let height_amount = if item_height > 0.0 {
        let by_height = (bin_height / item_height).floor().max(0.0);
        let by_attribute_height = (max_height / item_height).floor().max(0.0);
        by_height
            .min(max_layer as f64)
            .min(by_attribute_height)
            .min(remaining)
    } else {
        0.0
    };
    let remaining_height = bin_height - height_amount * item_height;
    let weight_score = if item_weight > 0.0 {
        item_weight
    } else {
        f64::INFINITY
    };
    remaining_height + weight_score * 1e-9
}

fn block_volume_score<V, U>(block: &Block<V, U>) -> V
where
    V: Field + num_traits::Float + Clone,
    U: CTUnit + Default + Clone,
{
    block.width().value * block.height().value * block.depth().value
}

fn block_unit_count<V, U>(block: &Block<V, U>) -> u64
where
    U: CTUnit + Default + Clone,
{
    match block {
        Block::Simple(simple) => simple.item_count,
        Block::Complex(complex) => complex
            .sub_blocks
            .iter()
            .map(|simple| simple.item_count)
            .sum(),
    }
}

