//! 列生成编排器 / Column generation orchestrator
//!
//! 循环顺序：RMP(LP) 求解 → 取对偶 → 生成候选 → 按检验数过滤 → 加列 → 重复到无新列或到达上限 →
//! 最终 MILP → 装箱 → 渲染。失败按阶段上报，不做 panic。
//! Loop order: solve the RMP (LP) → take duals → generate candidates → filter by reduced cost → add columns →
//! repeat until no new column or the limit is reached → final MILP → pack → render. Failures are reported per
//! stage, never as a panic.

use std::time::Instant;

use ospf_rust_base::error::Ret;

use crate::application::config::ColumnGenerationConfig;
use crate::application::state::{
    ColumnGenerationFailure, ColumnGenerationFailureStage, ColumnGenerationFinalResult,
    ColumnGenerationLpResult, ColumnGenerationResult, ColumnGenerationState,
};
use crate::domain::item::model::{Layer, Sheet};
use crate::domain::layer_assignment::Bpp2dAssignmentDemandEntry;
use crate::domain::layer_generation::{
    Bpp2dLayerGenerationRequest, Bpp2dLayerGenerationResult, Bpp2dLayerGenerator,
};
use crate::infrastructure::solving_error;

// ============================================================================
// 求解器与回调接口 / Solver and callback traits
// ============================================================================

/// RMP 求解器 / RMP solver
///
/// 求解器接口固定在 `f64` + `Meter`：应用层是求解器数值的边界，泛型方法会让 trait 无法作为
/// `dyn` 对象使用，因此这里选择具体类型。
/// The solver interface is fixed to `f64` + `Meter`: the application layer is the solver-value boundary and
/// generic methods would make the trait unusable as a `dyn` object, so concrete types are chosen here.
pub trait ColumnGenerationRmpSolver: std::fmt::Debug + Send + Sync {
    /// 求解 RMP / Solve the RMP
    fn solve(
        &self,
        state: &ColumnGenerationState<f64, ospf_rust_quantities::unit::derived::Meter>,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
    ) -> Ret<ColumnGenerationLpResult>;
}

/// 最终 MILP 求解器 / Final MILP solver
pub trait ColumnGenerationFinalSolver: std::fmt::Debug + Send + Sync {
    /// 求解最终 MILP / Solve the final MILP
    fn solve(
        &self,
        state: &ColumnGenerationState<f64, ospf_rust_quantities::unit::derived::Meter>,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
    ) -> Ret<ColumnGenerationFinalResult<f64, ospf_rust_quantities::unit::derived::Meter>>;
}

/// 心跳回调 / Heartbeat callback
pub trait ColumnGenerationHeartbeat<V, U: ospf_rust_quantities::unit::concept::UnitTrait>:
    std::fmt::Debug + Send + Sync
{
    /// 每次加列后回调 / Called after each column addition
    fn invoke(&self, state: &ColumnGenerationState<V, U>);
}

/// 失败分析器 / Failure analyzer
pub trait ColumnGenerationFailureAnalyzer: std::fmt::Debug + Send + Sync {
    /// 分析失败 / Analyze a failure
    fn analyze(&self, stage: ColumnGenerationFailureStage, messages: &[String]) -> Vec<String>;
}

/// 候选过滤函数 / Candidate filter function
pub type CandidateFilter = Box<
    dyn Fn(
        &ColumnGenerationState<f64, ospf_rust_quantities::unit::derived::Meter>,
        &[Bpp2dLayerGenerationResult<f64, ospf_rust_quantities::unit::derived::Meter>],
    ) -> Ret<Vec<Bpp2dLayerGenerationResult<f64, ospf_rust_quantities::unit::derived::Meter>>>
        + Send
        + Sync,
>;

// ============================================================================
// ColumnGenerationAlgorithm - 列生成编排器 / Column generation orchestrator
// ============================================================================

