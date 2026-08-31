    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_loads_smoke_no_run() {
        let suite = SolverDatasetFixtureSuite::smoke();
        let requests = suite.load().unwrap();
        let diagnostics = suite.run_no_run("fake", "serde").unwrap();

        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].items.len(), 1);
        assert_eq!(diagnostics.dataset_count, 3);
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|message| message.contains("serde CSV fixture")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_runs_feature_matrix_and_fake_flow() {
        let suite = SolverDatasetFixtureSuite::smoke();
        let matrix = suite
            .run_feature_matrix_no_run(
                "fake",
                vec!["serde".to_string(), "scip".to_string()],
            )
            .unwrap();
        let run = suite.run_fake_one_round();

        assert_eq!(matrix.features.len(), 2);
        assert_eq!(matrix.diagnostics.len(), 4);
        assert_eq!(run.fixture_results.len(), 3);
        assert_eq!(run.fixture_count, 3);
        assert_eq!(run.loaded_count, 3);
        assert_eq!(run.materialized_count, 3);
        assert_eq!(run.no_run_count, 3);
        assert_eq!(run.executed_count, 3);
        assert_eq!(run.backend_smoke_count, 3);
        assert!(run.diagnostic_count > 0);
        assert!(run.fixture_results.iter().all(|result| result.loaded));
        assert!(run.fixture_results.iter().all(|result| result.materialized));
        assert!(run.fixture_results.iter().all(|result| result.no_run_validated));
        assert!(run.fixture_results.iter().all(|result| result.executed));
        assert!(run.fixture_results.iter().all(|result| result.group.as_deref() == Some("smoke")));
        assert!(run
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("backend_smoke=3")));
        assert_eq!(run.backend.as_deref(), Some("fake"));
        assert!(run
            .fixture_results
            .iter()
            .any(|result| result.render_plan_count > 0));
        assert!(run
            .fixture_results
            .iter()
            .flat_map(|result| result.diagnostics.iter())
            .any(|diagnostic| diagnostic.contains("package attribute")));

        let no_run = suite.run_batch_report("fake", "serde", false);
        assert!(no_run.fixture_results.iter().all(|result| result.no_run_validated));
        assert!(no_run.fixture_results.iter().all(|result| !result.executed));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_filter_and_run_report_integration() {
        use crate::application::report::{FixtureFilter, Bpp3dFixtureStatus};

        let suite = SolverDatasetFixtureSuite::smoke();

        // 按 group 过滤 / Filter by group "smoke"
        let filter = FixtureFilter::all().with_group("smoke");
        let sub = suite.filter(&filter);
        assert_eq!(sub.fixtures.len(), 3);

        // 按 tag 过滤 / Filter by tag "cylinder"
        let cylinder_filter = FixtureFilter::all().with_tags(vec!["cylinder".to_string()]);
        let cylinder_sub = suite.filter(&cylinder_filter);
        assert_eq!(cylinder_sub.fixtures.len(), 2);
        assert!(cylinder_sub.fixtures.iter().all(|f| f.tags.contains(&"cylinder".to_string())));

        // 按 backend smoke 过滤 / Filter by backend smoke
        let smoke_filter = FixtureFilter::all().backend_smoke_only();
        let smoke_sub = suite.filter(&smoke_filter);
        assert_eq!(smoke_sub.fixtures.len(), 3);

        // 按名称前缀过滤 / Filter by name prefix
        let prefix_filter = FixtureFilter::all().with_name_prefix("pwl");
        let prefix_sub = suite.filter(&prefix_filter);
        assert_eq!(prefix_sub.fixtures.len(), 1);
        assert_eq!(prefix_sub.fixtures[0].name, "pwl-radius-smoke.csv");

        // 运行并生成报告 / Run batch and generate report
        let start = Instant::now();
        let batch = suite.run_batch_report("fake", "serde", true);
        let report = suite.to_run_report(&batch, "fake", Some("serde".to_string()), start.elapsed());

        assert!(report.all_passed());
        assert_eq!(report.summary.total, 3);
        assert_eq!(report.summary.succeeded, 3);
        assert_eq!(report.backend, "fake");
        assert_eq!(report.feature, Some("serde".to_string()));
        assert!(report.fixtures.iter().all(|f| f.status == Bpp3dFixtureStatus::Success));
        assert!(report.fixtures.iter().any(|f| f.render_plan_count > 0));
        assert!(report.fixtures.iter().any(|f| !f.selected_layers.is_empty()));
        assert!(report.fixtures.iter().any(|f| !f.packed_bins.is_empty()));
        assert!(report
            .fixtures
            .iter()
            .flat_map(|f| f.selected_layers.iter())
            .any(|layer| !layer.coverage.is_empty()));

        // JSON 序列化往返 / JSON roundtrip
        let json = report.to_json().unwrap();
        let deserialized = crate::application::report::Bpp3dRunReport::from_json(&json).unwrap();
        assert_eq!(deserialized.suite_name, report.suite_name);
        assert_eq!(deserialized.fixtures.len(), report.fixtures.len());
        assert!(deserialized.all_passed());

        // Display 格式化 / Display format
        let display = format!("{}", report);
        assert!(display.contains("BPP3D Run Report"));
        assert!(display.contains("fake"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_runs_backend_survey() {
        let suite = SolverDatasetFixtureSuite::smoke();
        let survey = suite.run_backend_survey("fake", "serde", true).unwrap();

        assert_eq!(survey.suite_name, "bpp3d-smoke");
        assert_eq!(survey.dataset_count, 3);
        assert!(survey.loaded);
        assert!(survey.materialized);
        assert!(survey.no_run_validated);
        assert!(survey.fallback_executed);
        assert!(survey
            .diagnostics
            .iter()
            .any(|message| message.contains("backend survey")));
        assert!(survey
            .diagnostics
            .iter()
            .any(|message| message.contains("compile_enabled")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_loads_manifest_and_directory() {
        let root = std::env::temp_dir().join(format!(
            "bpp3d-fixture-suite-{}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let csv = SolverDatasetFixtureSuite::smoke().fixtures[0].csv.clone();
        std::fs::write(root.join("a.csv"), &csv).unwrap();
        std::fs::write(root.join("b.csv"), &csv).unwrap();
        std::fs::write(root.join("nested").join("c.csv"), &csv).unwrap();
        std::fs::write(
            root.join("manifest.json"),
            r#"{"suite_name":"manifest-file-suite","entries":[{"name":"manifest-file-a.csv","path":"a.csv"}]}"#,
        )
        .unwrap();
        let manifest = SolverDatasetFixtureManifest {
            suite_name: "manifest-suite".to_string(),
            entries: vec![SolverDatasetFixtureManifestEntry {
                name: "manifest-a.csv".to_string(),
                path: "a.csv".to_string(),
                group: Some("manifest".to_string()),
                tags: vec!["tag-a".to_string()],
                backend_smoke: true,
            }],
        };

        let manifest_suite = SolverDatasetFixtureSuite::from_manifest(&root, manifest).unwrap();
        let manifest_file_suite = SolverDatasetFixtureSuite::from_manifest_file(root.join("manifest.json")).unwrap();
        let directory_suite = SolverDatasetFixtureSuite::from_directory("dir-suite", &root).unwrap();
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(manifest_suite.fixtures.len(), 1);
        assert_eq!(manifest_suite.fixtures[0].group.as_deref(), Some("manifest"));
        assert_eq!(manifest_suite.fixtures[0].tags, vec!["tag-a".to_string()]);
        assert!(manifest_suite.fixtures[0].backend_smoke);
        assert_eq!(manifest_file_suite.suite_name, "manifest-file-suite");
        assert_eq!(manifest_file_suite.fixtures[0].name, "manifest-file-a.csv");
        assert_eq!(directory_suite.fixtures.len(), 3);
        assert_eq!(directory_suite.fixtures[0].name, "a.csv");
        assert!(directory_suite
            .fixtures
            .iter()
            .any(|fixture| fixture.name == "nested/c.csv"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_loads_crate_fixtures() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("manifest.json");
        let suite = SolverDatasetFixtureSuite::from_manifest_file(manifest_path).unwrap();
        let report = suite.run_batch_report("fake", "serde", false);
        let requests = suite.load().unwrap();
        let depth_boundary_request = suite
            .fixtures
            .iter()
            .zip(requests.iter())
            .find(|(fixture, _)| fixture.name == "grouped-layer-depth-boundary-sample.csv")
            .map(|(_, request)| request)
            .unwrap();
        let dynamic_diameter_request = suite
            .fixtures
            .iter()
            .zip(requests.iter())
            .find(|(fixture, _)| fixture.name == "material-width-amount-dynamic-diameter-sample.csv")
            .map(|(_, request)| request)
            .unwrap();
        let continuous_radius_count = requests
            .iter()
            .filter(|request| request.continuous_radius_component.is_some())
            .count();

        assert_eq!(suite.suite_name, "bpp3d-regression");
        assert_eq!(suite.fixtures.len(), 22);
        assert!(suite.fixtures.iter().any(|fixture| fixture.tags.iter().any(|tag| tag == "kotlin")));
        assert_eq!(report.fixture_count, 22);
        assert_eq!(report.materialized_count, 22);
        assert!(report.backend_smoke_count >= 3);
        assert!(report.fixture_results.iter().all(|result| result.materialized));
        assert!(depth_boundary_request.depth_boundary_policy.is_some());
        assert!(continuous_radius_count >= 10);
        assert!(dynamic_diameter_request
            .items
            .iter()
            .filter_map(|item| match &item.shape_spec_override {
                Some(PackageShapeSpec::Cylinder { radius_candidates, .. }) => radius_candidates.as_ref(),
                _ => None,
            })
            .any(|candidates| candidates.len() >= 3));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("suite 'bpp3d-regression'")));
        assert!(report
            .fixture_results
            .iter()
            .any(|result| result.dataset_name.contains("pwl-radius")));
        assert!(report
            .fixture_results
            .iter()
            .any(|result| result.dataset_name.contains("grouped-layer")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_builds_kotlin_comparison_baseline() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("manifest.json");
        let suite = SolverDatasetFixtureSuite::from_manifest_file(manifest_path).unwrap();

        let baseline = suite.kotlin_comparison_baseline_report(
            "kotlin-gurobi",
            Some("manifest".to_string()),
        );

        assert_eq!(baseline.backend, "kotlin-gurobi");
        assert_eq!(baseline.summary.total, 19);
        assert_eq!(baseline.summary.skipped, 19);
        assert!(baseline
            .fixtures
            .iter()
            .all(|fixture| fixture.group.as_deref() == Some("kotlin-gurobi")));
        assert!(baseline
            .fixtures
            .iter()
            .any(|fixture| fixture.tags.iter().any(|tag| tag == "pwl-radius")));
        assert!(baseline
            .fixtures
            .iter()
            .any(|fixture| fixture.tags.iter().any(|tag| tag == "continuous-radius")));

        let candidate = crate::application::report::Bpp3dRunReport::new(
            "candidate",
            "rust-fake",
            Some("serde".to_string()),
            "candidate",
            Duration::default(),
            baseline.fixtures[0..18].to_vec(),
        );
        let comparison = baseline.compare_with(&candidate);

        assert!(comparison.has_regression());
        assert!(comparison
            .differences
            .iter()
            .any(|difference| difference.field == "fixtures"
                && difference.message.contains("missing")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_writes_kotlin_comparison_artifacts() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("manifest.json");
        let suite = SolverDatasetFixtureSuite::from_manifest_file(manifest_path).unwrap();
        let candidate_batch = suite.filter(
            &crate::application::report::FixtureFilter::all().with_group("kotlin-gurobi"),
        ).run_batch_report("fake", "serde", false);
        let candidate = suite.to_run_report(
            &candidate_batch,
            "rust-fake",
            Some("serde".to_string()),
            Duration::default(),
        );
        let output_dir = std::env::temp_dir().join(format!(
            "bpp3d-kotlin-artifacts-{}",
            std::process::id(),
        ));

        let comparison = suite
            .write_kotlin_comparison_artifacts(
                &output_dir,
                &candidate,
                "kotlin-gurobi",
                Some("manifest".to_string()),
            )
            .unwrap();
        let baseline_path = output_dir.join("kotlin-baseline-report.json");
        let candidate_path = output_dir.join("rust-candidate-report.json");
        let summary_path = output_dir.join("kotlin-comparison-summary.txt");
        let baseline = crate::application::report::Bpp3dRunReport::read_json_file(&baseline_path)
            .unwrap();
        let candidate_roundtrip =
            crate::application::report::Bpp3dRunReport::read_json_file(&candidate_path).unwrap();
        let summary = std::fs::read_to_string(&summary_path).unwrap();
        let _ = std::fs::remove_dir_all(&output_dir);

        assert_eq!(baseline.backend, "kotlin-gurobi");
        assert_eq!(candidate_roundtrip.backend, "rust-fake");
        assert!(summary.contains("baseline="));
        assert_eq!(
            comparison.baseline_fixture_count,
            baseline.summary.total,
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_builds_layer_generation_quality_report() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("manifest.json");
        let suite = SolverDatasetFixtureSuite::from_manifest_file(manifest_path).unwrap();
        let report = suite.layer_generation_quality_report(
            &crate::application::report::FixtureFilter::all().with_group("kotlin-gurobi"),
            16,
        );
        let path = std::env::temp_dir().join(format!(
            "bpp3d-layer-quality-{}.json",
            std::process::id(),
        ));

        report.write_json_file(&path).unwrap();
        let loaded = LayerGenerationSuiteQualityReport::read_json_file(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(report.fixture_count, 19);
        assert_eq!(report.loaded_count, 19);
        assert_eq!(report.materialized_count, 19);
        assert!(report.total_candidate_count > 0);
        assert!(report.total_placement_trace_count > 0);
        assert!(report
            .fixtures
            .iter()
            .any(|fixture| fixture.candidates_by_source.contains_key("pattern_layer_generator")
                || fixture.candidates_by_source.contains_key("pile_layer_generator")
                || fixture.candidates_by_source.contains_key("circle_packing_layer_generator")));
        assert!(report
            .source_quality
            .values()
            .any(|source| source.source == "pattern_layer_generator"
                || source.source == "pile_layer_generator"
                || source.source == "historical_layer_generator"));
        assert!(report
            .source_quality
            .values()
            .all(|source| source.candidate_count > 0));
        assert_eq!(loaded.fixture_count, report.fixture_count);
        assert_eq!(loaded.source_quality.len(), report.source_quality.len());
        assert!(loaded.diagnostics[0].contains("quality report"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn layer_generation_quality_report_compares_source_baselines() {
        let mut baseline = LayerGenerationSuiteQualityReport {
            suite_name: "baseline".to_string(),
            fixture_count: 1,
            loaded_count: 1,
            materialized_count: 1,
            total_candidate_count: 2,
            total_placement_trace_count: 2,
            total_block_trace_count: 1,
            ..Default::default()
        };
        baseline.source_quality.insert(
            "pattern_layer_generator".to_string(),
            LayerGenerationSourceQualityReport {
                source: "pattern_layer_generator".to_string(),
                candidate_count: 2,
                best_numeric_score: Some(4.0),
                total_coverage: 4.0,
                best_coverage: 3.0,
                placement_trace_count: 2,
                block_trace_count: 1,
                diagnostic_count: 0,
            },
        );
        baseline.fixtures.push(LayerGenerationFixtureQualityReport {
            dataset_name: "fixture-a".to_string(),
            loaded: true,
            materialized: true,
            candidate_count: 2,
            placement_trace_count: 2,
            block_trace_count: 1,
            source_quality: baseline.source_quality.clone(),
            ..Default::default()
        });

        let mut candidate = baseline.clone();
        candidate.suite_name = "candidate".to_string();
        candidate.total_candidate_count = 1;
        candidate.total_placement_trace_count = 1;
        candidate.total_block_trace_count = 0;
        candidate.fixtures[0].candidate_count = 1;
        candidate.fixtures[0].placement_trace_count = 1;
        candidate.fixtures[0].block_trace_count = 0;
        candidate.source_quality.get_mut("pattern_layer_generator").unwrap().candidate_count = 1;
        candidate.source_quality.get_mut("pattern_layer_generator").unwrap().best_coverage = 1.0;
        candidate.source_quality.get_mut("pattern_layer_generator").unwrap().placement_trace_count = 1;
        candidate.source_quality.get_mut("pattern_layer_generator").unwrap().best_numeric_score = Some(2.0);
        candidate.fixtures[0].source_quality = candidate.source_quality.clone();

        let comparison = baseline.compare_with(&candidate);

        assert!(comparison.has_regression());
        assert!(comparison.regression_count() >= 4);
        assert!(comparison
            .differences
            .iter()
            .any(|difference| difference.field == "source.best_coverage"
                && difference.source.as_deref() == Some("pattern_layer_generator")));
        assert!(comparison.summary_string().contains("regressions="));
    }

