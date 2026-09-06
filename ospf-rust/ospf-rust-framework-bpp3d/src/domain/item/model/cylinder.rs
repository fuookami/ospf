// ============================================================================
// CylinderShapeContract - 圆柱形状契约 / Cylinder shape contract
// ============================================================================

/// 圆柱能力状态 / Cylinder capability status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CylinderCapabilityStatus {
    /// 仅长方体 / Cuboid only
    CuboidOnly,
    /// 仅竖直候选 / Vertical candidate only
    VerticalCandidateOnly,
    /// 轴感知候选 / Axis-aware candidate
    AxisAwareCandidate,
    /// 验证过的生成放置 / Verified generated placement
    VerifiedGeneratedPlacement,
    /// 竖直竖向支撑 / Upright vertical support only
    UprightVerticalSupportOnly,
    /// 已知坐标最终验证 / Known-coordinate final validation
    KnownCoordinateFinalValidation,
}

/// 圆柱形状契约 / Cylinder shape contract
///
/// 定义了 BPP3D 中圆柱形状在不同算法路径下的能力验证规则。
/// Defines capability verification rules for cylinder shapes across
/// different algorithm paths in BPP3D.
pub struct CylinderShapeContract;

impl CylinderShapeContract {
    /// 检查是否有圆柱形状 / Check if there are any cylinder shapes
    pub fn has_cylinder<V, U: UnitTrait>(items: &[ActualItem<V, U>]) -> bool {
        items.iter().any(|item| {
            matches!(&item.shape_spec_override, Some(PackageShapeSpec::Cylinder { .. }))
        })
    }

    /// 要求竖直圆柱轴 / Require vertical cylinder axis
    pub fn require_vertical_axis(axis: Axis3) -> Result<(), String> {
        if axis != Axis3::Y {
            Err(format!(
                "Vertical cylinder axis required, got {:?}. / 要求竖直圆柱轴，得到 {:?}。",
                axis, axis
            ))
        } else {
            Ok(())
        }
    }

    /// 要求轴感知圆柱候选 / Require axis-aware cylinder candidate
    pub fn require_axis_aware_candidate(axis: Axis3) -> Result<(), String> {
        match axis {
            Axis3::X | Axis3::Y | Axis3::Z => Ok(()),
        }
    }
}

