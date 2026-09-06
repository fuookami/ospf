//! 编组生成领域模型 / Bunch generation domain model.
/// 航班任务反向索引 / Flight task reverse index
pub mod flight_task_reverse;
/// 路由图模型 / Route graph model
pub mod graph;

/// 航班任务反向索引重导出 / Flight task reverse index re-exports
pub use flight_task_reverse::*;
/// 路由图模型重导出 / Route graph model re-exports
pub use graph::*;
