//! 流控规则模型 / Flow control rule model

use time::{Duration, OffsetDateTime};

/// 锁定 / Lock
#[derive(Debug, Clone)]
pub struct Lock {
    /// 锁定标识 / Lock identifier
    pub id: String,
    /// 关联任务标识 / Associated task identifier
    pub task_id: String,
}

/// 流量控制场景 / Flow control scene (对齐 Kotlin FlowControlScene)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowControlScene {
    /// 出场控制 / Departure control
    Departure,
    /// 进场控制 / Arrival control
    Arrival,
    /// 出进场控制 / Departure and arrival control
    DepartureArrival,
    /// 停场控制 / Stay control
    Stay,
}

impl FlowControlScene {
    /// 评估机场是否匹配该场景 / Evaluate whether the airport matches this scene
    pub fn evaluate(&self, dep: &str, arr: &str, airport: &str) -> bool {
        match self {
            FlowControlScene::Departure => dep == airport,
            FlowControlScene::Arrival => arr == airport,
            FlowControlScene::DepartureArrival => dep == airport || arr == airport,
            FlowControlScene::Stay => dep == airport && arr == airport,
        }
    }
}

/// 流量控制条件 / Flow control condition
#[derive(Debug, Clone)]
pub struct FlowControlCondition {
    /// 受控机场 / Controlled airport
    pub airport: String,
    /// 流控场景 / Flow control scene
    pub scene: FlowControlScene,
    /// 流控时间范围 / Flow control time range
    pub time_range: (OffsetDateTime, OffsetDateTime),
}

/// 流量控制容量 / Flow control capacity
#[derive(Debug, Clone)]
pub struct FlowControlCapacity {
    /// 最大容量 / Maximum capacity
    pub max_capacity: u64,
    /// 当前已用容量 / Current usage
    pub current_usage: u64,
}

/// 流量控制 / Flow control (对齐 Kotlin FlowControl)
#[derive(Debug, Clone)]
pub struct FlowControl {
    /// 流控标识 / Flow control identifier
    pub id: String,
    /// 流控条件 / Flow control condition
    pub condition: FlowControlCondition,
    /// 流控容量 / Flow control capacity
    pub capacity: FlowControlCapacity,
}

impl FlowControl {
    /// 获取剩余可用容量 / Get remaining available capacity
    pub fn available(&self) -> u64 {
        self.capacity
            .max_capacity
            .saturating_sub(self.capacity.current_usage)
    }
}
