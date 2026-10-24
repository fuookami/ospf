//! 列生成状态与结果 / Column generation state and results
//!
//! 状态是算法与执行器之间的唯一交换面：执行器只读状态，算法只写状态。
//! The state is the only surface exchanged between the algorithm and the executors: executors only read it and
//! the algorithm only writes it.

use std::collections::BTreeMap;
use std::time::Duration;

use ospf_rust_base::error::Ret;

use ospf_rust_framework_geometry2d::Sheet2Shape;

use crate::domain::item::model::{DemandModeKey, ItemId, Layer, Sheet, SheetType};
use crate::domain::layer_assignment::Bpp2dAssignmentDemandEntry;
use crate::domain::packing::PackingResult;
use crate::infrastructure::dto::RenderLoadingPlanDto;

/// 列生成状态 / Column generation state
#[derive(Debug, Clone)]
pub struct ColumnGenerationState<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 当前迭代 / Current iteration
    pub iteration: usize,
    /// 当前列集合 / Current column set
    pub columns: Vec<Layer<V, U>>,
    /// 最终板材（可选）/ Final sheets (optional)
    pub sheets: Vec<Sheet<V, U>>,
    /// 需求影子价格 / Demand shadow prices
    pub shadow_prices: BTreeMap<DemandModeKey, f64>,
    /// 扩展影子价格 / Extension shadow prices
    pub additional_shadow_prices: BTreeMap<String, f64>,
}

impl<V, U> ColumnGenerationState<V, U>
where
    U: ospf_rust_quantities::unit::concept::UnitTrait + Clone,
{
    /// 创建初始状态 / Create the initial state
    pub fn new() -> Self {
        Self {
            iteration: 0,
            columns: Vec::new(),
            sheets: Vec::new(),
            shadow_prices: BTreeMap::new(),
            additional_shadow_prices: BTreeMap::new(),
        }
    }

    /// 是否已有列 / Whether any column exists
    pub fn has_columns(&self) -> bool {
        !self.columns.is_empty()
    }
}

impl<V, U> Default for ColumnGenerationState<V, U>
where
    U: ospf_rust_quantities::unit::concept::UnitTrait + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LP 与最终求解结果 / LP and final solve results
// ============================================================================

/// LP 终态 / LP terminal status
///
/// 与 csp1d 的 `LpTerminalStatus` 同构：只有最优允许交付定价结果，其余状态都必须走显式失败路径。
/// Isomorphic to csp1d's `LpTerminalStatus`: only optimal may deliver a pricing result; every other state must
/// take the explicit failure path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bpp2dLpTerminalStatus {
    /// 取得最优解 / Optimal solution obtained
    #[default]
    Optimal,
    /// 模型不可行 / The model is infeasible
    Infeasible,
    /// 模型无界 / The model is unbounded
    Unbounded,
    /// 其它非最优终态 / Any other non-optimal terminal state
    Other,
}

impl Bpp2dLpTerminalStatus {
    /// 是否为最优 / Whether optimal
    pub fn is_optimal(&self) -> bool {
        matches!(self, Self::Optimal)
    }

    /// 是否允许交付定价结果 / Whether a pricing result may be delivered
    pub fn allows_pricing(&self) -> bool {
        self.is_optimal()
    }
}

/// LP 终态是否允许定价，非最优时给出显式错误 / Validate that a terminal state allows pricing
pub fn require_pricing_ready(status: Bpp2dLpTerminalStatus) -> Ret<()> {
    if status.allows_pricing() {
        return Ok(());
    }
    Err(crate::infrastructure::error::validation_error(format!(
        "LP 终态 {status:?} 没有可用于定价的最优解 / LP terminal state {status:?} has no optimal pricing solution"
    )))
}

/// LP 求解结果 / LP solve result
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationLpResult {
    /// LP 终态 / LP terminal status
    ///
    /// 默认 `Optimal` 以兼容既有的启发式注入函数；接入真实求解器时实现方必须如实回报，
    /// 因为非最优终态不允许交付定价结果。
    /// Defaults to `Optimal` for compatibility with the existing heuristic injections; a real solver must report
    /// it faithfully, because a non-optimal terminal state may not deliver a pricing result.
    pub status: Bpp2dLpTerminalStatus,
    /// 需求影子价格 / Demand shadow prices
    pub shadow_prices: BTreeMap<DemandModeKey, f64>,
    /// 目标值 / Objective value
    pub objective: Option<f64>,
    /// 附加信息 / Additional information
    pub info: BTreeMap<String, String>,
    /// 扩展影子价格 / Extension shadow prices
    pub additional_shadow_prices: BTreeMap<String, f64>,
}

