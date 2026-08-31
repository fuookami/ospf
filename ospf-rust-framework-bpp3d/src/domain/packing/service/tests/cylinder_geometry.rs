    #[test]
    fn cylinder_box_real_footprint_no_overlap() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 竖直圆柱 r=2 at (0,0,0)，长方体 at (3.5,0,3.5)
        // 圆心 XZ=(2,2)，矩形(3.5..5.5, 3.5..5.5)，最近点(3.5,3.5)
        // 距离=sqrt(2.25+2.25)≈3.18>r=2 → 不重叠
        let cylinder = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_y".to_string(),
                name: "Cylinder Y".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        let box_ = make_cuboid_packed_item(1, 3.5, 0.0, 3.5, 2.0, 5.0, 2.0);

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![cylinder, box_],
        };

        // 真实 footprint 不重叠
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn cylinder_cylinder_same_axis_overlap_detected() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 两个竖直圆柱，圆心距离小于半径之和 → overlap
        let cyl0 = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_0".to_string(),
                name: "Cyl 0".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        // 圆心 (3, 3) → 圆心距 ≈1.41 < r0+r1=4 → overlap
        let cyl1 = PackedItem {
            item_index: 1,
            item: crate::domain::item::ActualItem {
                id: "cyl_1".to_string(),
                name: "Cyl 1".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(1.0), y: meters(0.0), z: meters(1.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 1,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: BinType {
                width: meters(20.0),
                height: meters(20.0),
                depth: meters(20.0),
                capacity: meters(8000.0),
                type_code: "BIN-20".to_string(),
                is_main: true,
            },
            batch_no: None,
            items: vec![cyl0, cyl1],
        };

        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("overlap")));
    }

    #[test]
    fn cylinder_cylinder_same_axis_no_overlap() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 两个竖直圆柱，圆心距离大于半径之和 → no overlap
        let cyl0 = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_0".to_string(),
                name: "Cyl 0".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        // 圆心 (6, 6) → 圆心距≈5.66 > r0+r1=4 → no overlap
        let cyl1 = PackedItem {
            item_index: 1,
            item: crate::domain::item::ActualItem {
                id: "cyl_1".to_string(),
                name: "Cyl 1".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(4.0), y: meters(0.0), z: meters(4.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 1,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: BinType {
                width: meters(20.0),
                height: meters(20.0),
                depth: meters(20.0),
                capacity: meters(8000.0),
                type_code: "BIN-20".to_string(),
                is_main: true,
            },
            batch_no: None,
            items: vec![cyl0, cyl1],
        };

        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

