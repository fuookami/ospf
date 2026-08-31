// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{
        HangingPolicy, PackageOrientationRule, PackagePairStackingRule,
        PackagePlacementStackingRule, PackageShapeSpec, PackageType,
    };
    use crate::infrastructure::orientation::OrientationCategory;
    use ospf_rust_quantities::unit::derived::Meter;
    use ospf_rust_quantities::quantity::Quantity;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }


    include!("tests/context.rs");
    include!("tests/basic_generators.rs");
    include!("tests/deferred_diagnostics.rs");
    include!("tests/pattern_core.rs");
    include!("tests/pattern_rules.rs");
    include!("tests/pile_and_deferred.rs");
    include!("tests/block_and_bla.rs");
    include!("tests/pattern_dimension.rs");
}
