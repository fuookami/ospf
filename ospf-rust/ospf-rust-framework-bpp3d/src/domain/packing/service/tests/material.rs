    #[test]
    fn material_packer_summarizes() {
        let result: PackingResult<f64, Meter> = PackingResult {
            packed_bins: vec![],
            material_summaries: vec![MaterialSummary {
                material: MaterialKey {
                    no: "MAT001".to_string(),
                    material_type: crate::domain::item::MaterialType::RawMaterial,
                    manufacturer: None,
                    supplier: None,
                },
                amount: 10,
            }],
            info: HashMap::new(),
        };
        let packer = MaterialPacker::new();
        let summaries = packer.invoke(&result);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].amount, 10);
    }

    #[test]
    fn packer_summarize_materials_from_package() {
        // 测试从 Package.materials 提取物料汇总
        let material_key = MaterialKey {
            no: "MAT001".to_string(),
            material_type: MaterialType::RawMaterial,
            manufacturer: None,
            supplier: None,
        };

        let pack = Package {
            code: Some("PKG001".to_string()),
            shape: crate::domain::item::PackageShape {
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(4.0),
                weight: meters(1.0),
                spec: PackageShapeSpec::Cuboid,
            },
            packages: None,
            materials: vec![(material_key.clone(), 5)],
            amount: 1,
        };

        let item = PackedItem {
            item_index: 0,
            item: ActualItem {
                id: "item_0".into(),
                name: "Item 0".to_string(),
                package_code: Some("PKG001".to_string()),
                pack: Some(pack),
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: None,
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cuboid_packing_shape(meters(2.0), meters(3.0), meters(4.0), meters(1.0)),
            loading_order: 0,
        };

        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![item],
        }];

        let packer = Packer::new();
        let result = packer.invoke(packed_bins);

        assert_eq!(result.material_summaries.len(), 1);
        assert_eq!(result.material_summaries[0].material.no, "MAT001");
        assert_eq!(result.material_summaries[0].amount, 5);
    }

