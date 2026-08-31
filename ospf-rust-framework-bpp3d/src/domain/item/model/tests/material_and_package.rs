    #[test]
    fn packing_program_material_values_support_amount_and_weight_semantics() {
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
        let shape = PackageShape {
            width: meters(1.0),
            height: meters(1.0),
            depth: meters(1.0),
            weight: meters(0.0),
            spec: PackageShapeSpec::Cuboid,
        };
        let catalog = vec![
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
        let amount_program = PackingProgram {
            shape: shape.clone(),
            materials: vec![(material_a.clone(), 3)],
            material_values: Vec::new(),
        };
        assert_eq!(amount_program.material_amount(&material_a), 3);
        assert_eq!(
            amount_program.material_weights(&catalog),
            vec![(material_a.clone(), meters(4.5))]
        );

        let value_program = PackingProgram {
            shape,
            materials: Vec::new(),
            material_values: vec![
                (material_a.clone(), PackingProgramMaterialValue::amount(2)),
                (material_b.clone(), PackingProgramMaterialValue::weight(meters(7.0))),
            ],
        };

        assert_eq!(value_program.material_amounts(), vec![(material_a.clone(), 2)]);
        assert_eq!(
            value_program.material_weights(&catalog),
            vec![
                (material_a, meters(3.0)),
                (material_b, meters(7.0)),
            ]
        );
        assert!(PackingProgramMaterialValue::<f64, Meter>::new(None, None).is_none());
    }

