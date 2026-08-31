/// 包装放置级堆叠输入 / Package placement stacking input
#[derive(Debug, Clone)]
pub struct PackagePlacementStackingInput<'a> {
    /// 待堆叠包装属性 / Stacked item package attribute
    pub item: &'a PackageAttribute,
    /// 待堆叠朝向 / Stacked item orientation
    pub item_orientation: Orientation,
    /// 待堆叠朝向是否在当前空间中允许 / Whether stacked orientation is enabled in current space
    pub item_orientation_enabled_at_space: bool,
    /// 待堆叠朝向是否在普通允许列表中 / Whether stacked orientation is normally enabled
    pub item_orientation_enabled: bool,
    /// 直接底部包装属性 / Direct bottom package attributes
    pub direct_bottom_items: Vec<&'a PackageAttribute>,
    /// 直接底部上下文 / Direct bottom contexts
    pub direct_bottom_contexts: Vec<PackagePlacementBottomContext<'a>>,
    /// 间接底部包装属性 / Indirect bottom package attributes
    pub indirect_bottom_items: Vec<&'a PackageAttribute>,
    /// 间接底部上下文 / Indirect bottom contexts
    pub indirect_bottom_contexts: Vec<PackagePlacementBottomContext<'a>>,
    /// 当前同类层数 / Current same-type layer count
    pub layer: u64,
}

impl<'a> PackagePlacementStackingInput<'a> {
    /// 使用属性创建放置输入 / Create placement input from attributes
    pub fn from_attributes(
        item: &'a PackageAttribute,
        direct_bottom_items: Vec<&'a PackageAttribute>,
        indirect_bottom_items: Vec<&'a PackageAttribute>,
    ) -> Self {
        let direct_bottom_contexts = direct_bottom_items
            .iter()
            .copied()
            .map(PackagePlacementBottomContext::from_attribute)
            .collect::<Vec<_>>();
        Self {
            item,
            item_orientation: Orientation::Upright,
            item_orientation_enabled_at_space: true,
            item_orientation_enabled: true,
            direct_bottom_items,
            direct_bottom_contexts,
            indirect_bottom_items,
            indirect_bottom_contexts: Vec::new(),
            layer: 0,
        }
    }
}

/// 包装放置级底部上下文 / Package placement bottom context
#[derive(Debug, Clone, Copy)]
pub struct PackagePlacementBottomContext<'a> {
    /// 底部包装属性 / Bottom package attribute
    pub item: &'a PackageAttribute,
    /// 底部朝向 / Bottom orientation
    pub orientation: Orientation,
    /// 底部朝向是否在普通允许列表中 / Whether bottom orientation is normally enabled
    pub orientation_enabled: bool,
}

impl<'a> PackagePlacementBottomContext<'a> {
    /// 使用属性创建默认底部上下文 / Create default bottom context from attribute
    pub fn from_attribute(item: &'a PackageAttribute) -> Self {
        Self {
            item,
            orientation: Orientation::Upright,
            orientation_enabled: true,
        }
    }

    /// 朝向后的顶面是否平整 / Whether oriented top surface is flat
    pub fn oriented_top_flat(self) -> bool {
        self.item.oriented_top_flat(self.orientation, self.orientation_enabled)
    }

    /// 是否原生允许当前朝向 / Whether current orientation is natively enabled
    pub fn orientation_enabled(self) -> bool {
        self.orientation_enabled
    }
}

/// 包装放置级堆叠规则 / Package placement stacking rule
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackagePlacementStackingRule {
    /// bottom-only 货物下方只能有 bottom-only 货物 / Bottom-only item requires all bottom items bottom-only
    BottomOnlyRequiresAllBottomOnlyBelow,
    /// 禁止任意非平整直接底部 / Forbid any non-flat direct bottom item
    ForbidAnyNonFlatDirectBottom,
    /// 禁止间接底部存在指定类型 / Forbid package type in indirect bottom items
    ForbidIndirectPackageType(PackageType),
}

impl PackagePlacementStackingRule {
    /// 判断放置级堆叠是否允许 / Check whether placement-level stacking is allowed
    pub fn allows(self, input: &PackagePlacementStackingInput<'_>) -> bool {
        match self {
            Self::BottomOnlyRequiresAllBottomOnlyBelow => {
                !input.item.bottom_only
                    || input
                        .direct_bottom_items
                        .iter()
                        .chain(input.indirect_bottom_items.iter())
                        .all(|item| item.bottom_only)
            }
            Self::ForbidAnyNonFlatDirectBottom => {
                if input.direct_bottom_contexts.is_empty() {
                    input.direct_bottom_items.iter().all(|item| item.top_flat)
                } else {
                    input
                        .direct_bottom_contexts
                        .iter()
                        .all(|item| item.oriented_top_flat())
                }
            }
            Self::ForbidIndirectPackageType(package_type) => input
                .indirect_bottom_items
                .iter()
                .all(|item| item.package_type != package_type),
        }
    }
}

