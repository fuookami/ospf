/// 堆叠检查输入 / Stacking check input
#[derive(Debug, Clone)]
pub struct PackageStackingInput<'a> {
    /// 待堆叠包装属性 / Stacked item package attribute
    pub item: &'a PackageAttribute,
    /// 底部包装属性 / Bottom item package attribute
    pub bottom_item: Option<&'a PackageAttribute>,
    /// 当前同类层数 / Current same-type layer count
    pub layer: u64,
    /// 当前同类高度 / Current same-type height
    pub height: f64,
    /// 待堆叠宽度 / Stacked item width
    pub item_width: f64,
    /// 待堆叠高度 / Stacked item height
    pub item_height: f64,
    /// 待堆叠深度 / Stacked item depth
    pub item_depth: f64,
    /// 待堆叠重量 / Stacked item weight
    pub item_weight: f64,
    /// 底部宽度 / Bottom item width
    pub bottom_width: f64,
    /// 底部深度 / Bottom item depth
    pub bottom_depth: f64,
    /// 底部重量 / Bottom item weight
    pub bottom_weight: f64,
    /// 待堆叠朝向 / Stacked item orientation
    pub item_orientation: Orientation,
    /// 底部朝向 / Bottom item orientation
    pub bottom_orientation: Orientation,
    /// 朝向是否在普通允许列表中 / Whether orientation is normally enabled
    pub item_orientation_enabled: bool,
    /// 朝向是否在当前空间中允许 / Whether orientation is enabled in current space
    pub item_orientation_enabled_at_space: bool,
    /// 底部朝向是否在普通允许列表中 / Whether bottom orientation is normally enabled
    pub bottom_orientation_enabled: bool,
    /// 当前空间宽度 / Current space width
    pub space_width: f64,
    /// 当前空间高度 / Current space height
    pub space_height: f64,
    /// 当前空间深度 / Current space depth
    pub space_depth: f64,
}

/// 包装朝向规则输入 / Package orientation rule input
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackageOrientationRuleInput {
    /// 朝向 / Orientation
    pub orientation: Orientation,
    /// 空间宽度 / Space width
    pub space_width: f64,
    /// 空间高度 / Space height
    pub space_height: f64,
    /// 空间深度 / Space depth
    pub space_depth: f64,
}
