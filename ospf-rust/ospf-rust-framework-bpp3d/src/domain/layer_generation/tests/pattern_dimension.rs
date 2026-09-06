    #[test]
    fn pattern_layer_generator_filters_by_bottom_dimension_range() {
        // 两个货物：大底面和小底面（depth >= width 以满足 Front pattern step）
        // Two items: large-bottom and small-bottom (depth >= width for Front pattern step)
        let items = vec![
            ActualItem {
                id: "large".into(),
                name: "Large".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(2.0),
                depth: meters(8.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "small".into(),
                name: "Small".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(3.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let request = LayerGenerationRequest::new(0, items.clone())
            .with_bin(BinType {
                width: meters(20.0),
                height: meters(10.0),
                depth: meters(20.0),
                capacity: meters(1000.0),
                type_code: "BIN".into(),
                is_main: true,
            })
            .with_demand_entries(vec![
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "large".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
                LayerGenerationDemandEntry {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "small".into() },
                    demand: 1.0,
                    satisfied: 0.0,
                },
            ])
            .with_max_candidates(5);

        // 无底面范围限制，应该产生候选 / Without bottom range filter, candidates should be produced
        let result_no_filter = pattern_step_layer_candidates(
            &request,
            "generator",
            "group",
            &[(0, items[0].clone()), (1, items[1].clone())],
            &DeferredLayerGeneratorConfig {
                pattern: PatternConfig::new().with_piling(1).with_patterns(vec![vec![
                    PatternStep::new(PatternProjectionOrientation::Front, None),
                ]]),
                max_candidates: 5,
                ..Default::default()
            },
        );
        assert!(!result_no_filter.is_empty());
        let no_filter_ids: std::collections::HashSet<String> = result_no_filter
            .iter()
            .flat_map(|r| r.placement_traces.iter().map(|t| t.item_id.to_string()))
            .collect();
        assert!(no_filter_ids.contains("large"));
        assert!(no_filter_ids.contains("small"));

        // 设置底面长度范围 [5, 20]（较大底面维度），只有 large 应通过
        // Set bottom length range [5, 20] (larger bottom dimension), only large should pass
        // large: max(depth=8, width=4) = 8 -> in [5,20] -> accepted
        // small: max(depth=3, width=2) = 3 -> not in [5,20] -> rejected
        use crate::domain::item::BottomDimensionRange;
        let result_filtered = pattern_step_layer_candidates(
            &request,
            "generator",
            "group",
            &[(0, items[0].clone()), (1, items[1].clone())],
            &DeferredLayerGeneratorConfig {
                pattern: PatternConfig::new()
                    .with_piling(1)
                    .with_patterns(vec![vec![
                        PatternStep::new(PatternProjectionOrientation::Front, None),
                    ]])
                    .with_bottom_length_range(BottomDimensionRange::between(5.0, 20.0)),
                max_candidates: 5,
                ..Default::default()
            },
        );
        assert!(!result_filtered.is_empty());
        assert!(result_filtered
            .iter()
            .all(|r| r.placement_traces.iter().all(|t| t.item_id == "large")));
    }
