    #[test]
    fn patterned_item_and_package_attribute_report_diagnostics() {
        let item = ActualItem {
            id: "i0".into(),
            name: "Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(1.0),
            height: meters(1.0),
            depth: meters(1.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        };
        let patterned = PatternedItem::new("p0", item);
        let attribute = PackageAttribute {
            allow_mixed_loading: Some(false),
            max_stack_layers: Some(2),
            tags: vec!["fragile".to_string()],
            ..Default::default()
        };

        assert_eq!(patterned.key.pattern_code, "p0");
        assert!(patterned.diagnostics[0].contains("conservative item-level demand coverage"));
        assert_eq!(
            patterned.demand_coverage(2.0).coefficient,
            2.0
        );
        assert!(attribute.diagnostics()[0].contains("max_stack_layers"));
        assert!(attribute
            .validate()
            .iter()
            .any(|diagnostic| diagnostic.contains("disallows mixed loading")));
        assert!(attribute
            .packing_diagnostics(3)
            .iter()
            .any(|diagnostic| diagnostic.contains("exceeded")));
    }

    #[test]
    fn patterned_item_from_actual_items_applies_deformation_and_average_weight() {
        let material_a = MaterialKey {
            no: "MAT-001".to_string(),
            material_type: MaterialType::RawMaterial,
            manufacturer: None,
            supplier: None,
        };
        let material_b = MaterialKey {
            no: "MAT-002".to_string(),
            material_type: MaterialType::FinishedProduct,
            manufacturer: None,
            supplier: None,
        };
        let small_pack = Package {
            code: Some("PKG-S".to_string()),
            shape: PackageShape {
                width: meters(1.0),
                height: meters(1.0),
                depth: meters(1.0),
                weight: meters(0.0),
                spec: PackageShapeSpec::Cuboid,
            },
            packages: None,
            materials: vec![(material_a.clone(), 2), (material_b.clone(), 1)],
            amount: 1,
        };
        let large_pack = Package {
            code: Some("PKG-L".to_string()),
            shape: PackageShape {
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(0.0),
                spec: PackageShapeSpec::Cuboid,
            },
            packages: None,
            materials: vec![(material_a.clone(), 3)],
            amount: 1,
        };
        let small = ActualItem {
            id: "small".into(),
            name: "Small".to_string(),
            package_code: None,
            pack: Some(small_pack),
            width: meters(1.0),
            height: meters(1.0),
            depth: meters(1.0),
            weight: meters(2.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        };
        let large = ActualItem {
            id: "large".into(),
            name: "Large".to_string(),
            package_code: None,
            pack: Some(large_pack),
            width: meters(2.0),
            height: meters(2.0),
            depth: meters(2.0),
            weight: meters(5.0),
            enabled_orientations: vec![Orientation::UprightRotated],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        };
        let pattern_shape = PackageShape {
            width: meters(3.0),
            height: meters(4.0),
            depth: meters(5.0),
            weight: meters(0.0),
            spec: PackageShapeSpec::Cuboid,
        };
        let attribute = PackageAttribute {
            deformation_attribute: DeformationAttribute::Linear {
                coefficient: 0.1,
            },
            ..Default::default()
        };

        let (patterned, amount) = PatternedItem::from_actual_items(
            "p0",
            &pattern_shape,
            &attribute,
            &[(small.clone(), 2), (large.clone(), 1)],
        )
        .unwrap();

        assert_eq!(amount, 3);
        assert_eq!(patterned.amount, 3);
        assert_eq!(patterned.amount_range, IntervalValue::new(3, 3));
        assert!((patterned.item.width.value - 3.3333333333333335).abs() < 1e-10);
        assert!((patterned.item.height.value - 4.333333333333333).abs() < 1e-10);
        assert!((patterned.item.depth.value - 5.333333333333333).abs() < 1e-10);
        assert_eq!(patterned.item.weight.value, 3.0);
        assert_eq!(patterned.item.enabled_orientations, vec![Orientation::Upright]);
        assert_eq!(
            patterned.material_amounts,
            vec![(material_a.clone(), 7), (material_b.clone(), 2)]
        );
        let pack = patterned.item.pack.as_ref().unwrap();
        assert_eq!(pack.amount, 3);
        assert_eq!(pack.materials, patterned.material_amounts);
        let material_catalog = vec![
            Material {
                no: "MAT-001".to_string(),
                material_type: MaterialType::RawMaterial,
                name: "Steel".to_string(),
                manufacturer: None,
                supplier: None,
                warehouse: None,
                weight: meters(1.5),
                cargo: None,
            },
            Material {
                no: "MAT-002".to_string(),
                material_type: MaterialType::FinishedProduct,
                name: "Finished".to_string(),
                manufacturer: None,
                supplier: None,
                warehouse: None,
                weight: meters(2.0),
                cargo: None,
            },
        ];
        let weights = patterned.item.material_weights(&material_catalog);
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0].0.no, "MAT-001");
        assert_eq!(weights[0].1.value, 10.5);
        assert_eq!(weights[1].0.no, "MAT-002");
        assert_eq!(weights[1].1.value, 4.0);
        assert_eq!(patterned.actual_items.len(), 2);
        assert_eq!(patterned.actual_item_at(0).unwrap().id, "small");
        assert_eq!(patterned.actual_item_at(2).unwrap().id, "large");
        assert!(patterned
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("aggregates 3 actual items")));

        let (_, _, ranged_amount) = PatternedItem::from_actual_items_with_ranges(
            "p1",
            &pattern_shape,
            &attribute,
            &[
                (small, 2, IntervalValue::new(1, 3)),
                (large, 1, IntervalValue::new(1, 2)),
            ],
        )
        .unwrap();
        assert_eq!(ranged_amount, IntervalValue::new(2, 5));

        let material_catalog = vec![
            Material {
                no: "MAT-001".to_string(),
                material_type: MaterialType::RawMaterial,
                name: "Steel".to_string(),
                manufacturer: None,
                supplier: None,
                warehouse: None,
                weight: meters(1.5),
                cargo: None,
            },
            Material {
                no: "MAT-002".to_string(),
                material_type: MaterialType::FinishedProduct,
                name: "Finished".to_string(),
                manufacturer: None,
                supplier: None,
                warehouse: None,
                weight: meters(2.0),
                cargo: None,
            },
        ];
        let (_, _, _) = PatternedItem::from_actual_items_with_ranges_and_material_catalog(
            "p2",
            &pattern_shape,
            &attribute,
            &[
                (
                    ActualItem {
                        id: "small".into(),
                        name: "Small".to_string(),
                        package_code: None,
                        pack: Some(Package {
                            code: Some("PKG-S".to_string()),
                            shape: PackageShape {
                                width: meters(1.0),
                                height: meters(1.0),
                                depth: meters(1.0),
                                weight: meters(0.0),
                                spec: PackageShapeSpec::Cuboid,
                            },
                            packages: None,
                            materials: vec![(material_a.clone(), 2), (material_b.clone(), 1)],
                            amount: 1,
                        }),
                        width: meters(1.0),
                        height: meters(1.0),
                        depth: meters(1.0),
                        weight: meters(2.0),
                        enabled_orientations: vec![Orientation::Upright],
                        shape_spec_override: Some(PackageShapeSpec::Cuboid),
                    },
                    2,
                    IntervalValue::new(1, 3),
                ),
            ],
            &material_catalog,
        )
        .unwrap();

        let ranged_item = PatternedItem::from_actual_items_with_ranges_and_material_catalog(
            "p3",
            &pattern_shape,
            &attribute,
            &[
                (
                    ActualItem {
                        id: "small".into(),
                        name: "Small".to_string(),
                        package_code: None,
                        pack: Some(Package {
                            code: Some("PKG-S".to_string()),
                            shape: PackageShape {
                                width: meters(1.0),
                                height: meters(1.0),
                                depth: meters(1.0),
                                weight: meters(0.0),
                                spec: PackageShapeSpec::Cuboid,
                            },
                            packages: None,
                            materials: vec![(material_a.clone(), 2), (material_b.clone(), 1)],
                            amount: 1,
                        }),
                        width: meters(1.0),
                        height: meters(1.0),
                        depth: meters(1.0),
                        weight: meters(2.0),
                        enabled_orientations: vec![Orientation::Upright],
                        shape_spec_override: Some(PackageShapeSpec::Cuboid),
                    },
                    2,
                    IntervalValue::new(1, 3),
                ),
                (
                    ActualItem {
                        id: "large".into(),
                        name: "Large".to_string(),
                        package_code: None,
                        pack: Some(Package {
                            code: Some("PKG-L".to_string()),
                            shape: PackageShape {
                                width: meters(2.0),
                                height: meters(2.0),
                                depth: meters(2.0),
                                weight: meters(0.0),
                                spec: PackageShapeSpec::Cuboid,
                            },
                            packages: None,
                            materials: vec![(material_a.clone(), 3)],
                            amount: 1,
                        }),
                        width: meters(2.0),
                        height: meters(2.0),
                        depth: meters(2.0),
                        weight: meters(5.0),
                        enabled_orientations: vec![Orientation::UprightRotated],
                        shape_spec_override: Some(PackageShapeSpec::Cuboid),
                    },
                    1,
                    IntervalValue::new(1, 2),
                ),
            ],
            &material_catalog,
        )
        .unwrap();
        assert_eq!(
            ranged_item.0.material_weights,
            vec![
                (material_a.clone(), meters(10.5)),
                (material_b.clone(), meters(4.0)),
            ]
        );

        let oriented_item = PatternedItem::from_actual_items_with_ranges_catalog_and_orientations(
            "p4",
            &pattern_shape,
            &attribute,
            &[(
                ActualItem {
                    id: "upright".into(),
                    name: "Upright".to_string(),
                    package_code: None,
                    pack: None,
                    width: meters(1.0),
                    height: meters(1.0),
                    depth: meters(1.0),
                    weight: meters(1.0),
                    enabled_orientations: vec![Orientation::Upright],
                    shape_spec_override: Some(PackageShapeSpec::Cuboid),
                },
                1,
                IntervalValue::new(1, 1),
            )],
            &material_catalog,
            &[Orientation::Side, Orientation::Upright],
        )
        .unwrap();
        assert_eq!(
            oriented_item.0.item.enabled_orientations,
            vec![Orientation::Upright, Orientation::Side],
        );
    }

