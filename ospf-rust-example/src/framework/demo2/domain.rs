use std::error::Error;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::solver::{FeasibleSolverOutput, solvers::GurobiSolver};
use ospf_rust_core::variable::{UContinuousVariableItem, VariableId};
use ospf_rust_framework::solver::{

    BendersIterationSnapshot, BendersRuntimeMetrics, FeasibleSolutionV, FrameworkSolveOptions,
    GurobiLinearBendersDecompositionSolver, LinearBendersDecompositionSolver,
};

use self::service::domain_pipeline::apply_domain_pipeline;
use self::shared::pipeline_mode::Demo2PipelineMode;
use crate::example_modeling::solve_linear_meta_model_typed_if_feasible;
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

pub mod aircraft;
pub mod airworthiness_security;
pub mod express_effectiveness;
pub mod loading_effectiveness;
pub mod mac;
pub mod mac_optimization;
pub mod payload_maximization;
pub mod recommended_weight_equalization;
pub mod redundancy;
pub mod service;
pub mod shared;
pub mod soft_security;
pub mod stowage;

pub struct FullLoadApplication;
pub struct PredistributionApplication;
pub struct WeightRecommendationApplication;
pub struct LoadingOrderApplication;

#[derive(Clone, Copy)]
struct EffectiveBendersAdaptiveConfig {
    min_binary_variables: usize,
    max_iterations: usize,
    tolerance: f64,
    max_stall_iterations: Option<usize>,
    objective_stall_iterations: Option<usize>,
}

enum SolveMode {
    Milp,
    Benders(EffectiveBendersAdaptiveConfig),
}

const BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED: &str = "gap_guard_exceeded";
const BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED: &str = "time_guard_exceeded";
const BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED: &str = "progress_guard_triggered";
const BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW: &str = "cut_efficiency_low";
const BENDERS_QUALITY_REASON_TRAJECTORY_WEAK: &str = "trajectory_weak";

#[derive(Clone, Copy)]
struct BendersQualityGuardConfig {
    weak_gap_multiplier: f64,
    weak_gap_floor: f64,
    iteration_pressure_percent: usize,
    cut_density_min_iterations: usize,
    cut_density_threshold: f64,
    trajectory_min_snapshots: usize,
    trajectory_step_multiplier: f64,
    trajectory_step_floor: f64,
    time_guard_min_ms: u128,
    score_gap_weight: f64,
    score_time_weight: f64,
    score_iteration_weight: f64,
    score_cut_density_weight: f64,
    score_trajectory_weight: f64,
}

fn supported_aircraft(aircraft_type: AircraftTypeInput) -> bool {
    matches!(
        aircraft_type,
        AircraftTypeInput::B737 | AircraftTypeInput::B757
    )
}

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

fn resolve_solve_mode(request: &Demo2Request, notes: &mut Vec<String>) -> SolveMode {
    let binary_variables = request.cargos.len() * request.positions.len();
    let tuned_adaptive = tune_benders_adaptive_config(request.benders_adaptive, binary_variables);
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

fn push_solver_path_note(notes: &mut Vec<String>, solver_path: &str) {
    notes.push(format!("solver_path={}", solver_path));
    push_grouped_note(
        notes,
        NOTE_LEVEL_DIAGNOSTIC,
        NOTE_GROUP_SOLVER,
        NOTE_CODE_SOLVER_PATH,
        &format!("solver path={}", solver_path),
    );
}

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

fn resolve_benders_gap_guard(adaptive: &EffectiveBendersAdaptiveConfig) -> f64 {
    (adaptive.tolerance * 100.0).max(1e-4).min(0.2)
}

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

fn no_solution_response(status: &str, notes: Vec<String>) -> Demo2Response {
    let diagnostics = build_structured_diagnostics(&notes);
    Demo2Response {
        status: status.to_string(),
        objective: None,
        assignments: Vec::new(),
        notes,
        diagnostics,
    }
}

fn demo2_response(
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
    }
}

