//! BPP3D 应用服务 / BPP3D application services

use std::collections::HashMap;
use std::fmt::Debug;
#[cfg(feature = "serde")]
use std::path::Path;
use std::time::{Duration, Instant};

use ospf_rust_math::algebra::Field;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::physical_unit::CTUnit;
use ospf_rust_core::model::meta_model::MetaModel;
#[cfg(not(feature = "async"))]
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};

use crate::domain::item::{
    ActualItem, BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode,
    Bpp3dLayerDemandCoverage,
};
use crate::domain::layer_assignment::{
    BetterLayerMaximization, BinAmountMinimization, BinDepthConstraint, Bpp3dDemandEntry,
    Capacity, DemandConstraint, DemandShadowPriceKey, ImpreciseAssignment, LayerAggregation,
    LayerAssignmentAggregation, LayerAssignmentContext, Load, PreciseAssignment,
    PreciseAssignmentActivationConstraint, SolutionExtractor, VariableArray1, VariableArray2,
};
use crate::domain::layer_generation::{
    BLLocalLayerGenerator, BLGlobalLayerGenerator, BlockLayerGenerator,
    CirclePackingLayerGenerator, HistoricalLayerGenerator, LayerGenerationContext,
    LayerGenerationDemandEntry, LayerBlockTrace, LayerGenerationRequest, LayerGenerationResult,
    LayerPlacementTrace, PatternLayerGenerator, PileLayerGenerator,
};
use crate::domain::packing::{
    LayerTraceReplayAdapter, PackedBin, Packer, PackingGeometryGuard, PackingRendererAdapter,
    PackingResult,
};
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::renderer::RenderLoadingPlanDto;

#[cfg(feature = "serde")]
use crate::application::csv::CsvMaterializedApplicationRequest;

// ============================================================================
// ColumnGenerationConfig - 列生成配置 / Column generation config
// ============================================================================

/// 目标方向 / Objective sense
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectiveSense {
    /// 最小化 / Minimize
    Minimize,
    /// 最大化 / Maximize
    Maximize,
}

impl Default for ObjectiveSense {
    fn default() -> Self {
        Self::Minimize
    }
}

/// 列生成配置 / Column generation config
///
/// 只描述应用编排策略，不持有 solver 私有状态。
/// Describes application orchestration policy only, without solver-private state.
#[derive(Debug, Clone)]
pub struct ColumnGenerationConfig {
    /// 最大迭代次数 / Maximum iterations
    pub max_iterations: usize,
    /// 最大未改进迭代次数 / Maximum non-improving iterations
    pub max_not_better_iterations: usize,
    /// 最大列数量 / Maximum column count
    pub max_column_amount: usize,
    /// 每轮最大候选数 / Maximum candidates per iteration
    pub max_candidates_per_iteration: usize,
    /// 时间限制 / Time limit
    pub time_limit: Duration,
    /// reduced cost 接受阈值 / Reduced-cost acceptance tolerance
    pub reduced_cost_tolerance: f64,
    /// 目标方向 / Objective sense
    pub objective_sense: ObjectiveSense,
}

impl Default for ColumnGenerationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_not_better_iterations: 10,
            max_column_amount: 50000,
            max_candidates_per_iteration: 256,
            time_limit: Duration::from_secs(30000),
            reduced_cost_tolerance: -1e-7,
            objective_sense: ObjectiveSense::Minimize,
        }
    }
}

impl ColumnGenerationConfig {
    /// 创建默认配置 / Create default config
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大迭代次数 / Set maximum iterations
    pub fn with_max_iterations(mut self, value: usize) -> Self {
        self.max_iterations = value;
        self
    }

    /// 设置最大未改进迭代次数 / Set maximum non-improving iterations
    pub fn with_max_not_better_iterations(mut self, value: usize) -> Self {
        self.max_not_better_iterations = value;
        self
    }

    /// 设置最大列数量 / Set maximum column count
    pub fn with_max_column_amount(mut self, value: usize) -> Self {
        self.max_column_amount = value;
        self
    }

    /// 设置每轮最大候选数 / Set maximum candidates per iteration
    pub fn with_max_candidates_per_iteration(mut self, value: usize) -> Self {
        self.max_candidates_per_iteration = value;
        self
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, value: Duration) -> Self {
        self.time_limit = value;
        self
    }
}

// ============================================================================
// ColumnGenerationState - 列生成状态 / Column generation state
// ============================================================================

/// 列生成状态枚举 / Column generation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnGenerationStatus {
    /// 未开始 / Not started
    NotStarted,
    /// 运行中 / Running
    Running,
    /// 收敛 / Converged
    Converged,
    /// 达到迭代限制 / Iteration limit reached
    IterationLimit,
    /// 达到列数量限制 / Column limit reached
    ColumnLimit,
    /// 达到时间限制 / Time limit reached
    TimeLimit,
    /// 失败 / Failed
    Failed,
}

/// 列生成状态 / Column generation state
#[derive(Debug, Clone)]
pub struct ColumnGenerationState {
    /// 当前迭代 / Current iteration
    pub iteration: usize,
    /// 总列数 / Total column count
    pub total_columns: usize,
    /// 未改进迭代次数 / Non-improving iteration count
    pub not_better_iterations: usize,
    /// 最佳目标值 / Best objective
    pub best_objective: Option<f64>,
    /// 状态 / Status
    pub status: ColumnGenerationStatus,
    /// 开始时间 / Start time
    pub started_at: Instant,
}

impl Default for ColumnGenerationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnGenerationState {
    /// 创建初始状态 / Create initial state
    pub fn new() -> Self {
        Self {
            iteration: 0,
            total_columns: 0,
            not_better_iterations: 0,
            best_objective: None,
            status: ColumnGenerationStatus::NotStarted,
            started_at: Instant::now(),
        }
    }

    /// 标记开始 / Mark as started
    pub fn start(&mut self) {
        self.status = ColumnGenerationStatus::Running;
        self.started_at = Instant::now();
    }

    /// 推进迭代 / Advance iteration
    pub fn advance_iteration(&mut self) {
        self.iteration += 1;
    }

    /// 登记新增列 / Register generated columns
    pub fn register_columns(&mut self, amount: usize) {
        self.total_columns += amount;
    }

    /// 观察目标值 / Observe objective value
    pub fn observe_objective(
        &mut self,
        objective: f64,
        sense: ObjectiveSense,
        tolerance: f64,
    ) -> bool {
        let improved = match self.best_objective {
            None => true,
            Some(best) => match sense {
                ObjectiveSense::Minimize => objective < best - tolerance.abs(),
                ObjectiveSense::Maximize => objective > best + tolerance.abs(),
            },
        };

        if improved {
            self.best_objective = Some(objective);
            self.not_better_iterations = 0;
        } else {
            self.not_better_iterations += 1;
        }
        improved
    }

    /// 判断是否应继续 / Check whether generation should continue
    pub fn should_continue(&mut self, config: &ColumnGenerationConfig) -> bool {
        if self.iteration >= config.max_iterations {
            self.status = ColumnGenerationStatus::IterationLimit;
            return false;
        }
        if self.total_columns >= config.max_column_amount {
            self.status = ColumnGenerationStatus::ColumnLimit;
            return false;
        }
        if self.not_better_iterations >= config.max_not_better_iterations {
            self.status = ColumnGenerationStatus::Converged;
            return false;
        }
        if self.started_at.elapsed() >= config.time_limit {
            self.status = ColumnGenerationStatus::TimeLimit;
            return false;
        }
        self.status = ColumnGenerationStatus::Running;
        true
    }
}

// ============================================================================
// ColumnGenerationResult - 列生成结果 / Column generation result
// ============================================================================

