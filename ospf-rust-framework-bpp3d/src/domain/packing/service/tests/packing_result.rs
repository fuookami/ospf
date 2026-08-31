    #[test]
    fn packer_produces_result() {
        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        }];
        let packer = Packer::new();
        let result = packer.invoke(packed_bins);
        assert_eq!(result.packed_bins.len(), 1);
    }

    #[test]
    fn packed_item_actual_volume_cuboid() {
        let item = make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 2.0, 3.0, 4.0);
        let volume: f64 = item.actual_volume().value.into();
        assert!((volume - 24.0).abs() < 1e-10);
    }

    #[test]
    fn packed_bin_total_weight() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
                make_cuboid_packed_item(1, 5.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        };
        let weight: f64 = packed_bin.total_weight().value.into();
        assert!((weight - 2.0).abs() < 1e-10);
    }

    #[test]
    fn packed_bin_total_actual_volume() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 2.0, 3.0, 4.0),
            ],
        };
        let vol: f64 = packed_bin.total_actual_volume().value.into();
        assert!((vol - 24.0).abs() < 1e-10);
    }

    #[test]
    fn packing_renderer_adapter_actual_volume() {
        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 2.0, 3.0, 4.0),
            ],
        }];
        let result = PackingResult {
            packed_bins,
            material_summaries: vec![],
            info: HashMap::new(),
        };
        let adapter = PackingRendererAdapter::new();
        let dtos = adapter.to_render_dto(&result);
        assert_eq!(dtos.len(), 1);
        // actualVolume 应为 24.0，不是 bounding cuboid volume
        assert_eq!(dtos[0].items.len(), 1);
        assert!((dtos[0].items[0].actual_volume - 24.0).abs() < 1e-10);
    }

    #[test]
    fn package_solution_like_adapter() {
        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        }];
        let adapter = PackageSolutionLikeAdapter::from_packing_result(packed_bins, vec![]);
        assert_eq!(adapter.bin_count(), 1);
        assert_eq!(adapter.total_item_count(), 1);
    }

