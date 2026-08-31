// ============================================================================
// Bpp3dRunReport - 顶层运行报告 / Top-level run report
// ============================================================================

/// 顶层运行报告 / Top-level run report
///
/// 统一 fake / no-run / 真实 solver backend 的运行报告。可序列化为 JSON，
/// 便于跨环境比较和回归追踪。
/// Unified run report for fake / no-run / real solver backends. Serializable
/// to JSON for cross-environment comparison and regression tracking.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dRunReport {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// backend 名称 / Backend name
    pub backend: String,
    /// feature 名称 / Feature name
    pub feature: Option<String>,
    /// 时间戳 / Timestamp
    pub timestamp: String,
    /// 总耗时（毫秒）/ Total duration (milliseconds)
    pub total_duration_ms: u64,
    /// 求解器可用性 / Solver availability
    pub solver_availability: Option<Bpp3dSolverAvailability>,
    /// fixture 报告 / Fixture reports
    pub fixtures: Vec<Bpp3dFixtureReport>,
    /// 汇总 / Summary
    pub summary: Bpp3dSuiteSummary,
}

impl Bpp3dRunReport {
    /// 创建运行报告 / Create run report
    pub fn new(
        suite_name: impl Into<String>,
        backend: impl Into<String>,
        feature: Option<String>,
        timestamp: impl Into<String>,
        total_duration: Duration,
        fixtures: Vec<Bpp3dFixtureReport>,
    ) -> Self {
        let summary = Bpp3dSuiteSummary::from_reports(&fixtures);
        Self {
            suite_name: suite_name.into(),
            backend: backend.into(),
            feature,
            timestamp: timestamp.into(),
            total_duration_ms: total_duration.as_millis() as u64,
            solver_availability: None,
            fixtures,
            summary,
        }
    }

    /// 设置求解器可用性 / Set solver availability
    pub fn with_availability(mut self, availability: Bpp3dSolverAvailability) -> Self {
        self.solver_availability = Some(availability);
        self
    }

    /// 是否全部通过 / Whether all fixtures passed
    pub fn all_passed(&self) -> bool {
        self.summary.all_passed()
    }

    /// 摘要字符串 / Summary string
    pub fn summary_string(&self) -> String {
        format!(
            "[{}] {} backend={} feature={} duration={}ms {}",
            self.timestamp,
            self.suite_name,
            self.backend,
            self.feature.as_deref().unwrap_or("none"),
            self.total_duration_ms,
            self.summary.summary_string(),
        )
    }

    /// 序列化为 JSON / Serialize to JSON
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 序列化为紧凑 JSON / Serialize to compact JSON
    #[cfg(feature = "serde")]
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从 JSON 反序列化 / Deserialize from JSON
    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 写入 JSON 文件 / Write JSON file
    #[cfg(feature = "serde")]
    pub fn write_json_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Bpp3dRunReportIoError> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// 从 JSON 文件读取 / Read from JSON file
    #[cfg(feature = "serde")]
    pub fn read_json_file(path: impl AsRef<Path>) -> Result<Self, Bpp3dRunReportIoError> {
        let json = std::fs::read_to_string(path)?;
        Ok(Self::from_json(&json)?)
    }

    /// 生成失败摘要 / Generate failure summary
    pub fn failure_reports(&self) -> Vec<&Bpp3dFixtureReport> {
        self.fixtures
            .iter()
            .filter(|r| r.status.is_failed())
            .collect()
    }