/// 列生成结果 / Column generation result
#[derive(Debug, Clone)]
pub struct ColumnGenerationResult<V, U: UnitTrait> {
    /// 状态 / State
    pub state: ColumnGenerationState,
    /// 层集合 / Layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 装箱结果 / Packing result
    pub packing_result: Option<PackingResult<V, U>>,
    /// 渲染装载计划 / Render loading plans
    pub render_loading_plans: Vec<RenderLoadingPlanDto>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

// ============================================================================
// Application executor seam - 应用执行器接缝 / Application executor seam
// ============================================================================

/// 应用层求解状态 / Application solve state
#[derive(Debug, Clone)]
pub struct ColumnGenerationApplicationState {
    /// 货物列表 / Items
    pub items: Vec<ActualItem<f64, Meter>>,
    /// 箱型列表 / Bin types
    pub bins: Vec<BinType<f64, Meter>>,
    /// 初始层 / Initial layers
    pub initial_layers: Vec<BinLayer<f64, Meter>>,
    /// 当前层 / Current layers
    pub layers: Vec<BinLayer<f64, Meter>>,
    /// 当前迭代 / Current iteration
    pub iteration: usize,
    /// 层块 trace / Layer block traces by layer index
    pub layer_block_traces: HashMap<usize, Vec<LayerBlockTrace<f64, Meter>>>,
    /// 层放置 trace / Layer placement traces by layer index
    pub layer_placement_traces: HashMap<usize, Vec<LayerPlacementTrace<f64, Meter>>>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

/// RMP 执行结果 / RMP execution result
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationRmpExecution {
    /// 目标值 / Objective
    pub objective: Option<f64>,
    /// 影子价格摘要 / Shadow price summary
    pub shadow_price_summary: HashMap<String, f64>,
    /// 模型诊断 / Model diagnostics
    pub diagnostics: Option<MetaModelExecutionDiagnostics>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

/// final MILP 执行结果 / Final MILP execution result
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationFinalExecution {
    /// 最终层 / Final layers
    pub layers: Vec<BinLayer<f64, Meter>>,
    /// 已装箱箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<f64, Meter>>,
    /// 目标值 / Objective
    pub objective: Option<f64>,
    /// 模型诊断 / Model diagnostics
    pub diagnostics: Option<MetaModelExecutionDiagnostics>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

/// application flow 结果 / Application flow result
#[derive(Debug, Clone)]
pub struct ColumnGenerationApplicationFlowResult {
    /// 列生成结果 / Column generation result
    pub result: ColumnGenerationResult<f64, Meter>,
    /// RMP 执行结果 / RMP execution
    pub rmp: ColumnGenerationRmpExecution,
    /// final MILP 执行结果 / Final MILP execution
    pub final_execution: ColumnGenerationFinalExecution,
}

/// 求解器数据集 suite 诊断 / Solver dataset suite diagnostics
#[derive(Debug, Clone, Default)]
pub struct SolverDatasetSuiteDiagnostics {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// dataset 数量 / Dataset count
    pub dataset_count: usize,
    /// backend 名称 / Backend name
    pub backend: String,
    /// feature 名称 / Feature name
    pub feature: String,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// 求解器 feature matrix 诊断 / Solver feature matrix diagnostics
#[derive(Debug, Clone, Default)]
pub struct SolverFeatureMatrixDiagnostics {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// backend 名称 / Backend name
    pub backend: String,
    /// feature 名称列表 / Feature names
    pub features: Vec<String>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// 求解器数据集 suite 骨架 / Solver dataset suite skeleton
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
            diagnostics: vec![format!(
                "solver dataset suite '{}' validated in no-run mode for backend '{}' feature '{}'",
                self.suite_name,
                backend,
                feature,
            )],
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
        let mut fixtures = std::fs::read_dir(root)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
            })
            .map(|entry| {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|file_name| file_name.to_str())
                    .unwrap_or("fixture.csv")
                    .to_string();
                std::fs::read_to_string(path).map(|csv| SolverDatasetFixture {
                    name,
                    csv,
                    group: None,
                    tags: Vec::new(),
                    backend_smoke: false,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
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
            .map(|feature| {
                format!(
                    "solver dataset suite '{}' validated {} dataset(s) in no-run mode for backend '{}' feature '{}'",
                    self.suite_name,
                    dataset_count,
                    backend,
                    feature,
                )
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
}

/// RMP executor / RMP executor
pub trait ColumnGenerationRmpExecutor: Debug + Send + Sync {
    /// 执行 RMP / Execute RMP
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution;
}

/// final MILP executor / Final MILP executor
pub trait ColumnGenerationFinalExecutor: Debug + Send + Sync {
    /// 执行 final MILP / Execute final MILP
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution;
}

/// MetaModel 执行诊断 / MetaModel execution diagnostics
#[derive(Debug, Clone, Default)]
pub struct MetaModelExecutionDiagnostics {
    /// 模型名称 / Model name
    pub model_name: String,
    /// 变量数量 / Variable count
    pub variable_count: usize,
    /// 约束数量 / Constraint count
    pub constraint_count: usize,
    /// 注册的需求数量 / Registered demand count
    pub demand_count: usize,
    /// 注册的层数量 / Registered layer count
    pub layer_count: usize,
    /// 注册的箱数量 / Registered bin count
    pub bin_count: usize,
}

impl MetaModelExecutionDiagnostics {
    /// 从模型创建诊断 / Create diagnostics from model
    pub fn from_model(
        model_name: impl Into<String>,
        model: &MetaModel<f64>,
        demand_count: usize,
        layer_count: usize,
        bin_count: usize,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            variable_count: model.num_tokens(),
            constraint_count: model.num_constraints(),
            demand_count,
            layer_count,
            bin_count,
        }
    }

    /// 写入 info map / Write into info map
    pub fn write_info(&self, info: &mut HashMap<String, String>) {
        info.insert("model_name".to_string(), self.model_name.clone());
        info.insert("variable_count".to_string(), self.variable_count.to_string());
        info.insert("constraint_count".to_string(), self.constraint_count.to_string());
        info.insert("demand_count".to_string(), self.demand_count.to_string());
        info.insert("layer_count".to_string(), self.layer_count.to_string());
        info.insert("bin_count".to_string(), self.bin_count.to_string());
    }
}

/// MetaModel RMP executor 配置 / MetaModel RMP executor config
#[derive(Debug, Clone)]
pub struct MetaModelRmpExecutorConfig {
    /// 模型名称 / Model name
    pub model_name: String,
    /// no-op shadow price 向量 / No-op shadow price vector
    pub shadow_prices: Vec<f64>,
    /// no-op 原始解向量 / No-op primal solution vector
    pub primal_solution: Vec<f64>,
    /// no-op 目标值 / No-op objective
    pub objective: Option<f64>,
}

impl Default for MetaModelRmpExecutorConfig {
    fn default() -> Self {
        Self {
            model_name: "bpp3d_rmp".to_string(),
            shadow_prices: Vec::new(),
            primal_solution: Vec::new(),
            objective: None,
        }
    }
}

/// MetaModel final executor 配置 / MetaModel final executor config
#[derive(Debug, Clone)]
pub struct MetaModelFinalExecutorConfig {
    /// 模型名称 / Model name
    pub model_name: String,
    /// no-op 原始解向量 / No-op primal solution vector
    pub primal_solution: Vec<f64>,
    /// no-op 目标值 / No-op objective
    pub objective: Option<f64>,
}

impl Default for MetaModelFinalExecutorConfig {
    fn default() -> Self {
        Self {
            model_name: "bpp3d_final_milp".to_string(),
            primal_solution: Vec::new(),
            objective: None,
        }
    }
}

/// MetaModel executor 求解结果 / MetaModel executor solve result
#[derive(Debug, Clone, Default)]
pub struct MetaModelExecutorSolveResult {
    /// 目标值 / Objective
    pub objective: Option<f64>,
    /// 原始解向量 / Primal solution vector
    pub primal_solution: Vec<f64>,
    /// 对偶解向量 / Dual solution vector
    pub dual_solution: Vec<f64>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

/// MetaModel solver backend / MetaModel solver backend
pub trait MetaModelSolverBackend: Debug + Send + Sync {
    /// backend 名称 / Backend name
    fn name(&self) -> &str;

    /// 求解 RMP LP / Solve RMP LP
    fn solve_rmp(
        &self,
        model: &MetaModel<f64>,
        diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String>;

    /// 求解 final MILP / Solve final MILP
    fn solve_final(
        &self,
        model: &MetaModel<f64>,
        diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String>;
}

/// no-op MetaModel solver backend / No-op MetaModel solver backend
#[derive(Debug, Clone, Default)]
pub struct NoopMetaModelSolverBackend;

impl MetaModelSolverBackend for NoopMetaModelSolverBackend {
    fn name(&self) -> &str {
        "noop"
    }

    fn solve_rmp(
        &self,
        _model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        Ok(MetaModelExecutorSolveResult::default())
    }

    fn solve_final(
        &self,
        _model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        Ok(MetaModelExecutorSolveResult::default())
    }
}

/// ColumnGenerationSolver MetaModel backend / ColumnGenerationSolver MetaModel backend
///
/// 该 adapter 复用 framework 的 solver 入口，RMP 使用 LP 求解并读取 dual，
/// final 使用 MILP 求解并读取 primal solution。
/// This adapter reuses framework solver entries: RMP uses LP solving with duals,
/// while final uses MILP solving with primal solution extraction.
#[cfg(not(feature = "async"))]
#[derive(Clone)]
pub struct ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver,
{
    /// 求解器 / Solver
    pub solver: S,
    /// RMP 求解参数 / RMP solve options
    pub rmp_options: FrameworkSolveOptions,
    /// final MILP 求解参数 / Final MILP solve options
    pub final_options: FrameworkSolveOptions,
}

#[cfg(not(feature = "async"))]
impl<S> Debug for ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnGenerationSolverMetaModelBackend")
            .field("solver", &self.solver.name())
            .finish()
    }
}

#[cfg(not(feature = "async"))]
impl<S> ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver,
{
    /// 使用默认参数创建 / Create with default options
    pub fn new(solver: S) -> Self {
        Self {
            solver,
            rmp_options: FrameworkSolveOptions::new(),
            final_options: FrameworkSolveOptions::new(),
        }
    }

    /// 设置 RMP 求解参数 / Set RMP solve options
    pub fn with_rmp_options(mut self, options: FrameworkSolveOptions) -> Self {
        self.rmp_options = options;
        self
    }

    /// 设置 final MILP 求解参数 / Set final MILP solve options
    pub fn with_final_options(mut self, options: FrameworkSolveOptions) -> Self {
        self.final_options = options;
        self
    }
}

#[cfg(not(feature = "async"))]
impl<S> MetaModelSolverBackend for ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver + Debug,
{
    fn name(&self) -> &str {
        self.solver.name()
    }

    fn solve_rmp(
        &self,
        model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        let triad_model = model
            .try_to_linear_triad_model()
            .map_err(|e| e.to_string())?;
        let result = self
            .solver
            .solve_lp_with_options(&triad_model, self.rmp_options.clone());
        let result = result.map_err(|e| e.to_string())?;
        Ok(MetaModelExecutorSolveResult {
            objective: Some(result.result.obj),
            primal_solution: result.result.solution,
            dual_solution: result.dual_solution.constraints,
            info: HashMap::from([
                ("backend_kind".to_string(), "column_generation_solver".to_string()),
                ("backend_phase".to_string(), "rmp".to_string()),
            ]),
        })
    }

    fn solve_final(
        &self,
        model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        let result = self
            .solver
            .solve_with_options(model, self.final_options.clone())
            .map_err(|e| e.to_string())?;
        Ok(MetaModelExecutorSolveResult {
            objective: Some(result.obj),
            primal_solution: result.solution,
            dual_solution: Vec::new(),
            info: HashMap::from([
                ("backend_kind".to_string(), "column_generation_solver".to_string()),
                ("backend_phase".to_string(), "final".to_string()),
            ]),
        })
    }
}

/// MetaModel RMP executor 骨架 / MetaModel RMP executor skeleton
#[derive(Debug, Clone, Default)]
pub struct MetaModelRmpExecutor {
    /// 配置 / Config
    pub config: MetaModelRmpExecutorConfig,
}

impl MetaModelRmpExecutor {
    /// 使用配置创建 / Create with config
    pub fn new(config: MetaModelRmpExecutorConfig) -> Self {
        Self { config }
    }

    fn build_context(
        &self,
        state: &ColumnGenerationApplicationState,
    ) -> (LayerAssignmentContext<f64, Meter>, ImpreciseAssignment<f64, Meter>, Vec<Bpp3dDemandEntry>) {
        let demand_entries = item_demand_entries(&state.items);
        let assignment = ImpreciseAssignment {
            layers: state.layers.clone(),
            x: VariableArray1::new("x"),
        };
        let aggregation = LayerAssignmentAggregation::rmp(
            assignment.clone(),
            Load::new(demand_entries.clone()),
            Capacity::new(),
        );
        let mut context = LayerAssignmentContext::new(aggregation);
        context.add_limit(Box::new(DemandConstraint::imprecise(
            demand_entries.clone(),
            assignment.clone(),
        )));
        context.add_objective(Box::new(BetterLayerMaximization::new(
            rmp_layer_value_terms(&assignment),
            1.0,
        )));
        (context, assignment, demand_entries)
    }
}

impl ColumnGenerationRmpExecutor for MetaModelRmpExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution {
        let backend = NoopMetaModelSolverBackend;
        self.execute_with_backend(state, &backend)
    }
}

/// 求解器驱动的 MetaModel RMP executor / Solver-backed MetaModel RMP executor
#[derive(Debug)]
pub struct SolverBackedMetaModelRmpExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 配置 / Config
    pub config: MetaModelRmpExecutorConfig,
    /// 求解 backend / Solver backend
    pub backend: B,
}

impl<B> SolverBackedMetaModelRmpExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 使用配置和 backend 创建 / Create with config and backend
    pub fn new(config: MetaModelRmpExecutorConfig, backend: B) -> Self {
        Self { config, backend }
    }
}

impl<B> ColumnGenerationRmpExecutor for SolverBackedMetaModelRmpExecutor<B>
where
    B: MetaModelSolverBackend,
{
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution {
        let skeleton = MetaModelRmpExecutor::new(self.config.clone());
        skeleton.execute_with_backend(state, &self.backend)
    }
}

impl MetaModelRmpExecutor {
    fn execute_with_backend(
        &self,
        state: &ColumnGenerationApplicationState,
        backend: &dyn MetaModelSolverBackend,
    ) -> ColumnGenerationRmpExecution {
        let (mut context, _assignment, demand_entries) = self.build_context(state);
        let mut model = MetaModel::<f64>::new(&self.config.model_name);
        if let Err(error) = context.register(&mut model).and_then(|_| context.invoke(&model)) {
            return ColumnGenerationRmpExecution {
                objective: None,
                shadow_price_summary: HashMap::new(),
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_rmp".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), error),
                ]),
            };
        }

        let diagnostics = MetaModelExecutionDiagnostics::from_model(
            self.config.model_name.clone(),
            &model,
            demand_entries.len(),
            state.layers.len(),
            state.bins.len(),
        );
        let solve = match backend.solve_rmp(&model, &diagnostics) {
            Ok(solve) => merge_noop_solve_result(
                solve,
                self.config.objective,
                self.config.primal_solution.clone(),
                self.config.shadow_prices.clone(),
            ),
            Err(error) => {
                return ColumnGenerationRmpExecution {
                    objective: None,
                    shadow_price_summary: HashMap::new(),
                    diagnostics: Some(diagnostics),
                    info: HashMap::from([
                        ("executor".to_string(), "meta_model_rmp".to_string()),
                        ("backend".to_string(), backend.name().to_string()),
                        ("status".to_string(), "solve_failed".to_string()),
                        ("error".to_string(), error),
                    ]),
                };
            }
        };
        let layer_values = context
            .aggregation
            .imprecise_assignment
            .as_ref()
            .map(|assignment| {
                SolutionExtractor::extract_values_1(
                    &solve.primal_solution,
                    &assignment.x,
                )
            })
            .unwrap_or_default();
        let shadow_price_summary = demand_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                (
                    demand_key_name(entry),
                    solve.dual_solution.get(index).copied().unwrap_or(0.0),
                )
            })
            .collect::<HashMap<_, _>>();
        let mut info = HashMap::from([
            ("executor".to_string(), "meta_model_rmp".to_string()),
            ("backend".to_string(), backend.name().to_string()),
            ("status".to_string(), "registered".to_string()),
            ("selected_layer_value_count".to_string(), layer_values.len().to_string()),
        ]);
        info.extend(solve.info);
        diagnostics.write_info(&mut info);
        ColumnGenerationRmpExecution {
            objective: solve.objective,
            shadow_price_summary,
            diagnostics: Some(diagnostics),
            info,
        }
    }
}

/// MetaModel final MILP executor 骨架 / MetaModel final MILP executor skeleton
#[derive(Debug, Clone, Default)]
pub struct MetaModelFinalExecutor {
    /// 配置 / Config
    pub config: MetaModelFinalExecutorConfig,
}

impl MetaModelFinalExecutor {
    /// 使用配置创建 / Create with config
    pub fn new(config: MetaModelFinalExecutorConfig) -> Self {
        Self { config }
    }

