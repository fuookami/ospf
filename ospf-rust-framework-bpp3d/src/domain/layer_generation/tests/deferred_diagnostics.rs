    #[test]
    fn deferred_layer_generators_report_diagnostics() {
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, vec![]);

        let pattern = PatternLayerGenerator::new().generate(&request);
        let pile = PileLayerGenerator::new().generate(&request);
        let historical = HistoricalLayerGenerator::new().generate(&request);

        assert_eq!(pattern[0].source, "pattern_layer_generator");
        assert!(pattern[0].diagnostics[0].contains("requires at least one item"));
        assert!(pattern[0].diagnostics[0].contains("item_count=0"));
        assert_eq!(pile[0].source, "pile_layer_generator");
        assert!(pile[0].diagnostics[0].contains("requires at least one item"));
        assert_eq!(historical[0].source, "historical_layer_generator");
        assert!(historical[0].diagnostics[0].contains("requires existing layers"));
    }

    #[test]
    fn deferred_layer_generators_produce_strategy_candidates() {
        let existing_layer = BinLayer {
            iteration: 0,
            from: "hint".to_string(),
            bin: None,
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };
        let mut request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(
            0,
            vec![ActualItem {
                id: "i0".to_string(),
                name: "Item".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(1.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            }],
        )
        .with_bin(BinType {
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            capacity: meters(100.0),
            type_code: "BIN".to_string(),
            is_main: true,
        })
        .with_max_candidates(2);
        request.existing_layers = vec![existing_layer];

        let config = DeferredLayerGeneratorConfig {
            coverage_coefficient: 0.75,
            max_candidates: 2,
            use_existing_layer_hint: true,
            ..Default::default()
        };
        let pattern = PatternLayerGenerator::with_config(config.clone()).generate(&request);
        let pile = PileLayerGenerator::new().generate(&request);
        let historical = HistoricalLayerGenerator::new().generate(&request);

        for (result, source) in [
            (&pattern, "pattern_layer_generator"),
            (&pile, "pile_layer_generator"),
            (&historical, "historical_layer_generator"),
        ] {
            assert!(!result.is_empty());
            assert_eq!(result[0].source, source);
        }
        assert!(pattern[0].diagnostics[0].contains("block-loading candidate"));
        assert!(pattern[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("pattern config")));
        assert!(pile[0].diagnostics[0].contains("candidate"));
        assert!(historical[0].diagnostics[0].contains("reused existing layer"));
        assert!(!pattern[0].block_traces.is_empty());
        assert_eq!(pattern[0].layer.demand_coverage[0].coefficient, 0.75);
        assert_eq!(historical[0].layer.depth, meters(2.0));
    }

