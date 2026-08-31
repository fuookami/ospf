/// 飞机子型号 / Aircraft minor model (对齐 Kotlin AircraftMinorModel)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftMinorModel(pub String);

/// 注册号 / Registration number (对齐 Kotlin RegNo)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegNo(pub String);

/// 航班号 / Flight number (对齐 Kotlin FlightNo)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightNo(pub String);

/// IATA 代码 (对齐 Kotlin IATA)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Iata(pub String);

/// MAC 值 (对齐 Kotlin MAC)
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct MAC(pub f64);

/// 水平安定面角度 / Horizontal stabilizer angle (对齐 Kotlin HorizontalStabilizerAngle)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HorizontalStabilizerAngle(pub String);
