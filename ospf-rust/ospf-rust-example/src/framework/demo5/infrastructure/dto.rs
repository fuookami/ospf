//! Demo5 DTO / Demo5 DTOs.

use std::time::Duration;

/// Solomon 车辆数据 / Solomon vehicle data.
#[derive(Debug, Clone, PartialEq)]
pub struct SolomonVehicleData {
    /// 车辆数量 / Vehicle amount.
    pub amount: usize,
    /// 单车容量 / Vehicle capacity.
    pub capacity: f64,
}

/// Solomon 节点数据 / Solomon node data.
#[derive(Debug, Clone, PartialEq)]
pub struct SolomonNodeData {
    /// 原始节点 ID / Source node ID.
    pub id: String,
    /// X 坐标 / X coordinate.
    pub x: f64,
    /// Y 坐标 / Y coordinate.
    pub y: f64,
    /// 需求 / Demand.
    pub demand: f64,
    /// 最早服务时刻（秒） / Earliest service time in seconds.
    pub ready_time: f64,
    /// 最晚服务时刻（秒） / Latest service time in seconds.
    pub due_time: f64,
    /// 服务时长（秒） / Service duration in seconds.
    pub service_time: f64,
}

/// Solomon 输入 / Solomon input.
#[derive(Debug, Clone, PartialEq)]
pub struct SolomonData {
    /// 实例名称 / Instance name.
    pub name: String,
    /// 车辆数据 / Vehicle data.
    pub vehicle: SolomonVehicleData,
    /// 唯一 depot / Unique depot.
    pub depot: SolomonNodeData,
    /// 客户数据；保留输入中的最后一行 / Customer data; the final input row is retained.
    pub customers: Vec<SolomonNodeData>,
}

/// Demo5 运行参数 / Demo5 run parameters.
#[derive(Debug, Clone)]
pub struct Demo5Input {
    /// inline Solomon 文本 / Inline Solomon text.
    pub solomon_text: String,
    /// 时间上限 / Time limit.
    pub time_limit: Option<Duration>,
    /// 节点上限 / Node limit.
    pub node_limit: usize,
    /// 相对 gap 容差 / Relative gap tolerance.
    pub relative_gap_tolerance: f64,
    /// 每节点列生成迭代上限 / Per-node column-generation iteration limit.
    pub max_cg_iterations_per_node: usize,
}

impl Default for Demo5Input {
    fn default() -> Self {
        Self {
            solomon_text: String::new(),
            time_limit: Some(Duration::from_secs(30)),
            node_limit: 10_000,
            relative_gap_tolerance: 1e-4,
            max_cg_iterations_per_node: 1_000,
        }
    }
}
