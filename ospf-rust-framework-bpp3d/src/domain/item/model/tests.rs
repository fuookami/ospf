// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::packing_shape::PackingShapeType;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn stacking_input<'a>(
        item: &'a PackageAttribute,
        bottom_item: &'a PackageAttribute,
    ) -> PackageStackingInput<'a> {
        PackageStackingInput {
            item,
            bottom_item: Some(bottom_item),
            layer: 0,
            height: 0.0,
            item_width: 10.0,
            item_height: 2.0,
            item_depth: 10.0,
            item_weight: 8.0,
            bottom_width: 10.0,
            bottom_depth: 10.0,
            bottom_weight: 8.0,
            item_orientation: Orientation::Upright,
            bottom_orientation: Orientation::Upright,
            item_orientation_enabled: true,
            item_orientation_enabled_at_space: true,
            bottom_orientation_enabled: true,
            space_width: 10.0,
            space_height: 10.0,
            space_depth: 10.0,
        }
    }


    include!("tests/shape_and_pattern.rs");
    include!("tests/patterned_item.rs");
    include!("tests/material_and_package.rs");
    include!("tests/package_attribute.rs");
    include!("tests/bin_and_cylinder.rs");
    include!("tests/continuous_radius.rs");
    include!("tests/cargo_metadata.rs");
    include!("tests/bottom_dimension.rs");
}
