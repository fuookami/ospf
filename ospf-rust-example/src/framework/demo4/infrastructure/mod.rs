pub mod dto;
pub mod instant;
pub mod semantic_parameter;
pub mod solver;

/// IATA 三字码 / IATA 3-letter code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Iata(pub String);

/// ICAO 四字码 / ICAO 4-letter code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Icao(pub String);

/// 飞机类型名称 / Aircraft type name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftTypeName(pub String);

/// 飞机类型代码 / Aircraft type code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftTypeCode(pub String);

/// 飞机子类型名称 / Aircraft minor type name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftMinorTypeName(pub String);

/// 飞机子类型代码 / Aircraft minor type code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftMinorTypeCode(pub String);

/// 翼型飞机类型代码 / Wing aircraft type code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WingAircraftTypeCode(pub String);

/// 飞机注册号 / Aircraft register number
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftRegisterNumber(pub String);

/// 旅客舱位 / Passenger class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PassengerClass(pub String);

/// 飞行员等级号 / Pilot rank number
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PilotRankNo(pub String);

/// 飞行员代码 / Pilot code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PilotCode(pub String);

/// 机组成员等级号 / Crew member rank number
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrewManRankNo(pub String);

/// 工人编号 / Worker number
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkerNo(pub String);
