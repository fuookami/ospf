// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_math::geometry::Axis3;
    use crate::domain::item::{PackageOrientationRule, PackageShapeSpec};
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn make_cuboid_item(id: &str, w: f64, h: f64, d: f64, weight: f64) -> ActualItem<f64, Meter> {
        ActualItem {
            id: id.into(),
            name: format!("Item {}", id),
            package_code: None,
            pack: None,
            width: meters(w),
            height: meters(h),
            depth: meters(d),
            weight: meters(weight),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }
    }

    fn make_cylinder_item(id: &str, radius: f64, h: f64, axis: Axis3, weight: f64) -> ActualItem<f64, Meter> {
        let diameter = 2.0 * radius;
        let (width, height, depth) = match axis {
            Axis3::X => (h, diameter, diameter),
            Axis3::Y => (diameter, h, diameter),
            Axis3::Z => (diameter, diameter, h),
        };
        ActualItem {
            id: id.into(),
            name: format!("Cylinder {}", id),
            package_code: None,
            pack: None,
            width: meters(width),
            height: meters(height),
            depth: meters(depth),
            weight: meters(weight),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis,
                radius: meters(radius),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        }
    }


    include!("tests/simple_cuboid.rs");
    include!("tests/simple_cylinder.rs");
    include!("tests/simple_limits.rs");
    include!("tests/advanced_generators.rs");
}