fn append_core_feasibility_diagnostics(request: &Demo2Request, notes: &mut Vec<String>) {
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

fn append_critical_constraint_notes(
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    solution: &[f64],
    notes: &mut Vec<String>,
) {
    const CRITICAL_RATIO: f64 = 0.98;
    const EPS: f64 = 1e-6;

    let mut position_loads = vec![0.0_f64; request.positions.len()];
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

    let mut longitudinal_moment = 0.0_f64;
    let mut lateral_moment = 0.0_f64;
    for c in 0..request.cargos.len() {
        for p in 0..request.positions.len() {
            let value = solution.get(x_idx[c][p]).copied().unwrap_or(0.0);
            let weight = value * request.cargos[c].weight;
            longitudinal_moment += weight * request.positions[p].longitudinal_arm;
            lateral_moment += weight * request.positions[p].lateral_arm;
        }
    }

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

fn solve_linear_benders(
    master_model: &MetaModel<f64>,
    sub_model: &MetaModel<f64>,
    fixed_variable_ids: Vec<VariableId>,
    adaptive: EffectiveBendersAdaptiveConfig,
) -> Result<FeasibleSolutionV<f64>, Box<dyn Error>> {
    let mechanism_model = sub_model.try_to_mechanism_model()?;
    let solver = GurobiLinearBendersDecompositionSolver::new().with_cut_context(
        mechanism_model,
        None,
        fixed_variable_ids,
    );
    let mut options =
        FrameworkSolveOptions::new().with_iterations(adaptive.max_iterations, adaptive.tolerance);
    if let Some(max_stall_iterations) = adaptive.max_stall_iterations {
        options = options.with_stall_iterations(max_stall_iterations);
    }
    if let Some(objective_stall_iterations) = adaptive.objective_stall_iterations {
        options = options.with_objective_stall_iterations(objective_stall_iterations);
    }
    Ok(solver.solve_meta_typed_with_options(master_model, sub_model, options)?)
}

fn solve_meta_typed_if_feasible(
    model: MetaModel<f64>,
) -> Result<Option<FeasibleSolverOutput<f64>>, Box<dyn Error>> {
    let solver = GurobiSolver::new();
    solve_linear_meta_model_typed_if_feasible(model, &solver)
}

fn analyze_solution_vector(
    request: &Demo2Request,
    x_idx: &[Vec<usize>],
    objective: f64,
    solution: &[f64],
    mut notes: Vec<String>,
) -> Demo2Response {
    let mut assignments = Vec::new();
    for c in 0..request.cargos.len() {
        for p in 0..request.positions.len() {
            let value = solution.get(x_idx[c][p]).copied().unwrap_or(0.0);
            if value > 0.5 {
                assignments.push(format!(
                    "{} -> {}",
                    request.cargos[c].name, request.positions[p].name
                ));
            }
        }
    }
    append_critical_constraint_notes(request, x_idx, solution, &mut notes);
    demo2_response(String::from("Optimal"), Some(objective), assignments, notes)
}

impl FullLoadApplication {
    pub fn execute(&self, request: Demo2Request) -> Result<Demo2Response, Box<dyn Error>> {
        self.init(&request)?;
        let mut notes = Vec::new();
        append_core_feasibility_diagnostics(&request, &mut notes);
        if !notes.is_empty() {
            return Ok(no_solution_response("NoSolution", notes));
        }
        if !supported_aircraft(request.aircraft_type) {
            notes.push(format!(
                "unsupported aircraft type for full-load path: {:?}",
                request.aircraft_type
            ));
            return Ok(no_solution_response("UnsupportedAircraft", notes));
        }
        let quality_guard = resolve_benders_quality_guard_config(request.benders_quality_overrides);
        match resolve_solve_mode(&request, &mut notes) {
            SolveMode::Milp => {
                push_solver_path_note(&mut notes, "milp_direct");
            }
            SolveMode::Benders(adaptive) => {
                match self.solve_benders(&request, adaptive) {
                    Ok((x_idx, result)) => {
                        let benders_iters = result.benders_iterations.unwrap_or(0);
                        let benders_time_ms = result.time.as_millis();
                        let quality_reason = resolve_benders_quality_reason(
                            &adaptive,
                            &quality_guard,
                            benders_iters,
                            result.gap,
                            benders_time_ms,
                            result.benders_runtime_metrics.as_ref(),
                        );
                        if let Some(reason) = quality_reason {
                            match reason {
                                BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED => {
                                    let benders_gap_guard = resolve_benders_gap_guard(&adaptive);
                                    push_benders_gap_guard_exceeded_note(
                                        &mut notes,
                                        result.gap,
                                        benders_gap_guard,
                                    );
                                }
                                BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED => {
                                    let time_guard_ms =
                                        resolve_benders_time_guard_ms(&adaptive, &quality_guard);
                                    push_benders_time_guard_exceeded_note(
                                        &mut notes,
                                        benders_time_ms,
                                        time_guard_ms,
                                    );
                                }
                                BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED => {
                                    push_benders_progress_guard_triggered_note(
                                        &mut notes,
                                        result
                                            .benders_runtime_metrics
                                            .as_ref()
                                            .map(|metrics| metrics.executed_iterations)
                                            .unwrap_or(benders_iters),
                                        adaptive.max_iterations,
                                        result.gap,
                                        adaptive.tolerance,
                                    );
                                }
                                BENDERS_QUALITY_REASON_TRAJECTORY_WEAK => {
                                    if let Some(metrics) = result.benders_runtime_metrics.as_ref() {
                                        let snapshots = &metrics.iteration_snapshots;
                                        if snapshots.len() >= 2 {
                                            let mut abs_step_sum = 0.0_f64;
                                            for index in 1..snapshots.len() {
                                                abs_step_sum += (snapshots[index].master_obj
                                                    - snapshots[index - 1].master_obj)
                                                    .abs();
                                            }
                                            let avg_step_improvement =
                                                abs_step_sum / ((snapshots.len() - 1) as f64);
                                            push_benders_trajectory_weak_note(
                                                &mut notes,
                                                snapshots,
                                                avg_step_improvement,
                                            );
                                        }
                                    }
                                }
                                BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW => {
                                    let executed_iterations = result
                                        .benders_runtime_metrics
                                        .as_ref()
                                        .map(|metrics| metrics.executed_iterations)
                                        .unwrap_or(benders_iters);
                                    let total_cuts = result
                                        .benders_runtime_metrics
                                        .as_ref()
                                        .map(|metrics| metrics.total_cuts)
                                        .unwrap_or(0);
                                    let cut_density = if executed_iterations > 0 {
                                        (total_cuts as f64) / (executed_iterations as f64)
                                    } else {
                                        0.0
                                    };
                                    push_benders_cut_efficiency_low_note(
                                        &mut notes,
                                        executed_iterations,
                                        total_cuts,
                                        cut_density,
                                    );
                                }
                                _ => {}
                            }
                            push_benders_runtime_notes(
                                &mut notes,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                            );
                            let benders_quality_score = resolve_benders_quality_score(
                                &adaptive,
                                &quality_guard,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                                result.benders_runtime_metrics.as_ref(),
                            );
                            push_benders_quality_score_note(&mut notes, benders_quality_score);
                            if request.solve_policy.benders_fallback_to_milp {
                                push_benders_quality_action_note(
                                    &mut notes,
                                    "fallback_to_milp",
                                    reason,
                                );
                            } else {
                                push_solver_path_note(&mut notes, "benders");
                                push_benders_quality_action_note(
                                    &mut notes,
                                    "accept_benders",
                                    reason,
                                );
                                return Ok(analyze_solution_vector(
                                    &request,
                                    &x_idx,
                                    result.obj,
                                    &result.solution,
                                    notes,
                                ));
                            }
                        } else {
                            push_solver_path_note(&mut notes, "benders");
                            push_benders_runtime_notes(
                                &mut notes,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                            );
                            let benders_quality_score = resolve_benders_quality_score(
                                &adaptive,
                                &quality_guard,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                                result.benders_runtime_metrics.as_ref(),
                            );
                            push_benders_quality_score_note(&mut notes, benders_quality_score);
                            return Ok(analyze_solution_vector(
                                &request,
                                &x_idx,
                                result.obj,
                                &result.solution,
                                notes,
                            ));
                        }
                    }
                    Err(err) => {
                        push_benders_failed_note(&mut notes, &err.to_string());
                        if !request.solve_policy.benders_fallback_to_milp {
                            return Ok(no_solution_response("BendersFailed", notes));
                        }
                    }
                }
                push_solver_path_note(&mut notes, "milp_fallback");
            }
        }

        let mut model = MetaModel::<f64>::new("framework_demo2_full_load");
        let x_idx = self.register(&request, &mut model)?;
        let output = self.solve(model)?;
        self.analyze(&request, &x_idx, output, notes)
    }

    fn init(&self, request: &Demo2Request) -> Result<(), Box<dyn Error>> {
        if request.cargos.is_empty() {
            return Err(String::from("demo2 request has no cargo").into());
        }
        if request.positions.is_empty() {
            return Err(String::from("demo2 request has no position").into());
        }
        Ok(())
    }

    fn register(
        &self,
        request: &Demo2Request,
        model: &mut MetaModel<f64>,
    ) -> Result<Vec<Vec<usize>>, Box<dyn Error>> {
        let registration = shared::model_registration::register_variables(
            request, model, "x", Demo2PipelineMode::FullLoad,
        )?;
        shared::model_registration::construct_objective(
            request, model, &registration, Demo2PipelineMode::FullLoad,
        )?;
        apply_domain_pipeline(
            Demo2PipelineMode::FullLoad, model, request,
            &registration.x_idx, registration.z,
            &registration.estimate_load_weight_idx,
            &registration.estimate_loaded_idx,
            &registration.loaded_idx,
        )?;

        Ok(registration.x_idx)
    }

    fn build_benders_models(
        &self,
        request: &Demo2Request,
    ) -> Result<
        (
            MetaModel<f64>,
            MetaModel<f64>,
            Vec<Vec<usize>>,
            Vec<VariableId>,
        ),
        Box<dyn Error>,
    > {
        let mut master_model = MetaModel::<f64>::new("framework_demo2_full_load_master");
        let mut sub_model = MetaModel::<f64>::new("framework_demo2_full_load_sub");

        let (x_idx_master, x_idx_sub, fixed_variable_ids) =
            shared::model_registration::register_benders_variables(
                request, &mut master_model, &mut sub_model, "x",
            )?;

        let registration = shared::model_registration::RegistrationResult {
            x_idx: x_idx_master.clone(),
            z: None,
            estimate_load_weight_idx: Vec::new(),
            estimate_loaded_idx: Vec::new(),
            loaded_idx: Vec::new(),
        };
        shared::model_registration::construct_objective(
            request, &mut master_model, &registration, Demo2PipelineMode::FullLoad,
        )?;

        // 注册中间符号 for master model
        let master_intermediates = shared::model_registration::register_intermediate_symbols(
            request, &mut master_model, &x_idx_master, 10000,
        )?;
        // 注册中间符号 for sub model
        let sub_intermediates = shared::model_registration::register_intermediate_symbols(
            request, &mut sub_model, &x_idx_sub, 10000,
        )?;

        stowage::service::apply_stowage_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::FullLoad,
            &master_intermediates.loaded_idx,
            &master_intermediates.estimate_loaded_idx,
        )?;
        loading_effectiveness::service::apply_loading_effectiveness_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::FullLoad,
        )?;
        express_effectiveness::service::apply_express_effectiveness_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::FullLoad,
        )?;
        soft_security::service::apply_soft_security_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::FullLoad,
        )?;
        redundancy::service::apply_redundancy_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::FullLoad,
        )?;
        mac_optimization::service::apply_mac_optimization_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            None,
            Demo2PipelineMode::FullLoad,
        )?;

        airworthiness_security::service::apply_airworthiness_security_pipeline(
            &mut sub_model,
            request,
            &x_idx_sub,
            Demo2PipelineMode::FullLoad,
            &sub_intermediates.estimate_load_weight_idx,
            &sub_intermediates.estimate_loaded_idx,
        )?;

        Ok((master_model, sub_model, x_idx_master, fixed_variable_ids))
    }

    fn solve_benders(
        &self,
        request: &Demo2Request,
        adaptive: EffectiveBendersAdaptiveConfig,
    ) -> Result<(Vec<Vec<usize>>, FeasibleSolutionV<f64>), Box<dyn Error>> {
        let (master_model, sub_model, x_idx_master, fixed_variable_ids) =
            self.build_benders_models(request)?;
        let benders_result =
            solve_linear_benders(&master_model, &sub_model, fixed_variable_ids, adaptive)?;
        Ok((x_idx_master, benders_result))
    }

    fn solve(
        &self,
        model: MetaModel<f64>,
    ) -> Result<Option<FeasibleSolverOutput<f64>>, Box<dyn Error>> {
        solve_meta_typed_if_feasible(model)
    }

    fn analyze(
        &self,
        request: &Demo2Request,
        x_idx: &[Vec<usize>],
        output: Option<FeasibleSolverOutput<f64>>,
        mut notes: Vec<String>,
    ) -> Result<Demo2Response, Box<dyn Error>> {
        let Some(feasible_output) = output else {
            notes.push(String::from(
                "solver returned no feasible solution; mapped to explicit NoSolution response",
            ));
            return Ok(no_solution_response("NoSolution", notes));
        };
        let mut assignments = Vec::new();
        let solution = feasible_output.solution;

        for c in 0..request.cargos.len() {
            for p in 0..request.positions.len() {
                let value = solution.get(x_idx[c][p]).copied().unwrap_or(0.0);
                if value > 0.5 {
                    assignments.push(format!(
                        "{} -> {}",
                        request.cargos[c].name, request.positions[p].name
                    ));
                }
            }
        }
        append_critical_constraint_notes(request, x_idx, &solution, &mut notes);

        Ok(demo2_response(
            format!("{:?}", feasible_output.status),
            feasible_output.objective_value,
            assignments,
            notes,
        ))
    }
}