    fn build_context(
        &self,
        state: &ColumnGenerationApplicationState,
    ) -> (LayerAssignmentContext<f64, Meter>, PreciseAssignment<f64, Meter>, Vec<Bpp3dDemandEntry>) {
        let demand_entries = item_demand_entries(&state.items);
        let bins = if state.bins.is_empty() {
            bins_from_layers(&state.layers)
        } else {
            state.bins.clone()
        };
        let assignment = PreciseAssignment {
            bins,
            layers: state.layers.clone(),
            x: VariableArray2::new("x"),
            v: VariableArray1::new("v"),
        };
        let aggregation = LayerAssignmentAggregation::final_milp(
            assignment.clone(),
            Load::new(demand_entries.clone()),
            Capacity::new(),
        );
        let mut context = LayerAssignmentContext::new(aggregation);
        context.add_limit(Box::new(DemandConstraint::precise(
            demand_entries.clone(),
            assignment.clone(),
        )));
        context.add_limit(Box::new(PreciseAssignmentActivationConstraint::new(
            assignment.clone(),
        )));
        context.add_limit(Box::new(BinDepthConstraint::from_bins(
            &assignment.bins,
            final_assignment_indices_by_bin(&assignment),
            layer_depths(&assignment.layers),
            &Default::default(),
        )));
        context.add_objective(Box::new(BinAmountMinimization::new(
            final_bin_marker_indices(&assignment),
            1.0,
        )));
        (context, assignment, demand_entries)
    }
}

impl ColumnGenerationFinalExecutor for MetaModelFinalExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution {
        let backend = NoopMetaModelSolverBackend;
        self.execute_with_backend(state, &backend)
    }
}

/// 求解器驱动的 MetaModel final MILP executor / Solver-backed MetaModel final MILP executor
#[derive(Debug)]
pub struct SolverBackedMetaModelFinalExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 配置 / Config
    pub config: MetaModelFinalExecutorConfig,
    /// 求解 backend / Solver backend
    pub backend: B,
}

impl<B> SolverBackedMetaModelFinalExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 使用配置和 backend 创建 / Create with config and backend
    pub fn new(config: MetaModelFinalExecutorConfig, backend: B) -> Self {
        Self { config, backend }
    }
}

impl<B> ColumnGenerationFinalExecutor for SolverBackedMetaModelFinalExecutor<B>
where
    B: MetaModelSolverBackend,
{
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution {
        let skeleton = MetaModelFinalExecutor::new(self.config.clone());
        skeleton.execute_with_backend(state, &self.backend)
    }
}

