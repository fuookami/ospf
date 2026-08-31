//! 旅客领域模型 / Passenger domain model.
/// 旅客模型 / Passenger model
pub mod passenger;
/// 旅客数量模型 / Passenger amount model
pub mod passenger_amount;
/// 旅客取消模型 / Passenger cancel model
pub mod passenger_cancel;
/// 旅客变更模型 / Passenger change model
pub mod passenger_change;

/// 旅客模型重导出 / Passenger model re-exports
pub use passenger::*;
/// 旅客人数模型重导出 / Passenger amount model re-exports
pub use passenger_amount::*;
/// 旅客取消模型重导出 / Passenger cancel model re-exports
pub use passenger_cancel::*;
/// 旅客签转模型重导出 / Passenger change model re-exports
pub use passenger_change::*;