impl PredistributionApplication {
    pub fn execute(&self, request: Demo2Request) -> Result<Demo2Response, Box<dyn Error>> {
        self.init(&request)?;
        let mut notes = Vec::new();
        append_core_feasibility_diagnostics(&request, &mut notes);
        if !notes.is_empty() {
            return Ok(no_solution_response("NoSolution", notes));
        }
        if !supported_aircraft(request.aircraft_type) {
            notes.push(format!(
                "unsupported aircraft type for predistribution path: {:?}",
                request.aircraft_type
            ));
            return Ok(no_solution_response("UnsupportedAircraft", notes));
        }
        let quality_guard = resolve_benders_quality_guard_config(request.benders_quality_overrides);
        match resolve_solve_mode(&request, &mut notes) {
            SolveMode::Milp => {
                push_solver_path_note(&mut notes, "milp_direct");
            }
            SolveMode::Benders(adaptive) => {
                match self.solve_benders(&request, adaptive) {
                    Ok((x_idx, result)) => {
                        let benders_iters = result.benders_iterations.unwrap_or(0);
                        let benders_time_ms = result.time.as_millis();
                        let quality_reason = resolve_benders_quality_reason(
                            &adaptive,
                            &quality_guard,
                            benders_iters,
                            result.gap,
                            benders_time_ms,
                            result.benders_runtime_metrics.as_ref(),
                        );
                        if let Some(reason) = quality_reason {
                            match reason {
                                BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED => {
                                    let benders_gap_guard = resolve_benders_gap_guard(&adaptive);
                                    push_benders_gap_guard_exceeded_note(
                                        &mut notes,
                                        result.gap,
                                        benders_gap_guard,
                                    );
                                }
                                BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED => {
                                    let time_guard_ms =
                                        resolve_benders_time_guard_ms(&adaptive, &quality_guard);
                                    push_benders_time_guard_exceeded_note(
                                        &mut notes,
                                        benders_time_ms,
                                        time_guard_ms,
                                    );
                                }
                                BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED => {
                                    push_benders_progress_guard_triggered_note(
                                        &mut notes,
                                        result
                                            .benders_runtime_metrics
                                            .as_ref()
                                            .map(|metrics| metrics.executed_iterations)
                                            .unwrap_or(benders_iters),
                                        adaptive.max_iterations,
                                        result.gap,
                                        adaptive.tolerance,
                                    );
                                }
                                BENDERS_QUALITY_REASON_TRAJECTORY_WEAK => {
                                    if let Some(metrics) = result.benders_runtime_metrics.as_ref() {
                                        let snapshots = &metrics.iteration_snapshots;
                                        if snapshots.len() >= 2 {
                                            let mut abs_step_sum = 0.0_f64;
                                            for index in 1..snapshots.len() {
                                                abs_step_sum += (snapshots[index].master_obj
                                                    - snapshots[index - 1].master_obj)
                                                    .abs();
                                            }
                                            let avg_step_improvement =
                                                abs_step_sum / ((snapshots.len() - 1) as f64);
                                            push_benders_trajectory_weak_note(
                                                &mut notes,
                                                snapshots,
                                                avg_step_improvement,
                                            );
                                        }
                                    }
                                }
                                BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW => {
                                    let executed_iterations = result
                                        .benders_runtime_metrics
                                        .as_ref()
                                        .map(|metrics| metrics.executed_iterations)
                                        .unwrap_or(benders_iters);
                                    let total_cuts = result
                                        .benders_runtime_metrics
                                        .as_ref()
                                        .map(|metrics| metrics.total_cuts)
                                        .unwrap_or(0);
                                    let cut_density = if executed_iterations > 0 {
                                        (total_cuts as f64) / (executed_iterations as f64)
                                    } else {
                                        0.0
                                    };
                                    push_benders_cut_efficiency_low_note(
                                        &mut notes,
                                        executed_iterations,
                                        total_cuts,
                                        cut_density,
                                    );
                                }
                                _ => {}
                            }
                            push_benders_runtime_notes(
                                &mut notes,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                            );
                            let benders_quality_score = resolve_benders_quality_score(
                                &adaptive,
                                &quality_guard,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                                result.benders_runtime_metrics.as_ref(),
                            );
                            push_benders_quality_score_note(&mut notes, benders_quality_score);
                            if request.solve_policy.benders_fallback_to_milp {
                                push_benders_quality_action_note(
                                    &mut notes,
                                    "fallback_to_milp",
                                    reason,
                                );
                            } else {
                                push_solver_path_note(&mut notes, "benders");
                                push_benders_quality_action_note(
                                    &mut notes,
                                    "accept_benders",
                                    reason,
                                );
                                return Ok(analyze_solution_vector(
                                    &request,
                                    &x_idx,
                                    result.obj,
                                    &result.solution,
                                    notes,
                                ));
                            }
                        } else {
                            push_solver_path_note(&mut notes, "benders");
                            push_benders_runtime_notes(
                                &mut notes,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                            );
                            let benders_quality_score = resolve_benders_quality_score(
                                &adaptive,
                                &quality_guard,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                                result.benders_runtime_metrics.as_ref(),
                            );
                            push_benders_quality_score_note(&mut notes, benders_quality_score);
                            return Ok(analyze_solution_vector(
                                &request,
                                &x_idx,
                                result.obj,
                                &result.solution,
                                notes,
                            ));
                        }
                    }
                    Err(err) => {
                        push_benders_failed_note(&mut notes, &err.to_string());
                    }
                }
                if !request.solve_policy.benders_fallback_to_milp {
                    return Ok(no_solution_response("BendersFailed", notes));
                }
                push_solver_path_note(&mut notes, "milp_fallback");
            }
        }

        let mut model = MetaModel::<f64>::new("framework_demo2_predistribution");
        let x_idx = self.register(&request, &mut model)?;
        let output = self.solve(model)?;
        self.analyze(&request, &x_idx, output, notes)
    }