impl MetaModelFinalExecutor {
    fn execute_with_backend(
        &self,
        state: &ColumnGenerationApplicationState,
        backend: &dyn MetaModelSolverBackend,
    ) -> ColumnGenerationFinalExecution {
        let (mut context, _assignment, demand_entries) = self.build_context(state);
        let mut model = MetaModel::<f64>::new(&self.config.model_name);
        if let Err(error) = context.register(&mut model).and_then(|_| context.invoke(&model)) {
            return ColumnGenerationFinalExecution {
                layers: Vec::new(),
                packed_bins: Vec::new(),
                objective: None,
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_final".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), error),
                ]),
            };
        }

        let diagnostics = MetaModelExecutionDiagnostics::from_model(
            self.config.model_name.clone(),
            &model,
            demand_entries.len(),
            state.layers.len(),
            context
                .aggregation
                .precise_assignment
                .as_ref()
                .map(|assignment| assignment.bins.len())
                .unwrap_or(0),
        );
        let solve = match backend.solve_final(&model, &diagnostics) {
            Ok(solve) => merge_noop_solve_result(
                solve,
                self.config.objective,
                self.config.primal_solution.clone(),
                Vec::new(),
            ),
            Err(error) => {
                return ColumnGenerationFinalExecution {
                    layers: Vec::new(),
                    packed_bins: Vec::new(),
                    objective: None,
                    diagnostics: Some(diagnostics),
                    info: HashMap::from([
                        ("executor".to_string(), "meta_model_final".to_string()),
                        ("backend".to_string(), backend.name().to_string()),
                        ("status".to_string(), "solve_failed".to_string()),
                        ("error".to_string(), error),
                    ]),
                };
            }
        };
        let Some(assignment) = context.aggregation.precise_assignment.as_ref() else {
            return ColumnGenerationFinalExecution {
                layers: Vec::new(),
                packed_bins: Vec::new(),
                objective: None,
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_final".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), "missing precise assignment after registration".to_string()),
                ]),
            };
        };
        let selected_assignments = SolutionExtractor::extract_binary_2(
            &solve.primal_solution,
            &assignment.x,
        );
        let selected_layer_indices = state.layers
            .iter()
            .enumerate()
            .filter_map(|(layer_index, _)| {
                let selected = assignment.bins
                    .iter()
                    .enumerate()
                    .any(|(bin_index, _)| {
                        selected_assignments
                            .get(&(bin_index, layer_index))
                            .copied()
                            .unwrap_or(false)
                    });
                selected.then_some(layer_index)
            })
            .collect::<Vec<_>>();
        let layers = selected_layer_indices
            .iter()
            .filter_map(|index| state.layers.get(*index).cloned())
            .collect::<Vec<_>>();
        let output_layers = if solve.primal_solution.is_empty() {
            state.layers.clone()
        } else {
            layers
        };
        let output_layer_indices = if solve.primal_solution.is_empty() {
            (0..state.layers.len()).collect::<Vec<_>>()
        } else {
            selected_layer_indices
        };
        let trace_layer_count = output_layers
            .iter()
            .filter(|layer| !layer.demand_coverage.is_empty())
            .count();
        let (packed_bins, mut packing_diagnostics) = packed_bins_from_selected_layers(
            &output_layers,
            &output_layer_indices,
            state,
        );
        if state.bins.is_empty() && bins_from_layers(&state.layers).is_empty() {
            packing_diagnostics.push("no available bin type for final packing".to_string());
        }
        let mut info = HashMap::from([
            ("executor".to_string(), "meta_model_final".to_string()),
            ("backend".to_string(), backend.name().to_string()),
            ("status".to_string(), "registered".to_string()),
            ("selected_assignment_count".to_string(), selected_assignments.len().to_string()),
            ("selected_layer_count".to_string(), output_layers.len().to_string()),
            ("selected_layer_with_coverage_count".to_string(), trace_layer_count.to_string()),
            ("packed_bin_count".to_string(), packed_bins.len().to_string()),
        ]);
        if !packing_diagnostics.is_empty() {
            info.insert(
                "packing_diagnostics".to_string(),
                packing_diagnostics.join("; "),
            );
        }
        if !packed_bins.is_empty() {
            info.insert(
                "final_diagnostics".to_string(),
                "selected_layers_renderable".to_string(),
            );
        } else if output_layers.is_empty() {
            info.insert(
                "final_diagnostics".to_string(),
                "no_selected_layer_assignment".to_string(),
            );
        } else if trace_layer_count == 0 {
            info.insert(
                "final_diagnostics".to_string(),
                "selected_layers_without_coverage_trace".to_string(),
            );
        } else {
            info.insert(
                "final_diagnostics".to_string(),
                "selected_layers_have_coverage_trace".to_string(),
            );
        }
        info.extend(solve.info);
        diagnostics.write_info(&mut info);
        ColumnGenerationFinalExecution {
            layers: output_layers,
            packed_bins,
            objective: solve.objective,
            diagnostics: Some(diagnostics),
            info,
        }
    }
}

fn packed_bins_from_selected_layers(
    layers: &[BinLayer<f64, Meter>],
    layer_indices: &[usize],
    state: &ColumnGenerationApplicationState,
) -> (Vec<PackedBin<f64, Meter>>, Vec<String>) {
    let replay = LayerTraceReplayAdapter::new().replay_selected_layers(
        layers,
        layer_indices,
        &state.items,
        &state.bins,
        &state.layer_block_traces,
        &state.layer_placement_traces,
    );
    (replay.packed_bins, replay.diagnostics)
}

fn merge_noop_solve_result(
    mut solve: MetaModelExecutorSolveResult,
    fallback_objective: Option<f64>,
    fallback_primal_solution: Vec<f64>,
    fallback_dual_solution: Vec<f64>,
) -> MetaModelExecutorSolveResult {
    if solve.objective.is_none() {
        solve.objective = fallback_objective;
    }
    if solve.primal_solution.is_empty() {
        solve.primal_solution = fallback_primal_solution;
    }
    if solve.dual_solution.is_empty() {
        solve.dual_solution = fallback_dual_solution;
    }
    solve
}

fn item_demand_entries(items: &[ActualItem<f64, Meter>]) -> Vec<Bpp3dDemandEntry> {
    items
        .iter()
        .map(|item| Bpp3dDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: item.id.clone() },
            demand: 1.0,
        })
        .collect()
}

fn bins_from_layers(layers: &[BinLayer<f64, Meter>]) -> Vec<BinType<f64, Meter>> {
    layers
        .iter()
        .filter_map(|layer| layer.bin.clone())
        .collect()
}

fn demand_key_name(entry: &Bpp3dDemandEntry) -> String {
    match &entry.key {
        Bpp3dDemandKey::Item { id } => format!("item:{}", id),
        Bpp3dDemandKey::Material { no } => format!("material:{}", no),
    }
}

fn rmp_layer_value_terms(
    assignment: &ImpreciseAssignment<f64, Meter>,
) -> Vec<(usize, f64)> {
    (0..assignment.layers.len())
        .filter_map(|layer_idx| {
            assignment
                .x
                .index(&layer_idx)
                .or(Some(layer_idx))
                .map(|model_idx| (model_idx, 1.0))
        })
        .collect()
}

fn final_assignment_indices_by_bin(
    assignment: &PreciseAssignment<f64, Meter>,
) -> Vec<Vec<(usize, usize)>> {
    assignment.bins
        .iter()
        .enumerate()
        .map(|(bin_idx, _)| {
            assignment.layers
                .iter()
                .enumerate()
                .filter_map(|(layer_idx, _)| {
                    assignment
                        .x
                        .index(&bin_idx, &layer_idx)
                        .or(Some(bin_idx * assignment.layers.len() + layer_idx))
                        .map(|model_idx| (layer_idx, model_idx))
                })
                .collect()
        })
        .collect()
}

fn final_bin_marker_indices(
    assignment: &PreciseAssignment<f64, Meter>,
) -> Vec<usize> {
    (0..assignment.bins.len())
        .filter_map(|bin_idx| {
            assignment
                .v
                .index(&bin_idx)
                .or(Some(assignment.bins.len() * assignment.layers.len() + bin_idx))
        })
        .collect()
}

fn layer_depths(layers: &[BinLayer<f64, Meter>]) -> Vec<f64> {
    layers
        .iter()
        .map(|layer| layer.depth.value)
        .collect()
}

fn layer_trace_key(layer: &BinLayer<f64, Meter>) -> String {
    format!("{}:{}", layer.from, layer.depth.value)
}

fn item_shadow_price_key(item_id: impl Into<String>) -> String {
    format!("item:{}", item_id.into())
}

fn application_state_from_algorithm(
    algorithm: &ColumnGenerationAlgorithm<f64, Meter>,
    items: Vec<ActualItem<f64, Meter>>,
    bins: Vec<BinType<f64, Meter>>,
    initial_layers: Vec<BinLayer<f64, Meter>>,
    layer_block_traces: HashMap<usize, Vec<LayerBlockTrace<f64, Meter>>>,
    layer_placement_traces: HashMap<usize, Vec<LayerPlacementTrace<f64, Meter>>>,
) -> ColumnGenerationApplicationState {
    ColumnGenerationApplicationState {
        items,
        bins,
        initial_layers,
        layers: algorithm.layer_aggregation.layers.clone(),
        iteration: algorithm.state.iteration,
        layer_block_traces,
        layer_placement_traces,
        info: HashMap::new(),
    }
}

