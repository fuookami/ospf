/// NodeBandwidth tracks per-node bandwidth demand.
/// In the Kotlin model this is an intermediate symbol derived from ServiceBandwidth.
/// In Rust we compute it on-the-fly in constraints; this struct holds no state.
pub struct NodeBandwidth;

impl NodeBandwidth {
    pub fn new() -> Self {
        Self
    }
}
