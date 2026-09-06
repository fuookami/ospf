/// 求解器数据集 suite / Solver dataset suite
#[derive(Debug, Clone)]
pub struct SolverDatasetSuite {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// dataset 名称 / Dataset names
    pub datasets: Vec<String>,
}

impl SolverDatasetSuite {
    /// 创建 suite / Create suite
    pub fn new(suite_name: impl Into<String>, datasets: Vec<String>) -> Self {
        Self {
            suite_name: suite_name.into(),
            datasets,
        }
    }

    /// 运行 fake/no-run 验收 / Run fake/no-run validation
    pub fn run_no_run(
        &self,
        backend: impl Into<String>,
        feature: impl Into<String>,
    ) -> SolverDatasetSuiteDiagnostics {
        let backend = backend.into();
        let feature = feature.into();
        SolverDatasetSuiteDiagnostics {
            suite_name: self.suite_name.clone(),
            dataset_count: self.datasets.len(),
            backend: backend.clone(),
            feature: feature.clone(),
            diagnostics: {
                let mut diagnostics = vec![format!(
                "solver dataset suite '{}' validated in no-run mode for backend '{}' feature '{}'",
                self.suite_name,
                backend,
                feature,
                )];
                diagnostics.extend(backend_feature_diagnostics(&backend, &feature));
                diagnostics
            },
        }
    }
}

/// 求解器数据集 fixture / Solver dataset fixture
#[cfg(feature = "serde")]
#[derive(Debug, Clone)]
pub struct SolverDatasetFixture {
    /// dataset 名称 / Dataset name
    pub name: String,
    /// CSV 内容 / CSV content
    pub csv: String,
    /// fixture 分组 / Fixture group
    pub group: Option<String>,
    /// 标签 / Tags
    pub tags: Vec<String>,
    /// 是否期望 backend smoke / Whether backend smoke is expected
    pub backend_smoke: bool,
}

/// 求解器数据集 fixture manifest 条目 / Solver dataset fixture manifest entry
#[cfg(feature = "serde")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SolverDatasetFixtureManifestEntry {
    /// dataset 名称 / Dataset name
    pub name: String,
    /// fixture 路径 / Fixture path
    pub path: String,
    /// fixture 分组 / Fixture group
    #[serde(default)]
    pub group: Option<String>,
    /// 标签 / Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// 是否期望 backend smoke / Whether backend smoke is expected
    #[serde(default)]
    pub backend_smoke: bool,
}

/// 求解器数据集 fixture manifest / Solver dataset fixture manifest
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct SolverDatasetFixtureManifest {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// 条目列表 / Entries
    pub entries: Vec<SolverDatasetFixtureManifestEntry>,
}

#[cfg(feature = "serde")]
impl SolverDatasetFixtureManifest {
    /// 从 JSON 字符串读取 manifest / Read manifest from JSON string
    pub fn from_json_str(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }
}

/// 求解器数据集 fixture 运行结果 / Solver dataset fixture run result
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default)]
pub struct SolverDatasetFixtureRunResult {
    /// dataset 名称 / Dataset name
    pub dataset_name: String,
    /// fixture 分组 / Fixture group
    pub group: Option<String>,
    /// 标签 / Tags
    pub tags: Vec<String>,
    /// 是否期望 backend smoke / Whether backend smoke is expected
    pub backend_smoke: bool,
    /// 是否加载成功 / Whether loading succeeded
    pub loaded: bool,
    /// 是否物化成功 / Whether materialization succeeded
    pub materialized: bool,
    /// 是否完成 no-run 验收 / Whether no-run validation passed
    pub no_run_validated: bool,
    /// 是否执行成功 / Whether execution succeeded
    pub executed: bool,
    /// backend 名称 / Backend name
    pub backend: Option<String>,
    /// feature 名称 / Feature name
    pub feature: Option<String>,
    /// 渲染方案数量 / Render plan count
    pub render_plan_count: usize,
    /// 候选层数量 / Layer candidate count
    pub layer_count: usize,
    /// 选中层数量 / Selected layer count
    pub selected_layer_count: usize,
    /// 已装箱箱数量 / Packed bin count
    pub packed_bin_count: usize,
    /// RMP 目标值 / RMP objective value
    pub rmp_objective: Option<f64>,
    /// 最终目标值 / Final objective value
    pub final_objective: Option<f64>,
    /// 选中层明细 / Selected layer details
    pub selected_layers: Vec<super::report::Bpp3dSelectedLayerReport>,
    /// 已装箱箱明细 / Packed bin details
    pub packed_bins: Vec<super::report::Bpp3dPackedBinReport>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// 求解器数据集 suite 运行结果 / Solver dataset suite run result
#[cfg(feature = "serde")]
#[derive(Debug, Clone, Default)]
pub struct SolverDatasetSuiteRunResult {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// backend 名称 / Backend name
    pub backend: Option<String>,
    /// feature 名称 / Feature name
    pub feature: Option<String>,
    /// fixture 结果 / Fixture results
    pub fixture_results: Vec<SolverDatasetFixtureRunResult>,
    /// fixture 总数 / Fixture count
    pub fixture_count: usize,
    /// 已加载数量 / Loaded count
    pub loaded_count: usize,
    /// 已物化数量 / Materialized count
    pub materialized_count: usize,
    /// no-run 验收数量 / No-run validated count
    pub no_run_count: usize,
    /// 执行数量 / Executed count
    pub executed_count: usize,
    /// backend smoke 期望数量 / Backend smoke expected count
    pub backend_smoke_count: usize,
    /// 渲染方案总数 / Render plan total
    pub render_plan_total: usize,
    /// 诊断总数 / Diagnostic count
    pub diagnostic_count: usize,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