fn default_item_coverage(items: &[ActualItem<f64, Meter>]) -> Vec<Bpp3dLayerDemandCoverage> {
    items
        .iter()
        .map(|item| {
            Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: item.id.clone() },
                1.0,
            )
        })
        .collect()
}

fn ensure_layer_demand_coverage(
    layers: Vec<BinLayer<f64, Meter>>,
    items: &[ActualItem<f64, Meter>],
) -> Vec<BinLayer<f64, Meter>> {
    let fallback = default_item_coverage(items);
    layers
        .into_iter()
        .map(|layer| {
            if layer.demand_coverage.is_empty() {
                layer.with_demand_coverage(fallback.clone())
            } else {
                layer
            }
        })
        .collect()
}

fn layer_generation_demand_entries(
    items: &[ActualItem<f64, Meter>],
    shadow_price_summary: &HashMap<String, f64>,
) -> Vec<LayerGenerationDemandEntry> {
    items
        .iter()
        .map(|item| LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: item.id.clone() },
            demand: 1.0,
            satisfied: if shadow_price_summary.contains_key(&item_shadow_price_key(&item.id)) {
                0.0
            } else {
                1.0
            },
        })
        .collect()
}

fn layer_generation_shadow_prices(
    shadow_price_summary: &HashMap<String, f64>,
) -> HashMap<DemandShadowPriceKey, f64> {
    shadow_price_summary
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("item:").map(|id| {
                (
                    DemandShadowPriceKey {
                        mode: Bpp3dDemandMode::Item,
                        key: Bpp3dDemandKey::Item { id: id.to_string() },
                    },
                    *value,
                )
            })
        })
        .collect()
}

/// mock RMP executor / Mock RMP executor
#[derive(Debug, Clone, Default)]
pub struct MockColumnGenerationRmpExecutor {
    /// 目标值 / Objective
    pub objective: Option<f64>,
}

impl ColumnGenerationRmpExecutor for MockColumnGenerationRmpExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution {
        ColumnGenerationRmpExecution {
            objective: self.objective,
            shadow_price_summary: HashMap::new(),
            diagnostics: None,
            info: HashMap::from([
                ("executor".to_string(), "mock_rmp".to_string()),
                ("layer_count".to_string(), state.layers.len().to_string()),
            ]),
        }
    }
}

/// mock final executor / Mock final executor
#[derive(Debug, Clone, Default)]
pub struct MockColumnGenerationFinalExecutor;

impl ColumnGenerationFinalExecutor for MockColumnGenerationFinalExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution {
        ColumnGenerationFinalExecution {
            layers: state.layers.clone(),
            packed_bins: Vec::new(),
            objective: None,
            diagnostics: None,
            info: HashMap::from([
                ("executor".to_string(), "mock_final".to_string()),
                ("layer_count".to_string(), state.layers.len().to_string()),
            ]),
        }
    }
}

// ============================================================================
// ColumnGenerationAlgorithm - 列生成算法 / Column generation algorithm
// ============================================================================

/// 列生成算法 / Column generation algorithm
///
/// 只编排层生成与列集合状态，RMP/final MILP 建模仍由 domain context 负责。
/// Orchestrates layer generation and column-set state only; RMP/final MILP
/// modeling remains owned by domain contexts.
#[derive(Debug)]
pub struct ColumnGenerationAlgorithm<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: UnitTrait + Debug + Clone + Send + Sync,
{
    /// 配置 / Config
    pub config: ColumnGenerationConfig,
    /// 层生成上下文 / Layer generation context
    pub layer_generation: LayerGenerationContext<V, U>,
    /// 层聚合 / Layer aggregation
    pub layer_aggregation: LayerAggregation<V, U>,
    /// 状态 / State
    pub state: ColumnGenerationState,
}

impl<V, U> ColumnGenerationAlgorithm<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建算法 / Create algorithm
    pub fn new(config: ColumnGenerationConfig) -> Self {
        Self {
            config,
            layer_generation: LayerGenerationContext::new(),
            layer_aggregation: LayerAggregation::new(),
            state: ColumnGenerationState::new(),
        }
    }

    /// 使用层生成上下文创建算法 / Create algorithm with layer generation context
    pub fn with_layer_generation(
        config: ColumnGenerationConfig,
        layer_generation: LayerGenerationContext<V, U>,
    ) -> Self {
        Self {
            config,
            layer_generation,
            layer_aggregation: LayerAggregation::new(),
            state: ColumnGenerationState::new(),
        }
    }

    /// 添加初始层 / Add initial layers
    pub fn add_initial_layers(&mut self, layers: Vec<BinLayer<V, U>>) -> Vec<BinLayer<V, U>> {
        let added = self.layer_aggregation.add_columns(layers);
        self.state.register_columns(added.len());
        added
    }

    /// 执行一轮层生成 / Execute one layer-generation iteration
    pub fn generate_once(
        &mut self,
        bin: Option<BinType<V, U>>,
        items: Vec<ActualItem<V, U>>,
        demand_entries: Vec<LayerGenerationDemandEntry>,
    ) -> Vec<LayerGenerationResult<V, U>> {
        if self.state.status == ColumnGenerationStatus::NotStarted {
            self.state.start();
        }

        let mut request = LayerGenerationRequest::new(self.state.iteration as i64, items)
            .with_demand_entries(demand_entries)
            .with_max_candidates(self.config.max_candidates_per_iteration);
        request.existing_layers = self.layer_aggregation.layers.clone();
        request.bin = bin;

        let results = self.layer_generation.generate(&request);
        let layers = results.iter().map(|result| result.layer.clone()).collect();
        let added = self.layer_aggregation.add_columns(layers);

        self.state.register_columns(added.len());
        self.state.advance_iteration();
        results
    }

    /// 判断是否继续 / Check whether to continue
    pub fn should_continue(&mut self) -> bool {
        self.state.should_continue(&self.config)
    }

    /// 构造当前结果 / Build current result
    pub fn result(&self) -> ColumnGenerationResult<V, U> {
        ColumnGenerationResult {
            state: self.state.clone(),
            layers: self.layer_aggregation.layers.clone(),
            packing_result: None,
            render_loading_plans: Vec::new(),
            info: HashMap::new(),
        }
    }
}

// ============================================================================
// ColumnGenerationStandardExecutors - 标准执行器 / Standard executors
// ============================================================================

/// 列生成标准执行器 / Column generation standard executors
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationStandardExecutors;

impl ColumnGenerationStandardExecutors {
    /// 创建默认层生成上下文 / Create default layer generation context
    pub fn default_layer_generation_context<V, U>() -> LayerGenerationContext<V, U>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
        U: CTUnit + Default + Debug + Clone + Send + Sync,
    {
        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(BlockLayerGenerator::new()));
        context.add_generator(Box::new(BLLocalLayerGenerator::new()));
        context.add_generator(Box::new(BLGlobalLayerGenerator::new()));
        context.add_generator(Box::new(CirclePackingLayerGenerator::new()));
        context.add_generator(Box::new(PatternLayerGenerator::new()));
        context.add_generator(Box::new(PileLayerGenerator::new()));
        context.add_generator(Box::new(HistoricalLayerGenerator::new()));
        context
    }
}

// ============================================================================
// DepthBoundaryLayerOrientationPolicy - 深度边界策略 / Depth boundary policy
// ============================================================================

/// 深度边界校验阶段 / Depth-boundary validation stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthBoundaryValidationStage {
    /// 层生成阶段 / Layer generation stage
    Generation,
    /// 最终已知坐标阶段 / Final known-coordinate stage
    FinalKnownCoordinate,
}

/// 深度边界层朝向策略 / Depth boundary layer orientation policy
///
/// 策略只在最终已知坐标阶段生效，生成阶段不提前过滤候选。
/// The policy is enforced only at final known-coordinate validation; generation
/// candidates are not filtered early.
#[derive(Debug, Clone)]
pub struct DepthBoundaryLayerOrientationPolicy {
    /// 是否允许深度边界上的旋转朝向 / Whether rotated orientation is allowed on depth boundary
    pub allow_rotated_on_depth_boundary: bool,
}

impl Default for DepthBoundaryLayerOrientationPolicy {
    fn default() -> Self {
        Self {
            allow_rotated_on_depth_boundary: false,
        }
    }
}

impl DepthBoundaryLayerOrientationPolicy {
    /// 创建默认策略 / Create default policy
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否允许旋转 / Set whether rotated orientation is allowed
    pub fn with_allow_rotated_on_depth_boundary(mut self, value: bool) -> Self {
        self.allow_rotated_on_depth_boundary = value;
        self
    }

    /// 校验朝向 / Validate orientation
    pub fn validate(
        &self,
        stage: DepthBoundaryValidationStage,
        orientation: Orientation,
        is_depth_boundary: bool,
    ) -> Result<(), String> {
        if stage == DepthBoundaryValidationStage::Generation {
            return Ok(());
        }
        if !is_depth_boundary || self.allow_rotated_on_depth_boundary || !orientation.is_rotated() {
            return Ok(());
        }

        Err(format!(
            "Rotated orientation {:?} is not allowed on depth boundary. / 深度边界不允许旋转朝向 {:?}。",
            orientation, orientation
        ))
    }
}