    fn init(&self, request: &Demo2Request) -> Result<(), Box<dyn Error>> {
        if request.cargos.is_empty() {
            return Err(String::from("demo2 request has no cargo").into());
        }
        if request.positions.is_empty() {
            return Err(String::from("demo2 request has no position").into());
        }
        Ok(())
    }

    fn register(
        &self,
        request: &Demo2Request,
        model: &mut MetaModel<f64>,
    ) -> Result<Vec<Vec<usize>>, Box<dyn Error>> {
        let registration = shared::model_registration::register_variables(
            request, model, "x_pre", Demo2PipelineMode::Predistribution,
        )?;
        shared::model_registration::construct_objective(
            request, model, &registration, Demo2PipelineMode::Predistribution,
        )?;
        apply_domain_pipeline(
            Demo2PipelineMode::Predistribution,
            model,
            request,
            &registration.x_idx,
            registration.z,
            &registration.estimate_load_weight_idx,
            &registration.estimate_loaded_idx,
            &registration.loaded_idx,
        )?;

        Ok(registration.x_idx)
    }

    fn solve(
        &self,
        model: MetaModel<f64>,
    ) -> Result<Option<FeasibleSolverOutput<f64>>, Box<dyn Error>> {
        solve_meta_typed_if_feasible(model)
    }

    fn build_benders_models(
        &self,
        request: &Demo2Request,
    ) -> Result<
        (
            MetaModel<f64>,
            MetaModel<f64>,
            Vec<Vec<usize>>,
            Vec<VariableId>,
        ),
        Box<dyn Error>,
    > {
        let mut master_model = MetaModel::<f64>::new("framework_demo2_predistribution_master");
        let mut sub_model = MetaModel::<f64>::new("framework_demo2_predistribution_sub");

        let (x_idx_master, x_idx_sub, fixed_variable_ids) =
            shared::model_registration::register_benders_variables(
                request, &mut master_model, &mut sub_model, "x_pre",
            )?;

        let z = master_model
            .register_variable(UContinuousVariableItem::auto("pre_benders_max_deviation"))?;
        let registration = shared::model_registration::RegistrationResult {
            x_idx: x_idx_master.clone(),
            z: Some(z),
            estimate_load_weight_idx: Vec::new(),
            estimate_loaded_idx: Vec::new(),
            loaded_idx: Vec::new(),
        };
        shared::model_registration::construct_objective(
            request, &mut master_model, &registration, Demo2PipelineMode::Predistribution,
        )?;

        let master_intermediates = shared::model_registration::register_intermediate_symbols(
            request, &mut master_model, &x_idx_master, 10000,
        )?;
        let sub_intermediates = shared::model_registration::register_intermediate_symbols(
            request, &mut sub_model, &x_idx_sub, 10000,
        )?;

        stowage::service::apply_stowage_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::Predistribution,
            &master_intermediates.loaded_idx,
            &master_intermediates.estimate_loaded_idx,
        )?;
        loading_effectiveness::service::apply_loading_effectiveness_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::Predistribution,
        )?;
        express_effectiveness::service::apply_express_effectiveness_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::Predistribution,
        )?;
        soft_security::service::apply_soft_security_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::Predistribution,
        )?;
        redundancy::service::apply_redundancy_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::Predistribution,
        )?;
        mac_optimization::service::apply_mac_optimization_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Some(z),
            Demo2PipelineMode::Predistribution,
        )?;

        airworthiness_security::service::apply_airworthiness_security_pipeline(
            &mut sub_model,
            request,
            &x_idx_sub,
            Demo2PipelineMode::Predistribution,
            &sub_intermediates.estimate_load_weight_idx,
            &sub_intermediates.estimate_loaded_idx,
        )?;

        Ok((master_model, sub_model, x_idx_master, fixed_variable_ids))
    }

    fn solve_benders(
        &self,
        request: &Demo2Request,
        adaptive: EffectiveBendersAdaptiveConfig,
    ) -> Result<(Vec<Vec<usize>>, FeasibleSolutionV<f64>), Box<dyn Error>> {
        let (master_model, sub_model, x_idx_master, fixed_variable_ids) =
            self.build_benders_models(request)?;
        let benders_result =
            solve_linear_benders(&master_model, &sub_model, fixed_variable_ids, adaptive)?;
        Ok((x_idx_master, benders_result))
    }

    fn analyze(
        &self,
        request: &Demo2Request,
        x_idx: &[Vec<usize>],
        output: Option<FeasibleSolverOutput<f64>>,
        mut notes: Vec<String>,
    ) -> Result<Demo2Response, Box<dyn Error>> {
        let Some(feasible_output) = output else {
            notes.push(String::from(
                "solver returned no feasible solution; mapped to explicit NoSolution response",
            ));
            return Ok(no_solution_response("NoSolution", notes));
        };
        let mut assignments = Vec::new();
        let solution = feasible_output.solution;

        for c in 0..request.cargos.len() {
            for p in 0..request.positions.len() {
                let value = solution.get(x_idx[c][p]).copied().unwrap_or(0.0);
                if value > 0.5 {
                    assignments.push(format!(
                        "{} -> {}",
                        request.cargos[c].name, request.positions[p].name
                    ));
                }
            }
        }
        append_critical_constraint_notes(request, x_idx, &solution, &mut notes);

        Ok(demo2_response(
            format!("{:?}", feasible_output.status),
            feasible_output.objective_value,
            assignments,
            notes,
        ))
    }
}

