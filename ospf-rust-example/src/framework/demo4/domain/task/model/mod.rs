//! 任务领域模型 / Task domain model.
/// 加班航班模型 / Additional flight model
pub mod additional_flight;
/// 航空器模型 / Aircraft model
pub mod aircraft;
/// 航空器类别模型 / Aircraft category model
pub mod aircraft_category;
/// 机型模型 / Aircraft type model
pub mod aircraft_type;
/// 机场模型 / Airport model
pub mod airport;
/// 停场航空器模型 / Aircraft on ground model
pub mod aog;
/// 航班循环模型 / Flight cycle model
pub mod flight_cycle;
/// 航段模型 / Flight leg model
pub mod flight_leg;
/// 航班任务模型 / Flight task model
pub mod flight_task;
/// 航班任务束模型 / Flight task bunch model
pub mod flight_task_bunch;
/// 航班类型模型 / Flight type model
pub mod flight_type;
/// 维修任务模型 / Maintenance task model
pub mod maintenance;
/// 恢复任务模型 / Recovery task model
pub mod recovery;
/// 影子价格映射模型 / Shadow price map model
pub mod shadow_price_map;
/// 中转任务模型 / Transfer task model
pub mod transfer;

/// 航空器模型重导出 / Aircraft model re-exports
pub use aircraft::*;
/// 机型模型重导出 / Aircraft type model re-exports
pub use aircraft_type::*;
/// 机场模型重导出 / Airport model re-exports
pub use airport::*;
/// 停场航空器模型重导出 / Aircraft on ground model re-exports
pub use aog::*;
/// 航班循环模型重导出 / Flight cycle model re-exports
pub use flight_cycle::*;
/// 航段模型重导出 / Flight leg model re-exports
pub use flight_leg::*;
/// 航班任务模型重导出 / Flight task model re-exports
pub use flight_task::*;
/// 航班任务束模型重导出 / Flight task bunch model re-exports
pub use flight_task_bunch::*;
/// 航班类型模型重导出 / Flight type model re-exports
pub use flight_type::*;
/// 维修任务模型重导出 / Maintenance task model re-exports
pub use maintenance::*;
/// 中转任务模型重导出 / Transfer task model re-exports
pub use transfer::*;
