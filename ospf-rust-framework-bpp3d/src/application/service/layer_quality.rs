/// 层生成来源质量报告 / Layer-generation source quality report
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct LayerGenerationSourceQualityReport {
    /// 来源生成器 / Source generator
    pub source: String,
    /// 候选数量 / Candidate count
    pub candidate_count: usize,
    /// 最佳数值评分 / Best numeric score
    pub best_numeric_score: Option<f64>,
    /// 总覆盖量 / Total coverage amount
    pub total_coverage: f64,
    /// 最佳覆盖量 / Best coverage amount
    pub best_coverage: f64,
    /// 放置 trace 数量 / Placement trace count
    pub placement_trace_count: usize,
    /// 块 trace 数量 / Block trace count
    pub block_trace_count: usize,
    /// 诊断数量 / Diagnostic count
    pub diagnostic_count: usize,
}

#[cfg(feature = "serde")]
impl LayerGenerationSourceQualityReport {
    /// 记录候选 / Record candidate
    pub fn record<V, U>(&mut self, result: &LayerGenerationResult<V, U>)
    where
        V: Debug + Clone + Send + Sync,
        U: UnitTrait + Debug + Clone + Send + Sync,
    {
        self.source = result.source.clone();
        self.candidate_count += 1;
        if let Some(score) = result.numeric_score {
            self.best_numeric_score = Some(
                self.best_numeric_score
                    .map(|best| best.max(score))
                    .unwrap_or(score),
            );
        }
        let coverage = result
            .layer
            .demand_coverage
            .iter()
            .map(|coverage| coverage.coefficient)
            .sum::<f64>();
        self.total_coverage += coverage;
        self.best_coverage = self.best_coverage.max(coverage);
        self.placement_trace_count += result.placement_traces.len();
        self.block_trace_count += result.block_traces.len();
        self.diagnostic_count += result.diagnostics.len();
    }
}

/// 层生成质量差异级别 / Layer-generation quality difference severity
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum LayerGenerationQualityDifferenceSeverity {
    /// 回归 / Regression
    Regression,
    /// 警告 / Warning
    Warning,
    /// 信息 / Info
    Info,
}

/// 层生成质量差异 / Layer-generation quality difference
#[cfg(feature = "serde")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LayerGenerationQualityDifference {
    /// 级别 / Severity
    pub severity: LayerGenerationQualityDifferenceSeverity,
    /// 字段 / Field
    pub field: String,
    /// fixture 名称 / Fixture name
    pub fixture: Option<String>,
    /// 来源生成器 / Source generator
    pub source: Option<String>,
    /// baseline 值 / Baseline value
    pub baseline: Option<String>,
    /// candidate 值 / Candidate value
    pub candidate: Option<String>,
    /// 说明 / Message
    pub message: String,
}

#[cfg(feature = "serde")]
impl LayerGenerationQualityDifference {
    /// 创建差异 / Create difference
    pub fn new(
        severity: LayerGenerationQualityDifferenceSeverity,
        field: impl Into<String>,
        fixture: Option<String>,
        source: Option<String>,
        baseline: Option<String>,
        candidate: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            field: field.into(),
            fixture,
            source,
            baseline,
            candidate,
            message: message.into(),
        }
    }
}

/// 层生成质量比较 / Layer-generation quality comparison
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct LayerGenerationQualityComparison {
    /// baseline fixture 数量 / Baseline fixture count
    pub baseline_fixture_count: usize,
    /// candidate fixture 数量 / Candidate fixture count
    pub candidate_fixture_count: usize,
    /// 差异列表 / Differences
    pub differences: Vec<LayerGenerationQualityDifference>,
}

#[cfg(feature = "serde")]
impl LayerGenerationQualityComparison {
    /// 是否存在回归 / Whether any regression exists
    pub fn has_regression(&self) -> bool {
        self.differences
            .iter()
            .any(|difference| difference.severity == LayerGenerationQualityDifferenceSeverity::Regression)
    }

    /// 回归数量 / Regression count
    pub fn regression_count(&self) -> usize {
        self.differences
            .iter()
            .filter(|difference| difference.severity == LayerGenerationQualityDifferenceSeverity::Regression)
            .count()
    }

    /// 摘要字符串 / Summary string
    pub fn summary_string(&self) -> String {
        format!(
            "layer_quality_comparison: baseline_fixtures={}, candidate_fixtures={}, differences={}, regressions={}",
            self.baseline_fixture_count,
            self.candidate_fixture_count,
            self.differences.len(),
            self.regression_count(),
        )
    }
}