impl WeightRecommendationApplication {
    pub fn execute(&self, request: Demo2Request) -> Result<Demo2Response, Box<dyn Error>> {
        self.init(&request)?;
        let mut notes = Vec::new();
        append_core_feasibility_diagnostics(&request, &mut notes);
        if !notes.is_empty() {
            return Ok(no_solution_response("NoSolution", notes));
        }
        if !supported_aircraft(request.aircraft_type) {
            notes.push(format!(
                "unsupported aircraft type for weight-recommendation path: {:?}",
                request.aircraft_type
            ));
            return Ok(no_solution_response("UnsupportedAircraft", notes));
        }
        let quality_guard = resolve_benders_quality_guard_config(request.benders_quality_overrides);
        match resolve_solve_mode(&request, &mut notes) {
            SolveMode::Milp => {
                push_solver_path_note(&mut notes, "milp_direct");
            }
            SolveMode::Benders(adaptive) => {
                match self.solve_benders(&request, adaptive) {
                    Ok((x_idx, result)) => {
                        let benders_iters = result.benders_iterations.unwrap_or(0);
                        let benders_time_ms = result.time.as_millis();
                        let quality_reason = resolve_benders_quality_reason(
                            &adaptive,
                            &quality_guard,
                            benders_iters,
                            result.gap,
                            benders_time_ms,
                            result.benders_runtime_metrics.as_ref(),
                        );
                        if let Some(reason) = quality_reason {
                            match reason {
                                BENDERS_QUALITY_REASON_GAP_GUARD_EXCEEDED => {
                                    let benders_gap_guard = resolve_benders_gap_guard(&adaptive);
                                    push_benders_gap_guard_exceeded_note(
                                        &mut notes,
                                        result.gap,
                                        benders_gap_guard,
                                    );
                                }
                                BENDERS_QUALITY_REASON_TIME_GUARD_EXCEEDED => {
                                    let time_guard_ms =
                                        resolve_benders_time_guard_ms(&adaptive, &quality_guard);
                                    push_benders_time_guard_exceeded_note(
                                        &mut notes,
                                        benders_time_ms,
                                        time_guard_ms,
                                    );
                                }
                                BENDERS_QUALITY_REASON_PROGRESS_GUARD_TRIGGERED => {
                                    push_benders_progress_guard_triggered_note(
                                        &mut notes,
                                        result
                                            .benders_runtime_metrics
                                            .as_ref()
                                            .map(|metrics| metrics.executed_iterations)
                                            .unwrap_or(benders_iters),
                                        adaptive.max_iterations,
                                        result.gap,
                                        adaptive.tolerance,
                                    );
                                }
                                BENDERS_QUALITY_REASON_TRAJECTORY_WEAK => {
                                    if let Some(metrics) = result.benders_runtime_metrics.as_ref() {
                                        let snapshots = &metrics.iteration_snapshots;
                                        if snapshots.len() >= 2 {
                                            let mut abs_step_sum = 0.0_f64;
                                            for index in 1..snapshots.len() {
                                                abs_step_sum += (snapshots[index].master_obj
                                                    - snapshots[index - 1].master_obj)
                                                    .abs();
                                            }
                                            let avg_step_improvement =
                                                abs_step_sum / ((snapshots.len() - 1) as f64);
                                            push_benders_trajectory_weak_note(
                                                &mut notes,
                                                snapshots,
                                                avg_step_improvement,
                                            );
                                        }
                                    }
                                }
                                BENDERS_QUALITY_REASON_CUT_EFFICIENCY_LOW => {
                                    let executed_iterations = result
                                        .benders_runtime_metrics
                                        .as_ref()
                                        .map(|metrics| metrics.executed_iterations)
                                        .unwrap_or(benders_iters);
                                    let total_cuts = result
                                        .benders_runtime_metrics
                                        .as_ref()
                                        .map(|metrics| metrics.total_cuts)
                                        .unwrap_or(0);
                                    let cut_density = if executed_iterations > 0 {
                                        (total_cuts as f64) / (executed_iterations as f64)
                                    } else {
                                        0.0
                                    };
                                    push_benders_cut_efficiency_low_note(
                                        &mut notes,
                                        executed_iterations,
                                        total_cuts,
                                        cut_density,
                                    );
                                }
                                _ => {}
                            }
                            push_benders_runtime_notes(
                                &mut notes,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                            );
                            let benders_quality_score = resolve_benders_quality_score(
                                &adaptive,
                                &quality_guard,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                                result.benders_runtime_metrics.as_ref(),
                            );
                            push_benders_quality_score_note(&mut notes, benders_quality_score);
                            if request.solve_policy.benders_fallback_to_milp {
                                push_benders_quality_action_note(
                                    &mut notes,
                                    "fallback_to_milp",
                                    reason,
                                );
                            } else {
                                push_solver_path_note(&mut notes, "benders");
                                push_benders_quality_action_note(
                                    &mut notes,
                                    "accept_benders",
                                    reason,
                                );
                                return Ok(analyze_solution_vector(
                                    &request,
                                    &x_idx,
                                    result.obj,
                                    &result.solution,
                                    notes,
                                ));
                            }
                        } else {
                            push_solver_path_note(&mut notes, "benders");
                            push_benders_runtime_notes(
                                &mut notes,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                            );
                            let benders_quality_score = resolve_benders_quality_score(
                                &adaptive,
                                &quality_guard,
                                benders_iters,
                                result.gap,
                                benders_time_ms,
                                result.benders_runtime_metrics.as_ref(),
                            );
                            push_benders_quality_score_note(&mut notes, benders_quality_score);
                            return Ok(analyze_solution_vector(
                                &request,
                                &x_idx,
                                result.obj,
                                &result.solution,
                                notes,
                            ));
                        }
                    }
                    Err(err) => {
                        push_benders_failed_note(&mut notes, &err.to_string());
                        if !request.solve_policy.benders_fallback_to_milp {
                            return Ok(no_solution_response("BendersFailed", notes));
                        }
                    }
                }
                push_solver_path_note(&mut notes, "milp_fallback");
            }
        }

        let mut model = MetaModel::<f64>::new("framework_demo2_weight_recommendation");
        let x_idx = self.register(&request, &mut model)?;
        let output = self.solve(model)?;
        self.analyze(&request, &x_idx, output, notes)
    }

    fn init(&self, request: &Demo2Request) -> Result<(), Box<dyn Error>> {
        if request.cargos.is_empty() {
            return Err(String::from("demo2 request has no cargo").into());
        }
        if request.positions.is_empty() {
            return Err(String::from("demo2 request has no position").into());
        }
        Ok(())
    }

    fn register(
        &self,
        request: &Demo2Request,
        model: &mut MetaModel<f64>,
    ) -> Result<Vec<Vec<usize>>, Box<dyn Error>> {
        let registration = shared::model_registration::register_variables(
            request, model, "x_wr", Demo2PipelineMode::WeightRecommendation,
        )?;
        shared::model_registration::construct_objective(
            request, model, &registration, Demo2PipelineMode::WeightRecommendation,
        )?;
        apply_domain_pipeline(
            Demo2PipelineMode::WeightRecommendation,
            model,
            request,
            &registration.x_idx,
            registration.z,
            &registration.estimate_load_weight_idx,
            &registration.estimate_loaded_idx,
            &registration.loaded_idx,
        )?;

        Ok(registration.x_idx)
    }

