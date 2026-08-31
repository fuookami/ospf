#[derive(Debug, Clone)]
struct PatternPlanePlacement<V>
where
    V: Debug + Clone + Send + Sync,
{
    pile_index: usize,
    item_index: usize,
    item_id: String,
    orientation: Orientation,
    x: V,
    y: V,
    z: V,
    width: V,
    depth: V,
    height: V,
    amount: u64,
    total_weight: V,
}

#[derive(Debug, Clone)]
struct PatternSelectedItem<V>
where
    V: Debug + Clone + Send + Sync,
{
    item_index: usize,
    item_id: String,
    orientation: Orientation,
    orientation_enabled: bool,
    width: V,
    depth: V,
    height: V,
    weight: V,
}

#[derive(Debug, Clone)]
struct PatternMixedPile<V>
where
    V: Debug + Clone + Send + Sync,
{
    units: Vec<PatternSelectedItem<V>>,
    width: V,
    depth: V,
    height: V,
}

