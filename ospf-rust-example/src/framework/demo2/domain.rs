//! 领域层 / Domain layer
use ospf_rust_framework::solver::{
    BendersIterationSnapshot, BendersRuntimeMetrics,
};
use std::error::Error;

use self::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::diagnostics::{
    NOTE_CODE_BENDERS_ADAPTIVE_EFFECTIVE, NOTE_CODE_BENDERS_CUT_EFFICIENCY_LOW,
    NOTE_CODE_BENDERS_FAILED, NOTE_CODE_BENDERS_GAP, NOTE_CODE_BENDERS_GAP_GUARD_EXCEEDED,
    NOTE_CODE_BENDERS_ITERATIONS, NOTE_CODE_BENDERS_PROBLEM_SIZE_BINARY_VARIABLES,
    NOTE_CODE_BENDERS_PROGRESS_GUARD_TRIGGERED, NOTE_CODE_BENDERS_QUALITY_ACTION,
    NOTE_CODE_BENDERS_QUALITY_GUARD_EFFECTIVE, NOTE_CODE_BENDERS_QUALITY_SCORE,
    NOTE_CODE_BENDERS_TIME_GUARD_EXCEEDED, NOTE_CODE_BENDERS_TIME_MS,
    NOTE_CODE_BENDERS_TRAJECTORY_WEAK, NOTE_CODE_CAPACITY_UTILIZATION_HIGH,
    NOTE_CODE_CARGO_EXCEEDS_ALL_POSITIONS, NOTE_CODE_ENVELOPE_LONGITUDINAL_MAX_CLOSE,
    NOTE_CODE_ENVELOPE_LONGITUDINAL_MIN_CLOSE, NOTE_CODE_ENVELOPE_RANGE_INVALID,
    NOTE_CODE_LATERAL_IMBALANCE_CLOSE, NOTE_CODE_MIN_PAYLOAD_GT_TOTAL_CAPACITY,
    NOTE_CODE_MIN_PAYLOAD_GT_UPPER, NOTE_CODE_MIN_PAYLOAD_RATIO_OUT_OF_RANGE,
    NOTE_CODE_PAYLOAD_LOWER_CLOSE, NOTE_CODE_PAYLOAD_UPPER_NEGATIVE,
    NOTE_CODE_PAYLOAD_UPPER_UTILIZATION_HIGH, NOTE_CODE_REDUNDANCY_DESTINATION_CONCENTRATION_HIGH,
    NOTE_CODE_SOLVER_PATH, NOTE_GROUP_AIRWORTHINESS, NOTE_GROUP_MAC_OPTIMIZATION,
    NOTE_GROUP_PAYLOAD, NOTE_GROUP_REDUNDANCY, NOTE_GROUP_SOLVER, NOTE_LEVEL_CRITICAL,
    NOTE_LEVEL_DIAGNOSTIC, build_structured_diagnostics, push_grouped_note,
};
use crate::framework::demo2::infrastructure::dto::{
    AircraftTypeInput, BendersAdaptiveConfig, BendersQualityOverrideConfig, Demo2Request,
    Demo2Response, LoadingOrderResponse,
};

/// Demo2 领域模块，包含飞机装载与配重平衡的各子域 / Demo2 domain module containing aircraft loading and weight balance sub-domains
pub mod aircraft;
/// 适航安全约束子域 / Airworthiness security constraints sub-domain
pub mod airworthiness_security;
/// 快递效能子域 / Express cargo effectiveness sub-domain
pub mod express_effectiveness;
/// 装载效能子域 / Loading effectiveness sub-domain
pub mod loading_effectiveness;
/// 平均气动弦长（MAC）计算子域 / Mean Aerodynamic Chord calculation sub-domain
pub mod mac;
/// MAC 优化子域 / MAC optimization sub-domain
pub mod mac_optimization;
/// 载荷最大化子域 / Payload maximization sub-domain
pub mod payload_maximization;
/// 推荐重量均衡子域 / Recommended weight equalization sub-domain
pub mod recommended_weight_equalization;
/// 冗余约束子域 / Redundancy constraints sub-domain
pub mod redundancy;
/// 领域服务子域 / Domain service sub-domain
pub mod service;
/// 共享工具与类型子域 / Shared utilities and types sub-domain
pub mod shared;
/// 软安全约束子域 / Soft security constraints sub-domain
pub mod soft_security;
/// 装载方案子域 / Stowage plan sub-domain
pub mod stowage;

/// 满载应用 / Full-load application
pub struct FullLoadApplication;
/// 预分配应用 / Predistribution application
pub struct PredistributionApplication;
/// 重量推荐应用 / Weight recommendation application
pub struct WeightRecommendationApplication;
/// 装机顺序应用 / Loading order application
pub struct LoadingOrderApplication;

/// 有效的 Benders 自适应配置 / Effective Benders adaptive configuration
#[derive(Clone, Copy)]
pub(crate) struct EffectiveBendersAdaptiveConfig {
    /// 最小二值变量数阈值 / Minimum binary variable count threshold
    pub(crate) min_binary_variables: usize,
    /// 最大迭代次数 / Maximum iteration count
    pub(crate) max_iterations: usize,
    /// 收敛容差 / Convergence tolerance
    pub(crate) tolerance: f64,
    /// 最大停滞迭代次数 / Maximum stall iterations
    max_stall_iterations: Option<usize>,
    /// 目标停滞迭代次数 / Objective stall iterations
    objective_stall_iterations: Option<usize>,
}

/// 求解模式 / Solve mode
pub(crate) enum SolveMode {
    /// 混合整数线性规划直接求解 / MILP direct solve
    Milp,
    /// Benders 分解求解 / Benders decomposition solve
    Benders(EffectiveBendersAdaptiveConfig),
}

const BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED: &str = "gap_guard_exceeded";
const BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED: &str = "time_guard_exceeded";
const BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED: &str = "progress_guard_triggered";
const BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW: &str = "cut_efficiency_low";
const BENDERS_QUALITY_REASON_TRAJECTORY_WEAK: &str = "trajectory_weak";

/// Benders 质量守卫配置 / Benders quality guard configuration
#[derive(Clone, Copy)]
struct BendersQualityGuardConfig {
    /// 弱间隙乘数 / Weak gap multiplier
    weak_gap_multiplier: f64,
    /// 弱间隙下限 / Weak gap floor
    weak_gap_floor: f64,
    /// 迭代压力百分比 / Iteration pressure percentage
    iteration_pressure_percent: usize,
    /// 切割密度最小迭代数 / Cut density minimum iterations
    cut_density_min_iterations: usize,
    /// 切割密度阈值 / Cut density threshold
    cut_density_threshold: f64,
    /// 轨迹最小快照数 / Trajectory minimum snapshots
    trajectory_min_snapshots: usize,
    /// 轨迹步长乘数 / Trajectory step multiplier
    trajectory_step_multiplier: f64,
    /// 轨迹步长下限 / Trajectory step floor
    trajectory_step_floor: f64,
    /// 时间守卫最小毫秒数 / Time guard minimum milliseconds
    time_guard_min_ms: u128,
    /// 评分-间隙权重 / Score gap weight
    score_gap_weight: f64,
    /// 评分-时间权重 / Score time weight
    score_time_weight: f64,
    /// 评分-迭代权重 / Score iteration weight
    score_iteration_weight: f64,
    /// 评分-切割密度权重 / Score cut density weight
    score_cut_density_weight: f64,
    /// 评分-轨迹权重 / Score trajectory weight
    score_trajectory_weight: f64,
}

/// 检查飞机类型是否受支持 / Check if the aircraft type is supported
fn supported_aircraft(aircraft_type: AircraftTypeInput) -> bool {
    matches!(
        aircraft_type,
        AircraftTypeInput::B737 | AircraftTypeInput::B757 | AircraftTypeInput::B767 | AircraftTypeInput::B747
    )
}

