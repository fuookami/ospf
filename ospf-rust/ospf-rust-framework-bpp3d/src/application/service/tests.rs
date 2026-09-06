// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    use ospf_rust_quantities::quantity::Quantity;
    use ospf_rust_quantities::unit::derived::Meter;
    #[cfg(not(feature = "async"))]
    use ospf_rust_framework::solver::{FeasibleSolution, LinearDualSolution, LPResult};

    use crate::domain::item::PackageShapeSpec;
    use crate::domain::packing::{KnownCoordinatePlacement, LayerPlacementAdapter};
    use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
    use crate::infrastructure::orientation::Orientation;

    fn meters(value: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(value)
    }

    fn bin_type() -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN".into(),
            is_main: true,
        }
    }

    fn item(id: &str) -> ActualItem<f64, Meter> {
        ActualItem {
            id: id.into(),
            name: id.to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }
    }


    include!("tests/config_state.rs");
    include!("tests/fixture_suite.rs");
    include!("tests/packing_analysis.rs");
    include!("tests/algorithm.rs");
    include!("tests/meta_model_executors.rs");
    include!("tests/application_flow.rs");
    include!("tests/csv_flow.rs");
    include!("tests/real_backends.rs");
    include!("tests/continuous_radius.rs");
    include!("tests/csv_mock_flow.rs");
}
