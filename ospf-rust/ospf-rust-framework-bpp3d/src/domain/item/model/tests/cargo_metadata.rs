    #[test]
    fn cargo_attribute_key_identity_and_equality() {
        let key_a = CargoAttributeKey::new("hazardous");
        let key_b = CargoAttributeKey::new("hazardous");
        let key_c = CargoAttributeKey::new("fragile");
        assert_eq!(key_a, key_b);
        assert_ne!(key_a, key_c);

        let key_with_tags = CargoAttributeKey::with_tags(
            "food-grade",
            vec!["cold-chain".to_string(), "organic".to_string()],
        );
        assert_eq!(key_with_tags.key, "food-grade");
        assert_eq!(key_with_tags.tags.len(), 2);
        assert_ne!(key_a, key_with_tags);
    }

    #[test]
    fn cargo_attribute_key_ordering() {
        let key_a = CargoAttributeKey::new("aaa");
        let key_b = CargoAttributeKey::new("bbb");
        assert!(key_a < key_b);
    }

    #[test]
    fn material_with_cargo_attribute() {
        let material = Material::<f64, Meter> {
            no: "MAT-CARGO".to_string(),
            material_type: MaterialType::RawMaterial,
            name: "Cargo Material".to_string(),
            manufacturer: Some("Acme".to_string()),
            supplier: Some("Global Supply".to_string()),
            warehouse: Some("WH-1".to_string()),
            weight: meters(1.0),
            cargo: Some(CargoAttributeKey::new("hazardous")),
        };
        let key = material.key();
        assert_eq!(key.no, "MAT-CARGO");
        assert_eq!(key.manufacturer.as_deref(), Some("Acme"));
        assert_eq!(key.supplier.as_deref(), Some("Global Supply"));
        assert!(material.cargo.is_some());
        assert_eq!(material.cargo.unwrap().key, "hazardous");
    }

    #[test]
    fn material_key_keeps_manufacturer_and_supplier_identity() {
        let from_a = Material::<f64, Meter> {
            no: "MAT-SAME".to_string(),
            material_type: MaterialType::RawMaterial,
            name: "Same Number From A".to_string(),
            manufacturer: Some("Maker-A".to_string()),
            supplier: Some("Supplier-A".to_string()),
            warehouse: None,
            weight: meters(1.0),
            cargo: None,
        }
        .key();
        let from_b = Material::<f64, Meter> {
            no: "MAT-SAME".to_string(),
            material_type: MaterialType::RawMaterial,
            name: "Same Number From B".to_string(),
            manufacturer: Some("Maker-B".to_string()),
            supplier: Some("Supplier-A".to_string()),
            warehouse: None,
            weight: meters(1.0),
            cargo: None,
        }
        .key();

        assert_ne!(from_a, from_b);
        assert_eq!(from_a.manufacturer.as_deref(), Some("Maker-A"));
        assert_eq!(from_b.manufacturer.as_deref(), Some("Maker-B"));
    }

    #[test]
    fn package_attribute_with_cargo_attribute() {
        let mut attr = PackageAttribute::default();
        assert!(attr.cargo_attribute.is_none());

        attr.cargo_attribute = Some(CargoAttributeKey::new("fragile"));
        assert!(attr.cargo_attribute.is_some());
        assert_eq!(attr.cargo_attribute.as_ref().unwrap().key, "fragile");

        let diagnostics = attr.diagnostics();
        assert!(!diagnostics.is_empty());
    }

