    #[test]
    fn meta_model_rmp_executor_registers_context() {
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0"), item("i1")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "test_rmp".to_string(),
            shadow_prices: vec![1.5, 2.5],
            primal_solution: vec![1.0],
            objective: Some(9.0),
        });

        let execution = executor.execute(&state);

        let diagnostics = execution.diagnostics.unwrap();
        assert_eq!(diagnostics.model_name, "test_rmp");
        assert_eq!(diagnostics.variable_count, 1);
        assert_eq!(diagnostics.demand_count, 2);
        assert_eq!(execution.shadow_price_summary["item:i0"], 1.5);
        assert_eq!(execution.shadow_price_summary["item:i1"], 2.5);
        assert_eq!(execution.info["status"], "registered");
    }

    #[test]
    fn meta_model_final_executor_registers_context_and_extracts_layers() {
        let bin = bin_type();
        let layer_a = BinLayer {
            iteration: 0,
            from: "seed-a".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer_b = BinLayer {
            iteration: 0,
            from: "seed-b".to_string(),
            bin: Some(bin.clone()),
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer_a.clone(), layer_b.clone()],
            layers: vec![layer_a, layer_b],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "test_final".to_string(),
            primal_solution: vec![0.0, 1.0, 1.0],
            objective: Some(5.0),
        });

        let execution = executor.execute(&state);

        let diagnostics = execution.diagnostics.unwrap();
        assert_eq!(diagnostics.model_name, "test_final");
        assert_eq!(diagnostics.variable_count, 3);
        assert_eq!(diagnostics.constraint_count, 6);
        assert_eq!(diagnostics.bin_count, 1);
        assert_eq!(execution.layers.len(), 1);
        assert_eq!(execution.layers[0].from, "seed-b");
        assert_eq!(execution.info["status"], "registered");
        assert_eq!(execution.info["constraint_count"], "6");
        assert_eq!(execution.info["framework_lifecycle_solution_len"], "3");
        assert_eq!(execution.info["framework_lifecycle_flush_count"], "0");
    }

    #[test]
    fn final_layer_capacity_coefficients_follow_item_coverage() {
        let mut heavy = item("heavy");
        heavy.weight = meters(6.0);
        let mut light = item("light");
        light.weight = meters(2.0);
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "coverage".to_string(),
            bin: Some(bin.clone()),
            depth: meters(3.0),
            demand_coverage: vec![
                Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::Item,
                    Bpp3dDemandKey::Item { id: "heavy".to_string() },
                    2.0,
                ),
                Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::ItemAmount,
                    Bpp3dDemandKey::Item { id: "light".to_string() },
                    3.0,
                ),
            ],
        };

        let weights = final_layer_weights(&[layer.clone()], &[heavy, light]);
        let volumes = final_layer_volumes(&[layer]);

        assert_eq!(weights, vec![18.0]);
        assert_eq!(volumes, vec![300.0]);
    }

    #[test]
    fn application_service_runs_meta_model_executor_flow() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "service_rmp".to_string(),
            shadow_prices: vec![4.0],
            primal_solution: vec![1.0],
            objective: Some(2.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "service_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_with_bins(
                vec![item("i0")],
                vec![bin],
                vec![layer],
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.rmp.info["executor"], "meta_model_rmp");
        assert_eq!(result.final_execution.info["executor"], "meta_model_final");
        assert_eq!(result.result.layers.len(), 1);
        assert_eq!(result.result.info["rmp_model_name"], "service_rmp");
        assert_eq!(result.result.info["final_model_name"], "service_final");
    }

    #[test]
    fn application_service_renders_selected_coverage_layer() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "i0".to_string() },
                1.0,
            )],
        };
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "render_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "render_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_with_bins(
                vec![item("i0")],
                vec![bin],
                vec![layer],
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.final_execution.packed_bins.len(), 1);
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn meta_model_final_executor_replays_layer_placement_traces() {
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "generated".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "i0".to_string() },
                2.0,
            )],
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::from([(
                0,
                vec![LayerPlacementTrace {
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
            )]),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "trace_replay_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let execution = executor.execute(&state);

        assert_eq!(execution.packed_bins.len(), 1);
        assert_eq!(execution.packed_bins[0].items.len(), 2);
        assert_eq!(execution.packed_bins[0].items[1].position.x.value, 2.0);
        assert_eq!(execution.info["final_diagnostics"], "selected_layers_renderable");
        assert_eq!(execution.info["packed_bin_count"], "1");
    }

    #[test]
    fn meta_model_final_executor_replays_layer_block_traces() {
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "generated".to_string(),
            bin: Some(bin.clone()),
            depth: meters(4.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "i0".to_string() },
                4.0,
            )],
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::from([(
                0,
                vec![LayerBlockTrace {
                    block_index: 0,
                    item_index: 0,
                    item_id: "i0".to_string(),
                    orientation: Orientation::Upright,
                    nx: 2,
                    ny: 2,
                    nz: 1,
                    item_count: 4,
                    size: MetricSize3 {
                        width: meters(4.0),
                        height: meters(6.0),
                        depth: meters(4.0),
                    },
                    origin: MetricPoint3 {
                        x: meters(0.0),
                        y: meters(0.0),
                        z: meters(0.0),
                    },
                }],
            )]),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "block_trace_replay_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let execution = executor.execute(&state);

        assert_eq!(execution.packed_bins.len(), 1);
        assert_eq!(execution.packed_bins[0].items.len(), 4);
        assert_eq!(execution.packed_bins[0].items[1].position.x.value, 2.0);
        assert_eq!(execution.packed_bins[0].items[2].position.y.value, 3.0);
        assert_eq!(execution.info["final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn solver_backed_meta_model_executors_report_solve_failures() {
        #[derive(Debug, Clone)]
        struct FailingBackend;

        impl MetaModelSolverBackend for FailingBackend {
            fn name(&self) -> &str {
                "failing"
            }

            fn solve_rmp(
                &self,
                _model: &MetaModel<f64>,
                _diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                Err("rmp unavailable".to_string())
            }

            fn solve_final(
                &self,
                _model: &MetaModel<f64>,
                _diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                Err("final infeasible".to_string())
            }
        }

        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig::default(),
            FailingBackend,
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig::default(),
            FailingBackend,
        );

        let rmp_execution = rmp.execute(&state);
        let final_execution = final_executor.execute(&state);

        assert_eq!(rmp_execution.info["status"], "solve_failed");
        assert_eq!(rmp_execution.info["backend"], "failing");
        assert_eq!(rmp_execution.info["error"], "rmp unavailable");
        assert!(rmp_execution.diagnostics.is_some());
        assert!(rmp_execution.shadow_price_summary.is_empty());
        assert_eq!(final_execution.info["status"], "solve_failed");
        assert_eq!(final_execution.info["backend"], "failing");
        assert_eq!(final_execution.info["error"], "final infeasible");
        assert!(final_execution.diagnostics.is_some());
        assert!(final_execution.layers.is_empty());
    }

    #[test]
    fn meta_model_final_executor_reports_no_selected_assignment() {
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "no_selection_final".to_string(),
            primal_solution: vec![0.0, 0.0],
            objective: Some(0.0),
        });

        let execution = executor.execute(&state);

        assert!(execution.layers.is_empty());
        assert!(execution.packed_bins.is_empty());
        assert_eq!(execution.info["final_diagnostics"], "no_selected_layer_assignment");
        assert_eq!(execution.info["packed_bin_count"], "0");
    }

    #[test]
    fn meta_model_final_executor_reports_packing_diagnostics() {
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "missing".to_string() },
                1.0,
            )],
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "packing_diagnostics_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let execution = executor.execute(&state);

        assert_eq!(execution.layers.len(), 1);
        assert!(execution.packed_bins.is_empty());
        assert_eq!(execution.info["final_diagnostics"], "selected_layers_have_coverage_trace");
        assert!(execution
            .info["packing_diagnostics"]
            .contains("selected layer 0 has no matching covered item"));
    }

    #[test]
    fn meta_model_final_executor_reports_no_available_bin() {
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "i0".to_string() },
                1.0,
            )],
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: Vec::new(),
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "no_bin_final".to_string(),
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });

        let execution = executor.execute(&state);

        assert!(execution.packed_bins.is_empty());
        assert_eq!(execution.info["final_diagnostics"], "no_selected_layer_assignment");
        assert!(execution
            .info["packing_diagnostics"]
            .contains("no available bin type for final packing"));
    }

    #[test]
    fn meta_model_final_executor_reports_trace_item_mismatch() {
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "generated".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "i0".to_string() },
                1.0,
            )],
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer.clone()],
            layers: vec![layer],
            iteration: 0,
            layer_block_traces: HashMap::from([(
                0,
                vec![LayerBlockTrace {
                    block_index: 0,
                    item_index: 99,
                    item_id: "missing".to_string(),
                    orientation: Orientation::Upright,
                    nx: 1,
                    ny: 1,
                    nz: 1,
                    item_count: 1,
                    size: MetricSize3 {
                        width: meters(2.0),
                        height: meters(3.0),
                        depth: meters(4.0),
                    },
                    origin: MetricPoint3 {
                        x: meters(0.0),
                        y: meters(0.0),
                        z: meters(0.0),
                    },
                }],
            )]),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "trace_mismatch_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let execution = executor.execute(&state);

        assert!(execution.packed_bins.is_empty());
        assert!(execution
            .info["packing_diagnostics"]
            .contains("block trace references missing item index 99"));
    }

    #[test]
    fn solver_backed_meta_model_executors_use_backend_solution() {
        #[derive(Debug, Clone)]
        struct FakeBackend;

        impl MetaModelSolverBackend for FakeBackend {
            fn name(&self) -> &str {
                "fake"
            }

            fn solve_rmp(
                &self,
                _model: &MetaModel<f64>,
                diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                Ok(MetaModelExecutorSolveResult {
                    objective: Some(7.0),
                    primal_solution: vec![1.0; diagnostics.variable_count],
                    dual_solution: vec![1.25; diagnostics.demand_count],
                    info: HashMap::from([("backend_phase".to_string(), "rmp".to_string())]),
                })
            }

            fn solve_final(
                &self,
                _model: &MetaModel<f64>,
                diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                let mut primal_solution = vec![0.0; diagnostics.variable_count];
                if diagnostics.variable_count > 1 {
                    primal_solution[1] = 1.0;
                }
                Ok(MetaModelExecutorSolveResult {
                    objective: Some(3.0),
                    primal_solution,
                    dual_solution: Vec::new(),
                    info: HashMap::from([("backend_phase".to_string(), "final".to_string())]),
                })
            }
        }

        let bin = bin_type();
        let layer_a = BinLayer {
            iteration: 0,
            from: "seed-a".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer_b = BinLayer {
            iteration: 0,
            from: "seed-b".to_string(),
            bin: Some(bin.clone()),
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer_a.clone(), layer_b.clone()],
            layers: vec![layer_a, layer_b],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig::default(),
            FakeBackend,
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig::default(),
            FakeBackend,
        );

        let rmp_execution = rmp.execute(&state);
        let final_execution = final_executor.execute(&state);

        assert_eq!(rmp_execution.objective, Some(7.0));
        assert_eq!(rmp_execution.shadow_price_summary["item:i0"], 1.25);
        assert_eq!(rmp_execution.info["backend"], "fake");
        assert_eq!(rmp_execution.info["backend_phase"], "rmp");
        assert_eq!(final_execution.objective, Some(3.0));
        assert_eq!(final_execution.layers.len(), 1);
        assert_eq!(final_execution.layers[0].from, "seed-b");
        assert_eq!(final_execution.info["backend_phase"], "final");
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn column_generation_solver_backend_adapts_lp_and_milp_results() {
        #[derive(Debug, Clone)]
        struct MockSolver;

        impl ColumnGenerationSolver for MockSolver {
            fn name(&self) -> &str {
                "mock_solver"
            }

            fn solve_milp_with_options(
                &self,
                model: &LinearTriadModel,
                _options: FrameworkSolveOptions,
            ) -> ospf_rust_core::error::Result<FeasibleSolution> {
                let mut solution = vec![0.0; model.num_variables()];
                if model.num_variables() > 1 {
                    solution[1] = 1.0;
                }
                Ok(FeasibleSolution::new(4.0, solution))
            }

            fn solve_lp_with_options(
                &self,
                model: &LinearTriadModel,
                _options: FrameworkSolveOptions,
            ) -> ospf_rust_core::error::Result<LPResult> {
                Ok(LPResult::new(
                    FeasibleSolution::new(8.0, vec![1.0; model.num_variables()]),
                    LinearDualSolution::new(vec![2.5], Vec::new()),
                ))
            }
        }

        let bin = bin_type();
        let layer_a = BinLayer {
            iteration: 0,
            from: "seed-a".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer_b = BinLayer {
            iteration: 0,
            from: "seed-b".to_string(),
            bin: Some(bin.clone()),
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };
        let state = ColumnGenerationApplicationState {
            items: vec![item("i0")],
            bins: vec![bin],
            initial_layers: vec![layer_a.clone(), layer_b.clone()],
            layers: vec![layer_a, layer_b],
            iteration: 0,
            layer_block_traces: HashMap::new(),
            layer_placement_traces: HashMap::new(),
            continuous_radius_component: None,
            info: HashMap::new(),
        };
        let backend = ColumnGenerationSolverMetaModelBackend::new(MockSolver);
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig::default(),
            backend.clone(),
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig::default(),
            backend,
        );

        let rmp_execution = rmp.execute(&state);
        let final_execution = final_executor.execute(&state);

        assert_eq!(rmp_execution.objective, Some(8.0));
        assert_eq!(rmp_execution.shadow_price_summary["item:i0"], 2.5);
        assert_eq!(rmp_execution.info["backend"], "mock_solver");
        assert_eq!(rmp_execution.info["backend_kind"], "column_generation_solver");
        assert_eq!(final_execution.objective, Some(4.0));
        assert_eq!(final_execution.layers.len(), 1);
        assert_eq!(final_execution.layers[0].from, "seed-b");
        assert_eq!(final_execution.info["backend_phase"], "final");
    }

