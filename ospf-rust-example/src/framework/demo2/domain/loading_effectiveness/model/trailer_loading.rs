use super::trailer::Trailer;

/// 拖车装载 / Trailer loading (对齐 Kotlin TrailerLoading)
#[derive(Debug, Clone)]
pub struct TrailerLoading {
    pub trailer: Trailer,
    pub position_id: String,
}
