// ============================================================================
// PackageShapeSpec - 包装形状规格 / Package shape specification
// ============================================================================

/// 包装形状规格 / Package shape specification
///
/// 区分长方体和轴感知圆柱，不沿用 Kotlin 的 `VerticalCylinder` 命名误导。
/// Distinguishes cuboids and axis-aware cylinders; does not follow Kotlin's
/// misleading `VerticalCylinder` naming.
#[derive(Debug, Clone)]
pub enum PackageShapeSpec<V, U: UnitTrait> {
    /// 长方体 / Cuboid
    Cuboid,
    /// 轴感知圆柱 / Axis-aware cylinder
    Cylinder {
        /// 对齐轴 / Alignment axis
        axis: Axis3,
        /// 半径 / Radius
        radius: Quantity<V, U>,
        /// 半径候选列表（离散半径）/ Radius candidates (discrete radius)
        radius_candidates: Option<Vec<Quantity<V, U>>>,
        /// 半径下界（连续半径）/ Radius lower bound (continuous radius)
        radius_lower_bound: Option<Quantity<V, U>>,
        /// 半径上界（连续半径）/ Radius upper bound (continuous radius)
        radius_upper_bound: Option<Quantity<V, U>>,
    },
}

// ============================================================================
// PackageShape - 包装形状 / Package shape
// ============================================================================

/// 包装形状 / Package shape
///
/// 包装的完整几何描述，包括尺寸、重量和形状规格。
/// Complete geometric description of a package, including dimensions, weight,
/// and shape specification.
#[derive(Debug, Clone)]
pub struct PackageShape<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 形状规格 / Shape specification
    pub spec: PackageShapeSpec<V, U>,
}

impl<V: Clone + Field + num_traits::FloatConst, U: CTUnit + Default + Clone> PackageShape<V, U> {
    /// 转换为 PackingShape3 / Convert to PackingShape3
    pub fn to_packing_shape(&self) -> PackingShape3<V, U> {
        match &self.spec {
            PackageShapeSpec::Cuboid => {
                cuboid_packing_shape(
                    self.width.clone(),
                    self.height.clone(),
                    self.depth.clone(),
                    self.weight.clone(),
                )
            }
            PackageShapeSpec::Cylinder { axis, radius, .. } => {
                let axis_length = match axis {
                    Axis3::X => self.width.clone(),
                    Axis3::Y => self.height.clone(),
                    Axis3::Z => self.depth.clone(),
                };
                cylinder_packing_shape(
                    radius.clone(),
                    axis_length,
                    *axis,
                    self.weight.clone(),
                )
            }
        }
    }
}

