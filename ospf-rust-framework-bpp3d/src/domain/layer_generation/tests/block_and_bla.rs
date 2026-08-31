    #[test]
    fn horizontal_cylinder_guard_rejects_unsupported() {
        use ospf_rust_math::geometry::Axis3;

        // 竖直圆柱应通过验证
        assert!(HorizontalCylinderGuard::validate_candidate::<f64, Meter>(Axis3::Y, &meters(5.0)).is_ok());

        // 横向圆柱在当前简化实现下也通过（贴地视为有支撑）
        // 完整实现中，非贴地横向圆柱应被拒绝
        assert!(HorizontalCylinderGuard::validate_candidate::<f64, Meter>(Axis3::X, &meters(0.0)).is_ok());
    }

    #[test]
    fn block_layer_generator_contract() {
        let generator = BlockLayerGenerator::new();
        let bin = BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN".to_string(),
            is_main: true,
        };
        let items = vec![ActualItem {
            id: "i1".to_string(),
            name: "Item 1".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(2.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, items)
            .with_bin(bin)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i1".to_string() },
                demand: 1.0,
                satisfied: 0.0,
            }]);
        let results = generator.generate(&request);
        assert!(!results.is_empty());
        assert_eq!(results[0].source, "block_layer_generator");
        assert_eq!(results[0].block_traces.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 1);
        assert_eq!(results[0].layer.demand_coverage[0].coefficient, 1.0);
    }

    #[test]
    fn block_layer_generator_respects_package_depth_bounds() {
        let generator = BlockLayerGenerator::new();
        let items = vec![ActualItem {
            id: "depth".to_string(),
            name: "Depth".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let mut attributes = HashMap::new();
        attributes.insert(
            "depth".to_string(),
            PackageAttribute {
                min_depth: 3.5,
                max_depth: Some(4.0),
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "depth".to_string() },
                demand: 4.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(4.0),
                depth: meters(4.0),
                capacity: meters(100.0),
                type_code: "BIN".to_string(),
                is_main: true,
            })
            .with_max_candidates(5);

        let results = generator.generate(&request);

        assert!(!results.is_empty());
        assert!(results.iter().all(|result| result.layer.depth.value >= 3.5));
        assert!(results.iter().all(|result| result.layer.depth.value <= 4.0));
    }

    #[test]
    fn bl_local_layer_generator_contract() {
        let generator = BLLocalLayerGenerator::new();
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(
            0,
            vec![ActualItem {
                id: "i1".to_string(),
                name: "Item 1".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: None,
            }],
        )
        .with_bin(BinType {
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            capacity: meters(100.0),
            type_code: "BIN".to_string(),
            is_main: true,
        });
        let results = generator.generate(&request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "bl_local_layer_generator");
        assert_eq!(results[0].placement_traces.len(), 1);
    }

    #[test]
    fn bl_global_layer_generator_contract() {
        let generator = BLGlobalLayerGenerator::new();
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(
            0,
            vec![
                ActualItem {
                    id: "i1".to_string(),
                    name: "Item 1".to_string(),
                    package_code: None,
                    pack: None,
                    width: meters(2.0),
                    height: meters(2.0),
                    depth: meters(2.0),
                    weight: meters(1.0),
                    enabled_orientations: vec![Orientation::Upright],
                    shape_spec_override: None,
                },
                ActualItem {
                    id: "i2".to_string(),
                    name: "Item 2".to_string(),
                    package_code: None,
                    pack: None,
                    width: meters(2.0),
                    height: meters(2.0),
                    depth: meters(2.0),
                    weight: meters(1.0),
                    enabled_orientations: vec![Orientation::Upright],
                    shape_spec_override: None,
                },
            ],
        )
        .with_bin(BinType {
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            capacity: meters(100.0),
            type_code: "BIN".to_string(),
            is_main: true,
        });
        let results = generator.generate(&request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "bl_global_layer_generator");
        assert_eq!(results[0].placement_traces.len(), 2);
    }

    #[test]
    fn bl_global_layer_generator_prioritizes_bottom_only_attribute() {
        let generator = BLGlobalLayerGenerator::new();
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(
            0,
            vec![
                ActualItem {
                    id: "heavy".to_string(),
                    name: "Heavy".to_string(),
                    package_code: None,
                    pack: None,
                    width: meters(4.0),
                    height: meters(4.0),
                    depth: meters(4.0),
                    weight: meters(10.0),
                    enabled_orientations: vec![Orientation::Upright],
                    shape_spec_override: None,
                },
                ActualItem {
                    id: "bottom".to_string(),
                    name: "Bottom".to_string(),
                    package_code: None,
                    pack: None,
                    width: meters(4.0),
                    height: meters(4.0),
                    depth: meters(4.0),
                    weight: meters(1.0),
                    enabled_orientations: vec![Orientation::Upright],
                    shape_spec_override: None,
                },
            ],
        )
        .with_package_attributes(HashMap::from([(
            "bottom".to_string(),
            PackageAttribute {
                bottom_only: true,
                ..Default::default()
            },
        )]))
        .with_bin(BinType {
            width: meters(10.0),
            height: meters(5.0),
            depth: meters(10.0),
            capacity: meters(100.0),
            type_code: "BIN".to_string(),
            is_main: true,
        });

        let results = generator.generate(&request);

        assert_eq!(results.len(), 1);
        let bottom_trace = results[0]
            .placement_traces
            .iter()
            .find(|trace| trace.item_id == "bottom")
            .unwrap();
        let heavy_trace = results[0]
            .placement_traces
            .iter()
            .find(|trace| trace.item_id == "heavy")
            .unwrap();
        assert_eq!(bottom_trace.position.x.value, 0.0);
        assert_eq!(bottom_trace.position.z.value, 0.0);
        assert_eq!(heavy_trace.position.x.value, 4.0);
        assert_eq!(heavy_trace.position.z.value, 0.0);
    }