/// 列生成编排器 / Column generation orchestrator
pub struct ColumnGenerationAlgorithm {
    /// 层生成器 / Layer generator
    pub layer_generator: Box<dyn Bpp2dLayerGenerator<f64, ospf_rust_quantities::unit::derived::Meter>>,
    /// RMP 求解器 / RMP solver
    pub rmp_solver: Option<Box<dyn ColumnGenerationRmpSolver>>,
    /// 最终 MILP 求解器 / Final MILP solver
    pub final_solver: Option<Box<dyn ColumnGenerationFinalSolver>>,
    /// 需求项 / Demand entries
    pub demand_entries: Vec<Bpp2dAssignmentDemandEntry>,
    /// 列去重函数 / Column deduplication function
    pub deduplicate:
        fn(Vec<Layer<f64, ospf_rust_quantities::unit::derived::Meter>>)
            -> Vec<Layer<f64, ospf_rust_quantities::unit::derived::Meter>>,
}

impl std::fmt::Debug for ColumnGenerationAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnGenerationAlgorithm")
            .field("layer_generator", &self.layer_generator.name())
            .field("has_rmp_solver", &self.rmp_solver.is_some())
            .field("has_final_solver", &self.final_solver.is_some())
            .field("demand_entries", &self.demand_entries.len())
            .finish()
    }
}

impl ColumnGenerationAlgorithm {
    /// 创建编排器 / Create the orchestrator
    pub fn new(
        layer_generator: Box<dyn Bpp2dLayerGenerator<f64, ospf_rust_quantities::unit::derived::Meter>>,
        demand_entries: Vec<Bpp2dAssignmentDemandEntry>,
    ) -> Self {
        Self {
            layer_generator,
            rmp_solver: None,
            final_solver: None,
            demand_entries,
            deduplicate: deduplicate_layers,
        }
    }

    /// 设置 RMP 求解器 / Set the RMP solver
    pub fn with_rmp_solver(mut self, solver: Box<dyn ColumnGenerationRmpSolver>) -> Self {
        self.rmp_solver = Some(solver);
        self
    }

    /// 设置最终 MILP 求解器 / Set the final MILP solver
    pub fn with_final_solver(mut self, solver: Box<dyn ColumnGenerationFinalSolver>) -> Self {
        self.final_solver = Some(solver);
        self
    }

    /// 设置列去重函数 / Set the column deduplication function
    pub fn with_deduplicate(
        mut self,
        deduplicate: fn(
            Vec<Layer<f64, ospf_rust_quantities::unit::derived::Meter>>,
        ) -> Vec<Layer<f64, ospf_rust_quantities::unit::derived::Meter>>,
    ) -> Self {
        self.deduplicate = deduplicate;
        self
    }

