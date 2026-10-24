    // ============================================================================
    // 多箱型层生成 / Multi-bin layer generation
    // ============================================================================

    /// 构造指定编码与主箱标志的箱型 / Build a bin type with the given code and main flag
    fn bin_type_with(code: &str, is_main: bool) -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: code.into(),
            is_main,
        }
    }

    /// 记录每种箱型被请求次数的生成器 / Generator recording how often each bin type is requested
    #[derive(Debug)]
    struct RecordingBinGenerator;

    impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for RecordingBinGenerator {
        fn name(&self) -> &str {
            "recording"
        }

        fn generate(
            &self,
            request: &LayerGenerationRequest<f64, Meter>,
        ) -> Vec<LayerGenerationResult<f64, Meter>> {
            let Some(bin) = request.bin.clone() else {
                return Vec::new();
            };
            vec![LayerGenerationResult {
                layer: BinLayer {
                    iteration: request.iteration,
                    from: self.name().to_string(),
                    bin: Some(bin),
                    depth: meters(1.0),
                    demand_coverage: Vec::new(),
                },
                reduced_cost: None,
                score: None,
                numeric_score: None,
                block_traces: Vec::new(),
                placement_traces: Vec::new(),
                diagnostics: Vec::new(),
                source: self.name().to_string(),
            }]
        }
    }

    /// 给定箱型序列都必须参与生成 / Every given bin type must take part in generation
    #[test]
    fn multi_bin_request_generates_per_type_candidates() {
        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(RecordingBinGenerator));

        let request = LayerGenerationRequest::new(0, vec![item("i0")])
            .with_bins(vec![bin_type_with("20GP", true), bin_type_with("40HQ", false)])
            .with_max_candidates(16);
        let results = context.generate(&request);

        assert_eq!(results.len(), 2);
        let mut codes: Vec<String> = results
            .iter()
            .map(|result| {
                result
                    .layer
                    .bin
                    .as_ref()
                    .map(|bin| bin.type_code.to_string())
                    .unwrap_or_default()
            })
            .collect();
        codes.sort();
        assert_eq!(codes, vec!["20GP".to_string(), "40HQ".to_string()]);
    }

    /// 未指定任何箱型时保持单次直通 / No bin type specified keeps a single pass-through
    ///
    /// 这是多箱型改造最容易引入的回归：若按箱型展开后循环体一次都不执行，不依赖箱型的生成器
    /// （历史层生成器）会直接静默失效。
    /// This is the regression most easily introduced by the multi-bin change: if expansion runs the loop body
    /// zero times, generators that do not depend on a bin type (the historical layer generator) silently stop
    /// working.
    #[test]
    fn no_bin_specified_keeps_single_pass_through() {
        #[derive(Debug)]
        struct NoBinGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for NoBinGenerator {
            fn name(&self) -> &str {
                "no_bin"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name().to_string(),
                        bin: request.bin.clone(),
                        depth: meters(1.0),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: None,
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(NoBinGenerator));
        let request = LayerGenerationRequest::new(0, vec![item("i0")]).with_max_candidates(8);
        let results = context.generate(&request);

        assert_eq!(results.len(), 1);
        assert!(results[0].layer.bin.is_none());
    }

    /// 同一排布放在不同箱型上是不同列 / The same layout on different bin types is a different column
    #[test]
    fn layer_identity_includes_bin_type() {
        let small = BinLayer {
            iteration: 0,
            from: "pattern".to_string(),
            bin: Some(bin_type_with("20GP", true)),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let large = BinLayer {
            bin: Some(bin_type_with("40HQ", false)),
            ..small.clone()
        };

        assert_ne!(small.identity_key(), large.identity_key());

        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(RecordingBinGenerator));
        let request = LayerGenerationRequest::new(0, vec![item("i0")])
            .with_bins(vec![
                bin_type_with("20GP", true),
                bin_type_with("40HQ", false),
            ])
            .with_max_candidates(16);
        let results = context.generate(&request);
        assert_eq!(
            results.len(),
            2,
            "different bin types must not be deduplicated into one column"
        );
    }

    /// 层聚合去重同样按箱型区分 / Layer aggregation deduplication also distinguishes bin types
    #[test]
    fn layer_aggregation_deduplicates_by_identity() {
        let mut aggregation = LayerAggregation::new();
        let small = BinLayer {
            iteration: 0,
            from: "pattern".to_string(),
            bin: Some(bin_type_with("20GP", true)),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let large = BinLayer {
            bin: Some(bin_type_with("40HQ", false)),
            ..small.clone()
        };

        let added = aggregation.add_columns(vec![small.clone(), large.clone()]);
        assert_eq!(added.len(), 2);

        // 完全相同的层不应重复加入。
        // An identical layer must not be added twice.
        let added_again = aggregation.add_columns(vec![small.clone()]);
        assert!(added_again.is_empty());
    }

    /// 主箱型排在候选序列首位 / The main bin type leads the candidate sequence
    #[test]
    fn candidate_bins_put_the_main_type_first() {
        let bins = vec![
            bin_type_with("40HQ", false),
            bin_type_with("20GP", true),
            bin_type_with("40HQ", false),
        ];
        let ordered = layer_generation_candidate_bins(&bins);

        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].type_code.to_string(), "20GP");
        assert!(ordered[0].is_main);
        assert_eq!(ordered[1].type_code.to_string(), "40HQ");
    }

    /// 多箱型算法入口为每种箱型产出列 / The multi-bin algorithm entry produces columns per bin type
    #[test]
    fn generate_once_with_bins_collects_every_type() {
        let config = ColumnGenerationConfig::new().with_max_candidates_per_iteration(16);
        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(RecordingBinGenerator));
        let mut algorithm = ColumnGenerationAlgorithm::with_layer_generation(config, context);

        let results = algorithm.generate_once_with_bins(
            Some(bin_type_with("20GP", true)),
            vec![
                bin_type_with("20GP", true),
                bin_type_with("40HQ", false),
            ],
            vec![item("i0")],
            vec![],
        );

        assert_eq!(results.len(), 2);
        assert_eq!(algorithm.active_layers().len(), 2);
    }
