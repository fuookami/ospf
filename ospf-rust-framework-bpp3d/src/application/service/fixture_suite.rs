/// 求解器数据集 fixture suite / Solver dataset fixture suite
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default)]
pub struct SolverDatasetFixtureSuite {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// fixture 列表 / Fixtures
    pub fixtures: Vec<SolverDatasetFixture>,
}

#[cfg(feature = "serde")]
impl SolverDatasetFixtureSuite {
    /// 创建 fixture suite / Create fixture suite
    pub fn new(
        suite_name: impl Into<String>,
        fixtures: Vec<SolverDatasetFixture>,
    ) -> Self {
        Self {
            suite_name: suite_name.into(),
            fixtures,
        }
    }

    /// 从 manifest 创建 fixture suite / Create fixture suite from manifest
    pub fn from_manifest(
        root: impl AsRef<Path>,
        manifest: SolverDatasetFixtureManifest,
    ) -> Result<Self, std::io::Error> {
        let root = root.as_ref();
        let fixtures = manifest
            .entries
            .iter()
            .map(|entry| {
                std::fs::read_to_string(root.join(&entry.path)).map(|csv| SolverDatasetFixture {
                    name: entry.name.clone(),
                    csv,
                    group: entry.group.clone(),
                    tags: entry.tags.clone(),
                    backend_smoke: entry.backend_smoke,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(manifest.suite_name, fixtures))
    }

    /// 从 manifest JSON 文件创建 fixture suite / Create fixture suite from manifest JSON file
    pub fn from_manifest_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let manifest = SolverDatasetFixtureManifest::from_json_str(&content)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let root = path.parent().unwrap_or_else(|| Path::new("."));
        Self::from_manifest(root, manifest)
    }

    /// 从目录创建 fixture suite / Create fixture suite from directory
    pub fn from_directory(
        suite_name: impl Into<String>,
        root: impl AsRef<Path>,
    ) -> Result<Self, std::io::Error> {
        let root = root.as_ref();
        let mut fixtures = Vec::new();
        collect_csv_fixtures(root, root, &mut fixtures)?;
        fixtures.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        Ok(Self::new(suite_name, fixtures))
    }

    /// 创建 smoke fixture suite / Create smoke fixture suite
    pub fn smoke() -> Self {
        Self::new(
            "bpp3d-smoke",
            vec![
                SolverDatasetFixture {
                    name: "cuboid-smoke.csv".to_string(),
                    group: Some("smoke".to_string()),
                    tags: vec!["cuboid".to_string(), "patterned-item".to_string()],
                    backend_smoke: true,
                    csv: r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,pattern_code,allow_mixed_loading,max_stack_layers,package_tags
i0,Item,cuboid,2,2,1,1,1,PAT-A,false,2,fragile|top
# table:bins
bin_id,type_code,width,height,depth,capacity
b0,BIN,5,5,5,100
# table:layers
layer_id,bin_id,depth
l0,b0,1
"#
                    .to_string(),
                },
                SolverDatasetFixture {
                    name: "vertical-cylinder-smoke.csv".to_string(),
                    group: Some("smoke".to_string()),
                    tags: vec!["cylinder".to_string(), "axis-y".to_string()],
                    backend_smoke: true,
                    csv: r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius,axis
c0,Cylinder,cylinder,2,2,2,1,1,1,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b0,BIN,5,5,5,100
# table:layers
layer_id,bin_id,depth
l0,b0,2
"#
                    .to_string(),
                },
                SolverDatasetFixture {
                    name: "pwl-radius-smoke.csv".to_string(),
                    group: Some("smoke".to_string()),
                    tags: vec!["cylinder".to_string(), "pwl-radius".to_string()],
                    backend_smoke: true,
                    csv: r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius_min,radius_max,radius_step,axis
p0,PwlCylinder,cylinder,6,5,6,1,1,1,3,1,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b0,BIN,10,10,10,100
# table:layers
layer_id,bin_id,depth
l0,b0,5
"#
                    .to_string(),
                },
            ],
        )
    }

    /// 加载 fixtures / Load fixtures
    pub fn load(&self) -> Result<Vec<CsvMaterializedApplicationRequest>, crate::application::csv::CsvDatasetError> {
        self.fixtures
            .iter()
            .map(|fixture| {
                crate::application::csv::CsvDatasetLoader::load_any_str(&fixture.csv)?
                    .materialize()
            })
            .collect()
    }

    /// 运行 no-run 验收 / Run no-run validation
    pub fn run_no_run(
        &self,
        backend: impl Into<String>,
        feature: impl Into<String>,
    ) -> Result<SolverDatasetSuiteDiagnostics, crate::application::csv::CsvDatasetError> {
        let requests = self.load()?;
        let suite = SolverDatasetSuite::new(
            self.suite_name.clone(),
            self.fixtures
                .iter()
                .map(|fixture| fixture.name.clone())
                .collect(),
        );
        let mut diagnostics = suite.run_no_run(backend, feature);
        diagnostics.dataset_count = requests.len();
        diagnostics.diagnostics.push(format!(
            "loaded {} serde CSV fixture(s)",
            requests.len(),
        ));
        Ok(diagnostics)
    }

    /// 运行 feature matrix no-run 验收 / Run feature matrix no-run validation
    pub fn run_feature_matrix_no_run(
        &self,
        backend: impl Into<String>,
        features: Vec<String>,
    ) -> Result<SolverFeatureMatrixDiagnostics, crate::application::csv::CsvDatasetError> {
        let backend = backend.into();
        let dataset_count = self.load()?.len();
        let diagnostics = features
            .iter()
            .flat_map(|feature| {
                let mut diagnostics = vec![format!(
                    "solver dataset suite '{}' validated {} dataset(s) in no-run mode for backend '{}' feature '{}'",
                    self.suite_name,
                    dataset_count,
                    backend,
                    feature,
                )];
                diagnostics.extend(backend_feature_diagnostics(&backend, feature));
                diagnostics
            })
            .collect();
        Ok(SolverFeatureMatrixDiagnostics {
            suite_name: self.suite_name.clone(),
            backend,
            features,
            diagnostics,
        })
    }
    /// 运行批量报告 / Run batch report
    pub fn run_batch_report(
        &self,
        backend: impl Into<String>,
        feature: impl Into<String>,
        execute_fake: bool,
    ) -> SolverDatasetSuiteRunResult {
        let backend = backend.into();
        let feature = feature.into();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "dataset_suite_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "dataset_suite_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });
        let fixture_results = self.fixtures
            .iter()
            .map(|fixture| {
                let request = match crate::application::csv::CsvDatasetLoader::load_any_str(&fixture.csv)
                    .and_then(|dataset| dataset.materialize())
                {
                    Ok(request) => request,
                    Err(error) => {
                        return SolverDatasetFixtureRunResult {
                            dataset_name: fixture.name.clone(),
                            group: fixture.group.clone(),
                            tags: fixture.tags.clone(),
                            backend_smoke: fixture.backend_smoke,
                            backend: Some(backend.clone()),
                            feature: Some(feature.clone()),
                            diagnostics: vec![error.to_string()],
                            ..Default::default()
                        };
                    }
                };
                let mut diagnostics = request.validate_business_rules();
                if !execute_fake {
                    diagnostics.push(format!(
                        "fixture '{}' validated in no-run mode for backend '{}' feature '{}'",
                        fixture.name,
                        backend,
                        feature,
                    ));
                    return SolverDatasetFixtureRunResult {
                        dataset_name: fixture.name.clone(),
                        group: fixture.group.clone(),
                        tags: fixture.tags.clone(),
                        backend_smoke: fixture.backend_smoke,
                        loaded: true,
                        materialized: true,
                        no_run_validated: true,
                        backend: Some(backend.clone()),
                        feature: Some(feature.clone()),
                        diagnostics,
                        ..Default::default()
                    };
                }
                match service.run_csv_materialized_with_bins(request, &rmp, &final_executor) {
                    Ok(flow) => {
                        diagnostics.extend(
                            flow.final_execution
                                .info
                                .values()
                                .cloned(),
                        );
                        SolverDatasetFixtureRunResult {
                            dataset_name: fixture.name.clone(),
                            group: fixture.group.clone(),
                            tags: fixture.tags.clone(),
                            backend_smoke: fixture.backend_smoke,
                            loaded: true,
                            materialized: true,
                            no_run_validated: true,
                            executed: true,
                            backend: Some(backend.clone()),
                            feature: Some(feature.clone()),
                            render_plan_count: flow.result.render_loading_plans.len(),
                            layer_count: flow.result.layers.len(),
                            selected_layer_count: flow.final_execution.layers.len(),
                            packed_bin_count: flow.final_execution.packed_bins.len(),
                            rmp_objective: flow.rmp.objective,
                            final_objective: flow.final_execution.objective,
                            selected_layers: selected_layer_reports(&flow.final_execution.layers),
                            packed_bins: packed_bin_reports(&flow.final_execution.packed_bins),
                            diagnostics,
                        }
                    }
                    Err(errors) => SolverDatasetFixtureRunResult {
                        dataset_name: fixture.name.clone(),
                        group: fixture.group.clone(),
                        tags: fixture.tags.clone(),
                        backend_smoke: fixture.backend_smoke,
                        loaded: true,
                        materialized: true,
                        no_run_validated: true,
                        backend: Some(backend.clone()),
                        feature: Some(feature.clone()),
                        diagnostics: errors,
                        ..Default::default()
                    },
                }
            })
            .collect::<Vec<_>>();
        let fixture_count = fixture_results.len();
        let loaded_count = fixture_results.iter().filter(|result| result.loaded).count();
        let materialized_count = fixture_results.iter().filter(|result| result.materialized).count();
        let no_run_count = fixture_results.iter().filter(|result| result.no_run_validated).count();
        let executed_count = fixture_results.iter().filter(|result| result.executed).count();
        let backend_smoke_count = fixture_results.iter().filter(|result| result.backend_smoke).count();
        let render_plan_total = fixture_results
            .iter()
            .map(|result| result.render_plan_count)
            .sum();
        let diagnostic_count = fixture_results
            .iter()
            .map(|result| result.diagnostics.len())
            .sum();
        let diagnostics = vec![format!(
            "solver dataset suite '{}' produced batch report for {} fixture(s) with backend '{}' feature '{}': loaded={}, materialized={}, no_run={}, executed={}, backend_smoke={}, render_plans={}, diagnostics={}",
            self.suite_name,
            fixture_count,
            backend,
            feature,
            loaded_count,
            materialized_count,
            no_run_count,
            executed_count,
            backend_smoke_count,
            render_plan_total,
            diagnostic_count,
        )];
        SolverDatasetSuiteRunResult {
            suite_name: self.suite_name.clone(),
            backend: Some(backend),
            feature: Some(feature),
            fixture_results,
            fixture_count,
            loaded_count,
            materialized_count,
            no_run_count,
            executed_count,
            backend_smoke_count,
            render_plan_total,
            diagnostic_count,
            diagnostics,
        }
    }