/// 返回默认的 Benders 质量守卫配置 / Return default Benders quality guard configuration
fn default_benders_quality_guard_config() -> BendersQualityGuardConfig {
    BendersQualityGuardConfig {
        weak_gap_multiplier: 20.0,
        weak_gap_floor: 1e-5,
        iteration_pressure_percent: 90,
        cut_density_min_iterations: 8,
        cut_density_threshold: 0.25,
        trajectory_min_snapshots: 6,
        trajectory_step_multiplier: 20.0,
        trajectory_step_floor: 1e-6,
        time_guard_min_ms: 500,
        score_gap_weight: 0.35,
        score_time_weight: 0.2,
        score_iteration_weight: 0.2,
        score_cut_density_weight: 0.15,
        score_trajectory_weight: 0.1,
    }
}

/// 归一化 Benders 质量评分权重 / Normalize Benders quality score weights
fn normalize_benders_quality_weights(
    gap_weight: f64,
    time_weight: f64,
    iteration_weight: f64,
    cut_density_weight: f64,
    trajectory_weight: f64,
) -> (f64, f64, f64, f64, f64) {
    let gap_weight = gap_weight.max(0.0);
    let time_weight = time_weight.max(0.0);
    let iteration_weight = iteration_weight.max(0.0);
    let cut_density_weight = cut_density_weight.max(0.0);
    let trajectory_weight = trajectory_weight.max(0.0);
    let sum = gap_weight + time_weight + iteration_weight + cut_density_weight + trajectory_weight;
    if sum <= 1e-12 {
        (0.35, 0.2, 0.2, 0.15, 0.1)
    } else {
        (
            gap_weight / sum,
            time_weight / sum,
            iteration_weight / sum,
            cut_density_weight / sum,
            trajectory_weight / sum,
        )
    }
}

/// 解析 Benders 质量守卫配置（合并覆盖值） / Resolve Benders quality guard config (merging overrides)
fn resolve_benders_quality_guard_config(
    override_config: Option<BendersQualityOverrideConfig>,
) -> BendersQualityGuardConfig {
    let default_config = default_benders_quality_guard_config();
    let override_config = override_config.unwrap_or_default();
    let (
        score_gap_weight,
        score_time_weight,
        score_iteration_weight,
        score_cut_density_weight,
        score_trajectory_weight,
    ) = normalize_benders_quality_weights(
        override_config
            .score_gap_weight
            .unwrap_or(default_config.score_gap_weight),
        override_config
            .score_time_weight
            .unwrap_or(default_config.score_time_weight),
        override_config
            .score_iteration_weight
            .unwrap_or(default_config.score_iteration_weight),
        override_config
            .score_cut_density_weight
            .unwrap_or(default_config.score_cut_density_weight),
        override_config
            .score_trajectory_weight
            .unwrap_or(default_config.score_trajectory_weight),
    );
    BendersQualityGuardConfig {
        weak_gap_multiplier: override_config
            .weak_gap_multiplier
            .unwrap_or(default_config.weak_gap_multiplier)
            .max(1.0),
        weak_gap_floor: override_config
            .weak_gap_floor
            .unwrap_or(default_config.weak_gap_floor)
            .max(1e-12),
        iteration_pressure_percent: override_config
            .iteration_pressure_percent
            .unwrap_or(default_config.iteration_pressure_percent)
            .clamp(1, 100),
        cut_density_min_iterations: override_config
            .cut_density_min_iterations
            .unwrap_or(default_config.cut_density_min_iterations)
            .max(1),
        cut_density_threshold: override_config
            .cut_density_threshold
            .unwrap_or(default_config.cut_density_threshold)
            .max(0.0),
        trajectory_min_snapshots: override_config
            .trajectory_min_snapshots
            .unwrap_or(default_config.trajectory_min_snapshots)
            .max(2),
        trajectory_step_multiplier: override_config
            .trajectory_step_multiplier
            .unwrap_or(default_config.trajectory_step_multiplier)
            .max(1.0),
        trajectory_step_floor: override_config
            .trajectory_step_floor
            .unwrap_or(default_config.trajectory_step_floor)
            .max(1e-12),
        time_guard_min_ms: override_config
            .time_guard_min_ms
            .unwrap_or(default_config.time_guard_min_ms)
            .max(1),
        score_gap_weight,
        score_time_weight,
        score_iteration_weight,
        score_cut_density_weight,
        score_trajectory_weight,
    }
}

/// 根据问题规模调整 Benders 自适应配置 / Tune Benders adaptive config based on problem size
fn tune_benders_adaptive_config(
    configured: BendersAdaptiveConfig,
    binary_variables: usize,
) -> EffectiveBendersAdaptiveConfig {
    if configured.max_iterations == 0 {
        return EffectiveBendersAdaptiveConfig {
            min_binary_variables: configured.min_binary_variables,
            max_iterations: configured.max_iterations,
            tolerance: configured.tolerance,
            max_stall_iterations: None,
            objective_stall_iterations: None,
        };
    }

    let iteration_boost = if binary_variables >= 400 {
        64
    } else if binary_variables >= 200 {
        32
    } else if binary_variables >= 100 {
        16
    } else if binary_variables >= 60 {
        8
    } else {
        0
    };
    let base_tolerance = if configured.tolerance > 0.0 {
        configured.tolerance
    } else {
        1e-6
    };
    let tuned_tolerance = if binary_variables >= 400 {
        base_tolerance.max(5e-5)
    } else if binary_variables >= 200 {
        base_tolerance.max(2e-5)
    } else if binary_variables >= 100 {
        base_tolerance.max(1e-5)
    } else if binary_variables >= 60 {
        base_tolerance.max(5e-6)
    } else {
        base_tolerance
    };
    let tuned_max_iterations = configured.max_iterations.saturating_add(iteration_boost);
    let stall_window_base = if binary_variables >= 400 {
        24
    } else if binary_variables >= 200 {
        16
    } else if binary_variables >= 100 {
        12
    } else if binary_variables >= 60 {
        8
    } else {
        6
    };
    let objective_stall_window_base = if binary_variables >= 400 {
        6
    } else if binary_variables >= 200 {
        5
    } else if binary_variables >= 100 {
        4
    } else if binary_variables >= 60 {
        3
    } else {
        2
    };

    EffectiveBendersAdaptiveConfig {
        min_binary_variables: configured.min_binary_variables,
        max_iterations: tuned_max_iterations,
        tolerance: tuned_tolerance,
        max_stall_iterations: Some(stall_window_base.min(tuned_max_iterations.max(1))),
        objective_stall_iterations: Some(
            objective_stall_window_base.min(tuned_max_iterations.max(1)),
        ),
    }
}

/// 解析求解模式（MILP 或 Benders） / Resolve solve mode (MILP or Benders)
#[cfg(test)]
fn resolve_solve_mode(request: &Demo2Request, notes: &mut Vec<String>) -> SolveMode {
    resolve_solve_mode_for_size(request, request.cargos.len() * request.positions.len(), notes)
}

