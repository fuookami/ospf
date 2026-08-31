    #[test]
    fn layer_placement_adapter_uses_oriented_cylinder_shape() {
        let adapter = LayerPlacementAdapter::new();
        let item = ActualItem {
            id: "cyl-y".into(),
            name: "Cylinder Y".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(8.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Side],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        };

        let packed_item = adapter.to_packed_item(KnownCoordinatePlacement {
            item_index: 0,
            item,
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Side,
        });

        assert_eq!(packed_item.packing_shape.axis, Some(Axis3::X));
        assert_eq!(packed_item.packing_shape.bounding_width.value, 8.0);
        assert_eq!(packed_item.packing_shape.bounding_height.value, 4.0);
        assert_eq!(packed_item.packing_shape.bounding_depth.value, 4.0);
    }

    #[test]
    fn horizontal_cylinder_on_floor_accepted() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        let item = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl".into(),
                name: "Cylinder".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::X,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0)),
            loading_order: 0,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![item],
        };

        // 贴地横向圆柱应通过验证
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn horizontal_cylinder_on_cuboid_support_accepted() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 横向圆柱放在长方体上方，长方体在径向轴上完全覆盖圆柱底部
        let cuboid = make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 10.0, 2.0, 10.0);

        // X 轴横向圆柱，放在 y=2.0（长方体顶部），径向轴 Z 上包围盒 0..10 被长方体覆盖
        let cylinder = PackedItem {
            item_index: 1,
            item: crate::domain::item::ActualItem {
                id: "cyl_x".into(),
                name: "Cylinder X".to_string(),
                package_code: None,
                pack: None,
                width: meters(5.0),
                height: meters(4.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::X,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(2.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0)),
            loading_order: 1,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![cuboid, cylinder],
        };

        // 方体完全支撑横向圆柱 → 应通过验证
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