// ============================================================================
// ColumnGenerationPackingAnalyzer - 装箱分析器 / Packing analyzer
// ============================================================================

/// 列生成装箱分析 / Column generation packing analysis
#[derive(Debug, Clone)]
pub struct ColumnGenerationPackingAnalysis<V, U: UnitTrait> {
    /// 装箱结果 / Packing result
    pub packing_result: PackingResult<V, U>,
    /// 渲染装载计划 / Render loading plans
    pub render_loading_plans: Vec<RenderLoadingPlanDto>,
}

/// 列生成装箱分析器 / Column generation packing analyzer
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationPackingAnalyzer {
    /// 装箱器 / Packer
    pub packer: Packer,
    /// 渲染适配器 / Renderer adapter
    pub renderer: PackingRendererAdapter,
}

impl ColumnGenerationPackingAnalyzer {
    /// 创建分析器 / Create analyzer
    pub fn new() -> Self {
        Self {
            packer: Packer::new(),
            renderer: PackingRendererAdapter::new(),
        }
    }

    /// 分析已装箱结果 / Analyze packed bins
    pub fn analyze<V, U>(
        &self,
        packed_bins: Vec<PackedBin<V, U>>,
    ) -> Result<ColumnGenerationPackingAnalysis<V, U>, Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        for packed_bin in &packed_bins {
            PackingGeometryGuard::validate(packed_bin)?;
        }

        let packing_result = self.packer.invoke(packed_bins);
        let render_loading_plans = self.renderer.to_render_dto(&packing_result);
        Ok(ColumnGenerationPackingAnalysis {
            packing_result,
            render_loading_plans,
        })
    }
}

// ============================================================================
// ColumnGenerationApplicationService - 列生成应用服务 / Application service
// ============================================================================

/// 列生成应用服务 / Column generation application service
///
/// 提供轻量应用入口，组合配置、层生成和装箱分析。
/// Provides a lightweight application entry point composing config, layer
/// generation, and packing analysis.
#[derive(Debug, Clone)]
pub struct ColumnGenerationApplicationService {
    /// 配置 / Config
    pub config: ColumnGenerationConfig,
    /// 装箱分析器 / Packing analyzer
    pub packing_analyzer: ColumnGenerationPackingAnalyzer,
}

impl Default for ColumnGenerationApplicationService {
    fn default() -> Self {
        Self::new(ColumnGenerationConfig::default())
    }
}

impl ColumnGenerationApplicationService {
    /// 创建应用服务 / Create application service
    pub fn new(config: ColumnGenerationConfig) -> Self {
        Self {
            config,
            packing_analyzer: ColumnGenerationPackingAnalyzer::new(),
        }
    }

    /// 创建列生成算法 / Create column generation algorithm
    pub fn create_algorithm<V, U>(&self) -> ColumnGenerationAlgorithm<V, U>
    where
        V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
        U: UnitTrait + Debug + Clone + Send + Sync,
    {
        ColumnGenerationAlgorithm::new(self.config.clone())
    }