pub(crate) fn resolve_solve_mode_for_size(request: &Demo2Request, binary_variables: usize, notes: &mut Vec<String>) -> SolveMode {
    let mut tuned_adaptive = tune_benders_adaptive_config(request.benders_adaptive, binary_variables);
    // 安全固定追索没有旧版启发式停滞开关，不报告未执行的控制项。
    // Safe fixed recourse has no legacy heuristic stall switches; never report inactive controls as effective.
    tuned_adaptive.max_stall_iterations = None;
    tuned_adaptive.objective_stall_iterations = None;
    let quality_guard = resolve_benders_quality_guard_config(request.benders_quality_overrides);
    if request.solve_policy.prefer_benders {
        if binary_variables < tuned_adaptive.min_binary_variables {
            notes.push(format!(
                "Benders requested but skipped: binary_variables={} < threshold={}",
                binary_variables, tuned_adaptive.min_binary_variables
            ));
            return SolveMode::Milp;
        }
        notes.push(String::from(
            "Benders requested; adaptive Benders path enabled in application layer",
        ));
        notes.push(format!(
            "benders_adaptive=min_binary_variables={},max_iterations={},tolerance={:.6}",
            request.benders_adaptive.min_binary_variables,
            request.benders_adaptive.max_iterations,
            request.benders_adaptive.tolerance
        ));
        notes.push(format!(
            "benders_adaptive_effective=min_binary_variables={},max_iterations={},tolerance={:.6},max_stall_iterations={},objective_stall_iterations={}",
            tuned_adaptive.min_binary_variables,
            tuned_adaptive.max_iterations,
            tuned_adaptive.tolerance,
            tuned_adaptive
                .max_stall_iterations
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("none")),
            tuned_adaptive
                .objective_stall_iterations
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("none"))
        ));
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_SOLVER,
            NOTE_CODE_BENDERS_ADAPTIVE_EFFECTIVE,
            &format!(
                "effective min_binary_variables={},max_iterations={},tolerance={:.6},max_stall_iterations={},objective_stall_iterations={}",
                tuned_adaptive.min_binary_variables,
                tuned_adaptive.max_iterations,
                tuned_adaptive.tolerance,
                tuned_adaptive
                    .max_stall_iterations
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| String::from("none")),
                tuned_adaptive
                    .objective_stall_iterations
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| String::from("none"))
            ),
        );
        notes.push(format!(
            "benders_problem_size_binary_variables={}",
            binary_variables
        ));
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_SOLVER,
            NOTE_CODE_BENDERS_PROBLEM_SIZE_BINARY_VARIABLES,
            &format!("binary_variables={}", binary_variables),
        );
        notes.push(format!(
            "benders_quality_guard_effective=weak_gap_multiplier={:.3},weak_gap_floor={:.6},iteration_pressure_percent={},cut_density_min_iterations={},cut_density_threshold={:.3},trajectory_min_snapshots={},trajectory_step_multiplier={:.3},trajectory_step_floor={:.6},time_guard_min_ms={},score_weights=gap:{:.3}|time:{:.3}|iter:{:.3}|cut:{:.3}|traj:{:.3}",
            quality_guard.weak_gap_multiplier,
            quality_guard.weak_gap_floor,
            quality_guard.iteration_pressure_percent,
            quality_guard.cut_density_min_iterations,
            quality_guard.cut_density_threshold,
            quality_guard.trajectory_min_snapshots,
            quality_guard.trajectory_step_multiplier,
            quality_guard.trajectory_step_floor,
            quality_guard.time_guard_min_ms,
            quality_guard.score_gap_weight,
            quality_guard.score_time_weight,
            quality_guard.score_iteration_weight,
            quality_guard.score_cut_density_weight,
            quality_guard.score_trajectory_weight
        ));
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_SOLVER,
            NOTE_CODE_BENDERS_QUALITY_GUARD_EFFECTIVE,
            &format!(
                "weak_gap_multiplier={:.3},weak_gap_floor={:.6},iteration_pressure_percent={},cut_density_min_iterations={},cut_density_threshold={:.3},trajectory_min_snapshots={},trajectory_step_multiplier={:.3},trajectory_step_floor={:.6},time_guard_min_ms={}",
                quality_guard.weak_gap_multiplier,
                quality_guard.weak_gap_floor,
                quality_guard.iteration_pressure_percent,
                quality_guard.cut_density_min_iterations,
                quality_guard.cut_density_threshold,
                quality_guard.trajectory_min_snapshots,
                quality_guard.trajectory_step_multiplier,
                quality_guard.trajectory_step_floor,
                quality_guard.time_guard_min_ms
            ),
        );
        SolveMode::Benders(tuned_adaptive)
    } else {
        SolveMode::Milp
    }
}

/// 记录求解路径诊断 / Push solver path diagnostic note
pub(crate) fn push_solver_path_note(notes: &mut Vec<String>, solver_path: &str) {
    notes.push(format!("solver_path={}", solver_path));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_SOLVER_PATH,
        &format!("solver path={}", solver_path),
    );
}

/// 记录 Benders 求解失败诊断 / Push Benders failure diagnostic note
fn push_benders_failed_note(notes: &mut Vec<String>, message: &str) {
    notes.push(format!("benders_failed: {}", message));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_FAILED,
        message,
    );
}

/// 记录 Benders 运行时信息（迭代数、间隙、耗时） / Push Benders runtime notes (iterations, gap, time)
fn push_benders_runtime_notes(
    notes: &mut Vec<String>,
    benders_iterations: usize,
    benders_gap: f64,
    benders_time_ms: u128,
) {
    notes.push(format!("benders_iters={}", benders_iterations));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_ITERATIONS,
        &format!("benders iterations={}", benders_iterations),
    );
    notes.push(format!("benders_gap={:.6}", benders_gap));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_GAP,
        &format!("benders gap={:.6}", benders_gap),
    );
    notes.push(format!("benders_time_ms={}", benders_time_ms));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_TIME_MS,
        &format!("benders time_ms={}", benders_time_ms),
    );
}

/// 解析 Benders 间隙守卫阈值 / Resolve Benders gap guard threshold
fn resolve_benders_gap_guard(adaptive: &EffectiveBendersAdaptiveConfig) -> f64 {
    (adaptive.tolerance * 100.0).max(1e-4).min(0.2)
}

/// 记录 Benders 间隙守卫超限诊断 / Push Benders gap guard exceeded diagnostic note
fn push_benders_gap_guard_exceeded_note(notes: &mut Vec<String>, benders_gap: f64, gap_guard: f64) {
    notes.push(format!(
        "benders_gap_guard_exceeded: gap={:.6} > guard={:.6}",
        benders_gap, gap_guard
    ));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_GAP_GUARD_EXCEEDED,
        &format!("gap={:.6} guard={:.6}", benders_gap, gap_guard),
    );
}

/// 解析 Benders 时间守卫阈值（毫秒） / Resolve Benders time guard threshold in milliseconds
fn resolve_benders_time_guard_ms(
    adaptive: &EffectiveBendersAdaptiveConfig,
    quality_guard: &BendersQualityGuardConfig,
) -> u128 {
    let per_iteration_ms = if adaptive.tolerance >= 1e-4 {
        40_u128
    } else if adaptive.tolerance >= 1e-5 {
        60_u128
    } else {
        80_u128
    };
    let stall_bonus = adaptive.max_stall_iterations.unwrap_or(0) as u128 * 30_u128;
    (adaptive.max_iterations.max(1) as u128)
        .saturating_mul(per_iteration_ms)
        .saturating_add(stall_bonus)
        .max(quality_guard.time_guard_min_ms)
}

/// 记录 Benders 时间守卫超限诊断 / Push Benders time guard exceeded diagnostic note
fn push_benders_time_guard_exceeded_note(
    notes: &mut Vec<String>,
    benders_time_ms: u128,
    time_guard_ms: u128,
) {
    notes.push(format!(
        "benders_time_guard_exceeded: time_ms={} > guard_ms={}",
        benders_time_ms, time_guard_ms
    ));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_TIME_GUARD_EXCEEDED,
        &format!("time_ms={} guard_ms={}", benders_time_ms, time_guard_ms),
    );
}

/// 记录 Benders 进度守卫触发诊断 / Push Benders progress guard triggered diagnostic note
fn push_benders_progress_guard_triggered_note(
    notes: &mut Vec<String>,
    benders_iterations: usize,
    max_iterations: usize,
    benders_gap: f64,
    tolerance: f64,
) {
    notes.push(format!(
        "benders_progress_guard_triggered: iterations={}/{},gap={:.6},tolerance={:.6}",
        benders_iterations, max_iterations, benders_gap, tolerance
    ));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_PROGRESS_GUARD_TRIGGERED,
        &format!(
            "iterations={}/{},gap={:.6},tolerance={:.6}",
            benders_iterations, max_iterations, benders_gap, tolerance
        ),
    );
}

