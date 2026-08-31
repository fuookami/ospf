    #[cfg(all(feature = "serde", feature = "gurobi10", not(feature = "async")))]
    #[test]
    fn csv_materialized_runs_real_gurobi10_backend_smoke() {
        use ospf_rust_core::solver::solvers::gurobi::GurobiConfig;
        use ospf_rust_framework::solver::GurobiColumnGenerationSolver;

        let request = real_solver_smoke_request();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp_backend = ColumnGenerationSolverMetaModelBackend::new(
            GurobiColumnGenerationSolver::with_config(
                GurobiConfig::new()
                    .with_output(false)
                    .with_time_limit(30.0),
            ),
        );
        let final_backend = ColumnGenerationSolverMetaModelBackend::new(
            GurobiColumnGenerationSolver::with_config(
                GurobiConfig::new()
                    .with_output(false)
                    .with_time_limit(30.0),
            ),
        );
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig {
                model_name: "bpp3d_gurobi10_smoke_rmp".to_string(),
                ..Default::default()
            },
            rmp_backend,
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig {
                model_name: "bpp3d_gurobi10_smoke_final".to_string(),
                ..Default::default()
            },
            final_backend,
        );

        let result = service
            .run_csv_materialized_with_bins(request, &rmp, &final_executor)
            .unwrap();

        assert_eq!(result.rmp.info.get("backend").map(String::as_str), Some("gurobi"), "{:?}", result.rmp.info);
        assert_eq!(result.rmp.info.get("backend_kind").map(String::as_str), Some("column_generation_solver"), "{:?}", result.rmp.info);
        assert_eq!(result.final_execution.info.get("backend").map(String::as_str), Some("gurobi"), "{:?}", result.final_execution.info);
        assert_eq!(result.final_execution.info.get("backend_kind").map(String::as_str), Some("column_generation_solver"), "{:?}", result.final_execution.info);
        assert!(result.rmp.objective.is_some());
        assert!(result.final_execution.objective.is_some());
        assert!(!result.final_execution.layers.is_empty());
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[cfg(all(feature = "serde", feature = "gurobi10", not(feature = "async")))]
    #[test]
    fn fixture_suite_runs_real_gurobi10_backend_smoke() {
        use ospf_rust_core::solver::solvers::gurobi::GurobiConfig;
        use ospf_rust_framework::solver::GurobiColumnGenerationSolver;

        let config = GurobiConfig::new()
            .with_output(false)
            .with_time_limit(30.0);
        let report = run_real_backend_smoke_fixtures(
            "gurobi",
            true,
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    GurobiColumnGenerationSolver::with_config(config.clone()),
                )
            },
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    GurobiColumnGenerationSolver::with_config(config.clone()),
                )
            },
        );
        assert!(report.all_passed(), "gurobi smoke report: {}", report.summary_string());
        assert!(report.summary.total > 0);
        assert!(report.fixtures.iter().all(|f| f.rmp_objective.is_some()));
        assert!(report.fixtures.iter().all(|f| f.final_objective.is_some()));
    }

    #[cfg(all(feature = "serde", feature = "gurobi10", not(feature = "async")))]
    #[test]
    fn fixture_suite_runs_real_gurobi10_full_manifest() {
        use ospf_rust_core::solver::solvers::gurobi::GurobiConfig;
        use ospf_rust_framework::solver::GurobiColumnGenerationSolver;

        let config = GurobiConfig::new()
            .with_output(false)
            .with_time_limit(30.0);
        let report = run_real_backend_smoke_fixtures(
            "gurobi",
            false,
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    GurobiColumnGenerationSolver::with_config(config.clone()),
                )
            },
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    GurobiColumnGenerationSolver::with_config(config.clone()),
                )
            },
        );
        assert!(report.all_passed(), "gurobi full report: {}", report.summary_string());
        assert!(report.summary.total >= 3);
    }

    #[cfg(all(feature = "serde", feature = "scip", not(feature = "async")))]
    #[test]
    fn csv_materialized_runs_real_scip_backend_smoke() {
        use ospf_rust_core::solver::solvers::scip::SCIPConfig;
        use ospf_rust_framework::solver::ScipColumnGenerationSolver;

        let request = real_solver_smoke_request();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp_backend = ColumnGenerationSolverMetaModelBackend::new(
            ScipColumnGenerationSolver::with_config(
                SCIPConfig::new()
                    .with_output(false)
                    .with_time_limit(30.0),
            )
            .with_lp_subproblem_defaults(),
        );
        let final_backend = ColumnGenerationSolverMetaModelBackend::new(
            ScipColumnGenerationSolver::with_config(
                SCIPConfig::new()
                    .with_output(false)
                    .with_time_limit(30.0),
            ),
        );
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig {
                model_name: "bpp3d_scip_smoke_rmp".to_string(),
                ..Default::default()
            },
            rmp_backend,
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig {
                model_name: "bpp3d_scip_smoke_final".to_string(),
                ..Default::default()
            },
            final_backend,
        );

        let result = service
            .run_csv_materialized_with_bins(request, &rmp, &final_executor)
            .unwrap();

        assert_eq!(result.rmp.info.get("backend").map(String::as_str), Some("scip"), "{:?}", result.rmp.info);
        assert_eq!(result.rmp.info.get("backend_kind").map(String::as_str), Some("column_generation_solver"), "{:?}", result.rmp.info);
        assert_eq!(result.final_execution.info.get("backend").map(String::as_str), Some("scip"), "{:?}", result.final_execution.info);
        assert_eq!(result.final_execution.info.get("backend_kind").map(String::as_str), Some("column_generation_solver"), "{:?}", result.final_execution.info);
        assert!(result.rmp.objective.is_some());
        assert!(result.final_execution.objective.is_some());
        assert!(!result.final_execution.layers.is_empty());
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[cfg(all(feature = "serde", feature = "scip", not(feature = "async")))]
    #[test]
    fn fixture_suite_runs_real_scip_backend_smoke() {
        use ospf_rust_core::solver::solvers::scip::SCIPConfig;
        use ospf_rust_framework::solver::ScipColumnGenerationSolver;

        let config = SCIPConfig::new()
            .with_output(false)
            .with_time_limit(30.0);
        let report = run_real_backend_smoke_fixtures(
            "scip",
            true,
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    ScipColumnGenerationSolver::with_config(config.clone())
                        .with_lp_subproblem_defaults(),
                )
            },
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    ScipColumnGenerationSolver::with_config(config.clone()),
                )
            },
        );
        assert!(report.all_passed(), "scip smoke report: {}", report.summary_string());
        assert!(report.summary.total > 0);
    }

    #[cfg(all(feature = "serde", feature = "scip", not(feature = "async")))]
    #[test]
    fn fixture_suite_runs_real_scip_full_manifest() {
        use ospf_rust_core::solver::solvers::scip::SCIPConfig;
        use ospf_rust_framework::solver::ScipColumnGenerationSolver;

        let config = SCIPConfig::new()
            .with_output(false)
            .with_time_limit(30.0);
        let report = run_real_backend_smoke_fixtures(
            "scip",
            false,
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    ScipColumnGenerationSolver::with_config(config.clone())
                        .with_lp_subproblem_defaults(),
                )
            },
            || {
                ColumnGenerationSolverMetaModelBackend::new(
                    ScipColumnGenerationSolver::with_config(config.clone()),
                )
            },
        );
        assert!(report.all_passed(), "scip full report: {}", report.summary_string());
        assert!(report.summary.total >= 3);
    }

    #[cfg(all(
        feature = "serde",
        not(feature = "async"),
        any(feature = "gurobi10", feature = "scip")
    ))]
    fn real_solver_smoke_request() -> crate::application::csv::CsvMaterializedApplicationRequest {
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
        crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap()
    }

    #[cfg(all(
        feature = "serde",
        not(feature = "async"),
        any(feature = "gurobi10", feature = "scip")
    ))]
    fn run_real_backend_smoke_fixtures<R, F, RB, FB>(
        backend_name: &str,
        backend_smoke_only: bool,
        rmp_backend: R,
        final_backend: F,
    ) -> crate::application::report::Bpp3dRunReport
    where
        R: Fn() -> RB,
        F: Fn() -> FB,
        RB: MetaModelSolverBackend + 'static,
        FB: MetaModelSolverBackend + 'static,
    {
        let filter = if backend_smoke_only {
            crate::application::report::FixtureFilter::all().backend_smoke_only()
        } else {
            crate::application::report::FixtureFilter::all()
        };
        run_real_backend_smoke_fixtures_with_filter(
            backend_name,
            &filter,
            rmp_backend,
            final_backend,
        )
    }

    /// 运行真实 backend fixture 并返回结构化报告 /
    /// Run real backend fixtures and return structured report
    ///
    /// 每个 fixture 的运行结果被收集到 `Bpp3dFixtureReport`，
    /// 包括 rmp/final objective、layer_count、selected_layer_count、
    /// packed_bin_count、duration_ms 和失败诊断。
    /// Each fixture result is collected into `Bpp3dFixtureReport` including
    /// rmp/final objective, layer/selected/packed counts, duration, and failure diagnostics.
    #[cfg(all(
        feature = "serde",
        not(feature = "async"),
        any(feature = "gurobi10", feature = "scip")
    ))]
    fn run_real_backend_smoke_fixtures_with_filter<R, F, RB, FB>(
        backend_name: &str,
        filter: &crate::application::report::FixtureFilter,
        rmp_backend: R,
        final_backend: F,
    ) -> crate::application::report::Bpp3dRunReport
    where
        R: Fn() -> RB,
        F: Fn() -> FB,
        RB: MetaModelSolverBackend + 'static,
        FB: MetaModelSolverBackend + 'static,
    {
        use crate::application::report::{
            Bpp3dFixtureReport, Bpp3dFixtureStatus, Bpp3dErrorCategory,
            Bpp3dRunReport, Bpp3dSolverFailure, Bpp3dSolverAvailability,
        };

        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("manifest.json");
        let suite = SolverDatasetFixtureSuite::from_manifest_file(manifest_path).unwrap();
        let fixtures = suite
            .fixtures
            .iter()
            .filter(|f| filter.matches(&f.name, &f.group, &f.tags, f.backend_smoke))
            .collect::<Vec<_>>();
        assert!(!fixtures.is_empty(), "no fixtures matched filter");

        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let suite_start = Instant::now();
        let mut fixture_reports = Vec::with_capacity(fixtures.len());

        for fixture in fixtures {
            let fixture_start = Instant::now();

            // 加载 CSV / Load CSV
            let request = match crate::application::csv::CsvDatasetLoader::load_any_str(&fixture.csv)
                .and_then(|ds| ds.materialize())
            {
                Ok(req) => req,
                Err(error) => {
                    let duration_ms = fixture_start.elapsed().as_millis() as u64;
                    fixture_reports.push(Bpp3dFixtureReport {
                        name: fixture.name.clone(),
                        group: fixture.group.clone(),
                        tags: fixture.tags.clone(),
                        status: Bpp3dFixtureStatus::Failed,
                        duration_ms,
                        backend: Some(backend_name.to_string()),
                        feature: None,
                        rmp_objective: None,
                        final_objective: None,
                        model_status: None,
                        layer_count: 0,
                        selected_layer_count: 0,
                        packed_bin_count: 0,
                        render_plan_count: 0,
                        selected_layers: Vec::new(),
                        packed_bins: Vec::new(),
                        diagnostics: vec![error.to_string()],
                        failure: Some(Bpp3dSolverFailure::error(
                            Bpp3dErrorCategory::LoadingError,
                            error.to_string(),
                        )),
                    });
                    continue;
                }
            };

            let rmp = SolverBackedMetaModelRmpExecutor::new(
                MetaModelRmpExecutorConfig {
                    model_name: format!("bpp3d_{}_{}_rmp", backend_name, fixture.name),
                    ..Default::default()
                },
                rmp_backend(),
            );
            let final_executor = SolverBackedMetaModelFinalExecutor::new(
                MetaModelFinalExecutorConfig {
                    model_name: format!("bpp3d_{}_{}_final", backend_name, fixture.name),
                    ..Default::default()
                },
                final_backend(),
            );

            match service.run_csv_materialized_with_bins(request, &rmp, &final_executor) {
                Ok(flow) => {
                    let duration_ms = fixture_start.elapsed().as_millis() as u64;
                    let rmp_backend_name = flow.rmp.info.get("backend").cloned();
                    let final_backend_name = flow.final_execution.info.get("backend").cloned();
                    let layer_count = flow.result.layers.len();
                    let selected_layer_count = flow.final_execution.layers.len();
                    let packed_bin_count = flow.final_execution.packed_bins.len();
                    let render_plan_count = flow.result.render_loading_plans.len();
                    let selected_layers = selected_layer_reports(&flow.final_execution.layers);
                    let packed_bins = packed_bin_reports(&flow.final_execution.packed_bins);
                    let mut diagnostics: Vec<String> = flow
                        .final_execution
                        .info
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    diagnostics.sort();

                    // 验证 backend 名称一致性 / Verify backend name consistency
                    let backend_ok = rmp_backend_name.as_deref() == Some(backend_name)
                        && final_backend_name.as_deref() == Some(backend_name);
                    let objectives_ok = flow.rmp.objective.is_some()
                        && flow.final_execution.objective.is_some();
                    let layers_ok = !flow.final_execution.layers.is_empty();

                    let (status, failure) = if backend_ok && objectives_ok && layers_ok {
                        (Bpp3dFixtureStatus::Success, None)
                    } else {
                        let mut messages = Vec::new();
                        if !backend_ok {
                            messages.push(format!(
                                "backend mismatch: rmp={:?}, final={:?}",
                                rmp_backend_name, final_backend_name,
                            ));
                        }
                        if !objectives_ok {
                            messages.push("missing objectives".to_string());
                        }
                        if !layers_ok {
                            messages.push("missing final layers".to_string());
                        }
                        let msg = messages.join("; ");
                        (
                            Bpp3dFixtureStatus::Failed,
                            Some(Bpp3dSolverFailure::error(
                                Bpp3dErrorCategory::ModelStatusError,
                                msg,
                            )),
                        )
                    };

                    fixture_reports.push(Bpp3dFixtureReport {
                        name: fixture.name.clone(),
                        group: fixture.group.clone(),
                        tags: fixture.tags.clone(),
                        status,
                        duration_ms,
                        backend: Some(backend_name.to_string()),
                        feature: None,
                        rmp_objective: flow.rmp.objective,
                        final_objective: flow.final_execution.objective,
                        model_status: None,
                        layer_count,
                        selected_layer_count,
                        packed_bin_count,
                        render_plan_count,
                        selected_layers,
                        packed_bins,
                        diagnostics,
                        failure,
                    });
                }
                Err(errors) => {
                    let duration_ms = fixture_start.elapsed().as_millis() as u64;
                    let message = errors.join("; ");
                    fixture_reports.push(Bpp3dFixtureReport {
                        name: fixture.name.clone(),
                        group: fixture.group.clone(),
                        tags: fixture.tags.clone(),
                        status: Bpp3dFixtureStatus::Failed,
                        duration_ms,
                        backend: Some(backend_name.to_string()),
                        feature: None,
                        rmp_objective: None,
                        final_objective: None,
                        model_status: None,
                        layer_count: 0,
                        selected_layer_count: 0,
                        packed_bin_count: 0,
                        render_plan_count: 0,
                        selected_layers: Vec::new(),
                        packed_bins: Vec::new(),
                        diagnostics: errors,
                        failure: Some(Bpp3dSolverFailure::error(
                            Bpp3dErrorCategory::SolveError,
                            message,
                        )),
                    });
                }
            }
        }

        let total_duration = suite_start.elapsed();
        let timestamp = format!("{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
        let report = Bpp3dRunReport::new(
            "real-backend-smoke",
            backend_name,
            None,
            &timestamp,
            total_duration,
            fixture_reports,
        )
        .with_availability(Bpp3dSolverAvailability::Available);

        // 断言所有 fixture 通过 / Assert all fixtures passed
        let failures = report.failure_reports();
        assert!(
            failures.is_empty(),
            "real backend smoke had {} failure(s):\n{}",
            failures.len(),
            failures.iter().map(|f| {
                f.failure.as_ref()
                    .map(|fail| fail.summary())
                    .unwrap_or_else(|| format!("{}: {}", f.name, f.status))
            }).collect::<Vec<_>>().join("\n"),
        );

        report
    }

