    #[test]
    fn package_shape_spec_cuboid() {
        let spec = PackageShapeSpec::<f64, Meter>::Cuboid;
        assert!(matches!(spec, PackageShapeSpec::Cuboid));
    }

    #[test]
    fn package_shape_spec_cylinder_axis_aware() {
        let spec = PackageShapeSpec::Cylinder {
            axis: Axis3::Y,
            radius: meters(2.0),
            radius_candidates: None,
            radius_lower_bound: None,
            radius_upper_bound: None,
        };
        if let PackageShapeSpec::Cylinder { axis, radius, .. } = spec {
            assert_eq!(axis, Axis3::Y);
            assert_eq!(radius.value, 2.0);
        } else {
            panic!("Expected Cylinder spec");
        }
    }

    #[test]
    fn pattern_config_and_definition_match_kotlin_controls() {
        let disabled = PatternConfig::new().with_piling(1);
        assert!(!disabled.enables_two_sum());
        assert!(!disabled.enables_three_sum());

        let two_sum = PatternConfig::new().with_piling(2);
        assert!(two_sum.enables_two_sum());
        assert!(!two_sum.enables_three_sum());

        let unbounded = PatternConfig::new().with_remainder(true);
        assert!(unbounded.enables_two_sum());
        assert!(unbounded.enables_three_sum());
        assert!(unbounded.with_remainder);

        let definition = PatternDefinition::new(
            vec![vec![
                PatternStep::new(PatternProjectionOrientation::Front, None),
                PatternStep::new(
                    PatternProjectionOrientation::Side,
                    Some(PatternNextPointPolicy::RightBottom),
                ),
            ]],
            unbounded,
        );
        assert!(definition.is_valid());
        assert!(definition.diagnostics()[0].contains("two_sum=true"));
    }

    #[test]
    fn actual_item_cuboid_packing_shape() {
        let item = ActualItem {
            id: "item1".to_string(),
            name: "Test Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0), // 重量单位不同，此处简化
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        };

        let shape = item.packing_shape();
        assert_eq!(shape.shape_type, PackingShapeType::Cuboid);
    }

    #[test]
    fn actual_item_cylinder_packing_shape() {
        let item = ActualItem {
            id: "cyl1".to_string(),
            name: "Cylinder Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0), // diameter
            height: meters(5.0),
            depth: meters(4.0), // diameter
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        };

        let shape = item.packing_shape();
        assert_eq!(shape.shape_type, PackingShapeType::Cylinder);
        assert_eq!(shape.axis, Some(Axis3::Y));
        assert_eq!(shape.bounding_width.value, 4.0);
        assert_eq!(shape.bounding_height.value, 5.0);
        assert_eq!(shape.bounding_depth.value, 4.0);
    }

    #[test]
    fn actual_item_cylinder_packing_shape_uses_axis_length() {
        let x_axis = ActualItem {
            id: "cyl-x".to_string(),
            name: "Cylinder X".to_string(),
            package_code: None,
            pack: None,
            width: meters(8.0),
            height: meters(4.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::X,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        };
        let z_axis = ActualItem {
            id: "cyl-z".to_string(),
            name: "Cylinder Z".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(4.0),
            depth: meters(9.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Z,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        };

        let x_shape = x_axis.packing_shape();
        let z_shape = z_axis.packing_shape();

        assert_eq!(x_shape.axis, Some(Axis3::X));
        assert_eq!(x_shape.bounding_width.value, 8.0);
        assert_eq!(x_shape.bounding_height.value, 4.0);
        assert_eq!(x_shape.bounding_depth.value, 4.0);
        assert_eq!(z_shape.axis, Some(Axis3::Z));
        assert_eq!(z_shape.bounding_width.value, 4.0);
        assert_eq!(z_shape.bounding_height.value, 4.0);
        assert_eq!(z_shape.bounding_depth.value, 9.0);
    }

    #[test]
    fn actual_item_oriented_cylinder_packing_shape_updates_axis() {
        let item = ActualItem {
            id: "cyl-y".to_string(),
            name: "Cylinder Y".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(8.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Side, Orientation::Lie],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        };

        let side_shape = item.oriented_packing_shape(Orientation::Side);
        let lie_shape = item.oriented_packing_shape(Orientation::Lie);

        assert_eq!(side_shape.axis, Some(Axis3::X));
        assert_eq!(side_shape.bounding_width.value, 8.0);
        assert_eq!(side_shape.bounding_height.value, 4.0);
        assert_eq!(side_shape.bounding_depth.value, 4.0);
        assert_eq!(lie_shape.axis, Some(Axis3::Z));
        assert_eq!(lie_shape.bounding_width.value, 4.0);
        assert_eq!(lie_shape.bounding_height.value, 4.0);
        assert_eq!(lie_shape.bounding_depth.value, 8.0);
    }

