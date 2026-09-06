// ============================================================================
// Bpp3dRunReportComparison - 运行报告比较 / Run report comparison
// ============================================================================

/// 报告差异严重程度 / Report difference severity
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bpp3dRunReportDifferenceSeverity {
    /// 信息 / Informational
    Info,
    /// 警告 / Warning
    Warning,
    /// 回归 / Regression
    Regression,
}

impl std::fmt::Display for Bpp3dRunReportDifferenceSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Regression => write!(f, "regression"),
        }
    }
}

/// 报告差异 / Report difference
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dRunReportDifference {
    /// 严重程度 / Severity
    pub severity: Bpp3dRunReportDifferenceSeverity,
    /// 字段路径 / Field path
    pub field: String,
    /// fixture 名称 / Fixture name
    pub fixture: Option<String>,
    /// 基线值 / Baseline value
    pub baseline: Option<String>,
    /// 候选值 / Candidate value
    pub candidate: Option<String>,
    /// 差异消息 / Difference message
    pub message: String,
}

impl Bpp3dRunReportDifference {
    /// 创建差异 / Create difference
    pub fn new(
        severity: Bpp3dRunReportDifferenceSeverity,
        field: impl Into<String>,
        fixture: Option<String>,
        baseline: Option<String>,
        candidate: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            field: field.into(),
            fixture,
            baseline,
            candidate,
            message: message.into(),
        }
    }
}

/// 报告比较结果 / Report comparison result
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dRunReportComparison {
    /// 基线 suite 名称 / Baseline suite name
    pub baseline_suite_name: String,
    /// 候选 suite 名称 / Candidate suite name
    pub candidate_suite_name: String,
    /// 基线 backend / Baseline backend
    pub baseline_backend: String,
    /// 候选 backend / Candidate backend
    pub candidate_backend: String,
    /// 基线 fixture 数 / Baseline fixture count
    pub baseline_fixture_count: usize,
    /// 候选 fixture 数 / Candidate fixture count
    pub candidate_fixture_count: usize,
    /// 差异列表 / Differences
    pub differences: Vec<Bpp3dRunReportDifference>,
}

impl Bpp3dRunReportComparison {
    /// 是否没有差异 / Whether there is no difference
    pub fn is_empty(&self) -> bool {
        self.differences.is_empty()
    }

    /// 是否存在回归 / Whether there is any regression
    pub fn has_regression(&self) -> bool {
        self.differences
            .iter()
            .any(|difference| difference.severity == Bpp3dRunReportDifferenceSeverity::Regression)
    }

    /// 回归数量 / Regression count
    pub fn regression_count(&self) -> usize {
        self.differences
            .iter()
            .filter(|difference| difference.severity == Bpp3dRunReportDifferenceSeverity::Regression)
            .count()
    }

    /// 摘要字符串 / Summary string
    pub fn summary_string(&self) -> String {
        let warnings = self
            .differences
            .iter()
            .filter(|difference| difference.severity == Bpp3dRunReportDifferenceSeverity::Warning)
            .count();
        let infos = self
            .differences
            .iter()
            .filter(|difference| difference.severity == Bpp3dRunReportDifferenceSeverity::Info)
            .count();
        format!(
            "baseline={} candidate={} regressions={} warnings={} infos={} differences={}",
            self.baseline_suite_name,
            self.candidate_suite_name,
            self.regression_count(),
            warnings,
            infos,
            self.differences.len(),
        )
    }
}