/// 记录 Benders 切割效率低诊断 / Push Benders cut efficiency low diagnostic note
fn push_benders_cut_efficiency_low_note(
    notes: &mut Vec<String>,
    executed_iterations: usize,
    total_cuts: usize,
    cut_density: f64,
) {
    notes.push(format!(
        "benders_cut_efficiency_low: iterations={},cuts={},cut_density={:.4}",
        executed_iterations, total_cuts, cut_density
    ));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_CUT_EFFICIENCY_LOW,
        &format!(
            "iterations={},cuts={},cut_density={:.4}",
            executed_iterations, total_cuts, cut_density
        ),
    );
}

/// 记录 Benders 轨迹弱诊断 / Push Benders trajectory weak diagnostic note
fn push_benders_trajectory_weak_note(
    notes: &mut Vec<String>,
    snapshots: &[BendersIterationSnapshot],
    avg_step_improvement: f64,
) {
    let first_obj = snapshots
        .first()
        .map(|snapshot| snapshot.master_obj)
        .unwrap_or(0.0);
    let last_obj = snapshots
        .last()
        .map(|snapshot| snapshot.master_obj)
        .unwrap_or(0.0);
    notes.push(format!(
        "benders_trajectory_weak: first_obj={:.6},last_obj={:.6},avg_step_abs_delta={:.6},iterations={}",
        first_obj,
        last_obj,
        avg_step_improvement,
        snapshots.len()
    ));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_TRAJECTORY_WEAK,
        &format!(
            "first_obj={:.6},last_obj={:.6},avg_step_abs_delta={:.6},iterations={}",
            first_obj,
            last_obj,
            avg_step_improvement,
            snapshots.len()
        ),
    );
}

/// 记录 Benders 质量动作诊断 / Push Benders quality action diagnostic note
fn push_benders_quality_action_note(notes: &mut Vec<String>, action: &str, reason: &str) {
    notes.push(format!(
        "benders_quality_action={},reason={}",
        action, reason
    ));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_QUALITY_ACTION,
        &format!("action={} reason={}", action, reason),
    );
}