/// 层生成 fixture 质量报告 / Layer-generation fixture quality report
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct LayerGenerationFixtureQualityReport {
    /// dataset 名称 / Dataset name
    pub dataset_name: String,
    /// fixture 分组 / Fixture group
    pub group: Option<String>,
    /// 标签 / Tags
    pub tags: Vec<String>,
    /// 是否加载成功 / Whether loading succeeded
    pub loaded: bool,
    /// 是否物化成功 / Whether materialization succeeded
    pub materialized: bool,
    /// 货物数量 / Item count
    pub item_count: usize,
    /// 箱型数量 / Bin count
    pub bin_count: usize,
    /// 候选数量 / Candidate count
    pub candidate_count: usize,
    /// 按来源统计候选数量 / Candidate count by source
    pub candidates_by_source: HashMap<String, usize>,
    /// 按来源统计质量 / Quality by source
    pub source_quality: BTreeMap<String, LayerGenerationSourceQualityReport>,
    /// 放置 trace 数量 / Placement trace count
    pub placement_trace_count: usize,
    /// 块 trace 数量 / Block trace count
    pub block_trace_count: usize,
    /// 诊断数量 / Diagnostic count
    pub diagnostic_count: usize,
    /// 耗时毫秒 / Duration milliseconds
    pub duration_ms: u64,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// 层生成 suite 质量报告 / Layer-generation suite quality report
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct LayerGenerationSuiteQualityReport {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// fixture 总数 / Fixture count
    pub fixture_count: usize,
    /// 已加载数量 / Loaded count
    pub loaded_count: usize,
    /// 已物化数量 / Materialized count
    pub materialized_count: usize,
    /// 候选总数 / Total candidate count
    pub total_candidate_count: usize,
    /// 放置 trace 总数 / Total placement trace count
    pub total_placement_trace_count: usize,
    /// 块 trace 总数 / Total block trace count
    pub total_block_trace_count: usize,
    /// 按来源统计质量 / Quality by source
    pub source_quality: BTreeMap<String, LayerGenerationSourceQualityReport>,
    /// 总耗时毫秒 / Total duration milliseconds
    pub total_duration_ms: u64,
    /// fixture 报告 / Fixture reports
    pub fixtures: Vec<LayerGenerationFixtureQualityReport>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

#[cfg(feature = "serde")]
impl LayerGenerationSuiteQualityReport {
    /// 序列化为 JSON / Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 从 JSON 反序列化 / Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 写入 JSON 文件 / Write JSON file
    pub fn write_json_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), super::report::Bpp3dRunReportIoError> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// 从 JSON 文件读取 / Read from JSON file
    pub fn read_json_file(
        path: impl AsRef<Path>,
    ) -> Result<Self, super::report::Bpp3dRunReportIoError> {
        let json = std::fs::read_to_string(path)?;
        Ok(Self::from_json(&json)?)
    }

    /// 与 baseline 比较 / Compare with baseline
    pub fn compare_with(
        &self,
        candidate: &Self,
    ) -> LayerGenerationQualityComparison {
        let mut comparison = LayerGenerationQualityComparison {
            baseline_fixture_count: self.fixture_count,
            candidate_fixture_count: candidate.fixture_count,
            differences: Vec::new(),
        };
        compare_layer_quality_usize_drop(
            "summary.total_candidate_count",
            None,
            None,
            self.total_candidate_count,
            candidate.total_candidate_count,
            &mut comparison.differences,
        );
        compare_layer_quality_usize_drop(
            "summary.total_placement_trace_count",
            None,
            None,
            self.total_placement_trace_count,
            candidate.total_placement_trace_count,
            &mut comparison.differences,
        );
        compare_layer_quality_usize_drop(
            "summary.total_block_trace_count",
            None,
            None,
            self.total_block_trace_count,
            candidate.total_block_trace_count,
            &mut comparison.differences,
        );
        compare_layer_source_quality(
            None,
            &self.source_quality,
            &candidate.source_quality,
            &mut comparison.differences,
        );

        let candidate_by_fixture = candidate
            .fixtures
            .iter()
            .map(|fixture| (fixture.dataset_name.clone(), fixture))
            .collect::<HashMap<_, _>>();
        for baseline_fixture in &self.fixtures {
            let Some(candidate_fixture) = candidate_by_fixture.get(&baseline_fixture.dataset_name) else {
                comparison.differences.push(LayerGenerationQualityDifference::new(
                    LayerGenerationQualityDifferenceSeverity::Regression,
                    "fixture.missing",
                    Some(baseline_fixture.dataset_name.clone()),
                    None,
                    Some("present".to_string()),
                    Some("missing".to_string()),
                    "candidate is missing a baseline fixture",
                ));
                continue;
            };
            compare_layer_quality_usize_drop(
                "fixture.candidate_count",
                Some(baseline_fixture.dataset_name.clone()),
                None,
                baseline_fixture.candidate_count,
                candidate_fixture.candidate_count,
                &mut comparison.differences,
            );
            compare_layer_source_quality(
                Some(baseline_fixture.dataset_name.clone()),
                &baseline_fixture.source_quality,
                &candidate_fixture.source_quality,
                &mut comparison.differences,
            );
        }
        comparison
    }
}

#[cfg(feature = "serde")]
fn compare_layer_source_quality(
    fixture: Option<String>,
    baseline: &BTreeMap<String, LayerGenerationSourceQualityReport>,
    candidate: &BTreeMap<String, LayerGenerationSourceQualityReport>,
    differences: &mut Vec<LayerGenerationQualityDifference>,
) {
    for (source, baseline_source) in baseline {
        let Some(candidate_source) = candidate.get(source) else {
            differences.push(LayerGenerationQualityDifference::new(
                LayerGenerationQualityDifferenceSeverity::Regression,
                "source.missing",
                fixture.clone(),
                Some(source.clone()),
                Some("present".to_string()),
                Some("missing".to_string()),
                "candidate is missing a baseline source generator",
            ));
            continue;
        };
        compare_layer_quality_usize_drop(
            "source.candidate_count",
            fixture.clone(),
            Some(source.clone()),
            baseline_source.candidate_count,
            candidate_source.candidate_count,
            differences,
        );
        compare_layer_quality_usize_drop(
            "source.placement_trace_count",
            fixture.clone(),
            Some(source.clone()),
            baseline_source.placement_trace_count,
            candidate_source.placement_trace_count,
            differences,
        );
        compare_layer_quality_f64_drop(
            "source.best_coverage",
            fixture.clone(),
            Some(source.clone()),
            baseline_source.best_coverage,
            candidate_source.best_coverage,
            differences,
        );
        if let Some(baseline_score) = baseline_source.best_numeric_score {
            compare_layer_quality_f64_drop(
                "source.best_numeric_score",
                fixture.clone(),
                Some(source.clone()),
                baseline_score,
                candidate_source.best_numeric_score.unwrap_or(f64::NEG_INFINITY),
                differences,
            );
        }
    }
}

#[cfg(feature = "serde")]
fn compare_layer_quality_usize_drop(
    field: &str,
    fixture: Option<String>,
    source: Option<String>,
    baseline: usize,
    candidate: usize,
    differences: &mut Vec<LayerGenerationQualityDifference>,
) {
    if candidate < baseline {
        differences.push(LayerGenerationQualityDifference::new(
            LayerGenerationQualityDifferenceSeverity::Regression,
            field,
            fixture,
            source,
            Some(baseline.to_string()),
            Some(candidate.to_string()),
            "candidate value dropped below baseline",
        ));
    } else if candidate > baseline {
        differences.push(LayerGenerationQualityDifference::new(
            LayerGenerationQualityDifferenceSeverity::Info,
            field,
            fixture,
            source,
            Some(baseline.to_string()),
            Some(candidate.to_string()),
            "candidate value increased from baseline",
        ));
    }
}

#[cfg(feature = "serde")]
fn compare_layer_quality_f64_drop(
    field: &str,
    fixture: Option<String>,
    source: Option<String>,
    baseline: f64,
    candidate: f64,
    differences: &mut Vec<LayerGenerationQualityDifference>,
) {
    const EPS: f64 = 1e-9;
    if candidate + EPS < baseline {
        differences.push(LayerGenerationQualityDifference::new(
            LayerGenerationQualityDifferenceSeverity::Regression,
            field,
            fixture,
            source,
            Some(format!("{baseline:.12}")),
            Some(format!("{candidate:.12}")),
            "candidate value dropped below baseline",
        ));
    } else if candidate > baseline + EPS {
        differences.push(LayerGenerationQualityDifference::new(
            LayerGenerationQualityDifferenceSeverity::Info,
            field,
            fixture,
            source,
            Some(format!("{baseline:.12}")),
            Some(format!("{candidate:.12}")),
            "candidate value increased from baseline",
        ));
    }
}

