    #[cfg(feature = "serde")]
    #[test]
    fn csv_materialized_solver_backed_flow_runs_one_generation_round() {
        #[derive(Debug)]
        struct CsvShadowPriceGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for CsvShadowPriceGenerator {
            fn name(&self) -> &str {
                "csv_shadow_generator"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                let key = DemandShadowPriceKey {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "i1".to_string() },
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
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: Some(-1.0),
                    score: Some(1.0),
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        #[derive(Debug, Clone)]
        struct CsvBackend;

        impl MetaModelSolverBackend for CsvBackend {
            fn name(&self) -> &str {
                "csv_fake"
            }

            fn solve_rmp(
                &self,
                _model: &MetaModel<f64>,
                diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                assert!(diagnostics.constraint_count >= diagnostics.demand_count);
                Ok(MetaModelExecutorSolveResult {
                    objective: Some(6.0 + diagnostics.layer_count as f64),
                    primal_solution: vec![1.0; diagnostics.variable_count],
                    dual_solution: vec![2.0; diagnostics.demand_count],
                    info: HashMap::from([("backend_phase".to_string(), "rmp".to_string())]),
                })
            }

            fn solve_final(
                &self,
                _model: &MetaModel<f64>,
                diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                assert!(diagnostics.constraint_count > diagnostics.demand_count);
                let mut primal_solution = vec![0.0; diagnostics.variable_count];
                if diagnostics.variable_count > 2 {
                    primal_solution[1] = 1.0;
                    primal_solution[2] = 1.0;
                }
                Ok(MetaModelExecutorSolveResult {
                    objective: Some(1.0),
                    primal_solution,
                    dual_solution: Vec::new(),
                    info: HashMap::from([("backend_phase".to_string(), "final".to_string())]),
                })
            }
        }

        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,cuboid,2,3,4,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,4
"#;
        let request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(CsvShadowPriceGenerator));
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig::default(),
            CsvBackend,
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig::default(),
            CsvBackend,
        );

        let result = service
            .run_csv_materialized_one_generation_round(
                request,
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "1");
        assert_eq!(result.rmp.shadow_price_summary["item:i1"], 2.0);
        assert_eq!(result.rmp.info["backend"], "csv_fake");
        assert_eq!(result.final_execution.info["backend_phase"], "final");
        assert_eq!(result.final_execution.layers[0].from, "csv_shadow_generator");
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.render_loading_plans[0].items.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
        assert!(result
            .result
            .layers
            .iter()
            .any(|layer| layer.from == "seed" || layer.from == "csv_shadow_generator"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn csv_one_generation_round_passes_package_attributes_to_layer_generation() {
        #[derive(Debug)]
        struct PackageAttributeProbeGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for PackageAttributeProbeGenerator {
            fn name(&self) -> &str {
                "package_attribute_probe"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                let Some(attribute) = request.package_attributes.get("i1") else {
                    return Vec::new();
                };
                if attribute.max_stack_layers != Some(2) {
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
                            1.0,
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

        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,max_stack_layers
i1,Item,cuboid,2,3,4,1,1,2
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,4
"#;
        let request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(PackageAttributeProbeGenerator));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "csv_package_attribute_probe_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "csv_package_attribute_probe_final".to_string(),
            primal_solution: Vec::new(),
            objective: Some(1.0),
        });

        let result = service
            .run_csv_materialized_one_generation_round(
                request,
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "1");
        assert!(result
            .result
            .layers
            .iter()
            .any(|layer| layer.from == "package_attribute_probe"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn csv_materialized_solver_backed_flow_reports_diagnostics() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,cuboid,2,3,4,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,4
"#;
        let mut request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        request.bins.clear();
        for layer in &mut request.initial_layers {
            layer.bin = None;
            layer.demand_coverage = vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "missing".to_string() },
                1.0,
            )];
        }
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "csv_diag_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "csv_diag_final".to_string(),
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_csv_materialized_with_bins(request, &rmp, &final_executor)
            .unwrap();

        assert!(result.result.render_loading_plans.is_empty());
        assert_eq!(result.result.info["final_final_diagnostics"], "no_selected_layer_assignment");
        assert!(result
            .result
            .info["final_packing_diagnostics"]
            .contains("no available bin type for final packing"));
    }