    /// 分析装箱结果 / Analyze packing result
    pub fn analyze_packing<V, U>(
        &self,
        packed_bins: Vec<PackedBin<V, U>>,
    ) -> Result<ColumnGenerationPackingAnalysis<V, U>, Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        self.packing_analyzer.analyze(packed_bins)
    }

    /// 运行 materialized request / Run materialized request
    ///
    /// 该入口只编排 executor，不直接注册 MetaModel。
    /// This entry only orchestrates executors and does not register MetaModel directly.
    pub fn run_materialized(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized_with_bins(
            items,
            Vec::new(),
            initial_layers,
            rmp_executor,
            final_executor,
        )
    }

    /// 运行带箱型的 materialized request / Run materialized request with bins
    ///
    /// 该入口只传递完整应用状态给 executor，不直接注册 MetaModel。
    /// This entry only passes complete application state to executors and does not register MetaModel directly.
    pub fn run_materialized_with_bins(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        bins: Vec<BinType<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let mut algorithm = self.create_algorithm::<f64, Meter>();
        let initial_layers = ensure_layer_demand_coverage(initial_layers, &items);
        algorithm.add_initial_layers(initial_layers.clone());
        let state = application_state_from_algorithm(
            &algorithm,
            items,
            bins,
            initial_layers,
            HashMap::new(),
            HashMap::new(),
        );
        let rmp = rmp_executor.execute(&state);
        if let Some(objective) = rmp.objective {
            algorithm.state.observe_objective(
                objective,
                self.config.objective_sense,
                self.config.reduced_cost_tolerance,
            );
        }
        let final_execution = final_executor.execute(&state);
        let packing_analysis = if final_execution.packed_bins.is_empty() {
            None
        } else {
            Some(self.analyze_packing(final_execution.packed_bins.clone())?)
        };
        let mut info = HashMap::new();
        info.extend(rmp.info.iter().map(|(k, v)| (format!("rmp_{}", k), v.clone())));
        info.extend(final_execution.info.iter().map(|(k, v)| (format!("final_{}", k), v.clone())));
        let result = ColumnGenerationResult {
            state: algorithm.state,
            layers: final_execution.layers.clone(),
            packing_result: packing_analysis.as_ref().map(|analysis| analysis.packing_result.clone()),
            render_loading_plans: packing_analysis
                .map(|analysis| analysis.render_loading_plans)
                .unwrap_or_default(),
            info,
        };
        Ok(ColumnGenerationApplicationFlowResult {
            result,
            rmp,
            final_execution,
        })
    }

    /// 运行一轮 RMP -> 生成列 -> RMP refresh -> final / Run one RMP -> generate columns -> RMP refresh -> final round
    ///
    /// 该入口验证 shadow price 能穿过 application 编排进入 layer generation。
    /// This entry verifies shadow prices pass through application orchestration into layer generation.
    pub fn run_materialized_one_generation_round(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        bins: Vec<BinType<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        layer_generation: LayerGenerationContext<f64, Meter>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let mut algorithm = ColumnGenerationAlgorithm::with_layer_generation(
            self.config.clone(),
            layer_generation,
        );
        let initial_layers = ensure_layer_demand_coverage(initial_layers, &items);
        algorithm.add_initial_layers(initial_layers.clone());
        let initial_state = application_state_from_algorithm(
            &algorithm,
            items.clone(),
            bins.clone(),
            initial_layers.clone(),
            HashMap::new(),
            HashMap::new(),
        );
        let first_rmp = rmp_executor.execute(&initial_state);
        if let Some(objective) = first_rmp.objective {
            algorithm.state.observe_objective(
                objective,
                self.config.objective_sense,
                self.config.reduced_cost_tolerance,
            );
        }

        let demand_entries = layer_generation_demand_entries(&items, &first_rmp.shadow_price_summary);
        let shadow_prices = layer_generation_shadow_prices(&first_rmp.shadow_price_summary);
        let mut request = LayerGenerationRequest::new(algorithm.state.iteration as i64, items.clone())
            .with_demand_entries(demand_entries)
            .with_shadow_prices(shadow_prices)
            .with_max_candidates(self.config.max_candidates_per_iteration);
        request.bin = bins.first().cloned();
        request.existing_layers = algorithm.layer_aggregation.layers.clone();
        let generated = algorithm.layer_generation.generate(&request);
        let generated_block_trace_by_key = generated
            .iter()
            .filter(|result| !result.block_traces.is_empty())
            .map(|result| {
                (
                    layer_trace_key(&result.layer),
                    result.block_traces.clone(),
                )
            })
            .collect::<HashMap<_, _>>();
        let generated_trace_by_key = generated
            .iter()
            .filter(|result| !result.placement_traces.is_empty())
            .map(|result| {
                (
                    layer_trace_key(&result.layer),
                    result.placement_traces.clone(),
                )
            })
            .collect::<HashMap<_, _>>();
        let generated_layers = generated
            .into_iter()
            .map(|result| result.layer)
            .collect::<Vec<_>>();
        let generated_layers = ensure_layer_demand_coverage(generated_layers, &items);
        algorithm.add_initial_layers(generated_layers);
        let layer_block_traces = algorithm
            .layer_aggregation
            .layers
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                generated_block_trace_by_key
                    .get(&layer_trace_key(layer))
                    .cloned()
                    .map(|traces| (index, traces))
            })
            .collect::<HashMap<_, _>>();
        let layer_placement_traces = algorithm
            .layer_aggregation
            .layers
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                generated_trace_by_key
                    .get(&layer_trace_key(layer))
                    .cloned()
                    .map(|traces| (index, traces))
            })
            .collect::<HashMap<_, _>>();
        algorithm.state.advance_iteration();

        let refreshed_state = application_state_from_algorithm(
            &algorithm,
            items,
            bins,
            initial_layers,
            layer_block_traces,
            layer_placement_traces,
        );
        let rmp = rmp_executor.execute(&refreshed_state);
        if let Some(objective) = rmp.objective {
            algorithm.state.observe_objective(
                objective,
                self.config.objective_sense,
                self.config.reduced_cost_tolerance,
            );
        }
        let final_execution = final_executor.execute(&refreshed_state);
        let packing_analysis = if final_execution.packed_bins.is_empty() {
            None
        } else {
            Some(self.analyze_packing(final_execution.packed_bins.clone())?)
        };
        let mut info = HashMap::from([
            ("first_rmp_objective".to_string(), format!("{:?}", first_rmp.objective)),
            ("generated_layer_count".to_string(), (algorithm.state.total_columns.saturating_sub(refreshed_state.initial_layers.len())).to_string()),
        ]);
        info.extend(rmp.info.iter().map(|(k, v)| (format!("rmp_{}", k), v.clone())));
        info.extend(final_execution.info.iter().map(|(k, v)| (format!("final_{}", k), v.clone())));
        let result = ColumnGenerationResult {
            state: algorithm.state,
            layers: final_execution.layers.clone(),
            packing_result: packing_analysis.as_ref().map(|analysis| analysis.packing_result.clone()),
            render_loading_plans: packing_analysis
                .map(|analysis| analysis.render_loading_plans)
                .unwrap_or_default(),
            info,
        };
        Ok(ColumnGenerationApplicationFlowResult {
            result,
            rmp,
            final_execution,
        })
    }

    /// 运行 CSV materialized request / Run CSV materialized request
    #[cfg(feature = "serde")]
    pub fn run_csv_materialized(
        &self,
        request: CsvMaterializedApplicationRequest,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized(
            request.items,
            request.initial_layers,
            rmp_executor,
            final_executor,
        )
    }

    /// 运行带箱型的 CSV materialized request / Run CSV materialized request with bins
    #[cfg(feature = "serde")]
    pub fn run_csv_materialized_with_bins(
        &self,
        request: CsvMaterializedApplicationRequest,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized_with_bins(
            request.items,
            request.bins,
            request.initial_layers,
            rmp_executor,
            final_executor,
        )
    }

    /// 运行一轮 CSV materialized request / Run one-round CSV materialized request
    #[cfg(feature = "serde")]
    pub fn run_csv_materialized_one_generation_round(
        &self,
        request: CsvMaterializedApplicationRequest,
        layer_generation: LayerGenerationContext<f64, Meter>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized_one_generation_round(
            request.items,
            request.bins,
            request.initial_layers,
            layer_generation,
            rmp_executor,
            final_executor,
        )
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    use ospf_rust_quantities::quantity::Quantity;
    use ospf_rust_quantities::unit::derived::Meter;
    #[cfg(not(feature = "async"))]
    use ospf_rust_framework::solver::{FeasibleSolution, LinearDualSolution, LPResult};

    use crate::domain::item::PackageShapeSpec;
    use crate::domain::packing::{KnownCoordinatePlacement, LayerPlacementAdapter};
    use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
    use crate::infrastructure::orientation::Orientation;

    fn meters(value: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(value)
    }

    fn bin_type() -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN".to_string(),
            is_main: true,
        }
    }

    fn item(id: &str) -> ActualItem<f64, Meter> {
        ActualItem {
            id: id.to_string(),
            name: id.to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cuboid),
        }
    }

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
        assert_eq!(matrix.diagnostics.len(), 2);
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
    fn solver_dataset_fixture_suite_loads_manifest_and_directory() {
        let root = std::env::temp_dir().join(format!(
            "bpp3d-fixture-suite-{}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        let csv = SolverDatasetFixtureSuite::smoke().fixtures[0].csv.clone();
        std::fs::write(root.join("a.csv"), &csv).unwrap();
        std::fs::write(root.join("b.csv"), &csv).unwrap();
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
        assert_eq!(directory_suite.fixtures.len(), 2);
        assert_eq!(directory_suite.fixtures[0].name, "a.csv");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn solver_dataset_fixture_suite_loads_crate_fixtures() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("manifest.json");
        let suite = SolverDatasetFixtureSuite::from_manifest_file(manifest_path).unwrap();
        let report = suite.run_batch_report("fake", "serde", false);

        assert_eq!(suite.suite_name, "bpp3d-regression");
        assert_eq!(suite.fixtures.len(), 5);
        assert!(suite.fixtures.iter().any(|fixture| fixture.tags.iter().any(|tag| tag == "kotlin")));
        assert_eq!(report.fixture_count, 5);
        assert_eq!(report.materialized_count, 5);
        assert!(report.backend_smoke_count >= 3);
        assert!(report.fixture_results.iter().all(|result| result.materialized));
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
            .any(|result| result.dataset_name.contains("kotlin-grouped-layer")));
    }

    #[test]
    fn depth_boundary_policy_only_checks_final_stage() {
        let policy = DepthBoundaryLayerOrientationPolicy::new();

        assert!(policy.validate(
            DepthBoundaryValidationStage::Generation,
            Orientation::UprightRotated,
            true,
        ).is_ok());
        assert!(policy.validate(
            DepthBoundaryValidationStage::FinalKnownCoordinate,
            Orientation::UprightRotated,
            true,
        ).is_err());
        assert!(policy.validate(
            DepthBoundaryValidationStage::FinalKnownCoordinate,
            Orientation::Upright,
            true,
        ).is_ok());
    }

    #[test]
    fn layer_placement_adapter_builds_valid_packed_bin() {
        let adapter = LayerPlacementAdapter::new();
        let placement = KnownCoordinatePlacement {
            item_index: 0,
            item: item("i0"),
            position: MetricPoint3 {
                x: meters(0.0),
                y: meters(0.0),
                z: meters(0.0),
            },
            orientation: Orientation::Upright,
        };

        let packed_bin = adapter
            .to_packed_bin("bin-1".to_string(), bin_type(), None, vec![placement])
            .unwrap();

        assert_eq!(packed_bin.items.len(), 1);
        assert_eq!(packed_bin.items[0].item.id, "i0");
    }

    #[test]
    fn packing_analyzer_outputs_render_plan() {
        let adapter = LayerPlacementAdapter::new();
        let placement = KnownCoordinatePlacement {
            item_index: 0,
            item: item("i0"),
            position: MetricPoint3 {
                x: meters(0.0),
                y: meters(0.0),
                z: meters(0.0),
            },
            orientation: Orientation::Upright,
        };
        let packed_bin = adapter
            .to_packed_bin("bin-1".to_string(), bin_type(), None, vec![placement])
            .unwrap();

        let analyzer = ColumnGenerationPackingAnalyzer::new();
        let analysis = analyzer.analyze(vec![packed_bin]).unwrap();

        assert_eq!(analysis.packing_result.packed_bins.len(), 1);
        assert_eq!(analysis.render_loading_plans.len(), 1);
        assert_eq!(analysis.render_loading_plans[0].items.len(), 1);
    }

    #[test]
    fn column_generation_algorithm_adds_mock_layer() {
        #[derive(Debug)]
        struct MockGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for MockGenerator {
            fn name(&self) -> &str {
                "mock"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name().to_string(),
                        bin: request.bin.clone(),
                        depth: meters(1.0),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: Some(-1.0),
                    score: None,
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let config = ColumnGenerationConfig::new()
            .with_max_candidates_per_iteration(8);
        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(MockGenerator));
        let mut algorithm = ColumnGenerationAlgorithm::with_layer_generation(config, context);

        let results = algorithm.generate_once(Some(bin_type()), vec![item("i0")], vec![]);

        assert_eq!(results.len(), 1);
        assert_eq!(algorithm.layer_aggregation.layers.len(), 1);
        assert_eq!(algorithm.state.total_columns, 1);
        assert_eq!(algorithm.state.iteration, 1);
    }

    #[test]
    fn application_service_runs_mock_executor_flow() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let rmp = MockColumnGenerationRmpExecutor {
            objective: Some(1.0),
        };
        let final_executor = MockColumnGenerationFinalExecutor;

        let result = service
            .run_materialized(vec![item("i0")], vec![layer], &rmp, &final_executor)
            .unwrap();

        assert_eq!(result.result.layers.len(), 1);
        assert_eq!(result.rmp.objective, Some(1.0));
        assert_eq!(result.final_execution.layers.len(), 1);
        assert_eq!(result.result.info["rmp_executor"], "mock_rmp");
        assert_eq!(result.result.info["final_executor"], "mock_final");
    }

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
        assert_eq!(diagnostics.bin_count, 1);
        assert_eq!(execution.layers.len(), 1);
        assert_eq!(execution.layers[0].from, "seed-b");
        assert_eq!(execution.info["status"], "registered");
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

    #[test]
    fn application_service_runs_one_shadow_price_generation_round() {
        #[derive(Debug)]
        struct ShadowPriceGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for ShadowPriceGenerator {
            fn name(&self) -> &str {
                "shadow_generator"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                let key = DemandShadowPriceKey {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "i0".to_string() },
                };
                if !request.shadow_prices.contains_key(&key) {
                    return Vec::new();
                }
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name().to_string(),
                        bin: request.bin.clone(),
                        depth: meters(2.0),
                        demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                            Bpp3dDemandMode::Item,
                            Bpp3dDemandKey::Item { id: "i1".to_string() },
                            2.0,
                        )],
                    },
                    reduced_cost: Some(-1.0),
                    score: Some(1.0),
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: vec![LayerPlacementTrace {
                        item_index: 0,
                        item_id: "i1".to_string(),
                        position: MetricPoint3 {
                            x: meters(0.0),
                            y: meters(0.0),
                            z: meters(0.0),
                        },
                        orientation: Orientation::Upright,
                        amount: 1,
                    }],
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(ShadowPriceGenerator));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "shadow_round_rmp".to_string(),
            shadow_prices: vec![3.0],
            primal_solution: vec![1.0, 1.0],
            objective: Some(2.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "shadow_round_final".to_string(),
            primal_solution: Vec::new(),
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "1");
        assert_eq!(result.result.layers.len(), 2);
        assert!(result.result.layers.iter().any(|layer| layer.from == "shadow_generator"));
        assert_eq!(result.rmp.shadow_price_summary["item:i0"], 3.0);
    }

    #[test]
    fn application_service_replays_generated_layer_traces_to_render() {
        #[derive(Debug)]
        struct TraceGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for TraceGenerator {
            fn name(&self) -> &str {
                "trace_generator"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name().to_string(),
                        bin: request.bin.clone(),
                        depth: meters(1.0),
                        demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                            Bpp3dDemandMode::Item,
                            Bpp3dDemandKey::Item { id: "i0".to_string() },
                            2.0,
                        )],
                    },
                    reduced_cost: Some(-1.0),
                    score: Some(2.0),
                    numeric_score: Some(2.0),
                    block_traces: Vec::new(),
                    placement_traces: vec![LayerPlacementTrace {
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
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(TraceGenerator));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "trace_round_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "trace_round_final".to_string(),
            primal_solution: vec![0.0, 1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.final_execution.packed_bins.len(), 1);
        assert_eq!(result.final_execution.packed_bins[0].items.len(), 2);
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.render_loading_plans[0].items.len(), 2);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn application_service_replays_generated_block_traces_to_render() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(BlockLayerGenerator::new()));
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "block_round_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "block_round_final".to_string(),
            primal_solution: vec![0.0, 1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.final_execution.layers[0].from, "block_layer_generator");
        assert_eq!(result.final_execution.packed_bins.len(), 1);
        assert_eq!(result.final_execution.packed_bins[0].items.len(), 1);
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn application_service_reports_empty_generation_round() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let bin = bin_type();
        let initial_layer = BinLayer {
            iteration: 0,
            from: "seed".to_string(),
            bin: Some(bin.clone()),
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer_generation = LayerGenerationContext::new();
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "empty_generation_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "empty_generation_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_materialized_one_generation_round(
                vec![item("i0")],
                vec![bin],
                vec![initial_layer],
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "0");
        assert_eq!(result.final_execution.layers.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
    }

    #[test]
    fn application_service_reports_empty_dataset_diagnostics() {
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig::default());
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig::default());

        let result = service
            .run_materialized_with_bins(Vec::new(), Vec::new(), Vec::new(), &rmp, &final_executor)
            .unwrap();

        assert!(result.result.layers.is_empty());
        assert_eq!(result.result.info["rmp_demand_count"], "0");
        assert_eq!(result.result.info["final_final_diagnostics"], "no_selected_layer_assignment");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn csv_materialized_solver_backed_flow_runs_one_generation_round() {
        #[derive(Debug)]
        struct CsvShadowPriceGenerator;

        impl crate::domain::layer_generation::LayerGenerator<f64, Meter> for CsvShadowPriceGenerator {
            fn name(&self) -> &str {
                "csv_shadow_generator"
            }

            fn generate(
                &self,
                request: &LayerGenerationRequest<f64, Meter>,
            ) -> Vec<LayerGenerationResult<f64, Meter>> {
                let key = DemandShadowPriceKey {
                    mode: Bpp3dDemandMode::Item,
                    key: Bpp3dDemandKey::Item { id: "i1".to_string() },
                };
                if !request.shadow_prices.contains_key(&key) {
                    return Vec::new();
                }
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name().to_string(),
                        bin: request.bin.clone(),
                        depth: meters(2.0),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: Some(-1.0),
                    score: Some(1.0),
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: self.name().to_string(),
                }]
            }
        }

        #[derive(Debug, Clone)]
        struct CsvBackend;

        impl MetaModelSolverBackend for CsvBackend {
            fn name(&self) -> &str {
                "csv_fake"
            }

            fn solve_rmp(
                &self,
                _model: &MetaModel<f64>,
                diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                assert!(diagnostics.constraint_count >= diagnostics.demand_count);
                Ok(MetaModelExecutorSolveResult {
                    objective: Some(6.0 + diagnostics.layer_count as f64),
                    primal_solution: vec![1.0; diagnostics.variable_count],
                    dual_solution: vec![2.0; diagnostics.demand_count],
                    info: HashMap::from([("backend_phase".to_string(), "rmp".to_string())]),
                })
            }

            fn solve_final(
                &self,
                _model: &MetaModel<f64>,
                diagnostics: &MetaModelExecutionDiagnostics,
            ) -> Result<MetaModelExecutorSolveResult, String> {
                assert!(diagnostics.constraint_count > diagnostics.demand_count);
                let mut primal_solution = vec![0.0; diagnostics.variable_count];
                if diagnostics.variable_count > 2 {
                    primal_solution[1] = 1.0;
                    primal_solution[2] = 1.0;
                }
                Ok(MetaModelExecutorSolveResult {
                    objective: Some(1.0),
                    primal_solution,
                    dual_solution: Vec::new(),
                    info: HashMap::from([("backend_phase".to_string(), "final".to_string())]),
                })
            }
        }

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
        let request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let mut layer_generation = LayerGenerationContext::new();
        layer_generation.add_generator(Box::new(CsvShadowPriceGenerator));
        let rmp = SolverBackedMetaModelRmpExecutor::new(
            MetaModelRmpExecutorConfig::default(),
            CsvBackend,
        );
        let final_executor = SolverBackedMetaModelFinalExecutor::new(
            MetaModelFinalExecutorConfig::default(),
            CsvBackend,
        );

        let result = service
            .run_csv_materialized_one_generation_round(
                request,
                layer_generation,
                &rmp,
                &final_executor,
            )
            .unwrap();

        assert_eq!(result.result.info["generated_layer_count"], "1");
        assert_eq!(result.rmp.shadow_price_summary["item:i1"], 2.0);
        assert_eq!(result.rmp.info["backend"], "csv_fake");
        assert_eq!(result.final_execution.info["backend_phase"], "final");
        assert_eq!(result.final_execution.layers[0].from, "csv_shadow_generator");
        assert_eq!(result.result.render_loading_plans.len(), 1);
        assert_eq!(result.result.render_loading_plans[0].items.len(), 1);
        assert_eq!(result.result.info["final_final_diagnostics"], "selected_layers_renderable");
        assert!(result
            .result
            .layers
            .iter()
            .any(|layer| layer.from == "seed" || layer.from == "csv_shadow_generator"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn csv_materialized_solver_backed_flow_reports_diagnostics() {
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
        let mut request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        request.bins.clear();
        for layer in &mut request.initial_layers {
            layer.bin = None;
            layer.demand_coverage = vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: "missing".to_string() },
                1.0,
            )];
        }
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "csv_diag_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "csv_diag_final".to_string(),
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_csv_materialized_with_bins(request, &rmp, &final_executor)
            .unwrap();

        assert!(result.result.render_loading_plans.is_empty());
        assert_eq!(result.result.info["final_final_diagnostics"], "no_selected_layer_assignment");
        assert!(result
            .result
            .info["final_packing_diagnostics"]
            .contains("no available bin type for final packing"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn csv_materialized_continuous_radius_renders_conservative_radius() {
        use crate::infrastructure::renderer::{RenderAlgorithmShapeType, RenderAxis3};

        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius_min,radius_max,radius_step,axis
c1,Cylinder,cylinder,6,5,6,1,1,1,3,1,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,5
"#;
        let request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
            model_name: "pwl_rmp".to_string(),
            shadow_prices: vec![1.0],
            primal_solution: vec![1.0],
            objective: Some(1.0),
        });
        let final_executor = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
            model_name: "pwl_final".to_string(),
            primal_solution: vec![1.0, 1.0],
            objective: Some(1.0),
        });

        let result = service
            .run_csv_materialized_with_bins(request, &rmp, &final_executor)
            .unwrap();

        let item = &result.result.render_loading_plans[0].items[0];
        assert_eq!(item.algorithm_shape_type, RenderAlgorithmShapeType::VerticalCylinder);
        assert_eq!(item.axis, Some(RenderAxis3::Y));
        assert_eq!(item.radius, Some(3.0));
        assert!(result
            .result
            .info
            .values()
            .any(|value| value.contains("selected_layers_renderable")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn application_service_runs_csv_materialized_mock_flow() {
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
        let request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MockColumnGenerationRmpExecutor {
            objective: Some(3.0),
        };
        let final_executor = MockColumnGenerationFinalExecutor;

        let result = service
            .run_csv_materialized(request, &rmp, &final_executor)
            .unwrap();

        assert_eq!(result.result.layers.len(), 1);
        assert_eq!(result.rmp.objective, Some(3.0));
    }
}
