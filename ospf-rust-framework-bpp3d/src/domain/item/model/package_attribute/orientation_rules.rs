
/// 包装朝向规则 / Package orientation rule
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageOrientationRule {
    /// 禁止朝向类别 / Forbid orientation category
    ForbidCategory(OrientationCategory),
    /// 禁止旋转朝向 / Forbid rotated orientations
    ForbidRotated,
    /// 需要空间宽度下界 / Require minimum space width
    RequireMinSpaceWidth(f64),
    /// 需要空间高度下界 / Require minimum space height
    RequireMinSpaceHeight(f64),
    /// 需要空间深度下界 / Require minimum space depth
    RequireMinSpaceDepth(f64),
}

impl PackageOrientationRule {
    /// 判断朝向是否允许 / Check whether orientation is allowed
    pub fn allows(self, input: &PackageOrientationRuleInput) -> bool {
        match self {
            Self::ForbidCategory(category) => input.orientation.category() != category,
            Self::ForbidRotated => !input.orientation.is_rotated(),
            Self::RequireMinSpaceWidth(width) => input.space_width >= width,
            Self::RequireMinSpaceHeight(height) => input.space_height >= height,
            Self::RequireMinSpaceDepth(depth) => input.space_depth >= depth,
        }
    }
}

/// 包装两两堆叠规则 / Package pair stacking rule
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackagePairStackingRule {
    /// 禁止同类堆叠 / Forbid same package type stacking
    ForbidSamePackageType,
    /// 要求上方不重于下方 / Require upper item not heavier than bottom item
    RequireNotHeavierThanBottom,
    /// 要求脚印不超过底部 / Require footprint no larger than bottom footprint
    RequireFootprintWithinBottom,
}

impl PackagePairStackingRule {
    /// 判断两两堆叠是否允许 / Check whether pair stacking is allowed
    pub fn allows(self, input: &PackageStackingInput<'_>) -> bool {
        let Some(bottom_item) = input.bottom_item else {
            return true;
        };
        match self {
            Self::ForbidSamePackageType => input.item.package_type != bottom_item.package_type,
            Self::RequireNotHeavierThanBottom => input.item_weight <= input.bottom_weight,
            Self::RequireFootprintWithinBottom => {
                input.item_width <= input.bottom_width && input.item_depth <= input.bottom_depth
            }
        }
    }
}

