// ============================================================================
// 报告明细 DTO / Report detail DTOs
// ============================================================================

/// 需求覆盖报告条目 / Demand coverage report entry
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dDemandCoverageReport {
    /// 需求模式 / Demand mode
    pub mode: String,
    /// 需求键 / Demand key
    pub key: String,
    /// 覆盖系数 / Coverage coefficient
    pub coefficient: f64,
}

/// 选中层报告 / Selected layer report
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dSelectedLayerReport {
    /// 层序号 / Layer index
    pub index: usize,
    /// 迭代编号 / Iteration index
    pub iteration: i64,
    /// 来源 / Source
    pub from: String,
    /// 箱型编码 / Bin type code
    pub bin_type: Option<String>,
    /// 层深度 / Layer depth
    pub depth: f64,
    /// 需求覆盖 / Demand coverage
    pub coverage: Vec<Bpp3dDemandCoverageReport>,
}

/// 已装箱箱报告 / Packed bin report
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dPackedBinReport {
    /// 箱名称 / Bin name
    pub name: String,
    /// 箱型编码 / Bin type code
    pub bin_type: String,
    /// 已装箱货物数量 / Packed item count
    pub item_count: usize,
    /// 批号 / Batch number
    pub batch_no: Option<String>,
    /// 装载顺序列表 / Loading order list
    pub loading_orders: Vec<u64>,
}

// ============================================================================
// Bpp3dFixtureReport - 单个 fixture 报告 / Per-fixture report
// ============================================================================

/// 单个 fixture 报告 / Per-fixture report
///
/// 统一 fake / no-run / 真实 backend 的报告格式，包括 objective、coverage、
/// diagnostics、render 和 selected layer/bin assignment 字段。
/// Unifies fake / no-run / real backend report format, including objective,
/// coverage, diagnostics, render, and selected layer/bin assignment fields.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dFixtureReport {
    /// fixture 名称 / Fixture name
    pub name: String,
    /// fixture 分组 / Fixture group
    pub group: Option<String>,
    /// fixture 标签 / Fixture tags
    pub tags: Vec<String>,
    /// 运行状态 / Run status
    pub status: Bpp3dFixtureStatus,
    /// 运行耗时 / Run duration
    pub duration_ms: u64,
    /// backend 名称 / Backend name
    pub backend: Option<String>,
    /// feature 名称 / Feature name
    pub feature: Option<String>,
    /// RMP 目标值 / RMP objective value
    pub rmp_objective: Option<f64>,
    /// 最终 MILP 目标值 / Final MILP objective value
    pub final_objective: Option<f64>,
    /// 求解器模型状态 / Solver model status
    pub model_status: Option<Bpp3dSolverModelStatus>,
    /// 层候选数量 / Layer candidate count
    pub layer_count: usize,
    /// 选中层数量 / Selected layer count
    pub selected_layer_count: usize,
    /// 装箱数量 / Packed bin count
    pub packed_bin_count: usize,
    /// 渲染方案数量 / Render plan count
    pub render_plan_count: usize,
    /// 选中层明细 / Selected layer details
    pub selected_layers: Vec<Bpp3dSelectedLayerReport>,
    /// 已装箱箱明细 / Packed bin details
    pub packed_bins: Vec<Bpp3dPackedBinReport>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
    /// 失败信息（如有）/ Failure info (if any)
    pub failure: Option<Bpp3dSolverFailure>,
}

impl Bpp3dFixtureReport {
    /// 创建成功报告 / Create success report
    pub fn success(name: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            name: name.into(),
            group: None,
            tags: Vec::new(),
            status: Bpp3dFixtureStatus::Success,
            duration_ms,
            backend: None,
            feature: None,
            rmp_objective: None,
            final_objective: None,
            model_status: None,
            layer_count: 0,
            selected_layer_count: 0,
            packed_bin_count: 0,
            render_plan_count: 0,
            selected_layers: Vec::new(),
            packed_bins: Vec::new(),
            diagnostics: Vec::new(),
            failure: None,
        }
    }

    /// 创建失败报告 / Create failure report
    pub fn failed(name: impl Into<String>, duration_ms: u64, failure: Bpp3dSolverFailure) -> Self {
        let diagnostics = vec![failure.summary()];
        Self {
            name: name.into(),
            group: None,
            tags: Vec::new(),
            status: Bpp3dFixtureStatus::Failed,
            duration_ms,
            backend: None,
            feature: None,
            rmp_objective: None,
            final_objective: None,
            model_status: Some(failure.model_status.clone()),
            layer_count: 0,
            selected_layer_count: 0,
            packed_bin_count: 0,
            render_plan_count: 0,
            selected_layers: Vec::new(),
            packed_bins: Vec::new(),
            diagnostics,
            failure: Some(failure),
        }
    }

    /// 创建跳过报告 / Create skipped report
    pub fn skipped(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            group: None,
            tags: Vec::new(),
            status: Bpp3dFixtureStatus::Skipped,
            duration_ms: 0,
            backend: None,
            feature: None,
            rmp_objective: None,
            final_objective: None,
            model_status: None,
            layer_count: 0,
            selected_layer_count: 0,
            packed_bin_count: 0,
            render_plan_count: 0,
            selected_layers: Vec::new(),
            packed_bins: Vec::new(),
            diagnostics: vec!["fixture skipped".to_string()],
            failure: None,
        }
    }

    /// 创建带元数据的跳过报告 / Create skipped report with metadata
    pub fn skipped_with_metadata(
        name: impl Into<String>,
        group: Option<String>,
        tags: Vec<String>,
        backend: Option<String>,
        feature: Option<String>,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            group,
            tags,
            status: Bpp3dFixtureStatus::Skipped,
            duration_ms: 0,
            backend,
            feature,
            rmp_objective: None,
            final_objective: None,
            model_status: None,
            layer_count: 0,
            selected_layer_count: 0,
            packed_bin_count: 0,
            render_plan_count: 0,
            selected_layers: Vec::new(),
            packed_bins: Vec::new(),
            diagnostics,
            failure: None,
        }
    }

    /// 是否为 backend smoke fixture / Whether this is a backend smoke fixture
    pub fn is_backend_smoke(&self) -> bool {
        self.tags.iter().any(|tag| tag == "backend-smoke")
    }
}

