    #[test]
    fn application_service_runs_one_shadow_price_generation_round() {
        #[derive(Debug)]
        struct ShadowPriceGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for ShadowPriceGenerator {
            fn name(&self) -> &str {
                "shadow_generator"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                let key = DemandShadowPriceKey {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "i0".to_string() },
                };
                if !request.shadow_prices.contains_key(&key) {
                    return Vec::new();
                }
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name().to_string(),
                        bin: request.bin.clone(),
                        depth: meters(2.0),
                        demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                            Bpp3dDemandMode::Item,
                            Bpp3dDemandKey::Item { id: "i1".to_string() },
                            2.0,
                        )],
                    },
                    reduced_cost: Some(-1.0),
                    score: Some(1.0),
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: vec![LayerPlacementTrace {
                        item_index: 0,
                        item_id: "i1".to_string(),
                        position: MetricPoint3 {
                            x: meters(0.0),
                            y: meters(0.0),
                            z: meters(0.0),
                        },
                        orientation: Orientation::Upright,
                        amount: 1,
                    }],
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(ShadowPriceGenerator));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "shadow_round_rmp".to_string(),
            shadow_prices: vec![3.0],
            primal_solution: vec![1.0, 1.0],
            objective: Some(2.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "shadow_round_final".to_string(),
            primal_solution: Vec::new(),
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "1");
        assert_eq!(result.result.layers.len(), 2);
        assert!(result.result.layers.iter().any(|layer| layer.from == "shadow_generator"));
        assert_eq!(result.rmp.shadow_price_summary["item:i0"], 3.0);
    }

    #[test]
    fn application_service_replays_generated_layer_traces_to_render() {
        #[derive(Debug)]
        struct TraceGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for TraceGenerator {
            fn name(&self) -> &str {
                "trace_generator"
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
                        demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                            Bpp3dDemandMode::Item,
                            Bpp3dDemandKey::Item { id: "i0".to_string() },
                            2.0,
                        )],
                    },
                    reduced_cost: Some(-1.0),
                    score: Some(2.0),
                    numeric_score: Some(2.0),
                    block_traces: Vec::new(),
                    placement_traces: vec![LayerPlacementTrace {
                        item_index: 0,
                        item_id: "i0".to_string(),
                        position: MetricPoint3 {
                            x: meters(0.0),
                            y: meters(0.0),
                            z: meters(0.0),
                        },
                        orientation: Orientation::Upright,
                        amount: 2,
                    }],
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(TraceGenerator));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "trace_round_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "trace_round_final".to_string(),
            primal_solution: vec![0.0, 1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.final_execution.packed_bins.len(), 1);
        assert_eq!(result.final_execution.packed_bins[0].items.len(), 2);
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.render_loading_plans[0].items.len(), 2);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn application_service_replays_generated_block_traces_to_render() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(BlockLayerGenerator::new()));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "block_round_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "block_round_final".to_string(),
            primal_solution: vec![0.0, 1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.final_execution.layers[0].from, "block_layer_generator");
        assert_eq!(result.final_execution.packed_bins.len(), 1);
        assert_eq!(result.final_execution.packed_bins[0].items.len(), 1);
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn application_service_reports_empty_generation_round() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer_generation = LayerGenerationContext::new();
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "empty_generation_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "empty_generation_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "0");
        assert_eq!(result.final_execution.layers.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn application_service_reports_empty_dataset_diagnostics() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig::default());
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig::default());

        let result = service
            .run_materialized_with_bins(Vec::new(), Vec::new(), Vec::new(), &rmp, &final_executor)
            .unwrap();

        assert!(result.result.layers.is_empty());
        assert_eq!(result.result.info["rmp_demand_count"], "0");
        assert_eq!(result.result.info["final_final_diagnostics"], "no_selected_layer_assignment");
    }

