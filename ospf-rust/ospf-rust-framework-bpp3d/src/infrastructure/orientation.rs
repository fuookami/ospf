//! 朝向 / Orientation
//!
//! 三维装箱中物体的朝向定义，映射到轴置换。
//! Orientation definitions for objects in 3D bin packing, mapped to axis permutations.

use ospf_rust_math::geometry::AxisPermutation3;

/// 朝向类别 / Orientation category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrientationCategory {
    /// 直立 / Upright
    Upright,
    /// 侧放 / Side
    Side,
    /// 平放 / Lie
    Lie,
}

/// 朝向 / Orientation
///
/// 六种标准朝向，对应长方体的六种放置方式。
/// Six standard orientations corresponding to six placement modes of a cuboid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// 直立 / Upright (W-H-D → width=width, height=height, depth=depth)
    Upright,
    /// 直立旋转 / Upright rotated (W-H-D → width=depth, height=height, depth=width)
    UprightRotated,
    /// 侧放 / Side (W-H-D → width=height, height=width, depth=depth)
    Side,
    /// 侧放旋转 / Side rotated (W-H-D → width=depth, height=width, depth=height)
    SideRotated,
    /// 平放 / Lie (W-H-D → width=width, height=depth, depth=height)
    Lie,
    /// 平放旋转 / Lie rotated (W-H-D → width=height, height=depth, depth=width)
    LieRotated,
}

impl Orientation {
    /// 获取朝向类别 / Get orientation category
    pub fn category(self) -> OrientationCategory {
        match self {
            Self::Upright | Self::UprightRotated => OrientationCategory::Upright,
            Self::Side | Self::SideRotated => OrientationCategory::Side,
            Self::Lie | Self::LieRotated => OrientationCategory::Lie,
        }
    }

    /// 是否为旋转变体 / Whether this is a rotated variant
    pub fn is_rotated(self) -> bool {
        matches!(
            self,
            Self::UprightRotated | Self::SideRotated | Self::LieRotated
        )
    }

    /// 获取对应的旋转变体 / Get the corresponding rotated variant
    pub fn rotation(self) -> Self {
        match self {
            Self::Upright => Self::UprightRotated,
            Self::UprightRotated => Self::Upright,
            Self::Side => Self::SideRotated,
            Self::SideRotated => Self::Side,
            Self::Lie => Self::LieRotated,
            Self::LieRotated => Self::Lie,
        }
    }

    /// 映射到轴置换 / Map to axis permutation
    ///
    /// 将朝向映射为从原始 (W, H, D) 到放置后 (W', H', D') 的轴置换。
    /// Maps an orientation to the axis permutation from original (W, H, D)
    /// to placed (W', H', D').
    pub fn to_axis_permutation(self) -> AxisPermutation3 {
        match self {
            // Upright: W→W, H→H, D→D
            Self::Upright => AxisPermutation3::XYZ,
            // UprightRotated: W→D, H→H, D→W
            Self::UprightRotated => AxisPermutation3::ZYX,
            // Side: W→H, H→W, D→D
            Self::Side => AxisPermutation3::YXZ,
            // SideRotated: W→D, H→W, D→H
            Self::SideRotated => AxisPermutation3::ZXY,
            // Lie: W→W, H→D, D→H
            Self::Lie => AxisPermutation3::XZY,
            // LieRotated: W→H, H→D, D→W
            Self::LieRotated => AxisPermutation3::YZX,
        }
    }

    /// 所有朝向 / All orientations
    pub const ALL: [Self; 6] = [
        Self::Upright,
        Self::UprightRotated,
        Self::Side,
        Self::SideRotated,
        Self::Lie,
        Self::LieRotated,
    ];
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_category_matches() {
        assert_eq!(
            Orientation::Upright.category(),
            OrientationCategory::Upright
        );
        assert_eq!(
            Orientation::UprightRotated.category(),
            OrientationCategory::Upright
        );
        assert_eq!(Orientation::Side.category(), OrientationCategory::Side);
        assert_eq!(Orientation::Lie.category(), OrientationCategory::Lie);
    }

    #[test]
    fn orientation_rotation_pairs() {
        assert_eq!(Orientation::Upright.rotation(), Orientation::UprightRotated);
        assert_eq!(Orientation::UprightRotated.rotation(), Orientation::Upright);
        assert_eq!(Orientation::Side.rotation(), Orientation::SideRotated);
        assert_eq!(Orientation::Lie.rotation(), Orientation::LieRotated);
    }

    #[test]
    fn orientation_is_rotated() {
        assert!(!Orientation::Upright.is_rotated());
        assert!(Orientation::UprightRotated.is_rotated());
        assert!(!Orientation::Side.is_rotated());
        assert!(Orientation::SideRotated.is_rotated());
        assert!(!Orientation::Lie.is_rotated());
        assert!(Orientation::LieRotated.is_rotated());
    }

    #[test]
    fn orientation_axis_permutation_identity() {
        // Upright 应为恒等置换
        assert_eq!(
            Orientation::Upright.to_axis_permutation(),
            AxisPermutation3::XYZ
        );
    }
}
