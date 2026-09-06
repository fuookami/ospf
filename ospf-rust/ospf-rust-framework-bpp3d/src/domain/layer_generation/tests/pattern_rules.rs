    #[test]
    fn package_rule_policy_filters_pattern_orientation_candidates() {
        #[derive(Debug)]
        struct ForbidUprightPolicy;

        impl LayerGenerationPackageRulePolicy<f64, Meter> for ForbidUprightPolicy {
            fn allows_orientation(
                &self,
                _item: &ActualItem<f64, Meter>,
                _attribute: Option<&PackageAttribute>,
                orientation: Orientation,
                _input: &PackageOrientationRuleInput,
            ) -> bool {
                orientation != Orientation::Upright
            }
        }

        let item = ActualItem {
            id: "i0".into(),
            name: "Item".to_string(),
            package_code: Some("p".to_string()),
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(3.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        };
        let request = LayerGenerationRequest::new(0, vec![item])
            .with_bin(BinType {
                width: meters(5.0),
                height: meters(5.0),
                depth: meters(5.0),
                capacity: meters(10.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i0".into() },
                demand: 1.0,
                satisfied: 0.0,
            }])
            .with_package_rule_policy(Arc::new(ForbidUprightPolicy));
        let results = pattern_step_layer_candidates(
            &request,
            "pattern_layer_generator",
            "test",
            &[(0, request.items[0].clone())],
            &DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(1).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
            },
        );

        assert!(results.is_empty());
    }

    #[test]
    fn pattern_layer_generator_reuses_item_in_three_sum_mixed_pile() {
        let items = vec![
            ActualItem {
                id: "thin".into(),
                name: "Thin".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "cap".into(),
                name: "Cap".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(3.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let request = LayerGenerationRequest::new(0, items)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "thin".into() },
                    demand: 2.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "cap".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
            ])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(4.0),
                depth: meters(2.0),
                capacity: meters(10.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);
        let generator = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(3).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
        });

        let results = generator.generate(&request);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 3);
        assert_eq!(results[0].placement_traces[0].item_id, "cap");
        assert_eq!(results[0].placement_traces[0].position.y, meters(0.0));
        assert_eq!(
            results[0]
                .placement_traces
                .iter()
                .filter(|trace| trace.item_id == "thin")
                .count(),
            2
        );
        assert!(results[0]
            .layer
            .demand_coverage
            .iter()
            .any(|coverage| {
                coverage.key == Bpp3dDemandKey::Item { id: "thin".into() }
                    && coverage.coefficient == 2.0
            }));
    }

    #[test]
    fn package_rule_policy_filters_pattern_placement_stacking() {
        #[derive(Debug)]
        struct ForbidStackingPolicy;

        impl LayerGenerationPackageRulePolicy<f64, Meter> for ForbidStackingPolicy {
            fn allows_placement_stacking(
                &self,
                units: &[LayerGenerationStackingUnit<f64>],
                top_index: usize,
                _input: &PackagePlacementStackingInput<'_>,
            ) -> bool {
                units.len() <= 1 || top_index == 0
            }
        }

        let units = vec![
            PatternSelectedItem {
                item_index: 0,
                item_id: "base".into(),
                orientation: Orientation::Upright,
                orientation_enabled: true,
                width: 2.0,
                depth: 2.0,
                height: 1.0,
                weight: 2.0,
            },
            PatternSelectedItem {
                item_index: 1,
                item_id: "top".into(),
                orientation: Orientation::Upright,
                orientation_enabled: true,
                width: 2.0,
                depth: 2.0,
                height: 1.0,
                weight: 1.0,
            },
        ];
        let attributes = HashMap::from([
            ("base".into(), PackageAttribute::default()),
            ("top".into(), PackageAttribute::default()),
        ]);
        let remaining = HashMap::from([(0usize, 1u64), (1usize, 1u64)]);
        let bin = BinType {
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(2.0),
            capacity: meters(10.0),
            type_code: "BIN".into(),
            is_main: true,
        };
        let default_policy = DefaultLayerGenerationPackageRulePolicy;
        let forbid_policy = ForbidStackingPolicy;

        assert!(build_mixed_pattern_pile(
            &units,
            &attributes,
            &default_policy,
            &remaining,
            &bin,
            &10.0,
        ).is_some());
        assert!(build_mixed_pattern_pile(
            &units,
            &attributes,
            &forbid_policy,
            &remaining,
            &bin,
            &10.0,
        ).is_none());
    }

    #[test]
    fn package_rule_policy_filters_block_layer_orientation_candidates() {
        #[derive(Debug)]
        struct ForbidUprightPolicy;

        impl LayerGenerationPackageRulePolicy<f64, Meter> for ForbidUprightPolicy {
            fn allows_orientation(
                &self,
                _item: &ActualItem<f64, Meter>,
                _attribute: Option<&PackageAttribute>,
                orientation: Orientation,
                _input: &PackageOrientationRuleInput,
            ) -> bool {
                orientation != Orientation::Upright
            }
        }

        let generator = BlockLayerGenerator::new();
        let request = LayerGenerationRequest::new(
            0,
            vec![ActualItem {
                id: "blocked".into(),
                name: "Blocked".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            }],
        )
        .with_bin(BinType {
            width: meters(4.0),
            height: meters(4.0),
            depth: meters(4.0),
            capacity: meters(100.0),
            type_code: "BIN".into(),
            is_main: true,
        })
        .with_demand_entries(vec![LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "blocked".into() },
            demand: 1.0,
            satisfied: 0.0,
        }])
        .with_package_rule_policy(Arc::new(ForbidUprightPolicy));

        assert!(generator.generate(&request).is_empty());
    }

    #[test]
    fn package_rule_policy_filters_bla_rotated_orientation_candidates() {
        #[derive(Debug)]
        struct ForbidRotatedPolicy;

        impl LayerGenerationPackageRulePolicy<f64, Meter> for ForbidRotatedPolicy {
            fn allows_orientation(
                &self,
                _item: &ActualItem<f64, Meter>,
                _attribute: Option<&PackageAttribute>,
                orientation: Orientation,
                _input: &PackageOrientationRuleInput,
            ) -> bool {
                orientation != Orientation::UprightRotated
            }
        }

        let request = LayerGenerationRequest::new(
            0,
            vec![ActualItem {
                id: "rot".into(),
                name: "Rotated".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(5.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright, Orientation::UprightRotated],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            }],
        )
        .with_bin(BinType {
            width: meters(5.0),
            height: meters(3.0),
            depth: meters(2.0),
            capacity: meters(100.0),
            type_code: "BIN".into(),
            is_main: true,
        })
        .with_package_rule_policy(Arc::new(ForbidRotatedPolicy));

        assert!(BLLocalLayerGenerator::new().generate(&request).is_empty());
    }

    #[test]
    fn package_rule_policy_filters_pile_placement_stacking() {
        #[derive(Debug)]
        struct ForbidStackingPolicy;

        impl LayerGenerationPackageRulePolicy<f64, Meter> for ForbidStackingPolicy {
            fn allows_placement_stacking(
                &self,
                _units: &[LayerGenerationStackingUnit<f64>],
                top_index: usize,
                _input: &PackagePlacementStackingInput<'_>,
            ) -> bool {
                top_index == 0
            }
        }

        let mut attributes = HashMap::new();
        attributes.insert("pile".to_string(), PackageAttribute::default());
        let request = LayerGenerationRequest::new(
            0,
            vec![ActualItem {
                id: "pile".into(),
                name: "Pile".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            }],
        )
        .with_package_attributes(attributes)
        .with_bin(BinType {
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(2.0),
            capacity: meters(100.0),
            type_code: "BIN".into(),
            is_main: true,
        })
        .with_demand_entries(vec![LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "pile".into() },
            demand: 3.0,
            satisfied: 0.0,
        }])
        .with_package_rule_policy(Arc::new(ForbidStackingPolicy));

        let results = PileLayerGenerator::new().generate(&request);

        assert_eq!(results.len(), 1);
        assert!(results[0].diagnostics[0].contains("found no stacking candidate"));
    }

    #[test]
    fn pattern_layer_generator_respects_package_orientation_rule() {
        let items = vec![ActualItem {
            id: "i0".into(),
            name: "Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Side, Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }];
        let mut attributes = HashMap::new();
        attributes.insert(
            "i0".to_string(),
            PackageAttribute {
                extra_orientation_rules: vec![PackageOrientationRule::ForbidCategory(
                    OrientationCategory::Side,
                )],
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_bin(BinType {
                width: meters(10.0),
                height: meters(10.0),
                depth: meters(10.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let result = pattern_step_layer_candidates(
            &request,
            "pattern_layer_generator",
            "test",
            &[(0, request.items[0].clone())],
            &DeferredLayerGeneratorConfig {
                pattern: PatternConfig::new().with_piling(1).with_patterns(vec![vec![
                    PatternStep::new(PatternProjectionOrientation::Front, None),
                ]]),
                max_candidates: 1,
                ..Default::default()
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].placement_traces[0].orientation, Orientation::Upright);
    }

    #[test]
    fn pattern_layer_generator_checks_same_item_pile_with_selected_orientation() {
        let items = vec![ActualItem {
            id: "side".into(),
            name: "Side item".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(1.0),
            depth: meters(3.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Side],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }];
        let mut attributes = HashMap::new();
        attributes.insert(
            "side".to_string(),
            PackageAttribute {
                extra_orientation_rules: vec![PackageOrientationRule::ForbidCategory(
                    OrientationCategory::Upright,
                )],
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "side".into() },
                demand: 2.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(10.0),
                height: meters(10.0),
                depth: meters(10.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let result = pattern_step_layer_candidates(
            &request,
            "pattern_layer_generator",
            "test",
            &[(0, request.items[0].clone())],
            &DeferredLayerGeneratorConfig {
                pattern: PatternConfig::new().with_piling(2).with_patterns(vec![vec![
                    PatternStep::new(PatternProjectionOrientation::Front, None),
                ]]),
                max_candidates: 1,
                ..Default::default()
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].placement_traces[0].orientation, Orientation::Side);
        assert_eq!(result[0].placement_traces[0].amount, 2);
    }

    #[test]
    fn pattern_layer_generator_same_item_pile_uses_bin_space_for_orientation_rules() {
        let items = vec![ActualItem {
            id: "wide_space".into(),
            name: "Wide-space item".to_string(),
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
            "wide_space".to_string(),
            PackageAttribute {
                extra_orientation_rules: vec![
                    PackageOrientationRule::RequireMinSpaceWidth(5.0),
                    PackageOrientationRule::RequireMinSpaceDepth(6.0),
                ],
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "wide_space".into() },
                demand: 2.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(5.0),
                height: meters(3.0),
                depth: meters(6.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let result = pattern_step_layer_candidates(
            &request,
            "pattern_layer_generator",
            "test",
            &[(0, request.items[0].clone())],
            &DeferredLayerGeneratorConfig {
                pattern: PatternConfig::new().with_piling(2).with_patterns(vec![vec![
                    PatternStep::new(PatternProjectionOrientation::Front, None),
                ]]),
                max_candidates: 1,
                ..Default::default()
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].placement_traces[0].orientation, Orientation::Upright);
        assert_eq!(result[0].placement_traces[0].amount, 2);
    }

    #[test]
    fn pattern_layer_generator_adds_side_on_top_orientation() {
        let items = vec![ActualItem {
            id: "top_side".into(),
            name: "Top-side item".to_string(),
            package_code: None,
            pack: None,
            width: meters(1.0),
            height: meters(3.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }];
        let mut attributes = HashMap::new();
        attributes.insert(
            "top_side".to_string(),
            PackageAttribute {
                side_on_top_layer: 1,
                extra_orientation_rules: vec![PackageOrientationRule::ForbidCategory(
                    OrientationCategory::Upright,
                )],
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "top_side".into() },
                demand: 2.0,
                satisfied: 0.0,
            }])
            .with_bin(BinType {
                width: meters(3.0),
                height: meters(2.0),
                depth: meters(2.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let result = pattern_step_layer_candidates(
            &request,
            "pattern_layer_generator",
            "test",
            &[(0, request.items[0].clone())],
            &DeferredLayerGeneratorConfig {
                pattern: PatternConfig::new().with_piling(2).with_patterns(vec![vec![
                    PatternStep::new(PatternProjectionOrientation::Side, None),
                ]]),
                max_candidates: 1,
                ..Default::default()
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].placement_traces.len(), 1);
        assert_eq!(result[0].placement_traces[0].orientation, Orientation::Side);
        assert_eq!(result[0].placement_traces[0].amount, 1);
    }

    #[test]
    fn pattern_layer_generator_respects_package_pair_stacking_rule() {
        let items = vec![
            ActualItem {
                id: "base".into(),
                name: "Base".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(3.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "wide".into(),
                name: "Wide".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(3.0),
                height: meters(1.0),
                depth: meters(3.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let mut attributes = HashMap::new();
        attributes.insert("base".to_string(), PackageAttribute::default());
        attributes.insert(
            "wide".to_string(),
            PackageAttribute {
                extra_pair_stacking_rules: vec![PackagePairStackingRule::RequireFootprintWithinBottom],
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "base".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "wide".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
            ])
            .with_bin(BinType {
                width: meters(3.0),
                height: meters(3.0),
                depth: meters(3.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let results = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(2).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
        })
        .generate(&request);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 1);
        assert_eq!(results[0].placement_traces[0].item_id, "base");
    }

    #[test]
    fn pattern_layer_generator_respects_package_placement_stacking_rule() {
        let items = vec![
            ActualItem {
                id: "pallet".into(),
                name: "Pallet".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(3.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "middle".into(),
                name: "Middle".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(2.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "top".into(),
                name: "Top".to_string(),
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
        let mut attributes = HashMap::new();
        attributes.insert(
            "pallet".to_string(),
            PackageAttribute {
                package_type: PackageType::Pallet,
                ..Default::default()
            },
        );
        attributes.insert("middle".to_string(), PackageAttribute::default());
        attributes.insert(
            "top".to_string(),
            PackageAttribute {
                extra_placement_stacking_rules: vec![
                    PackagePlacementStackingRule::ForbidIndirectPackageType(PackageType::Pallet),
                ],
                ..Default::default()
            },
        );
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "pallet".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "middle".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "top".into() },
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
            .with_max_candidates(1);

        let results = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(3).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
        })
        .generate(&request);

        assert_eq!(results.len(), 1);
        assert_ne!(results[0].placement_traces.len(), 3);
    }

    #[test]
    fn pattern_layer_generator_respects_hanging_policy_support_area() {
        let units = vec![
            PatternSelectedItem {
                item_index: 0,
                item_id: "bottom".into(),
                orientation: Orientation::Upright,
                orientation_enabled: true,
                width: 2.0,
                depth: 2.0,
                height: 1.0,
                weight: 10.0,
            },
            PatternSelectedItem {
                item_index: 1,
                item_id: "top".into(),
                orientation: Orientation::Upright,
                orientation_enabled: true,
                width: 4.0,
                depth: 4.0,
                height: 1.0,
                weight: 1.0,
            },
        ];
        let mut strict_attributes = HashMap::<ItemId, PackageAttribute>::new();
        strict_attributes.insert("bottom".into(), PackageAttribute::default());
        strict_attributes.insert(
            "top".into(),
            PackageAttribute {
                hanging_policy: HangingPolicy::Absolute {
                    max_difference: 0.0,
                    with_weight: false,
                },
                ..Default::default()
            },
        );
        let mut relaxed_attributes = HashMap::<ItemId, PackageAttribute>::new();
        relaxed_attributes.insert("bottom".into(), PackageAttribute::default());
        relaxed_attributes.insert(
            "top".into(),
            PackageAttribute {
                hanging_policy: HangingPolicy::Absolute {
                    max_difference: 100.0,
                    with_weight: false,
                },
                ..Default::default()
            },
        );
        let remaining = HashMap::from([(0usize, 1u64), (1usize, 1u64)]);
        let bin = BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(100.0),
            type_code: "BIN".into(),
            is_main: true,
        };
        let policy = DefaultLayerGenerationPackageRulePolicy;

        assert!(build_mixed_pattern_pile(&units, &strict_attributes, &policy, &remaining, &bin, &100.0).is_none());
        assert!(build_mixed_pattern_pile(&units, &relaxed_attributes, &policy, &remaining, &bin, &100.0).is_some());
    }

    #[test]
    fn pattern_layer_generator_respects_package_mixed_loading_rule() {
        let items = vec![
            ActualItem {
                id: "solo".into(),
                name: "Solo".to_string(),
                package_code: Some("mix".to_string()),
                pack: None,
                width: meters(2.0),
                height: meters(1.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "other".into(),
                name: "Other".to_string(),
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
        let mut attributes = HashMap::new();
        attributes.insert(
            "solo".to_string(),
            PackageAttribute {
                allow_mixed_loading: Some(false),
                ..Default::default()
            },
        );
        attributes.insert("other".to_string(), PackageAttribute::default());
        let request = LayerGenerationRequest::new(0, items)
            .with_package_attributes(attributes)
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "solo".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "other".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
            ])
            .with_bin(BinType {
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                capacity: meters(100.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_max_candidates(1);

        let results = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            pattern: PatternConfig::new().with_piling(2).with_patterns(vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
            ]]),
            max_candidates: 1,
            ..Default::default()
        })
        .generate(&request);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 1);
        assert_eq!(results[0].placement_traces[0].item_id, "solo");
    }

