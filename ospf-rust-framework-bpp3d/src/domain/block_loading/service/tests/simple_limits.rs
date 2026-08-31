    #[test]
    fn simple_block_generator_too_large() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cuboid_item("i1", 15.0, 15.0, 15.0, 1.0)];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 物品太大，放不下
        assert!(blocks.is_empty());
    }

    #[test]
    fn simple_block_generator_no_rotation() {
        let config = SimpleBlockGeneratorConfig {
            with_rotation: false,
            with_remainder: false,
            ..Default::default()
        };
        let generator = SimpleBlockGenerator::new(config);

        let items = vec![ActualItem {
            id: "i1".into(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright, Orientation::Side],
            shape_spec_override: None,
        }];
        let amounts = vec![10u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 两种朝向（Upright + Side）都应生成块
        assert!(!blocks.is_empty());

        // 验证存在不同朝向的块
        let upright_count = blocks.iter().filter(|b| {
            if let Block::Simple(sb) = b {
                sb.item_view.orientation == Orientation::Upright
            } else {
                false
            }
        }).count();
        let side_count = blocks.iter().filter(|b| {
            if let Block::Simple(sb) = b {
                sb.item_view.orientation == Orientation::Side
            } else {
                false
            }
        }).count();
        assert!(upright_count > 0);
        assert!(side_count > 0);
    }

    #[test]
    fn simple_block_generator_limits_vertical_stack_layers() {
        let generator = SimpleBlockGenerator::new(SimpleBlockGeneratorConfig {
            max_stack_layers: Some(1),
            ..Default::default()
        });
        let items = vec![ActualItem {
            id: "i1".into(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let container = MetricSize3 {
            width: meters(2.0),
            height: meters(4.0),
            depth: meters(2.0),
        };

        let blocks = generator.generate(&items, &[4], &container);

        assert!(!blocks.is_empty());
        assert!(blocks.iter().all(|block| match block {
            Block::Simple(simple) => simple.ny <= 1,
            Block::Complex(_) => true,
        }));
    }

    #[test]
    fn simple_block_generator_respects_package_depth_bounds() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![ActualItem {
            id: "i1".into(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let amounts = vec![4u64];
        let container = MetricSize3 {
            width: meters(2.0),
            height: meters(4.0),
            depth: meters(4.0),
        };
        let attribute = PackageAttribute {
            min_depth: 3.5,
            max_depth: Some(4.0),
            ..Default::default()
        };
        let attributes = vec![Some(&attribute)];

        let blocks = generator.generate_with_package_attributes(
            &items,
            &amounts,
            &container,
            &attributes,
        );

        assert!(!blocks.is_empty());
        assert!(blocks.iter().all(|block| match block {
            Block::Simple(simple) => simple.depth.value >= 3.5 && simple.depth.value <= 4.0,
            Block::Complex(_) => true,
        }));
    }

    #[test]
    fn simple_block_generator_adds_side_on_top_orientation() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![ActualItem {
            id: "i1".into(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(1.0),
            height: meters(3.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(3.0),
            height: meters(2.0),
            depth: meters(2.0),
        };
        let attribute = PackageAttribute {
            side_on_top_layer: 1,
            ..Default::default()
        };
        let attributes = vec![Some(&attribute)];

        let blocks = generator.generate_with_package_attributes(
            &items,
            &amounts,
            &container,
            &attributes,
        );

        assert!(blocks.iter().any(|block| match block {
            Block::Simple(simple) => simple.item_view.orientation == Orientation::Side,
            Block::Complex(_) => false,
        }));
        assert!(blocks.iter().all(|block| match block {
            Block::Simple(simple) if simple.item_view.orientation == Orientation::Side => {
                simple.ny <= 1
            }
            Block::Simple(_) | Block::Complex(_) => true,
        }));
    }

    #[test]
    fn simple_block_generator_orientation_rule_uses_container_space() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![ActualItem {
            id: "i1".into(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(1.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(5.0),
            height: meters(2.0),
            depth: meters(6.0),
        };
        let attribute = PackageAttribute {
            extra_orientation_rules: vec![
                PackageOrientationRule::RequireMinSpaceWidth(5.0),
                PackageOrientationRule::RequireMinSpaceDepth(6.0),
            ],
            ..Default::default()
        };
        let attributes = vec![Some(&attribute)];

        let blocks = generator.generate_with_package_attributes(
            &items,
            &amounts,
            &container,
            &attributes,
        );

        assert!(blocks.iter().any(|block| match block {
            Block::Simple(simple) => simple.item_view.orientation == Orientation::Upright,
            Block::Complex(_) => false,
        }));
    }

    #[test]
    fn simple_block_generator_respects_package_bounds_for_cylinder() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![ActualItem {
            id: "c1".into(),
            name: "Cylinder".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(2.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(crate::domain::item::PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        }];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(4.0),
            height: meters(4.0),
            depth: meters(4.0),
        };
        let attribute = PackageAttribute {
            min_depth: 5.0,
            max_height: Some(1.0),
            ..Default::default()
        };
        let attributes = vec![Some(&attribute)];

        let blocks = generator.generate_with_package_attributes(
            &items,
            &amounts,
            &container,
            &attributes,
        );

        assert!(blocks.is_empty());
    }

