//! Demo2 数据传输对象 / Demo2 data transfer objects.
pub mod kpi_response_dto;
pub mod loading_order_response_dto;
pub mod render_dto;
pub mod report_response_dto;
pub mod running_heartbeat_dto;

pub use kpi_response_dto::*;
pub use loading_order_response_dto::*;
pub use render_dto::*;
pub use report_response_dto::*;
pub use running_heartbeat_dto::*;

/// 货物输入数据 / Cargo input data
#[derive(Clone)]
pub struct CargoInput {
    /// 货物名称 / Cargo name
    pub name: String,
    /// 货物重量 / Cargo weight
    pub weight: f64,
    /// 优先级 / Priority level
    pub priority: u8,
    /// 出发地 / Source location
    pub source: String,
    /// 目的地 / Destination location
    pub destination: String,
    /// 是否需要隔离 / Whether separation is required
    pub requires_separation: bool,
    /// 货物代码 / Cargo code (对齐 Kotlin CargoCode / Aligned with Kotlin CargoCode)
    pub code: Option<crate::framework::demo2::domain::stowage::model::cargo::CargoCode>,
}

/// 货舱位置输入数据 / Cargo position input data
#[derive(Clone)]
pub struct PositionInput {
    /// 位置名称 / Position name
    pub name: String,
    /// 最大载重 / Maximum load weight
    pub max_weight: f64,
    /// 纵向力臂 / Longitudinal arm
    pub longitudinal_arm: f64,
    /// 横向力臂 / Lateral arm
    pub lateral_arm: f64,
    /// 面积 / Area
    pub area: f64,
    /// 长度 / Length
    pub length: f64,
    /// 最大装载件数 / Maximum load count
    pub max_load_count: u64,
    /// 已装载货物列表 / List of loaded item names
    pub loaded_items: Vec<String>,
    /// 最小预测装载重量 / Minimum predicate load weight (对齐 Kotlin plw.min / Aligned with Kotlin plw.min)
    pub predicate_load_weight_min: Option<f64>,
}

/// 飞机类型输入 / Aircraft type input
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum AircraftTypeInput {
    /// 波音 737 / Boeing 737
    B737,
    /// 波音 757 / Boeing 757
    B757,
    /// 波音 767 / Boeing 767
    B767,
    /// 波音 747 / Boeing 747
    B747,
    /// 未知机型 / Unknown aircraft type
    Unknown,
}

/// 求解策略配置 / Solve policy configuration
#[derive(Clone, Copy, Debug)]
pub struct SolvePolicy {
    /// 是否优先使用 Benders 分解 / Whether to prefer Benders decomposition
    pub prefer_benders: bool,
    /// Benders 失败时是否回退到 MILP / Whether to fall back to MILP when Benders fails
    pub benders_fallback_to_milp: bool,
}

/// Benders 自适应配置 / Benders adaptive configuration
#[derive(Clone, Copy, Debug)]
pub struct BendersAdaptiveConfig {
    /// 最小二值变量数（低于此数不启用 Benders） / Minimum binary variable count (Benders not enabled below this threshold)
    pub min_binary_variables: usize,
    /// 最大迭代次数 / Maximum number of iterations
    pub max_iterations: usize,
    /// 收敛容差 / Convergence tolerance
    pub tolerance: f64,
}

/// Benders 质量覆盖配置 / Benders quality override configuration
#[derive(Clone, Copy, Debug, Default)]
pub struct BendersQualityOverrideConfig {
    /// 弱间隙乘数 / Weak gap multiplier
    pub weak_gap_multiplier: Option<f64>,
    /// 弱间隙下限 / Weak gap floor
    pub weak_gap_floor: Option<f64>,
    /// 迭代压力百分比 / Iteration pressure percentage
    pub iteration_pressure_percent: Option<usize>,
    /// 割密度最小迭代数 / Cut density minimum iterations
    pub cut_density_min_iterations: Option<usize>,
    /// 割密度阈值 / Cut density threshold
    pub cut_density_threshold: Option<f64>,
    /// 轨迹最小快照数 / Trajectory minimum snapshots
    pub trajectory_min_snapshots: Option<usize>,
    /// 轨迹步长乘数 / Trajectory step multiplier
    pub trajectory_step_multiplier: Option<f64>,
    /// 轨迹步长下限 / Trajectory step floor
    pub trajectory_step_floor: Option<f64>,
    /// 时间守卫最小毫秒数 / Time guard minimum milliseconds
    pub time_guard_min_ms: Option<u128>,
    /// 评分 - 间隙权重 / Score - gap weight
    pub score_gap_weight: Option<f64>,
    /// 评分 - 时间权重 / Score - time weight
    pub score_time_weight: Option<f64>,
    /// 评分 - 迭代权重 / Score - iteration weight
    pub score_iteration_weight: Option<f64>,
    /// 评分 - 割密度权重 / Score - cut density weight
    pub score_cut_density_weight: Option<f64>,
    /// 评分 - 轨迹权重 / Score - trajectory weight
    pub score_trajectory_weight: Option<f64>,
}

/// 重量推荐目标配置 / Weight recommendation objective configuration
#[derive(Clone, Copy, Debug)]
pub struct WeightRecommendationObjectiveConfig {
    /// 平衡优先级 / Balance priority weight
    pub balance_priority: f64,
    /// 载重优先级 / Payload priority weight
    pub payload_priority: f64,
}

