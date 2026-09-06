// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::derived::Meter;

    type LengthF64 = Quantity<f64, Meter>;

    fn meters(v: f64) -> LengthF64 {
        Quantity::new_ct(v)
    }

    #[test]
    fn metric_point2_from_raw_and_access() {
        let p = MetricPoint2::<f64, Meter>::from_raw(1.0, 2.0);
        assert_eq!(p.x.value, 1.0);
        assert_eq!(p.y.value, 2.0);
    }

    #[test]
    fn metric_point3_from_raw_and_access() {
        let p = MetricPoint3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        assert_eq!(p.x.value, 1.0);
        assert_eq!(p.y.value, 2.0);
        assert_eq!(p.z.value, 3.0);
    }

    #[test]
    fn metric_vector2_add_sub() {
        let v1 = MetricVector2::<f64, Meter>::from_raw(1.0, 2.0);
        let v2 = MetricVector2::<f64, Meter>::from_raw(3.0, 4.0);

        let sum = v1.clone() + v2.clone();
        assert_eq!(sum.x.value, 4.0);
        assert_eq!(sum.y.value, 6.0);

        let diff = v1 - v2;
        assert_eq!(diff.x.value, -2.0);
        assert_eq!(diff.y.value, -2.0);
    }

    #[test]
    fn metric_point3_sub_returns_vector() {
        let p1 = MetricPoint3::<f64, Meter>::from_raw(4.0, 6.0, 8.0);
        let p2 = MetricPoint3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);

        let diff = p1 - p2;
        assert_eq!(diff.x.value, 3.0);
        assert_eq!(diff.y.value, 4.0);
        assert_eq!(diff.z.value, 5.0);
    }

    #[test]
    fn metric_size3_along_axis() {
        let size = MetricSize3::<f64, Meter>::from_raw(2.0, 3.0, 4.0);
        assert_eq!(size.along(Axis3::X).value, 2.0);
        assert_eq!(size.along(Axis3::Y).value, 3.0);
        assert_eq!(size.along(Axis3::Z).value, 4.0);
    }

    #[test]
    fn metric_aabb2_overlaps_and_contains() {
        let aabb = MetricAabb2::from_point_size(
            meters(0.0), meters(0.0),
            meters(4.0), meters(4.0),
        );

        let inside = MetricPoint2::new(meters(1.0), meters(1.0));
        let outside = MetricPoint2::new(meters(5.0), meters(5.0));
        assert!(aabb.contains_point(&inside));
        assert!(!aabb.contains_point(&outside));

        let other = MetricAabb2::from_point_size(
            meters(2.0), meters(2.0),
            meters(4.0), meters(4.0),
        );
        assert!(aabb.overlaps(&other));

        let disjoint = MetricAabb2::from_point_size(
            meters(5.0), meters(5.0),
            meters(2.0), meters(2.0),
        );
        assert!(!aabb.overlaps(&disjoint));
    }

    #[test]
    fn metric_aabb3_overlaps_and_contains() {
        let aabb = MetricAabb3::from_point_size(
            meters(0.0), meters(0.0), meters(0.0),
            meters(4.0), meters(4.0), meters(4.0),
        );

        let inside = MetricPoint3::new(meters(1.0), meters(1.0), meters(1.0));
        assert!(aabb.contains_point(&inside));

        let other = MetricAabb3::from_point_size(
            meters(2.0), meters(2.0), meters(2.0),
            meters(4.0), meters(4.0), meters(4.0),
        );
        assert!(aabb.overlaps(&other));
    }

    #[test]
    fn metric_point2_equality() {
        let p1 = MetricPoint2::<f64, Meter>::from_raw(1.0, 2.0);
        let p2 = MetricPoint2::<f64, Meter>::from_raw(1.0, 2.0);
        let p3 = MetricPoint2::<f64, Meter>::from_raw(1.0, 3.0);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn metric_size3_equality() {
        let s1 = MetricSize3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        let s2 = MetricSize3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        assert_eq!(s1, s2);
    }

    #[test]
    fn scalar_point3_conversion_roundtrip() {
        let scalar = Point3::new(1.0, 2.0, 3.0);
        let typed = scalar_point3_to_typed::<f64, Meter>(&scalar);
        assert_eq!(typed.x.value, 1.0);
        assert_eq!(typed.y.value, 2.0);
        assert_eq!(typed.z.value, 3.0);

        let back = typed_point3_to_scalar(&typed);
        assert_eq!(back.x(), 1.0);
        assert_eq!(back.y(), 2.0);
        assert_eq!(back.z(), 3.0);
    }

    #[test]
    fn scalar_cuboid3_conversion_roundtrip() {
        let scalar = Cuboid3::new(2.0, 3.0, 4.0);
        let typed = scalar_cuboid3_to_typed_size::<f64, Meter>(&scalar);
        assert_eq!(typed.width.value, 2.0);
        assert_eq!(typed.height.value, 3.0);
        assert_eq!(typed.depth.value, 4.0);

        let back = typed_size3_to_scalar_cuboid(&typed);
        assert_eq!(back.width, 2.0);
        assert_eq!(back.height, 3.0);
        assert_eq!(back.depth, 4.0);
    }

    #[test]
    fn scalar_point2_conversion_roundtrip() {
        let scalar = Point2::new(1.0, 2.0);
        let typed = scalar_point2_to_typed::<f64, Meter>(&scalar);
        assert_eq!(typed.x.value, 1.0);
        assert_eq!(typed.y.value, 2.0);

        let back = typed_point2_to_scalar(&typed);
        assert_eq!(back.x(), 1.0);
        assert_eq!(back.y(), 2.0);
    }

    #[test]
    fn metric_placement3_construction() {
        let position = MetricPoint3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        let shape = Cuboid3::new(2.0, 3.0, 4.0);
        let placement = MetricPlacement3::new(position, shape);
        assert_eq!(placement.position.x.value, 1.0);
        assert_eq!(placement.shape.width, 2.0);
    }
}
