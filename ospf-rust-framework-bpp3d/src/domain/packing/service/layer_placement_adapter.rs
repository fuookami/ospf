// ============================================================================
// LayerPlacementAdapter - 层放置适配器 / Layer placement adapter
// ============================================================================

/// 已知坐标放置 / Known-coordinate placement
#[derive(Debug, Clone)]
pub struct KnownCoordinatePlacement<V, U: UnitTrait> {
    /// 原始物品索引 / Original item index
    pub item_index: usize,
    /// 物品 / Item
    pub item: ActualItem<V, U>,
    /// 坐标 / Position
    pub position: MetricPoint3<V, U>,
    /// 朝向 / Orientation
    pub orientation: Orientation,
}

/// 层放置适配器 / Layer placement adapter
///
/// 将 Rust 版 typed geometry 放置转换为最终装箱模型，不使用 Kotlin `QuantityPlacement*`。
/// Converts Rust typed-geometry placements into final packing models without
/// Kotlin `QuantityPlacement*` migration types.
#[derive(Debug, Clone, Default)]
pub struct LayerPlacementAdapter;

impl LayerPlacementAdapter {
    /// 创建适配器 / Create adapter
    pub fn new() -> Self {
        Self
    }

    /// 转换单个放置 / Convert one placement
    pub fn to_packed_item<V, U>(&self, placement: KnownCoordinatePlacement<V, U>) -> PackedItem<V, U>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let packing_shape = placement.item.oriented_packing_shape(placement.orientation);
        PackedItem {
            item_index: placement.item_index,
            item: placement.item,
            position: placement.position,
            orientation: placement.orientation,
            packing_shape,
            loading_order: placement.item_index as u64,
        }
    }

    /// 转换并验证已知坐标箱 / Convert and validate known-coordinate bin
    pub fn to_packed_bin<V, U>(
        &self,
        name: String,
        bin_type: BinType<V, U>,
        batch_no: Option<String>,
        placements: Vec<KnownCoordinatePlacement<V, U>>,
    ) -> Result<PackedBin<V, U>, Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        let items = placements
            .into_iter()
            .map(|placement| self.to_packed_item(placement))
            .collect();
        let packed_bin = PackedBin {
            name,
            bin_type,
            batch_no,
            items,
        };

        PackingGeometryGuard::validate(&packed_bin)?;
        Ok(packed_bin)
    }
}

