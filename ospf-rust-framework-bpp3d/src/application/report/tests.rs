// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_status_display() {
        assert_eq!(Bpp3dFixtureStatus::Success.to_string(), "success");
        assert_eq!(Bpp3dFixtureStatus::Failed.to_string(), "failed");
        assert_eq!(Bpp3dFixtureStatus::Skipped.to_string(), "skipped");
    }

    #[test]
    fn solver_availability_diagnostics() {
        let avail = Bpp3dSolverAvailability::Available;
        assert!(avail.is_available());
        assert_eq!(avail.diagnostics(), "solver available");

        let not_enabled = Bpp3dSolverAvailability::FeatureNotEnabled("gurobi10".to_string());
        assert!(!not_enabled.is_available());
        assert!(not_enabled.diagnostics().contains("gurobi10"));
    }

    #[test]
    fn solver_failure_summary() {
        let failure = Bpp3dSolverFailure::error(
            Bpp3dErrorCategory::SolveError,
            "solver timed out",
        );
        let summary = failure.summary();
        assert!(summary.contains("solve_error"));
        assert!(summary.contains("solver timed out"));
    }

    #[test]
    fn solver_failure_unavailable() {
        let failure = Bpp3dSolverFailure::unavailable(
            Bpp3dSolverAvailability::FeatureNotEnabled("scip".to_string()),
        );
        assert_eq!(failure.category, Bpp3dErrorCategory::SolveError);
        assert_eq!(failure.model_status, Bpp3dSolverModelStatus::NotSolved);
        assert!(failure.message.contains("scip"));
    }

    #[test]
    fn fixture_report_success() {
        let report = Bpp3dFixtureReport::success("cuboid-basic", 150);
        assert!(report.status.is_success());
        assert_eq!(report.duration_ms, 150);
        assert!(report.failure.is_none());
    }

    #[test]
    fn fixture_report_failed() {
        let failure = Bpp3dSolverFailure::error(
            Bpp3dErrorCategory::SolveError,
            "infeasible model",
        );
        let report = Bpp3dFixtureReport::failed("cylinder-complex", 500, failure);
        assert!(report.status.is_failed());
        assert!(report.failure.is_some());
        assert!(!report.diagnostics.is_empty());
    }

    #[test]
    fn suite_summary_from_reports() {
        let reports = vec![
            Bpp3dFixtureReport::success("a", 100),
            Bpp3dFixtureReport::success("b", 200),
            Bpp3dFixtureReport::failed(
                "c",
                300,
                Bpp3dSolverFailure::error(Bpp3dErrorCategory::SolveError, "err"),
            ),
            Bpp3dFixtureReport::skipped("d"),
        ];
        let summary = Bpp3dSuiteSummary::from_reports(&reports);
        assert_eq!(summary.total, 4);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.all_passed());
    }

    #[test]
    fn run_report_creation_and_summary() {
        let fixtures = vec![
            Bpp3dFixtureReport::success("fix-1", 100),
            Bpp3dFixtureReport::success("fix-2", 200),
        ];
        let report = Bpp3dRunReport::new(
            "test-suite",
            "fake",
            Some("serde".to_string()),
            "2026-06-15T10:00:00+08:00",
            Duration::from_millis(300),
            fixtures,
        );
        assert!(report.all_passed());
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.succeeded, 2);
        let summary_str = report.summary_string();
        assert!(summary_str.contains("test-suite"));
        assert!(summary_str.contains("fake"));
    }

    #[test]
    fn run_report_display() {
        let fixtures = vec![
            Bpp3dFixtureReport::success("fix-1", 100),
        ];
        let report = Bpp3dRunReport::new(
            "display-test",
            "noop",
            None,
            "2026-06-15",
            Duration::from_millis(100),
            fixtures,
        );
        let display = format!("{}", report);
        assert!(display.contains("BPP3D Run Report"));
        assert!(display.contains("display-test"));
        assert!(display.contains("fix-1"));
    }

    #[test]
    fn run_report_failure_reports() {
        let fixtures = vec![
            Bpp3dFixtureReport::success("ok", 50),
            Bpp3dFixtureReport::failed(
                "fail",
                100,
                Bpp3dSolverFailure::error(Bpp3dErrorCategory::SolveError, "boom"),
            ),
        ];
        let report = Bpp3dRunReport::new(
            "fail-test",
            "gurobi",
            None,
            "2026-06-15",
            Duration::from_millis(150),
            fixtures,
        );
        let failures = report.failure_reports();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].name, "fail");
    }

    #[test]
    fn run_report_compare_detects_regressions() {
        let mut baseline_fixture = Bpp3dFixtureReport::success("fixture-a", 100);
        baseline_fixture.layer_count = 2;
        baseline_fixture.selected_layer_count = 1;
        baseline_fixture.packed_bin_count = 1;
        baseline_fixture.render_plan_count = 1;
        let mut candidate_fixture = Bpp3dFixtureReport::failed(
            "fixture-a",
            120,
            Bpp3dSolverFailure::error(Bpp3dErrorCategory::SolveError, "boom"),
        );
        candidate_fixture.layer_count = 1;
        let baseline = Bpp3dRunReport::new(
            "baseline-suite",
            "fake",
            None,
            "2026-06-15",
            Duration::from_millis(100),
            vec![baseline_fixture],
        );
        let candidate = Bpp3dRunReport::new(
            "candidate-suite",
            "fake",
            None,
            "2026-06-15",
            Duration::from_millis(120),
            vec![candidate_fixture],
        );

        let comparison = baseline.compare_with(&candidate);

        assert!(comparison.has_regression());
        assert!(comparison.regression_count() >= 2);
        assert!(comparison
            .differences
            .iter()
            .any(|difference| difference.field == "fixture.status"));
        assert!(comparison.summary_string().contains("regressions="));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn run_report_json_file_roundtrip() {
        let report = Bpp3dRunReport::new(
            "file-suite",
            "fake",
            Some("serde".to_string()),
            "2026-06-15",
            Duration::from_millis(42),
            vec![Bpp3dFixtureReport::success("file-fixture", 42)],
        );
        let path = std::env::temp_dir().join(format!(
            "bpp3d-run-report-{}-{}.json",
            std::process::id(),
            report.total_duration_ms,
        ));

        report.write_json_file(&path).unwrap();
        let loaded = Bpp3dRunReport::read_json_file(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.suite_name, report.suite_name);
        assert_eq!(loaded.backend, report.backend);
        assert_eq!(loaded.fixtures[0].name, "file-fixture");
    }

    #[test]
    fn fixture_filter_all() {
        let filter = FixtureFilter::all();
        assert!(filter.matches("any", &None, &[], false));
        assert!(filter.matches("any", &Some("g".to_string()), &["t".to_string()], true));
    }

    #[test]
    fn fixture_filter_by_group() {
        let filter = FixtureFilter::all().with_group("kotlin-gurobi");
        assert!(filter.matches("f1", &Some("kotlin-gurobi".to_string()), &[], false));
        assert!(!filter.matches("f2", &Some("rust-regression".to_string()), &[], false));
        assert!(!filter.matches("f3", &None, &[], false));
    }

    #[test]
    fn fixture_filter_by_tags() {
        let filter = FixtureFilter::all().with_tags(vec!["pwl".to_string(), "cylinder".to_string()]);
        assert!(filter.matches("f1", &None, &["pwl".to_string()], false));
        assert!(filter.matches("f2", &None, &["cylinder".to_string()], false));
        assert!(!filter.matches("f3", &None, &["cuboid".to_string()], false));
    }

    #[test]
    fn fixture_filter_backend_smoke() {
        let smoke_only = FixtureFilter::all().backend_smoke_only();
        assert!(smoke_only.matches("f1", &None, &[], true));
        assert!(!smoke_only.matches("f2", &None, &[], false));

        let exclude_smoke = FixtureFilter::all().exclude_backend_smoke();
        assert!(!exclude_smoke.matches("f1", &None, &[], true));
        assert!(exclude_smoke.matches("f2", &None, &[], false));
    }

    #[test]
    fn fixture_filter_name_prefix() {
        let filter = FixtureFilter::all().with_name_prefix("kotlin-");
        assert!(filter.matches("kotlin-gurobi-sample", &None, &[], false));
        assert!(!filter.matches("rust-regression", &None, &[], false));
    }

    #[test]
    fn fixture_filter_combined() {
        let filter = FixtureFilter::all()
            .with_group("kotlin-gurobi")
            .with_tags(vec!["pwl".to_string()]);
        assert!(filter.matches(
            "kotlin-pwl-sample",
            &Some("kotlin-gurobi".to_string()),
            &["pwl".to_string()],
            false,
        ));
        // Wrong group
        assert!(!filter.matches(
            "kotlin-pwl-sample",
            &Some("rust-regression".to_string()),
            &["pwl".to_string()],
            false,
        ));
        // Wrong tag
        assert!(!filter.matches(
            "kotlin-cylinder-sample",
            &Some("kotlin-gurobi".to_string()),
            &["cylinder".to_string()],
            false,
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn run_report_json_roundtrip() {
        let fixtures = vec![
            Bpp3dFixtureReport::success("json-test", 42),
        ];
        let report = Bpp3dRunReport::new(
            "json-suite",
            "fake",
            None,
            "2026-06-15",
            Duration::from_millis(42),
            fixtures,
        );
        let json = report.to_json().unwrap();
        let deserialized = Bpp3dRunReport::from_json(&json).unwrap();
        assert_eq!(deserialized.suite_name, "json-suite");
        assert_eq!(deserialized.fixtures.len(), 1);
        assert_eq!(deserialized.fixtures[0].name, "json-test");
    }
}
