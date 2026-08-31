//! 包装形状 / Packing shapes
//!
//! BPP3D 领域中使用的三维包装形状，包括长方体和圆柱体。
//! 3D packing shapes used in the BPP3D domain, including cuboids and cylinders.

use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use super::PackingAlgorithmShapeType;

// ============================================================================
// ShapeFootprint2 - 二维足迹 / 2D footprint
// ============================================================================

/// 二维足迹形状 / Two-dimensional footprint shape
///
/// 三维物体在投影平面上的二维足迹，可以是矩形或圆形。
/// Two-dimensional footprint of a 3D object on a projection plane,
/// which can be a rectangle or a circle.
#[derive(Debug, Clone)]
pub enum ShapeFootprint2<V, U: UnitTrait> {
    /// 矩形足迹 / Rectangle footprint
    Rectangle {
        /// 宽度 / Width
        width: Quantity<V, U>,
        /// 深度 / Depth
        depth: Quantity<V, U>,
    },
    /// 圆形足迹 / Circle footprint
    Circle {
        /// 半径 / Radius
        radius: Quantity<V, U>,
    },
}

// ============================================================================
// PackingShape3 - 三维包装形状 / 3D packing shape
// ============================================================================

/// 三维包装形状 / Three-dimensional packing shape
///
/// BPP3D 领域中物体的三维形状描述，统一了长方体和圆柱体。
/// 3D shape description of objects in the BPP3D domain,
/// unifying cuboids and cylinders.
#[derive(Debug, Clone)]
pub struct PackingShape3<V, U: UnitTrait> {
    /// 形状类型 / Shape type
    pub shape_type: PackingShapeType,
    /// 算法侧形状类型 / Algorithm-side shape type
    pub algorithm_shape_type: PackingAlgorithmShapeType,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 包围宽度 / Bounding width
    pub bounding_width: Quantity<V, U>,
    /// 包围高度 / Bounding height
    pub bounding_height: Quantity<V, U>,
    /// 包围深度 / Bounding depth
    pub bounding_depth: Quantity<V, U>,
    /// 实际体积 / Actual volume
    pub actual_volume: Quantity<V, U>,
    /// 对齐轴（圆柱专用）/ Alignment axis (cylinder only)
    pub axis: Option<Axis3>,
    /// 圆柱半径（圆柱专用）/ Cylinder radius (cylinder only)
    pub radius: Option<Quantity<V, U>>,
}

/// 包装形状类型 / Packing shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackingShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 圆柱 / Cylinder
    Cylinder,
}

impl<V: Clone, U: CTUnit + Default + Clone> PackingShape3<V, U> {
    /// 获取二维足迹 / Get 2D footprint
    pub fn footprint(&self) -> ShapeFootprint2<V, U> {
        match self.shape_type {
            PackingShapeType::Cuboid => ShapeFootprint2::Rectangle {
                width: self.bounding_width.clone(),
                depth: self.bounding_depth.clone(),
            },
            PackingShapeType::Cylinder => {
                if let Some(radius) = &self.radius {
                    ShapeFootprint2::Circle {
                        radius: radius.clone(),
                    }
                } else {
                    // 退化情况：无半径信息时使用包围矩形
                    ShapeFootprint2::Rectangle {
                        width: self.bounding_width.clone(),
                        depth: self.bounding_depth.clone(),
                    }
                }
            }
        }
    }
}

// ============================================================================
// 从长方体形状构造 / Construct from cuboid shape
// ============================================================================

/// 从长方体几何构造包装形状 / Construct packing shape from cuboid geometry
pub fn cuboid_packing_shape<V, U>(width: Quantity<V, U>, height: Quantity<V, U>, depth: Quantity<V, U>, weight: Quantity<V, U>) -> PackingShape3<V, U>
where
    U: CTUnit + Default + Clone,
    V: Field + Clone,
{
    let volume = Quantity::new_ct(width.value.clone() * height.value.clone() * depth.value.clone());
    PackingShape3 {
        shape_type: PackingShapeType::Cuboid,
        algorithm_shape_type: PackingAlgorithmShapeType::Cuboid,
        weight,
        bounding_width: width,
        bounding_height: height,
        bounding_depth: depth,
        actual_volume: volume,
        axis: None,
        radius: None,
    }
}

/// 从圆柱体几何构造包装形状 / Construct packing shape from cylinder geometry
pub fn cylinder_packing_shape<V, U>(
    radius: Quantity<V, U>,
    height: Quantity<V, U>,
    axis: Axis3,
    weight: Quantity<V, U>,
) -> PackingShape3<V, U>
where
    U: CTUnit + Default + Clone,
    V: Field + Clone + num_traits::FloatConst,
{
    let pi = V::PI();
    let actual_volume = Quantity::new_ct(pi * radius.value.clone() * radius.value.clone() * height.value.clone());

    let diameter = Quantity::new_ct(radius.value.clone() + radius.value.clone());

    let (bounding_width, bounding_height, bounding_depth) = match axis {
        Axis3::X => (height, diameter.clone(), diameter),
        Axis3::Y => (diameter.clone(), height, diameter),
        Axis3::Z => (diameter.clone(), diameter, height),
    };

    PackingShape3 {
        shape_type: PackingShapeType::Cylinder,
        algorithm_shape_type: PackingAlgorithmShapeType::from_cylinder_axis(axis),
        weight,
        bounding_width,
        bounding_height,
        bounding_depth,
        actual_volume,
        axis: Some(axis),
        radius: Some(radius),
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn cuboid_packing_shape_has_rectangle_footprint() {
        let shape = cuboid_packing_shape(meters(2.0), meters(3.0), meters(4.0), meters(1.0));
        assert_eq!(shape.shape_type, PackingShapeType::Cuboid);
        assert_eq!(shape.algorithm_shape_type, PackingAlgorithmShapeType::Cuboid);

        let footprint = shape.footprint();
        match footprint {
            ShapeFootprint2::Rectangle { width, depth } => {
                assert_eq!(width.value, 2.0);
                assert_eq!(depth.value, 4.0);
            }
            _ => panic!("Expected rectangle footprint"),
        }
    }

    #[test]
    fn vertical_cylinder_packing_shape_has_circle_footprint() {
        let shape = cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0));
        assert_eq!(shape.shape_type, PackingShapeType::Cylinder);
        assert_eq!(shape.algorithm_shape_type, PackingAlgorithmShapeType::VerticalCylinder);
        assert_eq!(shape.axis, Some(Axis3::Y));

        // 包围盒应为直径 x 高度 x 直径
        assert_eq!(shape.bounding_width.value, 4.0);
        assert_eq!(shape.bounding_height.value, 5.0);
        assert_eq!(shape.bounding_depth.value, 4.0);

        let footprint = shape.footprint();
        match footprint {
            ShapeFootprint2::Circle { radius } => {
                assert_eq!(radius.value, 2.0);
            }
            _ => panic!("Expected circle footprint"),
        }
    }

    #[test]
    fn horizontal_cylinder_x_axis_shape_type() {
        let shape = cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0));
        assert_eq!(shape.algorithm_shape_type, PackingAlgorithmShapeType::HorizontalCylinderX);
        assert_eq!(shape.bounding_width.value, 5.0); // height along X
        assert_eq!(shape.bounding_height.value, 4.0); // diameter along Y
        assert_eq!(shape.bounding_depth.value, 4.0); // diameter along Z
    }

    #[test]
    fn horizontal_cylinder_z_axis_shape_type() {
        let shape = cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Z, meters(1.0));
        assert_eq!(shape.algorithm_shape_type, PackingAlgorithmShapeType::HorizontalCylinderZ);
    }
}