/// 最终 MILP 求解结果 / Final MILP solve result
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationFinalResult<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 最终列集合 / Final column set
    pub columns: Vec<Layer<V, U>>,
    /// 最终板材 / Final sheets
    pub sheets: Vec<Sheet<V, U>>,
    /// 目标值 / Objective value
    pub objective: Option<f64>,
    /// 附加信息 / Additional information
    pub info: BTreeMap<String, String>,
}

// ============================================================================
// ColumnGenerationResult - 列生成完整结果 / Complete column generation result
// ============================================================================

/// 列生成完整结果 / Complete column generation result
#[derive(Debug, Clone)]
pub struct ColumnGenerationResult<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 最终列集合 / Final column set
    pub columns: Vec<Layer<V, U>>,
    /// 最终板材 / Final sheets
    pub sheets: Vec<Sheet<V, U>>,
    /// 迭代次数 / Iteration count
    pub iteration_count: usize,
    /// 是否因迭代上限终止 / Whether terminated by the iteration limit
    pub terminated_by_iteration_limit: bool,
    /// 是否因时间上限终止 / Whether terminated by the time limit
    pub terminated_by_time_limit: bool,
    /// LP 求解次数 / LP solve count
    pub lp_solved_times: usize,
    /// 是否求解了最终 MILP / Whether the final MILP was solved
    pub final_solved: bool,
    /// 各次 LP 目标值 / LP objective per iteration
    pub lp_objectives: Vec<Option<f64>>,
    /// 最终目标值 / Final objective
    pub final_objective: Option<f64>,
    /// 耗时 / Elapsed time
    pub elapsed: Duration,
    /// 各次 LP 求解信息 / LP solve info per iteration
    pub lp_infos: Vec<BTreeMap<String, String>>,
    /// 最终求解信息 / Final solve info
    pub final_info: BTreeMap<String, String>,
    /// 装箱结果 / Packing result
    pub packing: Option<PackingResult<V, U>>,
    /// 渲染 DTO / Render DTO
    pub render: Option<RenderLoadingPlanDto>,
}

impl<V, U> ColumnGenerationResult<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    /// 列数量 / Column count
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// 板材数量 / Sheet count
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// 是否完整成功 / Whether the run fully succeeded
    pub fn is_success(&self) -> bool {
        self.final_solved && !self.columns.is_empty()
    }
}

// ============================================================================
// 失败阶段 / Failure stages
// ============================================================================

/// 列生成失败阶段 / Column-generation failure stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnGenerationFailureStage {
    /// 初始列 / Initial columns
    InitialColumns,
    /// RMP 求解 / Restricted master problem
    RestrictedMasterProblem,
    /// 候选过滤 / Candidate filter
    CandidateFilter,
    /// 最终 MILP 求解 / Final MILP
    FinalMilp,
    /// 解分析 / Solution analysis
    SolutionAnalysis,
}

impl ColumnGenerationFailureStage {
    /// 稳定文本 / Stable text
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InitialColumns => "initial_columns",
            Self::RestrictedMasterProblem => "restricted_master_problem",
            Self::CandidateFilter => "candidate_filter",
            Self::FinalMilp => "final_milp",
            Self::SolutionAnalysis => "solution_analysis",
        }
    }
}

/// 列生成失败 / Column-generation failure
#[derive(Debug, Clone)]
pub struct ColumnGenerationFailure<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 失败阶段 / Failure stage
    pub stage: ColumnGenerationFailureStage,
    /// 失败时的状态 / State at failure
    pub state: ColumnGenerationState<V, U>,
    /// 错误信息 / Error messages
    pub errors: Vec<String>,
}

/// 列生成入口输入 / Column generation input
///
/// 把一次求解所需的问题数据聚合成一个结构，避免参数列表继续膨胀。
/// Aggregates the problem data of one solve, preventing the parameter list from growing further.
#[derive(Debug, Clone)]
pub struct ColumnGenerationInput<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 主体材类型 / Primary sheet type
    pub sheet: SheetType<V, U>,
    /// 参与生成的板材类型 / Sheet types taking part in generation
    pub sheets: Vec<SheetType<V, U>>,
    /// 物品与件数 / Items and amounts
    pub items: Vec<(crate::domain::item::model::ActualItem<V, U>, usize)>,
    /// 需求项 / Demand entries
    pub demands: Vec<Bpp2dAssignmentDemandEntry>,
    /// 刀缝间距 / Kerf spacing
    pub kerf: Option<ospf_rust_quantities::quantity::Quantity<V, U>>,
    /// 板材几何 / Sheet geometry
    pub sheet_shape: Sheet2Shape<V, U>,
    /// 物品目录：编号到是否允许旋转 / Item catalogue: identifier to rotation allowance
    pub item_catalog: BTreeMap<ItemId, bool>,
}
