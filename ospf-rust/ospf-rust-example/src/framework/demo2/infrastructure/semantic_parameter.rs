//! 语义参数类型 / Semantic parameter types
/// 飞机子型号 / Aircraft minor model (对齐 Kotlin AircraftMinorModel / Aligned with Kotlin AircraftMinorModel)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftMinorModel(pub String);

/// 注册号 / Registration number (对齐 Kotlin RegNo / Aligned with Kotlin RegNo)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegNo(pub String);

/// 航班号 / Flight number (对齐 Kotlin FlightNo / Aligned with Kotlin FlightNo)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightNo(pub String);

/// IATA 代码 / IATA code (对齐 Kotlin IATA / Aligned with Kotlin IATA)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Iata(pub String);

/// MAC 值 / MAC value (对齐 Kotlin MAC / Aligned with Kotlin MAC)
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct MAC(pub f64);

/// 水平安定面角度 / Horizontal stabilizer angle (对齐 Kotlin HorizontalStabilizerAngle / Aligned with Kotlin HorizontalStabilizerAngle)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HorizontalStabilizerAngle(pub String);