/// 解析 Benders 质量评分（0-100） / Resolve Benders quality score (0-100)
fn resolve_benders_quality_score(
    adaptive: &EffectiveBendersAdaptiveConfig,
    quality_guard: &BendersQualityGuardConfig,
    benders_iterations: usize,
    benders_gap: f64,
    benders_time_ms: u128,
    benders_runtime_metrics: Option<&BendersRuntimeMetrics>,
) -> f64 {
    let gap_guard = resolve_benders_gap_guard(adaptive);
    let gap_risk = if gap_guard > 0.0 {
        (benders_gap / gap_guard).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let time_guard_ms = resolve_benders_time_guard_ms(adaptive, quality_guard);
    let time_risk = if time_guard_ms > 0 {
        (benders_time_ms as f64 / time_guard_ms as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let executed_iterations = benders_runtime_metrics
        .map(|metrics| metrics.executed_iterations)
        .unwrap_or(benders_iterations);
    let total_cuts = benders_runtime_metrics
        .map(|metrics| metrics.total_cuts)
        .unwrap_or(0);
    let iteration_risk = if adaptive.max_iterations > 0 {
        (executed_iterations as f64 / adaptive.max_iterations as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cut_density_risk = if executed_iterations >= quality_guard.cut_density_min_iterations {
        let cut_density = total_cuts as f64 / executed_iterations as f64;
        if quality_guard.cut_density_threshold <= 1e-12 {
            0.0
        } else {
            (1.0 - (cut_density / quality_guard.cut_density_threshold)).clamp(0.0, 1.0)
        }
    } else {
        0.0
    };
    let trajectory_risk = if let Some(snapshots) =
        benders_runtime_metrics.map(|metrics| &metrics.iteration_snapshots)
    {
        if snapshots.len() >= quality_guard.trajectory_min_snapshots {
            let mut abs_step_sum = 0.0_f64;
            for index in 1..snapshots.len() {
                abs_step_sum +=
                    (snapshots[index].master_obj - snapshots[index - 1].master_obj).abs();
            }
            let avg_step_improvement = abs_step_sum / ((snapshots.len() - 1) as f64);
            let step_threshold = (adaptive.tolerance * quality_guard.trajectory_step_multiplier)
                .max(quality_guard.trajectory_step_floor);
            if avg_step_improvement <= 1e-12 {
                1.0
            } else {
                (step_threshold / avg_step_improvement).clamp(0.0, 1.0)
            }
        } else {
            0.0
        }
    } else {
        0.0
    };

    let score = gap_risk * quality_guard.score_gap_weight
        + time_risk * quality_guard.score_time_weight
        + iteration_risk * quality_guard.score_iteration_weight
        + cut_density_risk * quality_guard.score_cut_density_weight
        + trajectory_risk * quality_guard.score_trajectory_weight;
    (score * 100.0).clamp(0.0, 100.0)
}

/// 基于实际分解证据观察质量，不据启发式评分改写证明或丢弃 incumbent。
/// Observe quality from actual decomposition evidence without rewriting proofs or discarding incumbents based on heuristic scores.
pub(crate) fn append_decomposition_quality_notes(request: &Demo2Request, result: &ospf_rust_framework::solver::FixedRecourseResult,
    elapsed_ms: u128, notes: &mut Vec<String>) {
    let adaptive = tune_benders_adaptive_config(request.benders_adaptive, result.values.len());
    let guard = resolve_benders_quality_guard_config(request.benders_quality_overrides);
    if let (Some(objective), Some(lower)) = (result.objective, result.lower_bound) {
        let gap = (objective - lower).abs() / objective.abs().max(1.0);
        let metrics = BendersRuntimeMetrics { executed_iterations: result.iterations,
            total_cuts: result.feasibility_cuts + result.optimality_cuts, best_solution_iteration: None, no_cut_iterations: 0,
            no_obj_improvement_iterations: 0, stop_reason: None, iteration_snapshots: vec![] };
        push_benders_runtime_notes(notes, result.iterations, gap, elapsed_ms);
        push_benders_quality_score_note(notes, resolve_benders_quality_score(&adaptive, &guard, result.iterations,
            gap, elapsed_ms, Some(&metrics)));
        if let Some(reason) = resolve_benders_quality_reason(&adaptive, &guard, result.iterations, gap, elapsed_ms, Some(&metrics)) {
            push_benders_quality_action_note(notes, "observe_only", reason);
        }
    } else { notes.push("benders_quality_gap=unknown; no fabricated bound or trajectory".into()); }
}

/// 记录 Benders 质量评分诊断 / Push Benders quality score diagnostic note
fn push_benders_quality_score_note(notes: &mut Vec<String>, quality_score: f64) {
    notes.push(format!("benders_quality_score={:.2}", quality_score));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_BENDERS_QUALITY_SCORE,
        &format!("quality_score={:.2}", quality_score),
    );
}

/// 解析 Benders 质量降级原因 / Resolve Benders quality degradation reason
fn resolve_benders_quality_reason(
    adaptive: &EffectiveBendersAdaptiveConfig,
    quality_guard: &BendersQualityGuardConfig,
    benders_iterations: usize,
    benders_gap: f64,
    benders_time_ms: u128,
    benders_runtime_metrics: Option<&BendersRuntimeMetrics>,
) -> Option<&'static str> {
    let gap_guard = resolve_benders_gap_guard(adaptive);
    if benders_gap > gap_guard + 1e-12 {
        return Some(BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED);
    }

    let time_guard_ms = resolve_benders_time_guard_ms(adaptive, quality_guard);
    let weak_gap = benders_gap
        > (adaptive.tolerance * quality_guard.weak_gap_multiplier)
            .max(quality_guard.weak_gap_floor);
    if weak_gap && benders_time_ms > time_guard_ms {
        return Some(BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED);
    }

    let executed_iterations = benders_runtime_metrics
        .map(|metrics| metrics.executed_iterations)
        .unwrap_or(benders_iterations);
    let total_cuts = benders_runtime_metrics
        .map(|metrics| metrics.total_cuts)
        .unwrap_or(0);
    let iteration_pressure = adaptive.max_iterations > 0
        && executed_iterations.saturating_mul(100)
            >= adaptive
                .max_iterations
                .saturating_mul(quality_guard.iteration_pressure_percent);
    if iteration_pressure && weak_gap {
        return Some(BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED);
    }

    if weak_gap {
        if let Some(snapshots) = benders_runtime_metrics.map(|metrics| &metrics.iteration_snapshots)
        {
            if snapshots.len() >= quality_guard.trajectory_min_snapshots {
                let mut abs_step_sum = 0.0_f64;
                for index in 1..snapshots.len() {
                    abs_step_sum +=
                        (snapshots[index].master_obj - snapshots[index - 1].master_obj).abs();
                }
                let avg_step_improvement = abs_step_sum / ((snapshots.len() - 1) as f64);
                let step_threshold = (adaptive.tolerance
                    * quality_guard.trajectory_step_multiplier)
                    .max(quality_guard.trajectory_step_floor);
                if avg_step_improvement < step_threshold {
                    return Some(BENDERS_QUALITY_REASON_TRAJECTORY_WEAK);
                }
            }
        }
    }

    if weak_gap && executed_iterations >= quality_guard.cut_density_min_iterations {
        let cut_density = (total_cuts as f64) / (executed_iterations as f64);
        if cut_density < quality_guard.cut_density_threshold {
            return Some(BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW);
        }
    }

    None
}

/// 构造无解响应 / Construct no-solution response
pub(crate) fn no_solution_response(status: &str, notes: Vec<String>) -> Demo2Response {
    let diagnostics = build_structured_diagnostics(&notes);
    Demo2Response {
        status: status.to_string(),
        objective: None,
        assignments: Vec::new(),
        notes,
        diagnostics,
        solve_report: None, algorithm_reports: vec![], render: None, kpi: None,
        framework_path: String::new(), selected_layouts: Default::default(), occupied_positions: Default::default(), additional_weights: Default::default(),
    }
}

/// 构造 Demo2 响应 / Construct Demo2 response
pub(crate) fn demo2_response(
    status: String,
    objective: Option<f64>,
    assignments: Vec<String>,
    notes: Vec<String>,
) -> Demo2Response {
    let diagnostics = build_structured_diagnostics(&notes);
    Demo2Response {
        status,
        objective,
        assignments,
        notes,
        diagnostics,
        solve_report: None, algorithm_reports: vec![], render: None, kpi: None,
        framework_path: String::new(), selected_layouts: Default::default(), occupied_positions: Default::default(), additional_weights: Default::default(),
    }
}

/// 追加核心可行性诊断（检查载荷、包线、位置容量等） / Append core feasibility diagnostics (check payload, envelope, position capacity, etc.)
pub(crate) fn append_core_feasibility_diagnostics(request: &Demo2Request, notes: &mut Vec<String>) {
    let total_capacity: f64 = request
        .positions
        .iter()
        .map(|position| position.max_weight)
        .sum();
    let total_cargo_weight: f64 = request.cargos.iter().map(|cargo| cargo.weight).sum();
    let min_payload_required =
        request.payload_upper_bound.min(total_cargo_weight) * request.min_payload_ratio;

    if request.envelope_longitudinal_moment_min > request.envelope_longitudinal_moment_max {
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_AIRWORTHINESS,
            NOTE_CODE_ENVELOPE_RANGE_INVALID,
            "infeasible: envelope_longitudinal_moment_min > envelope_longitudinal_moment_max",
        );
    }
    if request.payload_upper_bound < 0.0 {
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_PAYLOAD,
            NOTE_CODE_PAYLOAD_UPPER_NEGATIVE,
            "infeasible: payload_upper_bound < 0",
        );
    }
    if request.min_payload_ratio < 0.0 || request.min_payload_ratio > 1.0 {
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_PAYLOAD,
            NOTE_CODE_MIN_PAYLOAD_RATIO_OUT_OF_RANGE,
            "infeasible: min_payload_ratio must be within [0, 1]",
        );
    }
    if min_payload_required > request.payload_upper_bound {
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_PAYLOAD,
            NOTE_CODE_MIN_PAYLOAD_GT_UPPER,
            "infeasible: min payload requirement exceeds payload upper bound",
        );
    }
    if min_payload_required > total_capacity {
        push_grouped_note(
            notes,
            NOTE_LEVEL_DIAGNOSTIC,
            NOTE_GROUP_AIRWORTHINESS,
            NOTE_CODE_MIN_PAYLOAD_GT_TOTAL_CAPACITY,
            "infeasible: min payload requirement exceeds total position capacity",
        );
    }
    for cargo in &request.cargos {
        if request.loading.cargos.get(&cargo.name).and_then(|policy| policy.required) == Some(false) { continue; }
        let can_fit = request
            .positions
            .iter()
            .any(|position| position.max_weight + 1e-9 >= cargo.weight);
        if !can_fit {
            push_grouped_note(
                notes,
                NOTE_LEVEL_DIAGNOSTIC,
                NOTE_GROUP_AIRWORTHINESS,
                NOTE_CODE_CARGO_EXCEEDS_ALL_POSITIONS,
                &format!(
                    "infeasible: cargo {} ({:.2}) exceeds every position max_weight",
                    cargo.name, cargo.weight
                ),
            );
        }
    }
}

/// 追加关键约束诊断（容量利用率、包线接近、横向不平衡、冗余集中度） / Append critical constraint notes (capacity utilization, envelope proximity, lateral imbalance, redundancy concentration)
fn append_critical_constraint_notes(
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    solution: &[f64],
    notes: &mut Vec<String>,
) {
    append_critical_constraint_notes_with_weights(request, x_idx, solution, &vec![0.0; request.positions.len()], notes);
}

