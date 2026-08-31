    #[test]
    fn pattern_layer_generator_uses_block_loading_fallback_for_cylinder_items() {
        let items = vec![ActualItem {
            id: "cyl".into(),
            name: "Cylinder".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(5.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
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

        let result = PatternLayerGenerator::new().generate(&request);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source, "pattern_layer_generator");
        assert_eq!(result[0].layer.demand_coverage[0].coefficient, 1.0);
        assert_eq!(result[0].placement_traces[0].item_id, "cyl");
        assert!(result[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("block-loading fallback for cylinder item")));
    }

    #[test]
    fn pattern_layer_generator_applies_with_piling_limit() {
        let items = vec![ActualItem {
            id: "i0".into(),
            name: "Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }];
        let request = LayerGenerationRequest::new(0, items)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i0".into() },
                demand: 4.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(4.0),
                depth: meters(2.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);
        let limited = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(1),
            ..Default::default()
        })
        .generate(&request);
        let unbounded = PatternLayerGenerator::new().generate(&request);

        assert_eq!(limited.len(), 1);
        assert!(limited[0].layer.demand_coverage[0].coefficient <= unbounded[0].layer.demand_coverage[0].coefficient);
        assert!(limited[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("with_piling=Some(1)")));
    }

    #[test]
    fn pattern_layer_generator_uses_step_pile_amount() {
        let items = vec![ActualItem {
            id: "i0".into(),
            name: "Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }];
        let request = LayerGenerationRequest::new(0, items)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i0".into() },
                demand: 6.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(4.0),
                height: meters(3.0),
                depth: meters(2.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);
        let generator = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(3).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
                PatternStep::new(
                    PatternProjectionOrientation::Front,
                    Some(PatternNextPointPolicy::RightBottom),
                ),
            ]]),
            max_candidates: 1,
            ..Default::default()
        });

        let results = generator.generate(&request);
        let step_result = results
            .iter()
            .find(|result| {
                result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.contains("step placement candidate"))
            })
            .expect("pattern step candidate must be generated");

        assert!(!results.is_empty());
        assert_eq!(step_result.placement_traces.len(), 2);
        assert_eq!(step_result.placement_traces[0].amount, 3);
        assert_eq!(step_result.placement_traces[1].amount, 3);
        assert_eq!(step_result.block_traces[0].ny, 3);
        assert_eq!(step_result.layer.demand_coverage[0].coefficient, 6.0);
        assert_eq!(step_result.layer.depth, meters(2.0));
        assert!(step_result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("placed_items=6")));
    }

    #[test]
    fn pattern_layer_generator_builds_mixed_height_pile() {
        let items = vec![
            ActualItem {
                id: "tall".into(),
                name: "Tall".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(2.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "short".into(),
                name: "Short".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let request = LayerGenerationRequest::new(0, items)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "short".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "tall".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
            ])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(2.0),
                capacity: meters(10.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);
        let generator = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(2).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
        });

        let results = generator.generate(&request);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 2);
        assert_eq!(results[0].placement_traces[0].item_id, "tall");
        assert_eq!(results[0].placement_traces[0].position.y, meters(0.0));
        assert_eq!(results[0].placement_traces[1].item_id, "short");
        assert_eq!(results[0].placement_traces[1].position.y, meters(2.0));
        assert_eq!(results[0].layer.depth, meters(2.0));
        assert!(results[0]
            .layer
            .demand_coverage
            .iter()
            .any(|coverage| {
                coverage.key == Bpp3dDemandKey::Item { id: "short".into() }
                    && coverage.coefficient == 1.0
            }));
        assert!(results[0]
            .layer
            .demand_coverage
            .iter()
            .any(|coverage| {
                coverage.key == Bpp3dDemandKey::Item { id: "tall".into() }
                    && coverage.coefficient == 1.0
            }));
        assert!(results[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("placed_piles=1")));
    }

    #[test]
    fn pattern_layer_generator_sorts_single_item_candidates_by_height_fit() {
        let items = vec![
            ActualItem {
                id: "tall".into(),
                name: "Tall".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(2.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "short".into(),
                name: "Short".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let request = LayerGenerationRequest::new(0, items)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "short".into() },
                    demand: 3.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "tall".into() },
                    demand: 2.0,
                    satisfied: 0.0,
                },
            ])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(2.0),
                capacity: meters(10.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);
        let generator = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(1).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
        });

        let results = generator.generate(&request);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 1);
        assert_eq!(results[0].placement_traces[0].item_id, "short");
        assert_eq!(results[0].placement_traces[0].amount, 1);
        assert!(results[0]
            .layer
            .demand_coverage
            .iter()
            .any(|coverage| {
                coverage.key == Bpp3dDemandKey::Item { id: "short".into() }
                    && coverage.coefficient == 1.0
        }));
    }