    /// 运行 fake backend one-round suite / Run fake backend one-round suite
    pub fn run_fake_one_round(
        &self,
    ) -> SolverDatasetSuiteRunResult {
        self.run_batch_report("fake", "serde", true)
    }

    /// 运行 backend survey 报告 / Run backend survey report
    pub fn run_backend_survey(
        &self,
        backend: impl Into<String>,
        feature: impl Into<String>,
        execute_fake: bool,
    ) -> Result<SolverBackendSurveyReport, crate::application::csv::CsvDatasetError> {
        let backend = backend.into();
        let feature = feature.into();
        let no_run = self.run_no_run(backend.clone(), feature.clone())?;
        let batch = self.run_batch_report(backend.clone(), feature.clone(), execute_fake);
        let feature_enabled = solver_feature_enabled(&feature);
        let fallback_required = !feature_enabled || batch.executed_count == 0;
        let mut diagnostics = no_run.diagnostics.clone();
        diagnostics.push(format!(
            "backend survey for suite '{}' collected no-run and batch diagnostics: fixtures={}, backend_smoke={}, executed={}, fallback={}",
            self.suite_name,
            batch.fixture_count,
            batch.backend_smoke_count,
            batch.executed_count,
            execute_fake,
        ));
        Ok(SolverBackendSurveyReport {
            suite_name: self.suite_name.clone(),
            backend,
            feature,
            feature_enabled,
            dataset_count: no_run.dataset_count,
            loaded: true,
            materialized: true,
            no_run_validated: true,
            fallback_executed: execute_fake && batch.executed_count > 0,
            fallback_required,
            backend_smoke_count: batch.backend_smoke_count,
            render_plan_total: batch.render_plan_total,
            diagnostics,
        })
    }

