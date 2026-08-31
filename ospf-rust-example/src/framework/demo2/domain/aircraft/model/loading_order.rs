/// 装载顺序 / Loading order (对齐 Kotlin LoadingOrder)
#[derive(Debug, Clone)]
pub struct LoadingOrder {
    pub position: String,
    pub order: u32,
    pub direct_prec: Option<String>,
    pub direct_succ: Option<String>,
}
