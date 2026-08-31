use time::{Duration, OffsetDateTime};

/// 锁定 / Lock
#[derive(Debug, Clone)]
pub struct Lock {
    pub id: String,
    pub task_id: String,
}

/// 流量控制场景 / Flow control scene (对齐 Kotlin FlowControlScene)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowControlScene {
    Departure,
    Arrival,
    DepartureArrival,
    Stay,
}

impl FlowControlScene {
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
    pub airport: String,
    pub scene: FlowControlScene,
    pub time_range: (OffsetDateTime, OffsetDateTime),
}

/// 流量控制容量 / Flow control capacity
#[derive(Debug, Clone)]
pub struct FlowControlCapacity {
    pub max_capacity: u64,
    pub current_usage: u64,
}

/// 流量控制 / Flow control (对齐 Kotlin FlowControl)
#[derive(Debug, Clone)]
pub struct FlowControl {
    pub id: String,
    pub condition: FlowControlCondition,
    pub capacity: FlowControlCapacity,
}

impl FlowControl {
    pub fn available(&self) -> u64 {
        self.capacity.max_capacity.saturating_sub(self.capacity.current_usage)
    }
}
