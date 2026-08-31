    #[test]
    fn simple_block_generator_cuboid() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cuboid_item("i1", 2.0, 3.0, 4.0, 1.0)];
        let amounts = vec![10u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 应生成多个块（1x1x1, 2x1x1, 1x2x1, ...）
        assert!(!blocks.is_empty());

        // 验证所有块都是简单块
        for block in &blocks {
            if let Block::Simple(sb) = block {
                assert_eq!(sb.item_view.item_index, 0);
            }
        }
    }

    #[test]
    fn simple_block_generator_single_item() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cuboid_item("i1", 5.0, 5.0, 5.0, 1.0)];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 单物品、5x5x5 在 10x10x10 中，nx <= 2, ny <= 2, nz <= 2
        // 但 amount = 1，所以只能 nx*ny*nz <= 1
        // 只有 1x1x1 满足
        assert_eq!(blocks.len(), 1);
        if let Block::Simple(sb) = &blocks[0] {
            assert_eq!(sb.nx, 1);
            assert_eq!(sb.ny, 1);
            assert_eq!(sb.nz, 1);
        }
    }

