// ============================================================================
// Item - 货物 / Item
// ============================================================================

/// 实际货物 / Actual item
///
/// 具有具体尺寸和属性的单个货物。
/// An individual item with concrete dimensions and attributes.
#[derive(Debug, Clone)]
pub struct ActualItem<V, U: UnitTrait> {
    /// 标识 / ID
    pub id: ItemId,
    /// 名称 / Name
    pub name: String,
    /// 包装编码 / Package code
    pub package_code: Option<String>,
    /// 包装 / Package
    pub pack: Option<Package<V, U>>,
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 允许朝向 / Enabled orientations
    pub enabled_orientations: Vec<Orientation>,
    /// 形状规格覆盖 / Shape specification override
    pub shape_spec_override: Option<PackageShapeSpec<V, U>>,
}

