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
    /// 连续半径模型组件 / Continuous radius model component
    pub continuous_radius_component: Option<ContinuousRadiusModelComponent>,
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
    /// 连续半径求解结果 / Continuous radius selected solutions
    pub selected_radius_solutions: Vec<ContinuousCylinderRadiusSolution>,
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

/// 求解器 backend survey 报告 / Solver backend survey report
#[derive(Debug, Clone, Default)]
pub struct SolverBackendSurveyReport {
    /// suite 名称 / Suite name
    pub suite_name: String,
    /// backend 名称 / Backend name
    pub backend: String,
    /// feature 名称 / Feature name
    pub feature: String,
    /// 编译 feature 是否启用 / Whether compile feature is enabled
    pub feature_enabled: bool,
    /// dataset 数量 / Dataset count
    pub dataset_count: usize,
    /// 是否已加载 / Whether loaded
    pub loaded: bool,
    /// 是否物化 / Whether materialized
    pub materialized: bool,
    /// 是否完成 no-run / Whether no-run validation passed
    pub no_run_validated: bool,
    /// 是否执行 fake fallback / Whether fake fallback executed
    pub fallback_executed: bool,
    /// 是否需要 fallback / Whether fallback is required
    pub fallback_required: bool,
    /// backend smoke 期望数量 / Backend smoke expected count
    pub backend_smoke_count: usize,
    /// 渲染方案数量 / Render plan count
    pub render_plan_total: usize,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

fn solver_feature_enabled(feature: &str) -> bool {
    match feature {
        "serde" => cfg!(feature = "serde"),
        "async" => cfg!(feature = "async"),
        "gurobi10" => cfg!(feature = "gurobi10"),
        "gurobi11" => cfg!(feature = "gurobi11"),
        "gurobi12" => cfg!(feature = "gurobi12"),
        "scip" => cfg!(feature = "scip"),
        "scip-bundled" => cfg!(feature = "scip-bundled"),
        "scip-from-source" => cfg!(feature = "scip-from-source"),
        "scip-quadratic" => cfg!(feature = "scip-quadratic"),
        _ => false,
    }
}

fn backend_feature_diagnostics(backend: &str, feature: &str) -> Vec<String> {
    let feature_enabled = solver_feature_enabled(feature);
    vec![format!(
        "backend '{}' feature '{}' compile_enabled={}; fallback_available={}",
        backend,
        feature,
        feature_enabled,
        backend == "fake" || backend == "noop",
    )]
}

