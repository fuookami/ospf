//! Demo4 输出数据传输对象 / Demo4 output data transfer object
//!
//! 定义航班调度恢复的输出数据结构。
//! Defines output data structures for airline scheduling and recovery.

/// 输出数据传输对象 / Output data transfer object
/// 对齐 Kotlin Output（Kotlin 也为空类）
#[derive(Debug, Clone, Default)]
pub struct Output {
    /// 求解状态 / Solver status
    pub status: String,
    /// 分配结果列表 / Assignment result list
    pub assignments: Vec<String>,
}