/// 诊断信息 / Diagnostic note
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticNote {
    /// 诊断级别 / Diagnostic level
    pub level: String,
    /// 诊断分组 / Diagnostic group
    pub group: Option<String>,
    /// 诊断代码 / Diagnostic code
    pub code: Option<String>,
    /// 诊断消息 / Diagnostic message
    pub message: String,
}

/// 相邻位置对 / Adjacent position pair
///
/// 对齐 Kotlin PositionPair，表示两个相邻的舱位。
/// Matches Kotlin PositionPair, represents two adjacent positions.
#[derive(Debug, Clone)]
pub struct PositionPair {
    /// 第一个位置索引 / First position index
    pub first: usize,
    /// 第二个位置索引 / Second position index
    pub second: usize,
}

/// Demo2 请求数据 / Demo2 request data
#[derive(Clone)]
pub struct Demo2Request {
    /// 货物列表 / Cargo list
    pub cargos: Vec<CargoInput>,
    /// 货舱位置列表 / Position list
    pub positions: Vec<PositionInput>,
    /// 飞机类型 / Aircraft type
    pub aircraft_type: AircraftTypeInput,
    /// 求解策略 / Solve policy
    pub solve_policy: SolvePolicy,
    /// Benders 自适应配置 / Benders adaptive configuration
    pub benders_adaptive: BendersAdaptiveConfig,
    /// Benders 质量覆盖配置 / Benders quality override configuration
    pub benders_quality_overrides: Option<BendersQualityOverrideConfig>,
    /// 重量推荐目标配置 / Weight recommendation objective configuration
    pub weight_recommendation_objective: WeightRecommendationObjectiveConfig,
    /// 载重上限 / Payload upper bound
    pub payload_upper_bound: f64,
    /// 最小载重比 / Minimum payload ratio
    pub min_payload_ratio: f64,
    /// 最大相邻装载间隙 / Maximum adjacent load gap
    pub max_adjacent_load_gap: f64,
    /// 最大累积前向载重 / Maximum cumulative forward load
    pub max_cumulative_forward_load: f64,
    /// 最大累积后向载重 / Maximum cumulative backward load
    pub max_cumulative_backward_load: f64,
    /// 包络纵向力矩下限 / Envelope longitudinal moment minimum
    pub envelope_longitudinal_moment_min: f64,
    /// 包络纵向力矩上限 / Envelope longitudinal moment maximum
    pub envelope_longitudinal_moment_max: f64,
    /// 目标纵向力矩 / Target longitudinal moment
    pub target_longitudinal_moment: f64,
    /// 最大纵向力矩偏差 / Maximum longitudinal moment deviation
    pub max_longitudinal_moment_deviation: f64,
    /// 最大横向不平衡量 / Maximum lateral imbalance
    pub max_lateral_imbalance: f64,
    /// 相邻位置对列表 / Adjacent position pairs
    ///
    /// 对齐 Kotlin adjacentPositions，用于 DivideEmptyLoading 等约束。
    /// Matches Kotlin adjacentPositions, used for DivideEmptyLoading constraints.
    pub adjacent_positions: Vec<PositionPair>,
}

impl Demo2Request {
    /// 生成示例请求数据 / Generate sample request data
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
                    code: None,
                },
                CargoInput {
                    name: String::from("C2"),
                    weight: 6.0,
                    priority: 6,
                    source: String::from("S2"),
                    destination: String::from("D1"),
                    requires_separation: false,
                    code: None,
                },
                CargoInput {
                    name: String::from("C3"),
                    weight: 4.0,
                    priority: 4,
                    source: String::from("S1"),
                    destination: String::from("D2"),
                    requires_separation: true,
                    code: None,
                },
            ],
            positions: vec![
                PositionInput {
                    name: String::from("P1"),
                    max_weight: 10.0,
                    longitudinal_arm: -1.0,
                    lateral_arm: -0.5,
                    area: 5.0,
                    length: 2.0,
                    max_load_count: 3,
                    loaded_items: Vec::new(),
                    predicate_load_weight_min: Some(1.0),
                },
                PositionInput {
                    name: String::from("P2"),
                    max_weight: 10.0,
                    longitudinal_arm: 1.0,
                    lateral_arm: 0.5,
                    area: 5.0,
                    length: 2.0,
                    max_load_count: 3,
                    loaded_items: Vec::new(),
                    predicate_load_weight_min: Some(1.0),
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
            adjacent_positions: vec![PositionPair {
                first: 0,
                second: 1,
            }],
        }
    }
}

/// Demo2 响应数据 / Demo2 response data
pub struct Demo2Response {
    /// 求解状态 / Solve status
    pub status: String,
    /// 目标函数值 / Objective function value
    pub objective: Option<f64>,
    /// 装载分配结果 / Assignment results
    pub assignments: Vec<String>,
    /// 备注信息 / Notes
    pub notes: Vec<String>,
    /// 诊断信息列表 / Diagnostic notes
    #[allow(dead_code)]
    pub diagnostics: Vec<DiagnosticNote>,
}

/// 装载顺序响应数据 / Loading order response data
pub struct LoadingOrderResponse {
    /// 求解状态 / Solve status
    pub status: String,
    /// 装载顺序列表 / Loading order list
    pub orders: Vec<String>,
    /// 备注信息 / Notes
    pub notes: Vec<String>,
    /// 诊断信息列表 / Diagnostic notes
    #[allow(dead_code)]
    pub diagnostics: Vec<DiagnosticNote>,
}
