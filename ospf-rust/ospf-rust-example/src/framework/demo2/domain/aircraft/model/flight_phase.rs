//! 飞行阶段定义 / Flight phase definitions
/// 飞行阶段 / Flight phase (对齐 Kotlin FlightPhase)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightPhase {
    /// 零油 / Zero fuel
    ZeroFuel,
    /// 起飞 / Take-off
    TakeOff,
    /// 着陆 / Landing
    Landing,
}
