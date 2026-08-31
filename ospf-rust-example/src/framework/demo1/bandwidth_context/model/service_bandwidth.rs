/// ServiceBandwidth tracks per-service bandwidth usage.
/// In the Kotlin model this is an intermediate symbol derived from EdgeBandwidth.
/// In Rust we compute it on-the-fly in constraints; this struct holds no state.
pub struct ServiceBandwidth;

impl ServiceBandwidth {
    pub fn new() -> Self {
        Self
    }
}