    fn build_benders_models(
        &self,
        request: &Demo2Request,
    ) -> Result<
        (
            MetaModel<f64>,
            MetaModel<f64>,
            Vec<Vec<usize>>,
            Vec<VariableId>,
        ),
        Box<dyn Error>,
    > {
        let mut master_model =
            MetaModel::<f64>::new("framework_demo2_weight_recommendation_master");
        let mut sub_model = MetaModel::<f64>::new("framework_demo2_weight_recommendation_sub");

        let (x_idx_master, x_idx_sub, fixed_variable_ids) =
            shared::model_registration::register_benders_variables(
                request, &mut master_model, &mut sub_model, "x_wr",
            )?;

        let z = master_model
            .register_variable(UContinuousVariableItem::auto("wr_benders_max_deviation"))?;
        let registration = shared::model_registration::RegistrationResult {
            x_idx: x_idx_master.clone(),
            z: Some(z),
            estimate_load_weight_idx: Vec::new(),
            estimate_loaded_idx: Vec::new(),
            loaded_idx: Vec::new(),
        };
        shared::model_registration::construct_objective(
            request, &mut master_model, &registration, Demo2PipelineMode::WeightRecommendation,
        )?;

        let master_intermediates = shared::model_registration::register_intermediate_symbols(
            request, &mut master_model, &x_idx_master, 10000,
        )?;
        let sub_intermediates = shared::model_registration::register_intermediate_symbols(
            request, &mut sub_model, &x_idx_sub, 10000,
        )?;

        stowage::service::apply_stowage_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::WeightRecommendation,
            &master_intermediates.loaded_idx,
            &master_intermediates.estimate_loaded_idx,
        )?;
        loading_effectiveness::service::apply_loading_effectiveness_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::WeightRecommendation,
        )?;
        express_effectiveness::service::apply_express_effectiveness_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::WeightRecommendation,
        )?;
        soft_security::service::apply_soft_security_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::WeightRecommendation,
        )?;
        redundancy::service::apply_redundancy_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Demo2PipelineMode::WeightRecommendation,
        )?;
        mac_optimization::service::apply_mac_optimization_pipeline(
            &mut master_model,
            request,
            &x_idx_master,
            Some(z),
            Demo2PipelineMode::WeightRecommendation,
        )?;

        airworthiness_security::service::apply_airworthiness_security_pipeline(
            &mut sub_model,
            request,
            &x_idx_sub,
            Demo2PipelineMode::WeightRecommendation,
            &sub_intermediates.estimate_load_weight_idx,
            &sub_intermediates.estimate_loaded_idx,
        )?;

        Ok((master_model, sub_model, x_idx_master, fixed_variable_ids))
    }

    fn solve_benders(
        &self,
        request: &Demo2Request,
        adaptive: EffectiveBendersAdaptiveConfig,
    ) -> Result<(Vec<Vec<usize>>, FeasibleSolutionV<f64>), Box<dyn Error>> {
        let (master_model, sub_model, x_idx_master, fixed_variable_ids) =
            self.build_benders_models(request)?;
        let benders_result =
            solve_linear_benders(&master_model, &sub_model, fixed_variable_ids, adaptive)?;
        Ok((x_idx_master, benders_result))
    }

    fn solve(
        &self,
        model: MetaModel<f64>,
    ) -> Result<Option<FeasibleSolverOutput<f64>>, Box<dyn Error>> {
        solve_meta_typed_if_feasible(model)
    }

    fn analyze(
        &self,
        request: &Demo2Request,
        x_idx: &[Vec<usize>],
        output: Option<FeasibleSolverOutput<f64>>,
        mut notes: Vec<String>,
    ) -> Result<Demo2Response, Box<dyn Error>> {
        let Some(feasible_output) = output else {
            notes.push(String::from(
                "solver returned no feasible solution; mapped to explicit NoSolution response",
            ));
            return Ok(no_solution_response("NoSolution", notes));
        };
        let mut assignments = Vec::new();
        let solution = feasible_output.solution;

        for c in 0..request.cargos.len() {
            for p in 0..request.positions.len() {
                let value = solution.get(x_idx[c][p]).copied().unwrap_or(0.0);
                if value > 0.5 {
                    assignments.push(format!(
                        "{} -> {}",
                        request.cargos[c].name, request.positions[p].name
                    ));
                }
            }
        }
        append_critical_constraint_notes(request, x_idx, &solution, &mut notes);

        Ok(demo2_response(
            format!("{:?}", feasible_output.status),
            feasible_output.objective_value,
            assignments,
            notes,
        ))
    }
}

impl LoadingOrderApplication {
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
    use crate::framework::demo2::infrastructure::dto::{
        AircraftTypeInput, BendersAdaptiveConfig, CargoInput, Demo2Request, PositionInput,
        SolvePolicy, WeightRecommendationObjectiveConfig,
    };

    #[test]
    fn full_load_returns_unsupported_aircraft_status() {
        let mut request = Demo2Request::sample();
        request.aircraft_type = AircraftTypeInput::B767;

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("unsupported aircraft should be handled as response");

        assert_eq!(output.status, "UnsupportedAircraft");
        assert!(output.assignments.is_empty());
    }

