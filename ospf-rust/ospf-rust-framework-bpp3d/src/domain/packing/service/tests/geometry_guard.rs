    #[test]
    fn packing_geometry_guard_valid_bin() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
                make_cuboid_packed_item(1, 5.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        };
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn packing_geometry_guard_overlap_detected() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
                make_cuboid_packed_item(1, 2.0, 0.0, 0.0, 5.0, 5.0, 5.0), // overlaps with item 0
            ],
        };
        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("overlap")));
    }

    #[test]
    fn packing_geometry_guard_out_of_bounds() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 8.0, 8.0, 8.0, 5.0, 5.0, 5.0), // exceeds 10.0
            ],
        };
        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("exceeds bin")));
    }

