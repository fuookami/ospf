    #[test]
    fn bla_contract_single_placement() {
        use crate::domain::bla::BottomUpLeftJustifiedAlgorithm;
        use crate::infrastructure::packing_shape::ShapeFootprint2;
        use crate::domain::bla::service::{BlaProjection, BlaConfig};

        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );
        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Rectangle {
                width: meters(3.0),
                depth: meters(4.0),
            },
            item_index: 0,
            bottom_only: false,
            height: meters(4.0),
            weight: meters(1.0),
            allow_rotation: true,
        }];
        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
    }

    #[test]
    fn bla_contract_no_overlap() {
        use crate::domain::bla::BottomUpLeftJustifiedAlgorithm;
        use crate::infrastructure::packing_shape::ShapeFootprint2;
        use crate::domain::bla::service::{BlaProjection, BlaConfig};
        use crate::infrastructure::geometry::MetricAabb2;

        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );
        let projections = vec![
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle { width: meters(5.0), depth: meters(5.0) },
                item_index: 0, bottom_only: false, height: meters(5.0), weight: meters(2.0), allow_rotation: true,
            },
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle { width: meters(5.0), depth: meters(5.0) },
                item_index: 1, bottom_only: false, height: meters(5.0), weight: meters(1.0), allow_rotation: true,
            },
        ];
        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        assert!(placements[1].is_some());
        // 验证不重叠
        let p0 = placements[0].as_ref().unwrap();
        let p1 = placements[1].as_ref().unwrap();
        let a0 = MetricAabb2::new(p0.position.clone(), crate::infrastructure::geometry::MetricSize2 { width: meters(5.0), height: meters(5.0) });
        let a1 = MetricAabb2::new(p1.position.clone(), crate::infrastructure::geometry::MetricSize2 { width: meters(5.0), height: meters(5.0) });
        assert!(!a0.overlaps(&a1));
    }

    #[test]
    fn simple_block_generator_contract() {
        use crate::domain::block_loading::{SimpleBlockGenerator, SimpleBlockGeneratorConfig};
        use crate::infrastructure::geometry::MetricSize3;
        use crate::domain::item::ActualItem;
        use crate::infrastructure::orientation::Orientation;

        let generator = SimpleBlockGenerator::new(SimpleBlockGeneratorConfig::default());
        let items = vec![ActualItem {
            id: "i1".into(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let amounts = vec![10u64];
        let container = MetricSize3 { width: meters(10.0), height: meters(10.0), depth: meters(10.0) };

        let blocks = generator.generate(&items, &amounts, &container);
        assert!(!blocks.is_empty());
        // 验证所有块的尺寸不超过容器
        for block in &blocks {
            assert!(block.width().value <= 10.0);
            assert!(block.height().value <= 10.0);
            assert!(block.depth().value <= 10.0);
        }
    }

    #[test]
    fn circle_packing_with_fixed_radius() {
        use crate::domain::item::{ActualItem, PackageShapeSpec};
        use ospf_rust_math::geometry::Axis3;

        let generator = CirclePackingLayerGenerator::new();
        let items = vec![ActualItem {
            id: "c1".into(),
            name: "Cylinder".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(5.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        }];
        let request = LayerGenerationRequest::new(0, items)
            .with_bin(BinType {
                width: meters(10.0),
                height: meters(10.0),
                depth: meters(10.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            });
        let results = generator.generate(&request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "circle_packing_layer_generator");
        assert_eq!(results[0].layer.demand_coverage[0].coefficient, 4.0);
        assert_eq!(results[0].placement_traces[0].amount, 4);
        assert!(results[0].diagnostics[0].contains("conservative grid candidate"));
    }

    #[test]
    fn circle_packing_generates_axis_x_and_z_cylinders() {
        use crate::domain::item::{ActualItem, PackageShapeSpec};
        use ospf_rust_math::geometry::Axis3;

        let generator = CirclePackingLayerGenerator::new();
        let items = vec![
            ActualItem {
                id: "cx".into(),
                name: "Cylinder X".to_string(),
                package_code: None,
                pack: None,
                width: meters(8.0),
                height: meters(4.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::X,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            ActualItem {
                id: "cz".into(),
                name: "Cylinder Z".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(4.0),
                depth: meters(9.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Z,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
        ];
        let request = LayerGenerationRequest::new(0, items)
            .with_bin(BinType {
                width: meters(10.0),
                height: meters(10.0),
                depth: meters(10.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(4);

        let results = generator.generate(&request);

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|result| {
            result.placement_traces[0].item_id == "cx"
                && result.layer.depth == meters(4.0)
                && result.layer.demand_coverage[0].coefficient == 4.0
        }));
        assert!(results.iter().any(|result| {
            result.placement_traces[0].item_id == "cz"
                && result.layer.depth == meters(9.0)
                && result.layer.demand_coverage[0].coefficient == 4.0
        }));
    }