/// 使用最终源模型的完整重量诊断。 / Diagnose complete weights from the final source model.
pub(crate) fn append_critical_constraint_notes_with_weights(
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    solution: &[f64],
    additional: &[f64],
    notes: &mut Vec<String>,
) {
    const CRITICAL_RATIO: f64 = 0.98;
    const EPS: f64 = 1e-6;

    let mut position_loads = additional.to_vec();
    for c in 0..request.cargos.len() {
        for (p, load) in position_loads.iter_mut().enumerate() {
            let value = solution.get(x_idx[c][p]).copied().unwrap_or(0.0);
            *load += value * request.cargos[c].weight;
        }
    }

    for (p, load) in position_loads.iter().enumerate() {
        let capacity = request.positions[p].max_weight;
        if capacity > EPS {
            let ratio = *load / capacity;
            if ratio + EPS >= CRITICAL_RATIO {
                push_grouped_note(
                    notes,
                    NOTE_LEVEL_CRITICAL,
                    NOTE_GROUP_AIRWORTHINESS,
                    NOTE_CODE_CAPACITY_UTILIZATION_HIGH,
                    &format!(
                        "airworthiness_security_capacity_{} utilization {:.2}%",
                        request.positions[p].name,
                        ratio * 100.0
                    ),
                );
            }
        }
    }

    let total_payload: f64 = position_loads.iter().sum();
    let total_cargo_weight: f64 = request.cargos.iter().map(|cargo| cargo.weight).sum();
    let min_payload =
        request.payload_upper_bound.min(total_cargo_weight) * request.min_payload_ratio;
    if request.payload_upper_bound > EPS
        && total_payload / request.payload_upper_bound + EPS >= CRITICAL_RATIO
    {
        push_grouped_note(
            notes,
            NOTE_LEVEL_CRITICAL,
            NOTE_GROUP_PAYLOAD,
            NOTE_CODE_PAYLOAD_UPPER_UTILIZATION_HIGH,
            &format!(
                "airworthiness_security_payload_upper utilization {:.2}%",
                total_payload / request.payload_upper_bound * 100.0
            ),
        );
    }
    if min_payload > EPS && total_payload / min_payload <= 1.0 + (1.0 - CRITICAL_RATIO) {
        push_grouped_note(
            notes,
            NOTE_LEVEL_CRITICAL,
            NOTE_GROUP_PAYLOAD,
            NOTE_CODE_PAYLOAD_LOWER_CLOSE,
            &format!(
                "airworthiness_security_payload_lower payload {:.2} close to minimum {:.2}",
                total_payload, min_payload
            ),
        );
    }

    let longitudinal_moment = position_loads.iter().zip(&request.positions).map(|(weight, position)| weight * position.longitudinal_arm).sum::<f64>();
    let lateral_moment = position_loads.iter().zip(&request.positions).map(|(weight, position)| weight * position.lateral_arm).sum::<f64>();

    let upper_gap = request.envelope_longitudinal_moment_max - longitudinal_moment;
    let lower_gap = longitudinal_moment - request.envelope_longitudinal_moment_min;
    let envelope_span =
        request.envelope_longitudinal_moment_max - request.envelope_longitudinal_moment_min;
    if envelope_span > EPS {
        if upper_gap / envelope_span <= (1.0 - CRITICAL_RATIO) + EPS {
            push_grouped_note(
                notes,
                NOTE_LEVEL_CRITICAL,
                NOTE_GROUP_AIRWORTHINESS,
                NOTE_CODE_ENVELOPE_LONGITUDINAL_MAX_CLOSE,
                &format!(
                    "airworthiness_security_envelope_longitudinal_max close ({:.3})",
                    longitudinal_moment
                ),
            );
        }
        if lower_gap / envelope_span <= (1.0 - CRITICAL_RATIO) + EPS {
            push_grouped_note(
                notes,
                NOTE_LEVEL_CRITICAL,
                NOTE_GROUP_AIRWORTHINESS,
                NOTE_CODE_ENVELOPE_LONGITUDINAL_MIN_CLOSE,
                &format!(
                    "airworthiness_security_envelope_longitudinal_min close ({:.3})",
                    longitudinal_moment
                ),
            );
        }
    }

    if request.max_lateral_imbalance > EPS
        && lateral_moment.abs() / request.max_lateral_imbalance + EPS >= CRITICAL_RATIO
    {
        push_grouped_note(
            notes,
            NOTE_LEVEL_CRITICAL,
            NOTE_GROUP_MAC_OPTIMIZATION,
            NOTE_CODE_LATERAL_IMBALANCE_CLOSE,
            &format!(
                "mac_lateral imbalance {:.3} close to limit {:.3}",
                lateral_moment.abs(),
                request.max_lateral_imbalance
            ),
        );
    }

    for destination in request
        .cargos
        .iter()
        .map(|cargo| cargo.destination.as_str())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let destination_cargos: Vec<usize> = request
            .cargos
            .iter()
            .enumerate()
            .filter_map(|(index, cargo)| (cargo.destination == destination).then_some(index))
            .collect();
        if destination_cargos.len() < 3 {
            continue;
        }
        let max_on_single_position = (destination_cargos.len() - 1) as f64;
        if max_on_single_position <= EPS {
            continue;
        }
        for p in 0..request.positions.len() {
            let loaded_count = destination_cargos
                .iter()
                .map(|cargo_idx| solution.get(x_idx[*cargo_idx][p]).copied().unwrap_or(0.0))
                .sum::<f64>();
            let ratio = loaded_count / max_on_single_position;
            if ratio + EPS >= CRITICAL_RATIO {
                push_grouped_note(
                    notes,
                    NOTE_LEVEL_CRITICAL,
                    NOTE_GROUP_REDUNDANCY,
                    NOTE_CODE_REDUNDANCY_DESTINATION_CONCENTRATION_HIGH,
                    &format!(
                        "redundancy_destination_{} concentration {:.2}% at {}",
                        destination,
                        ratio * 100.0,
                        request.positions[p].name
                    ),
                );
            }
        }
    }
}

impl FullLoadApplication {
    /// 执行满载业务，共享完整装载模型。 / Execute the business path using the complete shared loading model.
    pub fn execute(&self, request: Demo2Request) -> Result<Demo2Response, Box<dyn Error>> {
        self.execute_with_handle(request, &ospf_rust_core::solver::SolveHandle::new())
    }

    /// 使用外部取消句柄执行。 / Execute with an external cancellation handle.
    pub fn execute_with_handle(&self, request: Demo2Request, handle: &ospf_rust_core::solver::SolveHandle) -> Result<Demo2Response, Box<dyn Error>> {
        Ok(crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService::solve(request,
            Demo2PipelineMode::FullLoad, handle)?)
    }

    /// 使用心跳、完成通知和最终渲染选项执行。 / Execute with heartbeat, completion and final-render options.
    pub fn execute_with_options(&self, request: Demo2Request,
        options: crate::framework::demo2::infrastructure::application_service::Demo2ExecutionOptions<'_>) -> Result<Demo2Response, Box<dyn Error>> {
        Ok(crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService::solve_with_execution_options(
            request, Demo2PipelineMode::FullLoad, options)?)
    }

}

impl PredistributionApplication {
    /// 执行预分配业务，共享完整装载模型。 / Execute the business path using the complete shared loading model.
    pub fn execute(&self, request: Demo2Request) -> Result<Demo2Response, Box<dyn Error>> {
        self.execute_with_handle(request, &ospf_rust_core::solver::SolveHandle::new())
    }

    /// 使用外部取消句柄执行。 / Execute with an external cancellation handle.
    pub fn execute_with_handle(&self, request: Demo2Request, handle: &ospf_rust_core::solver::SolveHandle) -> Result<Demo2Response, Box<dyn Error>> {
        Ok(crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService::solve(request,
            Demo2PipelineMode::Predistribution, handle)?)
    }

    /// 使用心跳、完成通知和最终渲染选项执行。 / Execute with heartbeat, completion and final-render options.
    pub fn execute_with_options(&self, request: Demo2Request,
        options: crate::framework::demo2::infrastructure::application_service::Demo2ExecutionOptions<'_>) -> Result<Demo2Response, Box<dyn Error>> {
        Ok(crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService::solve_with_execution_options(
            request, Demo2PipelineMode::Predistribution, options)?)
    }

}

impl WeightRecommendationApplication {
    /// 执行重量推荐业务，共享完整装载模型。 / Execute the business path using the complete shared loading model.
    pub fn execute(&self, request: Demo2Request) -> Result<Demo2Response, Box<dyn Error>> {
        self.execute_with_handle(request, &ospf_rust_core::solver::SolveHandle::new())
    }

    /// 使用外部取消句柄执行。 / Execute with an external cancellation handle.
    pub fn execute_with_handle(&self, request: Demo2Request, handle: &ospf_rust_core::solver::SolveHandle) -> Result<Demo2Response, Box<dyn Error>> {
        Ok(crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService::solve(request,
            Demo2PipelineMode::WeightRecommendation, handle)?)
    }

    /// 使用心跳、完成通知和最终渲染选项执行。 / Execute with heartbeat, completion and final-render options.
    pub fn execute_with_options(&self, request: Demo2Request,
        options: crate::framework::demo2::infrastructure::application_service::Demo2ExecutionOptions<'_>) -> Result<Demo2Response, Box<dyn Error>> {
        Ok(crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService::solve_with_execution_options(
            request, Demo2PipelineMode::WeightRecommendation, options)?)
    }

}