    /// 与候选报告比较 / Compare with candidate report
    ///
    /// 将 `self` 作为基线报告，`candidate` 作为新运行报告。关键成功状态和
    /// 输出计数下降会标记为 regression；backend/feature/耗时差异只作为
    /// info 或 warning，便于跨环境比较。
    /// Treats `self` as the baseline report and `candidate` as the new run.
    /// Critical status changes and output count drops are marked as regressions;
    /// backend/feature/duration differences are informational or warnings for
    /// cross-environment comparison.
    pub fn compare_with(&self, candidate: &Self) -> Bpp3dRunReportComparison {
        let mut differences = Vec::new();
        if self.backend != candidate.backend {
            differences.push(Bpp3dRunReportDifference::new(
                Bpp3dRunReportDifferenceSeverity::Info,
                "backend",
                None,
                Some(self.backend.clone()),
                Some(candidate.backend.clone()),
                "backend changed between reports",
            ));
        }
        if self.feature != candidate.feature {
            differences.push(Bpp3dRunReportDifference::new(
                Bpp3dRunReportDifferenceSeverity::Info,
                "feature",
                None,
                self.feature.clone(),
                candidate.feature.clone(),
                "feature changed between reports",
            ));
        }
        compare_summary_counts(self, candidate, &mut differences);
        let baseline_by_name = self
            .fixtures
            .iter()
            .map(|fixture| (fixture.name.as_str(), fixture))
            .collect::<BTreeMap<_, _>>();
        let candidate_by_name = candidate
            .fixtures
            .iter()
            .map(|fixture| (fixture.name.as_str(), fixture))
            .collect::<BTreeMap<_, _>>();
        let names = baseline_by_name
            .keys()
            .chain(candidate_by_name.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        for name in names {
            match (baseline_by_name.get(name), candidate_by_name.get(name)) {
                (Some(baseline), Some(candidate)) => {
                    compare_fixture_reports(baseline, candidate, &mut differences);
                }
                (Some(_), None) => differences.push(Bpp3dRunReportDifference::new(
                    Bpp3dRunReportDifferenceSeverity::Regression,
                    "fixtures",
                    Some(name.to_string()),
                    Some("present".to_string()),
                    None,
                    "fixture missing from candidate report",
                )),
                (None, Some(_)) => differences.push(Bpp3dRunReportDifference::new(
                    Bpp3dRunReportDifferenceSeverity::Info,
                    "fixtures",
                    Some(name.to_string()),
                    None,
                    Some("present".to_string()),
                    "fixture added in candidate report",
                )),
                (None, None) => {}
            }
        }
        Bpp3dRunReportComparison {
            baseline_suite_name: self.suite_name.clone(),
            candidate_suite_name: candidate.suite_name.clone(),
            baseline_backend: self.backend.clone(),
            candidate_backend: candidate.backend.clone(),
            baseline_fixture_count: self.fixtures.len(),
            candidate_fixture_count: candidate.fixtures.len(),
            differences,
        }
    }
}

fn compare_summary_counts(
    baseline: &Bpp3dRunReport,
    candidate: &Bpp3dRunReport,
    differences: &mut Vec<Bpp3dRunReportDifference>,
) {
    compare_usize_drop(
        "summary.succeeded",
        None,
        baseline.summary.succeeded,
        candidate.summary.succeeded,
        differences,
    );
    compare_usize_increase_regression(
        "summary.failed",
        None,
        baseline.summary.failed,
        candidate.summary.failed,
        differences,
    );
    compare_usize_drop(
        "summary.total_render_plans",
        None,
        baseline.summary.total_render_plans,
        candidate.summary.total_render_plans,
        differences,
    );
    compare_usize_drop(
        "summary.total_packed_bins",
        None,
        baseline.summary.total_packed_bins,
        candidate.summary.total_packed_bins,
        differences,
    );
    compare_usize_drop(
        "summary.total_selected_layers",
        None,
        baseline.summary.total_selected_layers,
        candidate.summary.total_selected_layers,
        differences,
    );
}

fn compare_fixture_reports(
    baseline: &Bpp3dFixtureReport,
    candidate: &Bpp3dFixtureReport,
    differences: &mut Vec<Bpp3dRunReportDifference>,
) {
    let fixture = Some(baseline.name.clone());
    if baseline.status != candidate.status {
        let severity = match (&baseline.status, &candidate.status) {
            (Bpp3dFixtureStatus::Success, Bpp3dFixtureStatus::Failed)
            | (Bpp3dFixtureStatus::Success, Bpp3dFixtureStatus::Skipped) => {
                Bpp3dRunReportDifferenceSeverity::Regression
            }
            (Bpp3dFixtureStatus::Skipped, Bpp3dFixtureStatus::Failed) => {
                Bpp3dRunReportDifferenceSeverity::Warning
            }
            _ => Bpp3dRunReportDifferenceSeverity::Info,
        };
        differences.push(Bpp3dRunReportDifference::new(
            severity,
            "fixture.status",
            fixture.clone(),
            Some(baseline.status.to_string()),
            Some(candidate.status.to_string()),
            "fixture status changed",
        ));
    }
    compare_usize_drop(
        "fixture.layer_count",
        fixture.clone(),
        baseline.layer_count,
        candidate.layer_count,
        differences,
    );
    compare_usize_drop(
        "fixture.selected_layer_count",
        fixture.clone(),
        baseline.selected_layer_count,
        candidate.selected_layer_count,
        differences,
    );
    compare_usize_drop(
        "fixture.packed_bin_count",
        fixture.clone(),
        baseline.packed_bin_count,
        candidate.packed_bin_count,
        differences,
    );
    compare_usize_drop(
        "fixture.render_plan_count",
        fixture,
        baseline.render_plan_count,
        candidate.render_plan_count,
        differences,
    );
}

fn compare_usize_drop(
    field: &str,
    fixture: Option<String>,
    baseline: usize,
    candidate: usize,
    differences: &mut Vec<Bpp3dRunReportDifference>,
) {
    if candidate < baseline {
        differences.push(Bpp3dRunReportDifference::new(
            Bpp3dRunReportDifferenceSeverity::Regression,
            field,
            fixture,
            Some(baseline.to_string()),
            Some(candidate.to_string()),
            "candidate value dropped below baseline",
        ));
    } else if candidate > baseline {
        differences.push(Bpp3dRunReportDifference::new(
            Bpp3dRunReportDifferenceSeverity::Info,
            field,
            fixture,
            Some(baseline.to_string()),
            Some(candidate.to_string()),
            "candidate value increased from baseline",
        ));
    }
}

fn compare_usize_increase_regression(
    field: &str,
    fixture: Option<String>,
    baseline: usize,
    candidate: usize,
    differences: &mut Vec<Bpp3dRunReportDifference>,
) {
    if candidate > baseline {
        differences.push(Bpp3dRunReportDifference::new(
            Bpp3dRunReportDifferenceSeverity::Regression,
            field,
            fixture,
            Some(baseline.to_string()),
            Some(candidate.to_string()),
            "candidate failure count increased above baseline",
        ));
    } else if candidate < baseline {
        differences.push(Bpp3dRunReportDifference::new(
            Bpp3dRunReportDifferenceSeverity::Info,
            field,
            fixture,
            Some(baseline.to_string()),
            Some(candidate.to_string()),
            "candidate failure count decreased from baseline",
        ));
    }
}

impl std::fmt::Display for Bpp3dRunReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== BPP3D Run Report ===")?;
        writeln!(f, "Suite: {}", self.suite_name)?;
        writeln!(f, "Backend: {}", self.backend)?;
        if let Some(feature) = &self.feature {
            writeln!(f, "Feature: {}", feature)?;
        }
        writeln!(f, "Timestamp: {}", self.timestamp)?;
        writeln!(f, "Duration: {}ms", self.total_duration_ms)?;
        if let Some(avail) = &self.solver_availability {
            writeln!(f, "Solver: {}", avail)?;
        }
        writeln!(f, "---")?;
        for fixture in &self.fixtures {
            writeln!(
                f,
                "  [{}] {} ({}ms) layers={} selected={} bins={} render={}",
                fixture.status,
                fixture.name,
                fixture.duration_ms,
                fixture.layer_count,
                fixture.selected_layer_count,
                fixture.packed_bin_count,
                fixture.render_plan_count,
            )?;
            if let Some(failure) = &fixture.failure {
                writeln!(f, "    FAILURE: {}", failure.summary())?;
            }
        }
        writeln!(f, "---")?;
        writeln!(f, "Summary: {}", self.summary.summary_string())?;
        Ok(())
    }
}

