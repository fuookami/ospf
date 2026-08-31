    #[test]
    fn layer_generation_demand_entry_remaining() {
        let entry = LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "item1".to_string() },
            demand: 10.0,
            satisfied: 7.0,
        };
        assert_eq!(entry.remaining(), 3.0);
    }

    #[test]
    fn layer_generation_demand_entry_remaining_zero() {
        let entry = LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "item1".to_string() },
            demand: 5.0,
            satisfied: 8.0,
        };
        assert_eq!(entry.remaining(), 0.0);
    }

    #[test]
    fn layer_generation_request_construction() {
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, vec![])
            .with_max_candidates(128);
        assert_eq!(request.iteration, 0);
        assert!(request.items.is_empty());
        assert_eq!(request.max_candidates, 128);
    }

    #[test]
    fn layer_generation_context_empty() {
        let ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        assert!(ctx.generators.is_empty());

        let request = LayerGenerationRequest::new(0, vec![]);
        let results = ctx.generate(&request);
        assert!(results.is_empty());
    }

    #[test]
    fn layer_generation_context_with_mock_generator() {
        /// 测试用 mock 生成器 / Mock generator for testing
        #[derive(Debug)]
        struct MockGenerator;

        impl LayerGenerator<f64, Meter> for MockGenerator {
            fn name(&self) -> &str { "mock" }
            fn generate(&self, _request: &LayerGenerationRequest<f64, Meter>) -> Vec<LayerGenerationResult<f64, Meter>> {
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: 0,
                        from: "mock".to_string(),
                        bin: None,
                        depth: meters(1.0),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: "mock".to_string(),
                }]
            }
        }

        let mut ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        ctx.add_generator(Box::new(MockGenerator));

        let request = LayerGenerationRequest::new(0, vec![]);
        let results = ctx.generate(&request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "mock");
    }

    #[test]
    fn layer_generation_context_deduplication() {
        /// 生成相同层的 mock / Mock that generates identical layers
        #[derive(Debug)]
        struct DupGenerator;

        impl LayerGenerator<f64, Meter> for DupGenerator {
            fn name(&self) -> &str { "dup" }
            fn generate(&self, _request: &LayerGenerationRequest<f64, Meter>) -> Vec<LayerGenerationResult<f64, Meter>> {
                let layer = BinLayer {
                    iteration: 0,
                    from: "dup".to_string(),
                    bin: None,
                    depth: meters(1.0),
                    demand_coverage: Vec::new(),
                };
                vec![
                    LayerGenerationResult {
                        layer: layer.clone(),
                        reduced_cost: None,
                        score: None,
                        numeric_score: None,
                        block_traces: Vec::new(),
                        placement_traces: Vec::new(),
                        diagnostics: Vec::new(),
                        source: "dup".to_string(),
                    },
                    LayerGenerationResult {
                        layer: layer,
                        reduced_cost: None,
                        score: None,
                        numeric_score: None,
                        block_traces: Vec::new(),
                        placement_traces: Vec::new(),
                        diagnostics: Vec::new(),
                        source: "dup".to_string(),
                    },
                ]
            }
        }

        let mut ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        ctx.add_generator(Box::new(DupGenerator));

        let request = LayerGenerationRequest::new(0, vec![]);
        let results = ctx.generate(&request);
        // 重复层应被去重
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn layer_generation_context_max_candidates() {
        #[derive(Debug)]
        struct ManyGenerator;

        impl LayerGenerator<f64, Meter> for ManyGenerator {
            fn name(&self) -> &str { "many" }
            fn generate(&self, _request: &LayerGenerationRequest<f64, Meter>) -> Vec<LayerGenerationResult<f64, Meter>> {
                (0..10).map(|i| LayerGenerationResult {
                    layer: BinLayer {
                        iteration: 0,
                        from: format!("many_{}", i),
                        bin: None,
                        depth: meters(i as f64),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: None,
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: "many".to_string(),
                }).collect()
            }
        }

        let mut ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        ctx.add_generator(Box::new(ManyGenerator));

        let request = LayerGenerationRequest::new(0, vec![]).with_max_candidates(3);
        let results = ctx.generate(&request);
        assert_eq!(results.len(), 3);
    }

    // ========================================================================
    // 阶段 6 验收测试 / Phase 6 acceptance tests
    // ========================================================================

