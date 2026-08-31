#[derive(Clone)]
pub struct CargoInput {
    pub name: String,
    pub weight: f64,
    pub priority: u8,
    pub source: String,
    pub destination: String,
    pub requires_separation: bool,
}

#[derive(Clone)]
pub struct PositionInput {
    pub name: String,
    pub max_weight: f64,
    pub longitudinal_arm: f64,
    pub lateral_arm: f64,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum AircraftTypeInput {
    B737,
    B757,
    B767,
    B747,
    Unknown,
}

#[derive(Clone, Copy, Debug)]
pub struct SolvePolicy {
    pub prefer_benders: bool,
    pub benders_fallback_to_milp: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct BendersAdaptiveConfig {
    pub min_binary_variables: usize,
    pub max_iterations: usize,
    pub tolerance: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BendersQualityOverrideConfig {
    pub weak_gap_multiplier: Option<f64>,
    pub weak_gap_floor: Option<f64>,
    pub iteration_pressure_percent: Option<usize>,
    pub cut_density_min_iterations: Option<usize>,
    pub cut_density_threshold: Option<f64>,
    pub trajectory_min_snapshots: Option<usize>,
    pub trajectory_step_multiplier: Option<f64>,
    pub trajectory_step_floor: Option<f64>,
    pub time_guard_min_ms: Option<u128>,
    pub score_gap_weight: Option<f64>,
    pub score_time_weight: Option<f64>,
    pub score_iteration_weight: Option<f64>,
    pub score_cut_density_weight: Option<f64>,
    pub score_trajectory_weight: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
pub struct WeightRecommendationObjectiveConfig {
    pub balance_priority: f64,
    pub payload_priority: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticNote {
    pub level: String,
    pub group: Option<String>,
    pub code: Option<String>,
    pub message: String,
}

#[derive(Clone)]
pub struct Demo2Request {
    pub cargos: Vec<CargoInput>,
    pub positions: Vec<PositionInput>,
    pub aircraft_type: AircraftTypeInput,
    pub solve_policy: SolvePolicy,
    pub benders_adaptive: BendersAdaptiveConfig,
    pub benders_quality_overrides: Option<BendersQualityOverrideConfig>,
    pub weight_recommendation_objective: WeightRecommendationObjectiveConfig,
    pub payload_upper_bound: f64,
    pub min_payload_ratio: f64,
    pub max_adjacent_load_gap: f64,
    pub max_cumulative_forward_load: f64,
    pub max_cumulative_backward_load: f64,
    pub envelope_longitudinal_moment_min: f64,
    pub envelope_longitudinal_moment_max: f64,
    pub target_longitudinal_moment: f64,
    pub max_longitudinal_moment_deviation: f64,
    pub max_lateral_imbalance: f64,
}

impl Demo2Request {
    pub fn sample() -> Self {
        Self {
            cargos: vec![
                CargoInput {
                    name: String::from("C1"),
                    weight: 8.0,
                    priority: 10,
                    source: String::from("S1"),
                    destination: String::from("D1"),
                    requires_separation: true,
                },
                CargoInput {
                    name: String::from("C2"),
                    weight: 6.0,
                    priority: 6,
                    source: String::from("S2"),
                    destination: String::from("D1"),
                    requires_separation: false,
                },
                CargoInput {
                    name: String::from("C3"),
                    weight: 4.0,
                    priority: 4,
                    source: String::from("S1"),
                    destination: String::from("D2"),
                    requires_separation: true,
                },
            ],
            positions: vec![
                PositionInput {
                    name: String::from("P1"),
                    max_weight: 10.0,
                    longitudinal_arm: -1.0,
                    lateral_arm: -0.5,
                },
                PositionInput {
                    name: String::from("P2"),
                    max_weight: 10.0,
                    longitudinal_arm: 1.0,
                    lateral_arm: 0.5,
                },
            ],
            aircraft_type: AircraftTypeInput::B737,
            solve_policy: SolvePolicy {
                prefer_benders: false,
                benders_fallback_to_milp: true,
            },
            benders_adaptive: BendersAdaptiveConfig {
                min_binary_variables: 4,
                max_iterations: 64,
                tolerance: 1e-6,
            },
            benders_quality_overrides: None,
            weight_recommendation_objective: WeightRecommendationObjectiveConfig {
                balance_priority: 1000.0,
                payload_priority: 1.0,
            },
            payload_upper_bound: 20.0,
            min_payload_ratio: 0.6,
            max_adjacent_load_gap: 8.0,
            max_cumulative_forward_load: 20.0,
            max_cumulative_backward_load: 20.0,
            envelope_longitudinal_moment_min: -20.0,
            envelope_longitudinal_moment_max: 20.0,
            target_longitudinal_moment: 0.0,
            max_longitudinal_moment_deviation: 20.0,
            max_lateral_imbalance: 12.0,
        }
    }
}

pub struct Demo2Response {
    pub status: String,
    pub objective: Option<f64>,
    pub assignments: Vec<String>,
    pub notes: Vec<String>,
    pub diagnostics: Vec<DiagnosticNote>,
}

pub struct LoadingOrderResponse {
    pub status: String,
    pub orders: Vec<String>,
    pub notes: Vec<String>,
    pub diagnostics: Vec<DiagnosticNote>,
}
