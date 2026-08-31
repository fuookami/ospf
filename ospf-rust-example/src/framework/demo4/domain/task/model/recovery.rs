/// 恢复航班任务键 / Recovery flight task key (对齐 Kotlin RecoveryFlightTaskKey)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecoveryFlightTaskKey {
    pub task_id: String,
    pub recovery_type: RecoveryType,
}

/// 恢复类型 / Recovery type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryType {
    AircraftChange,
    TimeChange,
    RouteChange,
    Cancel,
}
