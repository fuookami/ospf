// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::model::PackageSolutionLikeAdapter;
    use crate::domain::item::{ActualItem, BinType, PackageShapeSpec, Package, MaterialType};
    use crate::infrastructure::geometry::MetricPoint3;
    use crate::infrastructure::orientation::Orientation;
    use crate::infrastructure::packing_shape::cuboid_packing_shape;
    use ospf_rust_quantities::quantity::Quantity;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn make_bin_type() -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN-10".into(),
            is_main: true,
        }
    }

    fn make_cuboid_packed_item(index: usize, x: f64, y: f64, z: f64, w: f64, h: f64, d: f64) -> PackedItem<f64, Meter> {
        PackedItem {
            item_index: index,
            item: crate::domain::item::ActualItem {
                id: format!("item_{}", index).into(),
                name: format!("Item {}", index),
                package_code: None,
                pack: None,
                width: meters(w),
                height: meters(h),
                depth: meters(d),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: None,
            },
            position: MetricPoint3 { x: meters(x), y: meters(y), z: meters(z) },
            orientation: Orientation::Upright,
            packing_shape: cuboid_packing_shape(meters(w), meters(h), meters(d), meters(1.0)),
            loading_order: 0,
        }
    }


    include!("tests/geometry_guard.rs");
    include!("tests/packing_result.rs");
    include!("tests/placement_and_support.rs");
    include!("tests/material.rs");
    include!("tests/cylinder_geometry.rs");
    include!("tests/renderer_and_hanging.rs");
}
