    #[test]
    fn simple_block_generator_cylinder_single_item() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cylinder_item("c1", 2.0, 5.0, Axis3::Y, 1.0)];
        let amounts = vec![10u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 圆柱只生成 1x1x1 块
        assert_eq!(blocks.len(), 1);
        if let Block::Simple(sb) = &blocks[0] {
            assert_eq!(sb.nx, 1);
            assert_eq!(sb.ny, 1);
            assert_eq!(sb.nz, 1);
            assert_eq!(sb.item_count, 1);
        }
    }

    #[test]
    fn simple_block_generator_cylinder_axis_bounding_box() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![
            make_cylinder_item("cx", 2.0, 8.0, Axis3::X, 1.0),
            make_cylinder_item("cz", 2.0, 9.0, Axis3::Z, 1.0),
        ];
        let amounts = vec![1u64, 1u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        let x_block = blocks.iter().find_map(|block| match block {
            Block::Simple(simple) if simple.item_view.item_index == 0 => Some(simple),
            Block::Simple(_) | Block::Complex(_) => None,
        }).unwrap();
        let z_block = blocks.iter().find_map(|block| match block {
            Block::Simple(simple) if simple.item_view.item_index == 1 => Some(simple),
            Block::Simple(_) | Block::Complex(_) => None,
        }).unwrap();
        assert_eq!(x_block.width.value, 8.0);
        assert_eq!(x_block.height.value, 4.0);
        assert_eq!(x_block.depth.value, 4.0);
        assert_eq!(z_block.width.value, 4.0);
        assert_eq!(z_block.height.value, 4.0);
        assert_eq!(z_block.depth.value, 9.0);
    }

    #[test]
    fn simple_block_generator_cylinder_rotation_updates_axis() {
        let generator = SimpleBlockGenerator::default_generator();
        let mut item = make_cylinder_item("cy", 2.0, 8.0, Axis3::Y, 1.0);
        item.enabled_orientations = vec![Orientation::Side, Orientation::Lie];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&[item], &[2], &container);

        assert!(blocks.iter().any(|block| match block {
            Block::Simple(simple) => {
                simple.item_view.orientation == Orientation::Side
                    && simple.item_view.packing_shape.axis == Some(Axis3::X)
                    && simple.width.value == 8.0
                    && simple.height.value == 4.0
                    && simple.depth.value == 4.0
            }
            Block::Complex(_) => false,
        }));
        assert!(blocks.iter().any(|block| match block {
            Block::Simple(simple) => {
                simple.item_view.orientation == Orientation::Lie
                    && simple.item_view.packing_shape.axis == Some(Axis3::Z)
                    && simple.width.value == 4.0
                    && simple.height.value == 4.0
                    && simple.depth.value == 8.0
            }
            Block::Complex(_) => false,
        }));
    }

