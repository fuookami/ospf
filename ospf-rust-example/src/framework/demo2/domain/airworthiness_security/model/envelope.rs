/// 包络线 / Envelope (对齐 Kotlin airworthiness_security Envelope)
#[derive(Debug, Clone)]
pub struct Envelope {
    pub points: Vec<(f64, f64)>, // (totalWeight, index)
}
