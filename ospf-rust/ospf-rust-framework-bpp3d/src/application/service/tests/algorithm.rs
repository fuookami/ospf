    #[test]
    fn column_generation_algorithm_adds_mock_layer() {
        #[derive(Debug)]
        struct MockGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for MockGenerator {
            fn name(&self) -> &str {
                "mock"
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
                    reduced_cost: Some(-1.0),
                    score: None,
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let config = ColumnGenerationConfig::new()
            .with_max_candidates_per_iteration(8);
        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(MockGenerator));
        let mut algorithm = ColumnGenerationAlgorithm::with_layer_generation(config, context);

        let results = algorithm.generate_once(Some(bin_type()), vec![item("i0")], vec![]);

        assert_eq!(results.len(), 1);
        assert_eq!(algorithm.layer_aggregation.layers.len(), 1);
        assert_eq!(algorithm.active_column_count(), 1);
        assert_eq!(algorithm.active_layers().len(), 1);
        assert_eq!(algorithm.state.total_columns, 1);
        assert_eq!(algorithm.state.iteration, 1);

        let result = algorithm.result();
        assert_eq!(
            result.info["framework_lifecycle_active_column_count"],
            "1"
        );
        assert_eq!(
            result.info["framework_lifecycle_removed_column_count"],
            "0"
        );
    }

    #[test]
    fn column_generation_algorithm_filters_removed_columns_from_state() {
        let first = BinLayer {
            iteration: 0,
            from: "seed-a".to_string(),
            bin: Some(bin_type()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let second = BinLayer {
            iteration: 0,
            from: "seed-b".to_string(),
            bin: Some(bin_type()),
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };
        let mut algorithm =
            ColumnGenerationAlgorithm::<f64, Meter>::new(ColumnGenerationConfig::default());

        algorithm.add_initial_layers(vec![first, second]);
        algorithm.remove_columns([0]);

        let active_layers = algorithm.active_layers();
        assert_eq!(active_layers.len(), 1);
        assert_eq!(active_layers[0].from, "seed-b");
        assert_eq!(algorithm.active_column_count(), 1);
        assert_eq!(algorithm.removed_column_count(), 1);

        let state = application_state_from_algorithm(
            &algorithm,
            vec![item("i0")],
            Vec::new(),
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert_eq!(state.layers.len(), 1);
        assert_eq!(state.layers[0].from, "seed-b");
    }

    #[test]
    fn application_service_runs_mock_executor_flow() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let rmp = MockColumnGenerationRmpExecutor {
            objective: Some(1.0),
        };
        let final_executor = MockColumnGenerationFinalExecutor;

        let result = service
            .run_materialized(vec![item("i0")], vec![layer], &rmp, &final_executor)
            .unwrap();

        assert_eq!(result.result.layers.len(), 1);
        assert_eq!(result.rmp.objective, Some(1.0));
        assert_eq!(result.final_execution.layers.len(), 1);
        assert_eq!(result.result.info["rmp_executor"], "mock_rmp");
        assert_eq!(result.result.info["final_executor"], "mock_final");
    }

