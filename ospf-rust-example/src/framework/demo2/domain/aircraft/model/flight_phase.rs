/// 飞行阶段 / Flight phase (对齐 Kotlin FlightPhase)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightPhase {
    ZeroFuel,
    TakeOff,
    Landing,
}
