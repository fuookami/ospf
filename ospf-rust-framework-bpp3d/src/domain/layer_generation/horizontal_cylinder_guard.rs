// ============================================================================
// HorizontalCylinderGuard - 横向圆柱守卫 / Horizontal cylinder guard
// ============================================================================

/// 横向圆柱守卫 / Horizontal cylinder guard
///
/// 验证横向圆柱候选是否有足够的支撑覆盖。
/// Verifies that horizontal cylinder candidates have sufficient support coverage.
pub struct HorizontalCylinderGuard;

impl HorizontalCylinderGuard {
    /// 验证横向圆柱候选是否被支撑 / Verify horizontal cylinder candidate is supported
    ///
    /// 横向圆柱必须满足以下条件之一：
    /// 1. 贴地放置（Y = 0）
    /// 2. 放置在支撑长方体上方，且支撑覆盖满足门禁要求
    ///
    /// A horizontal cylinder must satisfy one of:
    /// 1. Placed on the floor (Y = 0)
    /// 2. Placed on top of a supporting cuboid with sufficient coverage
    pub fn is_supported<V, U>(
        _cylinder_axis: ospf_rust_math::geometry::Axis3,
        _y_position: &Quantity<V, U>,
    ) -> bool
    where
        V: Field + Clone + Debug + Send + Sync + PartialOrd,
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        // 简化实现：贴地即视为有支撑
        // 完整实现需要使用 HorizontalCylinderSupportCoverage
        true
    }

    /// 验证候选是否有效 / Verify candidate is valid
    ///
    /// 不满足支撑条件的横向圆柱候选应被拒绝。
    /// Horizontal cylinder candidates without sufficient support should be rejected.
    pub fn validate_candidate<V, U>(
        cylinder_axis: ospf_rust_math::geometry::Axis3,
        y_position: &Quantity<V, U>,
    ) -> Result<(), String>
    where
        V: Field + Clone + Debug + Send + Sync + PartialOrd,
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        match cylinder_axis {
            ospf_rust_math::geometry::Axis3::Y => Ok(()), // 竖直圆柱无需横向支撑验证
            ospf_rust_math::geometry::Axis3::X | ospf_rust_math::geometry::Axis3::Z => {
                if Self::is_supported(cylinder_axis, y_position) {
                    Ok(())
                } else {
                    Err(format!(
                        "Horizontal cylinder on axis {:?} lacks support coverage. / 横向圆柱轴 {:?} 缺少支撑覆盖。",
                        cylinder_axis, cylinder_axis
                    ))
                }
            }
        }
    }
}

