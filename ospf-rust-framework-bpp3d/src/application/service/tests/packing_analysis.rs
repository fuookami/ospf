    #[test]
    fn depth_boundary_policy_only_checks_final_stage() {
        let policy = DepthBoundaryLayerOrientationPolicy::new();

        assert!(policy.validate(
            DepthBoundaryValidationStage::Generation,
            Orientation::UprightRotated,
            true,
        ).is_ok());
        assert!(policy.validate(
            DepthBoundaryValidationStage::FinalKnownCoordinate,
            Orientation::UprightRotated,
            true,
        ).is_err());
        assert!(policy.validate(
            DepthBoundaryValidationStage::FinalKnownCoordinate,
            Orientation::Upright,
            true,
        ).is_ok());
    }

    #[test]
    fn layer_placement_adapter_builds_valid_packed_bin() {
        let adapter = LayerPlacementAdapter::new();
        let placement = KnownCoordinatePlacement {
            item_index: 0,
            item: item("i0"),
            position: MetricPoint3 {
                x: meters(0.0),
                y: meters(0.0),
                z: meters(0.0),
            },
            orientation: Orientation::Upright,
        };

        let packed_bin = adapter
            .to_packed_bin("bin-1".to_string(), bin_type(), None, vec![placement])
            .unwrap();

        assert_eq!(packed_bin.items.len(), 1);
        assert_eq!(packed_bin.items[0].item.id, "i0");
    }

    #[test]
    fn packing_analyzer_outputs_render_plan() {
        let adapter = LayerPlacementAdapter::new();
        let placement = KnownCoordinatePlacement {
            item_index: 0,
            item: item("i0"),
            position: MetricPoint3 {
                x: meters(0.0),
                y: meters(0.0),
                z: meters(0.0),
            },
            orientation: Orientation::Upright,
        };
        let packed_bin = adapter
            .to_packed_bin("bin-1".to_string(), bin_type(), None, vec![placement])
            .unwrap();

        let analyzer = ColumnGenerationPackingAnalyzer::new();
        let analysis = analyzer.analyze(vec![packed_bin]).unwrap();

        assert_eq!(analysis.packing_result.packed_bins.len(), 1);
        assert_eq!(analysis.render_loading_plans.len(), 1);
        assert_eq!(analysis.render_loading_plans[0].items.len(), 1);
    }

