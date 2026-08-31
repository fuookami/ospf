    #[test]
    fn packing_renderer_adapter_cylinder_item() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 验证圆柱渲染 DTO 包含正确的 actualVolume (πr²h)
        let item = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl".to_string(),
                name: "Cylinder".to_string(),
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

        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![item],
        }];

        let result = PackingResult {
            packed_bins,
            material_summaries: vec![],
            info: HashMap::new(),
        };

        let adapter = PackingRendererAdapter::new();
        let dtos = adapter.to_render_dto(&result);
        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].items.len(), 1);
        // actualVolume 应为 π*2²*5 = 20π ≈ 62.83
        let expected_volume = std::f64::consts::PI * 4.0 * 5.0;
        assert!((dtos[0].items[0].actual_volume - expected_volume).abs() < 1e-10);
        // 应包含圆柱相关字段
        assert_eq!(dtos[0].items[0].shape_type, crate::infrastructure::renderer::RenderShapeType::Cylinder);
        assert_eq!(dtos[0].items[0].algorithm_shape_type, crate::infrastructure::renderer::RenderAlgorithmShapeType::VerticalCylinder);
        assert!(dtos[0].items[0].radius.is_some());
        assert!((dtos[0].items[0].radius.unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn horizontal_cylinder_hanging_rejected() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 横向圆柱不在地面也没有长方体支撑 → 应被拒绝
        // Horizontal cylinder not on floor and no cuboid support → should be rejected
        let cylinder = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_x".to_string(),
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
            position: MetricPoint3 { x: meters(0.0), y: meters(5.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0)),
            loading_order: 0,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![cylinder],
        };

        // 悬空横向圆柱应被拒绝
        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("support coverage")));
    }