impl LoadingOrderApplication {
    /// 执行装机顺序应用 / Execute loading order application
    pub fn execute(&self, request: Demo2Request) -> Result<LoadingOrderResponse, Box<dyn Error>> {
        self.init(&request)?;
        let mut notes = Vec::new();
        if !supported_aircraft(request.aircraft_type) {
            notes.push(format!(
                "unsupported aircraft type for loading-order path: {:?}",
                request.aircraft_type
            ));
            let diagnostics = build_structured_diagnostics(&notes);
            return Ok(LoadingOrderResponse {
                status: String::from("UnsupportedAircraft"),
                orders: Vec::new(),
                notes,
                diagnostics,
            });
        }
        self.export_loading_order(&request, notes)
    }

    fn init(&self, request: &Demo2Request) -> Result<(), Box<dyn Error>> {
        if request.cargos.is_empty() {
            return Err(String::from("demo2 request has no cargo").into());
        }
        Ok(())
    }

    fn export_loading_order(
        &self,
        request: &Demo2Request,
        notes: Vec<String>,
    ) -> Result<LoadingOrderResponse, Box<dyn Error>> {
        let mut cargos = request.cargos.clone();
        cargos.sort_by(|lhs, rhs| {
            rhs.weight
                .partial_cmp(&lhs.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let orders: Vec<String> = cargos
            .iter()
            .enumerate()
            .map(|(idx, cargo)| format!("{}. {} ({:.2})", idx + 1, cargo.name, cargo.weight))
            .collect();
        let diagnostics = build_structured_diagnostics(&notes);

        Ok(LoadingOrderResponse {
            status: String::from("OK"),
            orders,
            notes,
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::demo2::infrastructure::dto::{CargoInput, PositionInput, SolvePolicy};
    #[test]
    fn tune_benders_adaptive_config_scales_iterations_and_tolerance() {
        let configured = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 32,
            tolerance: 1e-6,
        };
        let tuned = tune_benders_adaptive_config(configured, 240);

        assert_eq!(tuned.min_binary_variables, 1);
        assert_eq!(tuned.max_iterations, 64);
        assert!((tuned.tolerance - 2e-5).abs() <= 1e-12);
        assert_eq!(tuned.max_stall_iterations, Some(16));
        assert_eq!(tuned.objective_stall_iterations, Some(5));
    }

    #[test]
    fn tune_benders_adaptive_config_keeps_zero_iterations_for_failure_injection() {
        let configured = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };
        let tuned = tune_benders_adaptive_config(configured, 400);

        assert_eq!(tuned.max_iterations, 0);
        assert!((tuned.tolerance - 1e-6).abs() <= 1e-12);
        assert_eq!(tuned.max_stall_iterations, None);
        assert_eq!(tuned.objective_stall_iterations, None);
    }

    #[test]
    fn resolve_benders_quality_reason_prefers_gap_guard() {
        let adaptive = EffectiveBendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 64,
            tolerance: 1e-6,
            max_stall_iterations: Some(8),
            objective_stall_iterations: Some(3),
        };
        let quality_guard = default_benders_quality_guard_config();

        let reason = resolve_benders_quality_reason(&adaptive, &quality_guard, 10, 0.3, 400, None);
        assert_eq!(reason, Some(BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED));
    }

    #[test]
    fn resolve_benders_quality_reason_uses_time_guard_on_weak_gap() {
        let adaptive = EffectiveBendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 32,
            tolerance: 1e-6,
            max_stall_iterations: Some(6),
            objective_stall_iterations: Some(2),
        };
        let quality_guard = default_benders_quality_guard_config();
        let time_guard_ms = resolve_benders_time_guard_ms(&adaptive, &quality_guard);

        let reason = resolve_benders_quality_reason(
            &adaptive,
            &quality_guard,
            8,
            5e-5,
            time_guard_ms + 1,
            None,
        );
        assert_eq!(reason, Some(BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED));
    }

    #[test]
    fn resolve_benders_quality_reason_uses_progress_guard_near_iteration_cap() {
        let adaptive = EffectiveBendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 40,
            tolerance: 1e-6,
            max_stall_iterations: Some(6),
            objective_stall_iterations: Some(2),
        };
        let quality_guard = default_benders_quality_guard_config();
        let time_guard_ms = resolve_benders_time_guard_ms(&adaptive, &quality_guard);

        let reason = resolve_benders_quality_reason(
            &adaptive,
            &quality_guard,
            38,
            5e-5,
            time_guard_ms,
            None,
        );
        assert_eq!(
            reason,
            Some(BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED)
        );
    }

    #[test]
    fn resolve_benders_quality_reason_detects_low_cut_efficiency_from_runtime_metrics() {
        let adaptive = EffectiveBendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 200,
            tolerance: 1e-6,
            max_stall_iterations: Some(16),
            objective_stall_iterations: Some(4),
        };
        let runtime_metrics = BendersRuntimeMetrics {
            executed_iterations: 20,
            best_solution_iteration: Some(9),
            total_cuts: 2,
            no_cut_iterations: 3,
            no_obj_improvement_iterations: 1,
            stop_reason: None,
            iteration_snapshots: Vec::new(),
        };
        let quality_guard = default_benders_quality_guard_config();
        let time_guard_ms = resolve_benders_time_guard_ms(&adaptive, &quality_guard);

        let reason = resolve_benders_quality_reason(
            &adaptive,
            &quality_guard,
            9,
            5e-5,
            time_guard_ms,
            Some(&runtime_metrics),
        );
        assert_eq!(reason, Some(BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW));
    }

    #[test]
    fn resolve_benders_quality_reason_detects_trajectory_weak_from_iteration_snapshots() {
        let adaptive = EffectiveBendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 200,
            tolerance: 1e-6,
            max_stall_iterations: Some(16),
            objective_stall_iterations: Some(4),
        };
        let snapshots = (1..=8)
            .map(|iteration| BendersIterationSnapshot {
                iteration,
                master_obj: 100.0 + (iteration as f64) * 1e-7,
                master_gap: Some(1e-4),
                cuts_added: 1,
                total_cuts: iteration,
                no_cut_iterations: 0,
                no_obj_improvement_iterations: 0,
                ..BendersIterationSnapshot::default()
            })
            .collect();
        let runtime_metrics = BendersRuntimeMetrics {
            executed_iterations: 8,
            best_solution_iteration: Some(8),
            total_cuts: 8,
            no_cut_iterations: 0,
            no_obj_improvement_iterations: 0,
            stop_reason: None,
            iteration_snapshots: snapshots,
        };
        let quality_guard = default_benders_quality_guard_config();
        let time_guard_ms = resolve_benders_time_guard_ms(&adaptive, &quality_guard);

