//! 恢复模型模块 / Recovery model module

/// 恢复航班任务键 / Recovery flight task key (对齐 Kotlin RecoveryFlightTaskKey)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecoveryFlightTaskKey {
    /// 任务标识 / Task identifier
    pub task_id: String,
    /// 恢复类型 / Recovery type
    pub recovery_type: RecoveryType,
}

/// 恢复类型 / Recovery type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryType {
    /// 换飞机 / Aircraft change
    AircraftChange,
    /// 时间调整 / Time change
    TimeChange,
    /// 航线调整 / Route change
    RouteChange,
    /// 取消 / Cancel
    Cancel,
}
