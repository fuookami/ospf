/// 分离空装载 / Divide empty loading (对齐 Kotlin DivideEmptyLoading)
#[derive(Debug, Clone)]
pub struct DivideEmptyLoading {
    pub item_id: String,
    pub position_id: String,
    pub empty_ratio: f64,
}
