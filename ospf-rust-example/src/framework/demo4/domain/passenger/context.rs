//! 旅客上下文模块 / Passenger context module.
use super::model::Passenger;

/// 旅客上下文 / Passenger context
/// 对齐 Kotlin PassengerContext / Aligned with Kotlin PassengerContext
#[derive(Debug)]
pub struct PassengerContext {
    /// 旅客列表 / Passenger list
    pub passengers: Vec<Passenger>,
}

impl PassengerContext {
    /// 创建新的旅客上下文 / Create new passenger context
    pub fn new() -> Self {
        Self { passengers: Vec::new() }
    }
}
