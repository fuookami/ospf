    #[test]
    fn package_type_category_and_layer_limit_match_kotlin_semantics() {
        assert_eq!(PackageType::WoodenContainer.category(), PackageCategory::HardBox);
        assert_eq!(PackageType::Pallet.category(), PackageCategory::Pallet);
        assert_eq!(PackageType::CartonContainer.category(), PackageCategory::SoftBox);
        assert_eq!(PackageType::PackingFoam.category(), PackageCategory::Filler);

        let attribute = PackageAttribute {
            package_max_layer: Some(5),
            max_stack_layers: Some(4),
            weight_attribute: WeightAttribute {
                max_layer: Some(3),
            },
            ..Default::default()
        };
        assert_eq!(attribute.max_layer(), Some(3));
        assert!(attribute.packing_diagnostics(4).iter().any(|diagnostic| {
            diagnostic.contains("max_layer exceeded")
        }));
    }

    #[test]
    fn package_attribute_hanging_policy_checks_weight_and_area() {
        let absolute = PackageAttribute {
            hanging_policy: HangingPolicy::Absolute {
                max_difference: 1.0,
                with_weight: true,
            },
            ..Default::default()
        };
        assert!(absolute.enabled_stacking_on_support(5.0, 100.0, 10.0, 92.0, 5.0));
        assert!(!absolute.enabled_stacking_on_support(5.0, 100.0, 10.0, 80.0, 5.0));
        assert!(!absolute.enabled_stacking_on_support(5.0, 100.0, 10.0, 92.0, 4.0));

        let relative = PackageAttribute {
            hanging_policy: HangingPolicy::Relative {
                hanging_percentage: 0.25,
                with_weight: false,
            },
            ..Default::default()
        };
        assert!(relative.enabled_stacking_on_support(5.0, 100.0, 10.0, 80.0, 0.0));
        assert!(!relative.enabled_stacking_on_support(5.0, 100.0, 10.0, 70.0, 0.0));
    }

    #[test]
    fn package_attribute_stacking_policy_checks_compatibility_and_limits() {
        let item = PackageAttribute {
            package_type: PackageType::CartonContainer,
            package_max_layer: Some(2),
            max_height: Some(5.0),
            stacking_on_policy: StackingOnPolicy::Box {
                max_difference: 1.0,
                max_over_weight: 2.0,
            },
            ..Default::default()
        };
        let bottom = PackageAttribute {
            package_type: PackageType::WoodenContainer,
            over_package_types: vec![PackageType::CartonContainer],
            ..Default::default()
        };

        let input = stacking_input(&item, &bottom);
        assert!(item.enabled_stacking_on(&input));

        let mut too_wide = input.clone();
        too_wide.item_width = 12.0;
        assert!(!item.enabled_stacking_on(&too_wide));

        let mut too_heavy = input.clone();
        too_heavy.item_weight = 11.0;
        assert!(!item.enabled_stacking_on(&too_heavy));

        let mut too_high_layer = input.clone();
        too_high_layer.layer = 2;
        assert!(!item.enabled_stacking_on(&too_high_layer));

        let incompatible_bottom = PackageAttribute {
            over_package_types: vec![PackageType::PackingFoam],
            ..bottom
        };
        let incompatible_input = stacking_input(&item, &incompatible_bottom);
        assert!(!item.enabled_stacking_on(&incompatible_input));
    }

    #[test]
    fn package_attribute_orientation_and_bottom_only_match_kotlin_semantics() {
        let item = PackageAttribute {
            bottom_only: true,
            side_on_top_layer: 1,
            ..Default::default()
        };
        let bottom = PackageAttribute {
            bottom_only: false,
            ..Default::default()
        };
        let input = stacking_input(&item, &bottom);
        assert!(!item.enabled_stacking_on(&input));

        let bottom = PackageAttribute {
            bottom_only: true,
            ..Default::default()
        };
        let mut side_input = stacking_input(&item, &bottom);
        side_input.item_orientation = Orientation::Side;
        side_input.item_orientation_enabled = false;
        side_input.layer = 0;
        assert!(item.enabled_stacking_on(&side_input));
        side_input.layer = 1;
        assert!(!item.enabled_stacking_on(&side_input));

        let non_flat_bottom = PackageAttribute {
            bottom_only: true,
            top_flat: false,
            ..Default::default()
        };
        let non_flat_input = stacking_input(&item, &non_flat_bottom);
        assert!(!item.enabled_stacking_on(&non_flat_input));

        let filler = PackageAttribute {
            package_type: PackageType::PackingFoam,
            ..item
        };
        let filler_input = stacking_input(&filler, &non_flat_bottom);
        assert!(filler.enabled_stacking_on(&filler_input));
    }

    #[test]
    fn package_attribute_pair_stacking_honors_side_and_lie_layers() {
        let item = PackageAttribute {
            side_on_top_layer: 2,
            lie_on_top_layer: 3,
            ..Default::default()
        };
        let bottom = PackageAttribute {
            top_flat: true,
            ..Default::default()
        };
        let mut input = stacking_input(&item, &bottom);
        input.item_orientation = Orientation::Side;
        input.item_orientation_enabled = false;
        input.layer = 1;
        assert!(item.enabled_stacking_on(&input));
        input.layer = 2;
        assert!(!item.enabled_stacking_on(&input));

        input.item_orientation = Orientation::Lie;
        input.layer = 2;
        assert!(item.enabled_stacking_on(&input));
        input.layer = 3;
        assert!(!item.enabled_stacking_on(&input));
    }

    #[test]
    fn package_attribute_max_layer_depends_on_orientation_top_flatness() {
        let attribute = PackageAttribute {
            package_max_layer: Some(5),
            top_flat: false,
            side_on_top_layer: 2,
            lie_on_top_layer: 3,
            ..Default::default()
        };

        assert_eq!(
            attribute.max_layer_for_orientation(Orientation::Upright, true),
            Some(1),
        );
        assert_eq!(
            attribute.max_layer_for_orientation(Orientation::Side, false),
            Some(2),
        );
        assert_eq!(
            attribute.max_layer_for_orientation(Orientation::Lie, false),
            Some(3),
        );
        assert_eq!(
            attribute.max_layer_for_orientation(Orientation::Side, true),
            Some(5),
        );
    }

    #[test]
    fn package_attribute_bottom_top_flat_depends_on_orientation() {
        let item = PackageAttribute {
            package_type: PackageType::CartonContainer,
            ..Default::default()
        };
        let bottom = PackageAttribute {
            package_type: PackageType::WoodenContainer,
            top_flat: false,
            over_package_types: vec![PackageType::CartonContainer],
            ..Default::default()
        };
        let mut input = stacking_input(&item, &bottom);
        input.bottom_orientation = Orientation::Side;
        input.bottom_orientation_enabled = true;

        assert!(item.enabled_stacking_on(&input));

        input.bottom_orientation_enabled = false;
        assert!(!item.enabled_stacking_on(&input));
    }

    #[test]
    fn package_attribute_extra_orientation_rule_filters_rotated_orientations() {
        let attribute = PackageAttribute {
            extra_orientation_rules: vec![
                PackageOrientationRule::ForbidCategory(OrientationCategory::Side),
                PackageOrientationRule::ForbidRotated,
            ],
            ..Default::default()
        };
        let upright = PackageOrientationRuleInput {
            orientation: Orientation::Upright,
            space_width: 10.0,
            space_height: 10.0,
            space_depth: 10.0,
        };
        let side = PackageOrientationRuleInput {
            orientation: Orientation::Side,
            ..upright
        };
        let rotated = PackageOrientationRuleInput {
            orientation: Orientation::UprightRotated,
            ..upright
        };

        assert!(attribute.enabled_orientation_by_rule(&upright));
        assert!(!attribute.enabled_orientation_by_rule(&side));
        assert!(!attribute.enabled_orientation_by_rule(&rotated));
    }

    #[test]
    fn actual_item_enabled_orientations_apply_package_orientation_rules() {
        let item = ActualItem {
            id: "i0".to_string(),
            name: "Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(3.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        };
        let bin = BinType {
            width: meters(4.0),
            height: meters(3.0),
            depth: meters(4.0),
            capacity: meters(10.0),
            type_code: "b0".to_string(),
            is_main: true,
        };
        let attribute = PackageAttribute {
            side_on_top_layer: 1,
            extra_orientation_rules: vec![PackageOrientationRule::ForbidCategory(
                OrientationCategory::Side,
            )],
            ..Default::default()
        };

        let orientations = item.enabled_orientations_at_bin(Some(&attribute), &bin);

        assert_eq!(orientations, vec![Orientation::Upright]);
    }

    #[test]
    fn package_attribute_extra_orientation_rule_respects_min_space_height() {
        let attribute = PackageAttribute {
            extra_orientation_rules: vec![PackageOrientationRule::RequireMinSpaceHeight(5.0)],
            ..Default::default()
        };
        let enough_space = PackageOrientationRuleInput {
            orientation: Orientation::Upright,
            space_width: 10.0,
            space_height: 5.0,
            space_depth: 10.0,
        };
        let not_enough_space = PackageOrientationRuleInput {
            space_height: 4.9,
            ..enough_space
        };

        assert!(attribute.enabled_orientation_by_rule(&enough_space));
        assert!(!attribute.enabled_orientation_by_rule(&not_enough_space));
    }

    #[test]
    fn package_attribute_extra_orientation_rule_respects_space_width_depth() {
        let attribute = PackageAttribute {
            extra_orientation_rules: vec![
                PackageOrientationRule::RequireMinSpaceWidth(5.0),
                PackageOrientationRule::RequireMinSpaceDepth(6.0),
            ],
            ..Default::default()
        };
        let enough_space = PackageOrientationRuleInput {
            orientation: Orientation::Upright,
            space_width: 5.0,
            space_height: 10.0,
            space_depth: 6.0,
        };
        let narrow_space = PackageOrientationRuleInput {
            space_width: 4.9,
            ..enough_space
        };
        let shallow_space = PackageOrientationRuleInput {
            space_depth: 5.9,
            ..enough_space
        };

        assert!(attribute.enabled_orientation_by_rule(&enough_space));
        assert!(!attribute.enabled_orientation_by_rule(&narrow_space));
        assert!(!attribute.enabled_orientation_by_rule(&shallow_space));
    }

    #[test]
    fn package_attribute_extra_pair_stacking_rule_filters_pairs() {
        let item = PackageAttribute {
            package_type: PackageType::CartonContainer,
            extra_pair_stacking_rules: vec![
                PackagePairStackingRule::ForbidSamePackageType,
                PackagePairStackingRule::RequireNotHeavierThanBottom,
                PackagePairStackingRule::RequireFootprintWithinBottom,
            ],
            ..Default::default()
        };
        let bottom = PackageAttribute {
            package_type: PackageType::WoodenContainer,
            over_package_types: vec![PackageType::CartonContainer],
            ..Default::default()
        };
        let mut input = stacking_input(&item, &bottom);
        assert!(item.enabled_stacking_on(&input));

        input.item_weight = 11.0;
        assert!(!item.enabled_stacking_on(&input));

        input.item_weight = 8.0;
        input.item_width = 11.0;
        assert!(!item.enabled_stacking_on(&input));
    }

    #[test]
    fn package_attribute_extra_placement_stacking_rule_filters_bottoms() {
        let item = PackageAttribute {
            bottom_only: true,
            ..Default::default()
        };
        let bottom = PackageAttribute {
            bottom_only: true,
            top_flat: true,
            ..Default::default()
        };
        let direct_bottom = PackageAttribute {
            bottom_only: false,
            top_flat: false,
            ..Default::default()
        };
        let input = PackagePlacementStackingInput::from_attributes(
            &item,
            vec![&bottom],
            vec![&bottom],
        );
        assert!(item.enabled_placement_stacking(&input));

        let blocked = PackagePlacementStackingInput::from_attributes(
            &item,
            vec![&direct_bottom],
            vec![&bottom],
        );
        assert!(!item.enabled_placement_stacking(&blocked));

        let filler = PackageAttribute {
            package_type: PackageType::PackingFoam,
            ..Default::default()
        };
        let filler_on_non_flat = PackagePlacementStackingInput::from_attributes(
            &filler,
            vec![&direct_bottom],
            vec![&bottom],
        );
        assert!(filler.enabled_placement_stacking(&filler_on_non_flat));

        let item = PackageAttribute {
            extra_placement_stacking_rules: vec![PackagePlacementStackingRule::ForbidIndirectPackageType(
                PackageType::WoodenContainer,
            )],
            ..Default::default()
        };
        let wooden = PackageAttribute {
            package_type: PackageType::WoodenContainer,
            bottom_only: true,
            top_flat: true,
            ..Default::default()
        };
        let blocked_by_extra = PackagePlacementStackingInput::from_attributes(
            &item,
            vec![&bottom],
            vec![&wooden],
        );
        assert!(!item.enabled_placement_stacking(&blocked_by_extra));
    }

    #[test]
    fn package_attribute_placement_stacking_uses_orientation_context() {
        let item = PackageAttribute {
            side_on_top_layer: 2,
            ..Default::default()
        };
        let non_flat_bottom = PackageAttribute {
            top_flat: false,
            ..Default::default()
        };
        let mut input = PackagePlacementStackingInput {
            item: &item,
            item_orientation: Orientation::Side,
            item_orientation_enabled_at_space: true,
            item_orientation_enabled: false,
            direct_bottom_items: vec![&non_flat_bottom],
            direct_bottom_contexts: vec![PackagePlacementBottomContext {
                item: &non_flat_bottom,
                orientation: Orientation::Side,
                orientation_enabled: false,
            }],
            indirect_bottom_items: Vec::new(),
            indirect_bottom_contexts: Vec::new(),
            layer: 1,
        };

        assert!(item.enabled_placement_stacking(&input));

        input.layer = 2;
        assert!(!item.enabled_placement_stacking(&input));

        input.layer = 1;
        input.direct_bottom_contexts[0].orientation_enabled = true;
        assert!(item.enabled_placement_stacking(&input));

        input.direct_bottom_contexts[0].orientation = Orientation::Upright;
        assert!(!item.enabled_placement_stacking(&input));
    }

