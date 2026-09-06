//! 编组编制领域模型 / Bunch compilation domain model.
/// 编制模型 / Compilation model
pub mod compilation;
/// 机队平衡模型 / Fleet balance model
pub mod fleet_balance;
/// 航班容量模型 / Flight capacity model
pub mod flight_capacity;
/// 航班链接模型 / Flight link model
pub mod flight_link;

/// 编制 / Compilation
pub use compilation::Compilation;
/// 机队平衡 / Fleet balance
pub use fleet_balance::{FleetBalance, FleetBalanceCheckpoint, FleetBalanceLimit};
/// 航班容量 / Flight capacity
pub use flight_capacity::FlightCapacity;
/// 航班链接 / Flight link
pub use flight_link::FlightLink;
