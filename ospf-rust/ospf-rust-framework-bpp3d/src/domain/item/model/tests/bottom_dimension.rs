    #[test]
    fn bottom_dimension_range_unbounded() {
        let range = BottomDimensionRange::unbounded();
        assert!(range.is_unbounded());
        assert!(range.contains(0.0));
        assert!(range.contains(100.0));
        assert!(range.contains(-10.0));
    }

    #[test]
    fn bottom_dimension_range_bounded() {
        let range = BottomDimensionRange::between(10.0, 50.0);
        assert!(!range.is_unbounded());
        assert!(range.contains(10.0));
        assert!(range.contains(30.0));
        assert!(range.contains(50.0));
        assert!(!range.contains(9.9));
        assert!(!range.contains(50.1));
    }

    #[test]
    fn bottom_dimension_range_half_bounded() {
        let at_least = BottomDimensionRange::at_least(5.0);
        assert!(at_least.contains(5.0));
        assert!(at_least.contains(1000.0));
        assert!(!at_least.contains(4.9));

        let at_most = BottomDimensionRange::at_most(100.0);
        assert!(at_most.contains(0.0));
        assert!(at_most.contains(100.0));
        assert!(!at_most.contains(100.1));
    }

    #[test]
    fn pattern_config_accepts_bottom_dimensions() {
        let config = PatternConfig::default();
        // 默认无约束，所以任意尺寸都应通过 / Default is unbounded, so any dimensions should pass
        assert!(config.accepts_bottom_dimensions(10.0, 5.0));
        assert!(config.accepts_bottom_dimensions(0.0, 0.0));
        assert!(config.accepts_bottom_dimensions(1000.0, 500.0));
    }

    #[test]
    fn pattern_config_bottom_range_filtering() {
        let config = PatternConfig::default()
            .with_bottom_length_range(BottomDimensionRange::between(20.0, 100.0))
            .with_bottom_width_range(BottomDimensionRange::between(5.0, 40.0));

        // bottom_length = max(depth, width), bottom_width = min(depth, width)
        // depth=50, width=10: length=50, width=10 -> length in [20,100], width in [5,40] -> accept
        assert!(config.accepts_bottom_dimensions(50.0, 10.0));
        // depth=10, width=50: same normalization -> accept
        assert!(config.accepts_bottom_dimensions(10.0, 50.0));
        // depth=15, width=10: length=15, width=10 -> length < 20 -> reject
        assert!(!config.accepts_bottom_dimensions(15.0, 10.0));
        // depth=50, width=3: length=50, width=3 -> width < 5 -> reject
        assert!(!config.accepts_bottom_dimensions(50.0, 3.0));
        // depth=150, width=10: length=150, width=10 -> length > 100 -> reject
        assert!(!config.accepts_bottom_dimensions(150.0, 10.0));
    }

    #[test]
    fn pattern_config_bottom_range_diagnostics() {
        let config = PatternConfig::default()
            .with_bottom_length_range(BottomDimensionRange::between(10.0, 50.0))
            .with_bottom_width_range(BottomDimensionRange::at_least(5.0));
        let diag = PatternDefinition::new(
            config.effective_patterns(),
            config.clone(),
        ).diagnostics();
        assert!(!diag.is_empty());
        let msg = &diag[0];
        assert!(msg.contains("bottom_length"));
        assert!(msg.contains("bottom_width"));
    }