        let reason = resolve_benders_quality_reason(
            &adaptive,
            &quality_guard,
            8,
            5e-5,
            time_guard_ms,
            Some(&runtime_metrics),
        );
        assert_eq!(reason, Some(BENDERS_QUALITY_REASON_TRAJECTORY_WEAK));
    }

    #[test]
    fn resolve_benders_quality_guard_config_applies_overrides() {
        let quality_guard =
            resolve_benders_quality_guard_config(Some(BendersQualityOverrideConfig {
                weak_gap_multiplier: Some(30.0),
                weak_gap_floor: Some(2e-5),
                iteration_pressure_percent: Some(80),
                cut_density_min_iterations: Some(10),
                cut_density_threshold: Some(0.4),
                trajectory_min_snapshots: Some(8),
                trajectory_step_multiplier: Some(25.0),
                trajectory_step_floor: Some(2e-6),
                time_guard_min_ms: Some(1200),
                score_gap_weight: Some(0.4),
                score_time_weight: Some(0.3),
                score_iteration_weight: Some(0.1),
                score_cut_density_weight: Some(0.1),
                score_trajectory_weight: Some(0.1),
            }));

        assert!((quality_guard.weak_gap_multiplier - 30.0).abs() <= 1e-12);
        assert!((quality_guard.weak_gap_floor - 2e-5).abs() <= 1e-12);
        assert_eq!(quality_guard.iteration_pressure_percent, 80);
        assert_eq!(quality_guard.cut_density_min_iterations, 10);
        assert!((quality_guard.cut_density_threshold - 0.4).abs() <= 1e-12);
        assert_eq!(quality_guard.trajectory_min_snapshots, 8);
        assert!((quality_guard.trajectory_step_multiplier - 25.0).abs() <= 1e-12);
        assert!((quality_guard.trajectory_step_floor - 2e-6).abs() <= 1e-12);
        assert_eq!(quality_guard.time_guard_min_ms, 1200);
        assert!((quality_guard.score_gap_weight - 0.4).abs() <= 1e-12);
        assert!((quality_guard.score_time_weight - 0.3).abs() <= 1e-12);
        assert!((quality_guard.score_iteration_weight - 0.1).abs() <= 1e-12);
        assert!((quality_guard.score_cut_density_weight - 0.1).abs() <= 1e-12);
        assert!((quality_guard.score_trajectory_weight - 0.1).abs() <= 1e-12);
    }

    #[test]
    fn resolve_solve_mode_emits_effective_benders_adaptive_notes() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
            source_milp_finishing: Default::default(),
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 32,
            tolerance: 1e-6,
        };
        request.cargos = (0..20)
            .map(|idx| CargoInput {
                name: format!("C{}", idx),
                weight: 1.0,
                priority: 1,
                source: String::from("S"),
                destination: String::from("D"),
                requires_separation: false,
                code: None,
            })
            .collect();
        request.positions = (0..10)
            .map(|idx| PositionInput {
                name: format!("P{}", idx),
                max_weight: 100.0,
                longitudinal_arm: idx as f64,
                lateral_arm: 0.0,
                area: 5.0,
                length: 2.0,
                max_load_count: 3,
                loaded_items: Vec::new(),
                predicate_load_weight_min: None,
            })
            .collect();

        let mut notes = Vec::new();
        match resolve_solve_mode(&request, &mut notes) {
            SolveMode::Milp => panic!("expected benders mode for large problem"),
            SolveMode::Benders(adaptive) => {
                assert_eq!(adaptive.max_iterations, 64);
                assert!((adaptive.tolerance - 2e-5).abs() <= 1e-12);
                assert_eq!(adaptive.max_stall_iterations, None);
                assert_eq!(adaptive.objective_stall_iterations, None);
            }
        }
        assert!(
            notes
                .iter()
                .any(|note| note.contains("benders_adaptive_effective="))
        );
        assert!(
            notes
                .iter()
                .any(|note| note.contains("max_stall_iterations=none"))
        );
        assert!(
            notes
                .iter()
                .any(|note| note.contains("objective_stall_iterations=none"))
        );
        assert!(
            notes
                .iter()
                .any(|note| note.contains("code=benders_adaptive_effective"))
        );
        assert!(
            notes
                .iter()
                .any(|note| note.contains("code=benders_problem_size_binary_variables"))
        );
    }

    #[test]
    fn no_solution_response_includes_structured_diagnostics() {
        let mut request = Demo2Request::sample();
        request.cargos[0].weight = 100.0;

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("hard-infeasible data should be mapped into response");

        assert!(["Infeasible", "Unknown"].contains(&output.status.as_str()));
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("airworthiness")
                && note.code.as_deref() == Some("cargo_exceeds_all_positions")
        }));
    }

    #[test]
    fn feasible_response_includes_structured_critical_diagnostics() {
        let mut request = Demo2Request::sample();
        request.cargos = vec![CargoInput {
            name: String::from("H1"),
            weight: 10.0,
            priority: 10,
            source: String::from("S1"),
            destination: String::from("D1"),
            requires_separation: false,
            code: None,
        }];
        request.positions = vec![PositionInput {
            name: String::from("P1"),
            max_weight: 10.0,
            longitudinal_arm: 0.0,
            lateral_arm: 0.0,
            area: 5.0,
            length: 2.0,
            max_load_count: 3,
            loaded_items: Vec::new(),
            predicate_load_weight_min: None,
        }];
        request.loading.positions.retain(|id, _| id == "P1");
        request.loading.adjacent_positions.clear();
        request.adjacent_positions.clear();
        request.payload_upper_bound = 10.0;
        request.min_payload_ratio = 0.0;
        request.max_adjacent_load_gap = 10.0;
        request.max_cumulative_forward_load = 10.0;
        request.max_cumulative_backward_load = 10.0;
        request.envelope_longitudinal_moment_min = -100.0;
        request.envelope_longitudinal_moment_max = 100.0;
        request.max_longitudinal_moment_deviation = 100.0;
        request.max_lateral_imbalance = 100.0;

        let app = FullLoadApplication;
        let output = app.execute(request).expect("full-load should run");

        assert!(
            output.status == "Optimal" || output.status == "Feasible",
            "unexpected status `{}` with notes {:?}",
            output.status,
            output.notes
        );
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "critical"
                && note.group.as_deref() == Some("airworthiness")
                && note.code.as_deref() == Some("capacity_utilization_high")
        }));
    }

    #[test]
    fn feasible_response_includes_redundancy_critical_diagnostics() {
        let mut request = Demo2Request::sample();
        request.cargos = vec![
            CargoInput {
                name: String::from("D1_1"),
                weight: 2.0,
                priority: 9,
                source: String::from("S1"),
                destination: String::from("D1"),
                requires_separation: false,
                code: None,
            },
            CargoInput {
                name: String::from("D1_2"),
                weight: 2.0,
                priority: 8,
                source: String::from("S2"),
                destination: String::from("D1"),
                requires_separation: false,
                code: None,
            },
            CargoInput {
                name: String::from("D1_3"),
                weight: 2.0,
                priority: 7,
                source: String::from("S3"),
                destination: String::from("D1"),
                requires_separation: false,
                code: None,
            },
        ];
        request.positions = vec![
            PositionInput {
                name: String::from("P1"),
                max_weight: 10.0,
                longitudinal_arm: 0.0,
                lateral_arm: 0.0,
                area: 5.0,
                length: 2.0,
                max_load_count: 3,
                loaded_items: Vec::new(),
                predicate_load_weight_min: None,
            },
            PositionInput {
                name: String::from("P2"),
                max_weight: 10.0,
                longitudinal_arm: 0.0,
                lateral_arm: 0.0,
                area: 5.0,
                length: 2.0,
                max_load_count: 3,
                loaded_items: Vec::new(),
                predicate_load_weight_min: None,
            },
        ];
        request.payload_upper_bound = 20.0;
        request.min_payload_ratio = 0.0;
        request.max_adjacent_load_gap = 20.0;
        request.max_cumulative_forward_load = 20.0;
        request.max_cumulative_backward_load = 20.0;
        request.envelope_longitudinal_moment_min = -100.0;
        request.envelope_longitudinal_moment_max = 100.0;
        request.max_longitudinal_moment_deviation = 100.0;
        request.max_lateral_imbalance = 100.0;

        let app = FullLoadApplication;
        let output = app.execute(request).expect("full-load should run");

        assert!(
            output.status == "Optimal" || output.status == "Feasible",
            "unexpected status `{}` with notes {:?}",
            output.status,
            output.notes
        );
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "critical"
                && note.group.as_deref() == Some("redundancy")
                && note.code.as_deref() == Some("destination_concentration_high")
        }));
    }
}

#[cfg(test)]
mod migration_tests;
