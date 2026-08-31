//! Demo4 输入数据传输对象 / Demo4 input data transfer object
//!
//! 定义航班调度恢复的输入数据结构。
//! Defines input data structures for airline scheduling and recovery.

/// 输入数据传输对象 / Input data transfer object
/// 对齐 Kotlin Input（Kotlin 也为空类）
#[derive(Debug, Clone, Default)]
pub struct Input {
    /// 航班标识列表 / Flight identifier list
    pub flights: Vec<String>,
    /// 飞机标识列表 / Aircraft identifier list
    pub aircrafts: Vec<String>,
}
