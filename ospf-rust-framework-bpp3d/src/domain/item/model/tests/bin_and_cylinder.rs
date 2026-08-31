    #[test]
    fn material_key_extraction() {
        let material = Material {
            no: "MAT001".to_string(),
            material_type: MaterialType::RawMaterial,
            name: "Steel".to_string(),
            manufacturer: None,
            supplier: None,
            warehouse: Some("WH-A".to_string()),
            weight: meters(1.0),
            cargo: None,
        };
        let key = material.key();
        assert_eq!(key.no, "MAT001");
        assert_eq!(key.material_type, MaterialType::RawMaterial);
        assert_eq!(key.manufacturer.as_deref(), None);
        assert_eq!(key.supplier.as_deref(), None);
    }

    #[test]
    fn bin_type_construction() {
        let bin_type = BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN-10".into(),
            is_main: true,
        };
        assert!(bin_type.is_main);
        assert_eq!(bin_type.type_code, "BIN-10");
    }

    #[test]
    fn bin_layer_demand_coverage_lookup() {
        let key = Bpp3dDemandKey::Item { id: "i1".into() };
        let layer: BinLayer<f64, Meter> = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                key.clone(),
                2.0,
            )],
        };

        assert_eq!(
            layer.demand_coverage_coefficient(Bpp3dDemandMode::Item, &key),
            2.0
        );
    }

    #[test]
    fn cylinder_shape_contract_require_vertical_axis() {
        assert!(CylinderShapeContract::require_vertical_axis(Axis3::Y).is_ok());
        assert!(CylinderShapeContract::require_vertical_axis(Axis3::X).is_err());
    }

    #[test]
    fn cylinder_shape_contract_has_cylinder() {
        let items = vec![
            ActualItem {
                id: "cuboid".into(),
                name: "Cuboid".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: None,
            },
        ];
        assert!(!CylinderShapeContract::has_cylinder(&items));

        let items_with_cyl = vec![
            ActualItem {
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
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
        ];
        assert!(CylinderShapeContract::has_cylinder(&items_with_cyl));
    }