    /// 按过滤条件创建子 suite / Create filtered sub-suite
    ///
    /// 支持按 tag、group、backend smoke 状态选择 fixture，
    /// 便于把重型回归从普通 lib test 中拆出来。
    /// Supports selecting fixtures by tag, group, and backend smoke status,
    /// making it easy to separate heavy regression from regular lib tests.
    pub fn filter(&self, filter: &super::report::FixtureFilter) -> Self {
        let filtered = self
            .fixtures
            .iter()
            .filter(|f| filter.matches(&f.name, &f.group, &f.tags, f.backend_smoke))
            .cloned()
            .collect();
        Self {
            suite_name: format!("{}[filtered]", self.suite_name),
            fixtures: filtered,
        }
    }

    /// 从 batch 运行结果生成结构化报告 / Generate structured run report from batch results
    pub fn to_run_report(
        &self,
        results: &SolverDatasetSuiteRunResult,
        backend: impl Into<String>,
        feature: Option<String>,
        total_duration: std::time::Duration,
    ) -> super::report::Bpp3dRunReport {
        let backend = backend.into();
        let timestamp = format!("{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
        let fixture_reports: Vec<super::report::Bpp3dFixtureReport> = results
            .fixture_results
            .iter()
            .map(|fr| {
                let status = if fr.executed {
                    super::report::Bpp3dFixtureStatus::Success
                } else if fr.no_run_validated {
                    super::report::Bpp3dFixtureStatus::Success
                } else {
                    super::report::Bpp3dFixtureStatus::Failed
                };
                super::report::Bpp3dFixtureReport {
                    name: fr.dataset_name.clone(),
                    group: fr.group.clone(),
                    tags: fr.tags.clone(),
                    status,
                    duration_ms: 0,
                    backend: fr.backend.clone(),
                    feature: fr.feature.clone(),
                    rmp_objective: fr.rmp_objective,
                    final_objective: fr.final_objective,
                    model_status: None,
                    layer_count: fr.layer_count,
                    selected_layer_count: fr.selected_layer_count,
                    packed_bin_count: fr.packed_bin_count,
                    render_plan_count: fr.render_plan_count,
                    selected_layers: fr.selected_layers.clone(),
                    packed_bins: fr.packed_bins.clone(),
                    diagnostics: fr.diagnostics.clone(),
                    failure: None,
                }
            })
            .collect();
        super::report::Bpp3dRunReport::new(
            &self.suite_name,
            &backend,
            feature,
            &timestamp,
            total_duration,
            fixture_reports,
        )
    }

    /// 生成 Kotlin 对比基线报告 / Generate Kotlin comparison baseline report
    ///
    /// 该报告不执行求解，只把 manifest 中的 Kotlin fixture 固化为可比较基线。
    /// This report does not solve; it freezes Kotlin fixtures from the manifest
    /// into a comparable baseline.
    pub fn kotlin_comparison_baseline_report(
        &self,
        backend: impl Into<String>,
        feature: Option<String>,
    ) -> super::report::Bpp3dRunReport {
        let backend = backend.into();
        let filter = super::report::FixtureFilter::all().with_group("kotlin-gurobi");
        let fixtures = self
            .fixtures
            .iter()
            .filter(|fixture| {
                filter.matches(
                    &fixture.name,
                    &fixture.group,
                    &fixture.tags,
                    fixture.backend_smoke,
                )
            })
            .map(|fixture| {
                super::report::Bpp3dFixtureReport::skipped_with_metadata(
                    fixture.name.clone(),
                    fixture.group.clone(),
                    fixture.tags.clone(),
                    Some(backend.clone()),
                    feature.clone(),
                    vec![
                        "kotlin comparison baseline fixture registered from manifest".to_string(),
                    ],
                )
            })
            .collect::<Vec<_>>();
        super::report::Bpp3dRunReport::new(
            format!("{}::kotlin-comparison-baseline", self.suite_name),
            backend,
            feature,
            "manifest-baseline",
            std::time::Duration::default(),
            fixtures,
        )
        .with_availability(super::report::Bpp3dSolverAvailability::NotApplicable)
    }

    /// 写出 Kotlin 对比 artifact / Write Kotlin comparison artifacts
    ///
    /// 写出候选报告、manifest Kotlin 基线报告和比较摘要，供 CI 或本地大规模
    /// solver 回归直接归档。
    /// Writes candidate report, manifest Kotlin baseline report, and comparison
    /// summary for CI or local large-scale solver regression artifacts.
    pub fn write_kotlin_comparison_artifacts(
        &self,
        output_dir: impl AsRef<Path>,
        candidate_report: &super::report::Bpp3dRunReport,
        kotlin_backend: impl Into<String>,
        kotlin_feature: Option<String>,
    ) -> Result<super::report::Bpp3dRunReportComparison, super::report::Bpp3dRunReportIoError> {
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;
        let baseline = self.kotlin_comparison_baseline_report(kotlin_backend, kotlin_feature);
        let comparison = baseline.compare_with(candidate_report);

        baseline.write_json_file(output_dir.join("kotlin-baseline-report.json"))?;
        candidate_report.write_json_file(output_dir.join("rust-candidate-report.json"))?;
        std::fs::write(
            output_dir.join("kotlin-comparison-summary.txt"),
            comparison.summary_string(),
        )?;
        Ok(comparison)
    }

    /// 生成层候选质量报告 / Generate layer candidate quality report
    ///
    /// 该报告只运行 layer generation，不调用 RMP/final solver，用于沉淀
    /// ComplexBlock/DFS/MLHS/Pattern/Pile/Historical 的大规模候选基线。
    /// This report runs layer generation only and does not invoke RMP/final
    /// solver, providing a large-scale candidate baseline for
    /// ComplexBlock/DFS/MLHS/Pattern/Pile/Historical.
    pub fn layer_generation_quality_report(
        &self,
        filter: &super::report::FixtureFilter,
        max_candidates: usize,
    ) -> LayerGenerationSuiteQualityReport {
        let suite_start = Instant::now();
        let config = ColumnGenerationConfig::default()
            .with_max_candidates_per_iteration(max_candidates.max(1));
        let mut fixture_reports = Vec::new();
        for fixture in self.fixtures.iter().filter(|fixture| {
            filter.matches(
                &fixture.name,
                &fixture.group,
                &fixture.tags,
                fixture.backend_smoke,
            )
        }) {
            let fixture_start = Instant::now();
            let mut report = LayerGenerationFixtureQualityReport {
                dataset_name: fixture.name.clone(),
                group: fixture.group.clone(),
                tags: fixture.tags.clone(),
                ..Default::default()
            };
            let request = match crate::application::csv::CsvDatasetLoader::load_any_str(&fixture.csv)
                .and_then(|dataset| dataset.materialize())
            {
                Ok(request) => request,
                Err(error) => {
                    report.diagnostics.push(error.to_string());
                    report.duration_ms = fixture_start.elapsed().as_millis() as u64;
                    fixture_reports.push(report);
                    continue;
                }
            };
            report.loaded = true;
            report.materialized = true;
            report.item_count = request.items.len();
            report.bin_count = request.bins.len();
            let mut context =
                ColumnGenerationStandardExecutors::default_layer_generation_context::<f64, Meter>();
            let mut algorithm = ColumnGenerationAlgorithm::with_layer_generation(
                config.clone(),
                std::mem::take(&mut context),
            );
            let initial_layers = ensure_layer_demand_coverage(
                request.initial_layers.clone(),
                &request.items,
            );
            algorithm.add_initial_layers(initial_layers);
            let mut generation_request =
                LayerGenerationRequest::new(0, request.items.clone())
                    .with_demand_entries(layer_generation_demand_entries(
                        &request.items,
                        &HashMap::new(),
                    ))
                    .with_package_attributes(
                        request
                            .package_attributes
                            .iter()
                            .cloned()
                            .collect::<HashMap<_, _>>(),
                    )
                    .with_max_candidates(max_candidates.max(1));
            generation_request.bin = request.bins.first().cloned();
            generation_request.existing_layers = algorithm.active_layers();
            let generated = algorithm.layer_generation.generate(&generation_request);

            for result in &generated {
                *report
                    .candidates_by_source
                    .entry(result.source.clone())
                    .or_default() += 1;
                report
                    .source_quality
                    .entry(result.source.clone())
                    .or_insert_with(|| LayerGenerationSourceQualityReport {
                        source: result.source.clone(),
                        ..Default::default()
                    })
                    .record(result);
                report.placement_trace_count += result.placement_traces.len();
                report.block_trace_count += result.block_traces.len();
                report.diagnostic_count += result.diagnostics.len();
                report.diagnostics.extend(result.diagnostics.clone());
            }
            report.candidate_count = generated.len();
            report.duration_ms = fixture_start.elapsed().as_millis() as u64;
            fixture_reports.push(report);
        }

        let loaded_count = fixture_reports.iter().filter(|report| report.loaded).count();
        let materialized_count = fixture_reports
            .iter()
            .filter(|report| report.materialized)
            .count();
        let total_candidate_count = fixture_reports
            .iter()
            .map(|report| report.candidate_count)
            .sum();
        let total_placement_trace_count = fixture_reports
            .iter()
            .map(|report| report.placement_trace_count)
            .sum();
        let total_block_trace_count = fixture_reports
            .iter()
            .map(|report| report.block_trace_count)
            .sum();
        let mut source_quality = BTreeMap::<String, LayerGenerationSourceQualityReport>::new();
        for fixture_report in &fixture_reports {
            for source_report in fixture_report.source_quality.values() {
                let aggregate = source_quality
                    .entry(source_report.source.clone())
                    .or_insert_with(|| LayerGenerationSourceQualityReport {
                        source: source_report.source.clone(),
                        ..Default::default()
                    });
                aggregate.candidate_count += source_report.candidate_count;
                aggregate.best_numeric_score = match (
                    aggregate.best_numeric_score,
                    source_report.best_numeric_score,
                ) {
                    (Some(lhs), Some(rhs)) => Some(lhs.max(rhs)),
                    (None, Some(rhs)) => Some(rhs),
                    (lhs, None) => lhs,
                };
                aggregate.total_coverage += source_report.total_coverage;
                aggregate.best_coverage = aggregate.best_coverage.max(source_report.best_coverage);
                aggregate.placement_trace_count += source_report.placement_trace_count;
                aggregate.block_trace_count += source_report.block_trace_count;
                aggregate.diagnostic_count += source_report.diagnostic_count;
            }
        }
        let total_duration_ms = suite_start.elapsed().as_millis() as u64;
        let fixture_count = fixture_reports.len();
        LayerGenerationSuiteQualityReport {
            suite_name: self.suite_name.clone(),
            fixture_count,
            loaded_count,
            materialized_count,
            total_candidate_count,
            total_placement_trace_count,
            total_block_trace_count,
            source_quality,
            total_duration_ms,
            fixtures: fixture_reports,
            diagnostics: vec![format!(
                "layer generation quality report collected {} fixture(s): loaded={}, materialized={}, candidates={}, placement_traces={}, block_traces={}, duration_ms={}",
                fixture_count,
                loaded_count,
                materialized_count,
                total_candidate_count,
                total_placement_trace_count,
                total_block_trace_count,
                total_duration_ms,
            )],
        }
    }
}

#[cfg(feature = "serde")]
fn collect_csv_fixtures(
    root: &Path,
    dir: &Path,
    fixtures: &mut Vec<SolverDatasetFixture>,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_csv_fixtures(root, &path, fixtures)?;
            continue;
        }
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
        {
            continue;
        }
        let name = path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.to_str())
            .map(|value| value.replace('\\', "/"))
            .or_else(|| {
                path.file_name()
                    .and_then(|file_name| file_name.to_str())
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "fixture.csv".to_string());
        let csv = std::fs::read_to_string(&path)?;
        fixtures.push(SolverDatasetFixture {
            name,
            csv,
            group: None,
            tags: Vec::new(),
            backend_smoke: false,
        });
    }
    Ok(())
}
