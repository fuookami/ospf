    #[test]
    fn column_generation_config_builder() {
        let config = ColumnGenerationConfig::new()
            .with_max_iterations(8)
            .with_max_column_amount(32)
            .with_max_candidates_per_iteration(4);

        assert_eq!(config.max_iterations, 8);
        assert_eq!(config.max_column_amount, 32);
        assert_eq!(config.max_candidates_per_iteration, 4);
    }

    #[test]
    fn column_generation_state_tracks_improvement() {
        let config = ColumnGenerationConfig::new()
            .with_max_not_better_iterations(1);
        let mut state = ColumnGenerationState::new();

        assert!(state.observe_objective(10.0, ObjectiveSense::Minimize, 1e-6));
        assert!(!state.observe_objective(10.5, ObjectiveSense::Minimize, 1e-6));
        assert!(!state.should_continue(&config));
        assert_eq!(state.status, ColumnGenerationStatus::Converged);
    }

    #[test]
    fn solver_dataset_suite_reports_no_run_diagnostics() {
        let suite = SolverDatasetSuite::new(
            "bpp3d-smoke",
            vec!["cuboid.csv".to_string(), "cylinder.csv".to_string()],
        );

        let diagnostics = suite.run_no_run("fake", "scip");

        assert_eq!(diagnostics.suite_name, "bpp3d-smoke");
        assert_eq!(diagnostics.dataset_count, 2);
        assert_eq!(diagnostics.backend, "fake");
        assert_eq!(diagnostics.feature, "scip");
        assert!(diagnostics.diagnostics[0].contains("no-run mode"));
    }

