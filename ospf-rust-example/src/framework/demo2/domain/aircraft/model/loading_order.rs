//! 装载顺序定义 / Loading order definitions
/// 装载顺序 / Loading order (对齐 Kotlin LoadingOrder)
#[derive(Debug, Clone)]
pub struct LoadingOrder {
    /// 舱位标识 / Position identifier
    pub position: String,
    /// 装载序号 / Loading order number
    pub order: u32,
    /// 直接前驱舱位 / Direct predecessor position
    pub direct_prec: Option<String>,
    /// 直接后继舱位 / Direct successor position
    pub direct_succ: Option<String>,
}