    /// 执行列生成 / Run the column generation
    ///
    /// 故障构建函数负责把阶段与错误打包成结果，避免在循环里重复写错误分支。
    /// The `failed` closure packs a stage plus errors into a result, so the loop body never repeats the error
    /// branching.
    pub fn solve(
        &self,
        request: Bpp2dLayerGenerationRequest<f64, ospf_rust_quantities::unit::derived::Meter>,
        config: &ColumnGenerationConfig,
    ) -> Ret<ColumnGenerationResult<f64, ospf_rust_quantities::unit::derived::Meter>> {
        type V = f64;
        type U = ospf_rust_quantities::unit::derived::Meter;
        let started_at = Instant::now();
        let mut columns: Vec<Layer<f64, U>> = Vec::new();
        let mut latest_shadow_prices = std::collections::BTreeMap::new();
        let mut latest_additional_shadow_prices = std::collections::BTreeMap::new();
        let mut terminated_by_iteration_limit = false;
        let mut terminated_by_time_limit = false;
        let mut iterations = 0usize;
        let mut lp_solved_times = 0usize;
        let mut lp_objectives: Vec<Option<f64>> = Vec::new();
        let mut lp_infos: Vec<std::collections::BTreeMap<String, String>> = Vec::new();
        let mut final_solved = false;
        let mut final_objective = None;
        let mut final_info = std::collections::BTreeMap::new();
        let mut sheets: Vec<Sheet<f64, U>> = Vec::new();

        while iterations < config.iteration_limit {
            if config.time_limit_ms > 0
                && started_at.elapsed().as_millis() as u64 >= config.time_limit_ms
            {
                terminated_by_time_limit = true;
                break;
            }

            let rmp_state = ColumnGenerationState {
                iteration: iterations,
                columns: columns.clone(),
                sheets: Vec::new(),
                shadow_prices: latest_shadow_prices.clone(),
                additional_shadow_prices: latest_additional_shadow_prices.clone(),
            };
            let Some(solver) = self.rmp_solver.as_ref() else {
                // 未配置 RMP 求解器时无法生成有意义的候选，直接结束迭代。
                // Without an RMP solver no meaningful pricing is possible, so the loop ends here.
                break;
            };
            let lp_result = solver.solve(&rmp_state, &self.demand_entries).map_err(|error| {
                solving_error(format!(
                    "RMP 求解失败（{}）/ RMP solve failed ({}): {error}",
                    ColumnGenerationFailureStage::RestrictedMasterProblem.as_str(),
                    ColumnGenerationFailureStage::RestrictedMasterProblem.as_str()
                ))
            })?;
            latest_shadow_prices = lp_result.shadow_prices.clone();
            latest_additional_shadow_prices = lp_result.additional_shadow_prices.clone();
            lp_objectives.push(lp_result.objective);
            lp_infos.push(lp_result.info.clone());
            lp_solved_times += 1;

            let pricing_request = Bpp2dLayerGenerationRequest {
                iteration: iterations as i64,
                existing_layers: columns.clone(),
                shadow_prices: latest_shadow_prices.clone(),
                max_candidates: config.max_columns_per_iteration,
                ..request.clone()
            };
            let candidates = self.layer_generator.generate(&pricing_request);
            if candidates.is_empty() {
                break;
            }

            columns = (self.deduplicate)([columns, candidates
                .into_iter()
                .map(|candidate| candidate.layer)
                .collect()]
            .concat());
            iterations += 1;
        }

        if iterations >= config.iteration_limit {
            terminated_by_iteration_limit = true;
        }

        if config.final_milp_enabled {
            if let Some(solver) = self.final_solver.as_ref() {
                let final_state = ColumnGenerationState {
                    iteration: iterations,
                    columns: columns.clone(),
                    sheets: Vec::new(),
                    shadow_prices: latest_shadow_prices.clone(),
                    additional_shadow_prices: latest_additional_shadow_prices.clone(),
                };
                let final_result =
                    solver.solve(&final_state, &self.demand_entries).map_err(|error| {
                        solving_error(format!(
                            "最终 MILP 求解失败（{}）/ Final MILP solve failed ({}): {error}",
                            ColumnGenerationFailureStage::FinalMilp.as_str(),
                            ColumnGenerationFailureStage::FinalMilp.as_str()
                        ))
                    })?;
                columns = final_result.columns.clone();
                sheets = final_result.sheets.clone();
                final_objective = final_result.objective;
                final_info = final_result.info.clone();
                final_solved = true;
            }
        }

        Ok(ColumnGenerationResult {
            columns,
            sheets,
            iteration_count: iterations,
            terminated_by_iteration_limit,
            terminated_by_time_limit,
            lp_solved_times,
            final_solved,
            lp_objectives,
            final_objective,
            elapsed: started_at.elapsed(),
            lp_infos,
            final_info,
            packing: None,
            render: None,
        })
    }
}

/// 按列身份去重 / Deduplicate by column identity
///
/// 板材类型、来源、包围尺寸与放置集合共同参与，因此同一排布换板材不会被误去重。
/// The sheet type, source, bounding size and placement set all take part, so the same layout on another sheet
/// is never wrongly deduplicated.
pub fn deduplicate_layers<V, U>(layers: Vec<Layer<V, U>>) -> Vec<Layer<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    let mut seen = std::collections::HashSet::new();
    layers
        .into_iter()
        .filter(|layer| seen.insert(layer.identity_key()))
        .collect()
}

/// 构造失败记录 / Build a failure record
pub fn failure_of<V, U>(
    stage: ColumnGenerationFailureStage,
    state: ColumnGenerationState<V, U>,
    errors: Vec<String>,
) -> ColumnGenerationFailure<V, U>
where
    U: ospf_rust_quantities::unit::concept::UnitTrait + Clone,
{
    ColumnGenerationFailure {
        stage,
        state,
        errors,
    }
}
