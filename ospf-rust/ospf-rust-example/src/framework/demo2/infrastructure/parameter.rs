//! 业务参数定义 / Business parameter definitions
/// 业务参数 / Business parameters (对齐 Kotlin Parameter / Aligned with Kotlin Parameter)
#[derive(Debug, Clone)]
pub struct Parameter {
    /// 重心优化 - MAC 范围 C 系数 / Center-of-gravity optimization - MAC range C coefficient
    pub mac_range_c: f64,
    /// 重心优化 - 纵向平衡系数 / Center-of-gravity optimization - longitudinal balance coefficient
    pub longitudinal_balance: f64,
    /// 重心优化 - B737 纵向平衡系数 / Center-of-gravity optimization - B737 longitudinal balance coefficient
    pub b737_longitudinal_balance: f64,
    /// 重心优化 - 横向平衡系数 / Center-of-gravity optimization - lateral balance coefficient
    pub lateral_balance: f64,
    /// 重心优化 - 水平安定面警告系数 / Center-of-gravity optimization - horizontal stabilizer warning coefficient
    pub horizontal_stabilizer_warn: f64,
    /// 软性安全 - 压舱重量系数 / Soft security - ballast weight coefficient
    pub ballast_weight: f64,
    /// 软性安全 - 空舱惩罚系数 / Soft security - empty hold penalty coefficient
    pub empty_hated: f64,
    /// 软性安全 - 舱门旁主舱位系数 / Soft security - beside-door main position coefficient
    pub beside_door_main_position: f64,
    /// 软性安全 - 分散空舱系数 / Soft security - divided empty hold coefficient
    pub divided_empty: f64,
    /// 装卸效率 - 建议装载数量系数 / Loading efficiency - advised load amount coefficient
    pub advice_load_amount: f64,
    /// 装卸效率 - 建议装载重量系数 / Loading efficiency - advised load weight coefficient
    pub advice_load_weight: f64,
    /// 装卸效率 - 同流向转入系数 / Loading efficiency - same-flow transfer-in coefficient
    pub same_flow_transfer_in: f64,
    /// 装卸效率 - 同流向转出系数 / Loading efficiency - same-flow transfer-out coefficient
    pub same_flow_transfer_out: f64,
    /// 装卸效率 - 货物排序系数 / Loading efficiency - item order coefficient
    pub item_order: f64,
    /// 装卸效率 - 拖车更换系数 / Loading efficiency - trailer change coefficient
    pub trailer_change: f64,
    /// 装卸效率 - 拖车绕行系数 / Loading efficiency - trailer circling coefficient
    pub trailer_circling: f64,
    /// 货物时效 - 优先级系数 / Cargo timeliness - priority coefficient
    pub priority: f64,
    /// 货物时效 - 优先级类别系数 / Cargo timeliness - priority category coefficient
    pub priority_category: f64,
    /// 余度 - 实验性纵向平衡系数 / Redundancy - experimental longitudinal balance coefficient
    pub experimental_longitudinal_balance: f64,
    /// 余度 - 冗余范围系数 / Redundancy - redundancy range coefficient
    pub redundancy_range: f64,
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            mac_range_c: 1.0,
            longitudinal_balance: 1.0,
            b737_longitudinal_balance: 1.0,
            lateral_balance: 1.0,
            horizontal_stabilizer_warn: 1.0,
            ballast_weight: 1.0,
            empty_hated: 1.0,
            beside_door_main_position: 1.0,
            divided_empty: 1.0,
            advice_load_amount: 1.0,
            advice_load_weight: 1.0,
            same_flow_transfer_in: 1.0,
            same_flow_transfer_out: 1.0,
            item_order: 1.0,
            trailer_change: 1.0,
            trailer_circling: 1.0,
            priority: 1.0,
            priority_category: 1.0,
            experimental_longitudinal_balance: 1.0,
            redundancy_range: 1.0,
        }
    }
}
