use super::model::Passenger;

/// 旅客上下文 / Passenger context
/// 对齐 Kotlin PassengerContext
#[derive(Debug)]
pub struct PassengerContext {
    pub passengers: Vec<Passenger>,
}

impl PassengerContext {
    pub fn new() -> Self {
        Self { passengers: Vec::new() }
    }
}