    #[test]
    fn full_load_prefers_benders_without_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("benders-no-fallback should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=benders"))
                || output
                    .notes
                    .iter()
                    .any(|note| note.contains("solver_path=milp_fallback"))
        );
    }

    #[test]
    fn full_load_returns_benders_failed_when_benders_errors_without_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("full-load benders-failed branch should be handled as response");

        assert_eq!(output.status, "BendersFailed");
        assert!(output.assignments.is_empty());
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_failed:"))
        );
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("benders_failed")
        }));
    }

    #[test]
    fn full_load_falls_back_to_milp_when_benders_errors_with_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("full-load benders fallback branch should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=milp_fallback"))
        );
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_failed:"))
        );
    }

    #[test]
    fn predistribution_prefers_benders_without_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };

        let app = PredistributionApplication;
        let output = app
            .execute(request)
            .expect("predistribution benders branch should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=benders"))
                || output
                    .notes
                    .iter()
                    .any(|note| note.contains("solver_path=milp_fallback"))
        );
    }

    #[test]
    fn predistribution_returns_benders_failed_when_benders_errors_without_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let app = PredistributionApplication;
        let output = app
            .execute(request)
            .expect("predistribution benders-failed branch should be handled as response");

        assert_eq!(output.status, "BendersFailed");
        assert!(output.assignments.is_empty());
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_failed:"))
        );
    }

    #[test]
    fn predistribution_falls_back_to_milp_when_benders_errors_with_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let app = PredistributionApplication;
        let output = app
            .execute(request)
            .expect("predistribution benders fallback branch should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=milp_fallback"))
        );
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_failed:"))
        );
    }

    #[test]
    fn full_load_returns_no_solution_when_capacity_is_too_small() {
        let mut request = Demo2Request::sample();
        for position in &mut request.positions {
            position.max_weight = 5.0;
        }

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("no-solution should be mapped into response");

        assert_eq!(output.status, "NoSolution");
        assert!(output.assignments.is_empty());
    }

    #[test]
    fn predistribution_minimal_path_is_feasible() {
        let request = Demo2Request::sample();
        let app = PredistributionApplication;
        let output = app.execute(request).expect("predistribution should run");
        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(!output.assignments.is_empty());
    }

    #[test]
    fn weight_recommendation_minimal_path_is_feasible() {
        let request = Demo2Request::sample();
        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight recommendation should run");
        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(!output.assignments.is_empty());
    }

    #[test]
    fn loading_order_exports_sorted_sequence() {
        let request = Demo2Request::sample();
        let app = LoadingOrderApplication;
        let output = app.execute(request).expect("loading order should run");
        assert_eq!(output.status, "OK");
        assert_eq!(output.orders.len(), 3);
        assert!(output.orders[0].contains("C1"));
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn full_load_returns_no_solution_when_envelope_is_too_tight() {
        let mut request = Demo2Request::sample();
        request.envelope_longitudinal_moment_min = -1.0;
        request.envelope_longitudinal_moment_max = 1.0;

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("tight envelope should be mapped into response");

        assert_eq!(output.status, "NoSolution");
        assert!(output.assignments.is_empty());
    }

    #[test]
    fn full_load_returns_no_solution_when_cumulative_load_limit_is_too_small() {
        let mut request = Demo2Request::sample();
        request.max_cumulative_forward_load = 7.0;
        request.max_cumulative_backward_load = 7.0;

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("tight cumulative limits should be mapped into response");

        assert_eq!(output.status, "NoSolution");
        assert!(output.assignments.is_empty());
    }

    #[test]
    fn full_load_returns_no_solution_when_lateral_imbalance_is_too_small() {
        let mut request = Demo2Request::sample();
        request.max_lateral_imbalance = 0.0;

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("tight lateral imbalance should be mapped into response");

        assert_eq!(output.status, "NoSolution");
        assert!(output.assignments.is_empty());
    }

    #[test]
    fn full_load_returns_no_solution_with_diagnostics_when_cargo_exceeds_all_positions() {
        let mut request = Demo2Request::sample();
        request.cargos[0].weight = 100.0;

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("hard-infeasible data should be mapped into response");

        assert_eq!(output.status, "NoSolution");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("code=cargo_exceeds_all_positions"))
        );
    }

    #[test]
    fn weight_recommendation_returns_no_solution_with_diagnostics_when_min_payload_infeasible() {
        let mut request = Demo2Request::sample();
        request.payload_upper_bound = 10.0;
        request.min_payload_ratio = 1.5;

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("invalid payload bounds should be mapped into response");

        assert_eq!(output.status, "NoSolution");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("code=min_payload_ratio_out_of_range"))
        );
    }

    #[test]
    fn weight_recommendation_prioritizes_balance_over_small_extra_payload() {
        let mut request = Demo2Request::sample();
        request.cargos = vec![
            crate::framework::demo2::infrastructure::dto::CargoInput {
                name: String::from("H1"),
                weight: 10.0,
                priority: 6,
                source: String::from("S1"),
                destination: String::from("D1"),
                requires_separation: false,
                code: None,
            },
            crate::framework::demo2::infrastructure::dto::CargoInput {
                name: String::from("H2"),
                weight: 10.0,
                priority: 6,
                source: String::from("S2"),
                destination: String::from("D2"),
                requires_separation: false,
                code: None,
            },
            crate::framework::demo2::infrastructure::dto::CargoInput {
                name: String::from("L1"),
                weight: 1.0,
                priority: 1,
                source: String::from("S3"),
                destination: String::from("D3"),
                requires_separation: false,
                code: None,
            },
        ];
        request.positions = vec![
            crate::framework::demo2::infrastructure::dto::PositionInput {
                name: String::from("P1"),
                max_weight: 20.0,
                longitudinal_arm: -1.0,
                lateral_arm: 0.0,
                    area: 5.0,
                    length: 2.0,
                    max_load_count: 3,
                    loaded_items: Vec::new(),
                predicate_load_weight_min: None,
            },
            crate::framework::demo2::infrastructure::dto::PositionInput {
                name: String::from("P2"),
                max_weight: 20.0,
                longitudinal_arm: 1.0,
                lateral_arm: 0.0,
                    area: 5.0,
                    length: 2.0,
                    max_load_count: 3,
                    loaded_items: Vec::new(),
                predicate_load_weight_min: None,
            },
        ];
        request.payload_upper_bound = 40.0;
        request.min_payload_ratio = 0.0;
        request.max_adjacent_load_gap = 40.0;
        request.max_cumulative_forward_load = 40.0;
        request.max_cumulative_backward_load = 40.0;
        request.envelope_longitudinal_moment_min = -100.0;
        request.envelope_longitudinal_moment_max = 100.0;
        request.max_longitudinal_moment_deviation = 100.0;
        request.max_lateral_imbalance = 100.0;

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight recommendation should run");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output.objective.unwrap_or(0.0) < 0.0,
            "balance-first objective should dominate payload term and keep objective negative"
        );
    }

    #[test]
    fn weight_recommendation_objective_config_can_shift_payload_preference() {
        let mut request = Demo2Request::sample();
        request.weight_recommendation_objective = WeightRecommendationObjectiveConfig {
            balance_priority: 0.1,
            payload_priority: 10.0,
        };

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight recommendation should run");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output.objective.unwrap_or(0.0) > 0.0,
            "payload-first objective should dominate and keep objective positive"
        );
    }

    #[test]
    fn weight_recommendation_prefers_benders_without_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight recommendation benders-no-fallback should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=benders"))
                || output
                    .notes
                    .iter()
                    .any(|note| note.contains("solver_path=milp_fallback"))
        );
    }

    #[test]
    fn weight_recommendation_returns_benders_failed_when_benders_errors_without_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight-recommendation benders-failed branch should be handled as response");

        assert_eq!(output.status, "BendersFailed");
        assert!(output.assignments.is_empty());
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_failed:"))
        );
    }

    #[test]
    fn weight_recommendation_falls_back_to_milp_when_benders_errors_with_fallback() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight-recommendation benders fallback branch should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=milp_fallback"))
        );
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_failed:"))
        );
    }

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
                assert_eq!(adaptive.max_stall_iterations, Some(16));
                assert_eq!(adaptive.objective_stall_iterations, Some(5));
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
                .any(|note| note.contains("max_stall_iterations=16"))
        );
        assert!(
            notes
                .iter()
                .any(|note| note.contains("objective_stall_iterations=5"))
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
    fn full_load_skips_benders_when_problem_size_below_threshold() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 999,
            max_iterations: 8,
            tolerance: 1e-4,
        };

        let app = FullLoadApplication;
        let output = app
            .execute(request)
            .expect("full-load threshold branch should be handled as response");

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("Benders requested but skipped"))
        );
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("solver_path=milp_direct"))
        );
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("solver_path")
                && note.message.contains("milp_direct")
        }));
    }

    #[test]
    fn full_load_benders_notes_include_adaptive_config() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };

        let app = FullLoadApplication;
        let output = app.execute(request).expect("full-load should run");

        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_adaptive=min_binary_variables="))
        );
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_adaptive_effective=min_binary_variables="))
        );
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.contains("benders_problem_size_binary_variables="))
        );
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("benders_adaptive_effective")
        }));
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("benders_problem_size_binary_variables")
        }));
        if output
            .notes
            .iter()
            .any(|note| note.contains("solver_path=benders"))
        {
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("benders_iters="))
            );
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("benders_gap="))
            );
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("benders_time_ms="))
            );
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("benders_iterations")
            }));
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("benders_gap")
            }));
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("benders_time_ms")
            }));
        }
    }

    #[test]
    fn notes_and_diagnostics_contract_for_milp_direct_is_consistent_across_apps() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: false,
            benders_fallback_to_milp: true,
        };

        let full_load = FullLoadApplication
            .execute(request.clone())
            .expect("full-load milp-direct should run");
        let predistribution = PredistributionApplication
            .execute(request.clone())
            .expect("predistribution milp-direct should run");
        let weight_recommendation = WeightRecommendationApplication
            .execute(request)
            .expect("weight-recommendation milp-direct should run");

        for output in [&full_load, &predistribution, &weight_recommendation] {
            assert_eq!(output.notes.len(), output.diagnostics.len());
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("solver_path=milp_direct"))
            );
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("solver_path")
                    && note.message.contains("milp_direct")
            }));
        }
    }

    #[test]
    fn notes_and_diagnostics_contract_for_benders_failed_is_consistent_across_apps() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: false,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let full_load = FullLoadApplication
            .execute(request.clone())
            .expect("full-load benders-failed should run");
        let predistribution = PredistributionApplication
            .execute(request.clone())
            .expect("predistribution benders-failed should run");
        let weight_recommendation = WeightRecommendationApplication
            .execute(request)
            .expect("weight-recommendation benders-failed should run");

        for output in [&full_load, &predistribution, &weight_recommendation] {
            assert_eq!(output.notes.len(), output.diagnostics.len());
            assert_eq!(output.status, "BendersFailed");
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("benders_failed:"))
            );
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("benders_failed")
            }));
        }
    }

    #[test]
    fn notes_and_diagnostics_contract_for_milp_fallback_is_consistent_across_apps() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_adaptive = BendersAdaptiveConfig {
            min_binary_variables: 1,
            max_iterations: 0,
            tolerance: 1e-6,
        };

        let full_load = FullLoadApplication
            .execute(request.clone())
            .expect("full-load milp-fallback should run");
        let predistribution = PredistributionApplication
            .execute(request.clone())
            .expect("predistribution milp-fallback should run");
        let weight_recommendation = WeightRecommendationApplication
            .execute(request)
            .expect("weight-recommendation milp-fallback should run");

        for output in [&full_load, &predistribution, &weight_recommendation] {
            assert_eq!(output.notes.len(), output.diagnostics.len());
            assert!(output.status == "Optimal" || output.status == "Feasible");
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("solver_path=milp_fallback"))
            );
            assert!(
                output
                    .notes
                    .iter()
                    .any(|note| note.contains("benders_failed:"))
            );
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("solver_path")
                    && note.message.contains("milp_fallback")
            }));
            assert!(output.diagnostics.iter().any(|note| {
                note.level == "diagnostic"
                    && note.group.as_deref() == Some("solver")
                    && note.code.as_deref() == Some("benders_failed")
            }));
        }
    }

    #[test]
    fn full_load_benders_quality_overrides_are_reflected_in_effective_notes() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_quality_overrides = Some(BendersQualityOverrideConfig {
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
        });

        let app = FullLoadApplication;
        let output = app.execute(request).expect("full-load should run");

        assert!(output.notes.iter().any(|note| {
            note.contains("benders_quality_guard_effective=")
                && note.contains("weak_gap_multiplier=30.000")
                && note.contains("weak_gap_floor=0.000020")
                && note.contains("iteration_pressure_percent=80")
                && note.contains("cut_density_min_iterations=10")
                && note.contains("cut_density_threshold=0.400")
                && note.contains("trajectory_min_snapshots=8")
                && note.contains("trajectory_step_multiplier=25.000")
                && note.contains("trajectory_step_floor=0.000002")
                && note.contains("time_guard_min_ms=1200")
                && note
                    .contains("score_weights=gap:0.400|time:0.300|iter:0.100|cut:0.100|traj:0.100")
        }));
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("benders_quality_guard_effective")
                && note.message.contains("weak_gap_multiplier=30.000")
                && note.message.contains("time_guard_min_ms=1200")
        }));
    }

    #[test]
    fn predistribution_benders_quality_overrides_are_reflected_in_effective_notes() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_quality_overrides = Some(BendersQualityOverrideConfig {
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
        });

        let app = PredistributionApplication;
        let output = app.execute(request).expect("predistribution should run");

        assert!(output.notes.iter().any(|note| {
            note.contains("benders_quality_guard_effective=")
                && note.contains("weak_gap_multiplier=30.000")
                && note.contains("weak_gap_floor=0.000020")
                && note.contains("iteration_pressure_percent=80")
                && note.contains("cut_density_min_iterations=10")
                && note.contains("cut_density_threshold=0.400")
                && note.contains("trajectory_min_snapshots=8")
                && note.contains("trajectory_step_multiplier=25.000")
                && note.contains("trajectory_step_floor=0.000002")
                && note.contains("time_guard_min_ms=1200")
                && note
                    .contains("score_weights=gap:0.400|time:0.300|iter:0.100|cut:0.100|traj:0.100")
        }));
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("benders_quality_guard_effective")
                && note.message.contains("weak_gap_multiplier=30.000")
                && note.message.contains("time_guard_min_ms=1200")
        }));
    }

    #[test]
    fn weight_recommendation_benders_quality_overrides_are_reflected_in_effective_notes() {
        let mut request = Demo2Request::sample();
        request.solve_policy = SolvePolicy {
            prefer_benders: true,
            benders_fallback_to_milp: true,
        };
        request.benders_quality_overrides = Some(BendersQualityOverrideConfig {
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
        });

        let app = WeightRecommendationApplication;
        let output = app
            .execute(request)
            .expect("weight-recommendation should run");

        assert!(output.notes.iter().any(|note| {
            note.contains("benders_quality_guard_effective=")
                && note.contains("weak_gap_multiplier=30.000")
                && note.contains("weak_gap_floor=0.000020")
                && note.contains("iteration_pressure_percent=80")
                && note.contains("cut_density_min_iterations=10")
                && note.contains("cut_density_threshold=0.400")
                && note.contains("trajectory_min_snapshots=8")
                && note.contains("trajectory_step_multiplier=25.000")
                && note.contains("trajectory_step_floor=0.000002")
                && note.contains("time_guard_min_ms=1200")
                && note
                    .contains("score_weights=gap:0.400|time:0.300|iter:0.100|cut:0.100|traj:0.100")
        }));
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "diagnostic"
                && note.group.as_deref() == Some("solver")
                && note.code.as_deref() == Some("benders_quality_guard_effective")
                && note.message.contains("weak_gap_multiplier=30.000")
                && note.message.contains("time_guard_min_ms=1200")
        }));
    }

    #[test]
    fn full_load_benders_constraint_grouping_routes_airworthiness_to_sub() {
        let request = Demo2Request::sample();
        let app = FullLoadApplication;
        let (master_model, sub_model, _, _) = app
            .build_benders_models(&request)
            .expect("full-load benders models should build");

        let master_constraint_names: Vec<&str> = master_model
            .as_basic()
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect();
        let sub_constraint_names: Vec<&str> = sub_model
            .as_basic()
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect();

        assert!(!master_constraint_names.is_empty());
        assert!(!sub_constraint_names.is_empty());
        assert!(
            master_constraint_names
                .iter()
                .all(|name| !name.starts_with("airworthiness_security_"))
        );
        assert!(
            sub_constraint_names
                .iter()
                .all(|name| name.starts_with("airworthiness_security_"))
        );
    }

    #[test]
    fn weight_recommendation_benders_constraint_grouping_routes_airworthiness_to_sub() {
        let request = Demo2Request::sample();
        let app = WeightRecommendationApplication;
        let (master_model, sub_model, _, _) = app
            .build_benders_models(&request)
            .expect("weight-recommendation benders models should build");

        let master_constraint_names: Vec<&str> = master_model
            .as_basic()
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect();
        let sub_constraint_names: Vec<&str> = sub_model
            .as_basic()
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect();

        assert!(!master_constraint_names.is_empty());
        assert!(!sub_constraint_names.is_empty());
        assert!(
            master_constraint_names
                .iter()
                .all(|name| !name.starts_with("airworthiness_security_"))
        );
        assert!(
            sub_constraint_names
                .iter()
                .all(|name| name.starts_with("airworthiness_security_"))
        );
    }

    #[test]
    fn predistribution_benders_constraint_grouping_routes_airworthiness_to_sub() {
        let request = Demo2Request::sample();
        let app = PredistributionApplication;
        let (master_model, sub_model, _, _) = app
            .build_benders_models(&request)
            .expect("predistribution benders models should build");

        let master_constraint_names: Vec<&str> = master_model
            .as_basic()
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect();
        let sub_constraint_names: Vec<&str> = sub_model
            .as_basic()
            .constraints()
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect();

        assert!(!master_constraint_names.is_empty());
        assert!(!sub_constraint_names.is_empty());
        assert!(
            master_constraint_names
                .iter()
                .all(|name| !name.starts_with("airworthiness_security_"))
        );
        assert!(
            sub_constraint_names
                .iter()
                .all(|name| name.starts_with("airworthiness_security_"))
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

        assert_eq!(output.status, "NoSolution");
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

        assert!(output.status == "Optimal" || output.status == "Feasible");
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

        assert!(output.status == "Optimal" || output.status == "Feasible");
        assert!(output.diagnostics.iter().any(|note| {
            note.level == "critical"
                && note.group.as_deref() == Some("redundancy")
                && note.code.as_deref() == Some("destination_concentration_high")
        }));
    }
}
