    #[test]
    fn pile_layer_generator_respects_package_max_stack_layers() {
        let items = vec![ActualItem {
            id: "limited".into(),
            name: "Limited".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }];
        let mut attributes = HashMap::new();
        attributes.insert(
            "limited".to_string(),
            PackageAttribute {
                max_stack_layers: Some(2),
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "limited".into() },
                demand: 5.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(5.0),
                depth: meters(2.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let results = PileLayerGenerator::new().generate(&request);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 2);
        assert_eq!(results[0].layer.demand_coverage[0].coefficient, 2.0);
    }

    #[test]
    fn pattern_layer_generator_allows_remainder_pattern() {
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
                demand: 1.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(6.0),
                height: meters(1.0),
                depth: meters(2.0),
                capacity: meters(10.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);
        let pattern = vec![vec![
            PatternStep::new(PatternProjectionOrientation::Front, None),
            PatternStep::new(
                PatternProjectionOrientation::Front,
                Some(PatternNextPointPolicy::RightBottom),
            ),
        ]];
        let strict = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_patterns(pattern.clone()),
            max_candidates: 1,
            ..Default::default()
        })
        .generate(&request);
        let remainder = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_remainder(true).with_patterns(pattern),
            max_candidates: 1,
            ..Default::default()
        })
        .generate(&request);

        assert!(strict[0].diagnostics[0].contains("block-loading candidate"));
        assert_eq!(remainder[0].placement_traces.len(), 1);
        assert_eq!(remainder[0].layer.demand_coverage[0].coefficient, 1.0);
        assert!(remainder[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("with_remainder=true")));
    }

    #[test]
    fn pattern_layer_generator_repeats_pattern_until_remaining_is_empty() {
        let items = vec![ActualItem {
            id: "tall".into(),
            name: "Tall".to_string(),
            package_code: Some("mix".to_string()),
            pack: None,
            width: meters(2.0),
            height: meters(2.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }, ActualItem {
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
        }];
        let request = LayerGenerationRequest::new(0, items)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "tall".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "short".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
            ])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(2.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(2);
        let results = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(1).with_remainder(true).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 2,
            ..Default::default()
        })
        .generate(&request);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|result| result.placement_traces.len() == 1));
        assert_ne!(results[0].placement_traces[0].item_id, results[1].placement_traces[0].item_id);
    }

    #[test]
    fn deferred_layer_generators_rotate_items_and_use_shadow_prices() {
        let items = vec![
            ActualItem {
                id: "i0".into(),
                name: "Item0".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(1.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "i1".into(),
                name: "Item1".to_string(),
                package_code: None,
                pack: None,
                width: meters(3.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let mut shadow_prices = HashMap::new();
        shadow_prices.insert(
            DemandShadowPriceKey {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i1".into() },
            },
            2.5,
        );
        let request = LayerGenerationRequest::new(1, items)
            .with_bin(BinType {
                width: meters(5.0),
                height: meters(5.0),
                depth: meters(5.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_shadow_prices(shadow_prices)
            .with_max_candidates(2);
        let generator = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            coverage_coefficient: 0.5,
            max_candidates: 2,
            use_existing_layer_hint: false,
            ..Default::default()
        });

        let results = generator.generate(&request);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].layer.demand_coverage[0].coefficient, 0.5);
        assert_eq!(
            results[0].layer.demand_coverage[0].key,
            Bpp3dDemandKey::Item { id: "i1".into() },
        );
        assert_eq!(
            results[1].layer.demand_coverage[0].key,
            Bpp3dDemandKey::Item { id: "i0".into() },
        );
        assert!(results[0].numeric_score.unwrap() > results[1].numeric_score.unwrap());
        assert_eq!(results[0].score, Some(1.25));
        assert!(results[0].diagnostics[0].contains("block-loading candidate"));
    }

