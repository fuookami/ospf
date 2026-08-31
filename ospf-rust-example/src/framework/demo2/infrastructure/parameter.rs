/// 业务参数 / Business parameter (对齐 Kotlin Parameter)
#[derive(Debug, Clone)]
pub struct Parameter {
    // 重心优化上下文
    pub mac_range_c: f64,
    pub longitudinal_balance: f64,
    pub b737_longitudinal_balance: f64,
    pub lateral_balance: f64,
    pub horizontal_stabilizer_warn: f64,
    // 软性安全上下文
    pub ballast_weight: f64,
    pub empty_hated: f64,
    pub beside_door_main_position: f64,
    pub divided_empty: f64,
    // 装卸效率上下文
    pub advice_load_amount: f64,
    pub advice_load_weight: f64,
    pub same_flow_transfer_in: f64,
    pub same_flow_transfer_out: f64,
    pub item_order: f64,
    pub trailer_change: f64,
    pub trailer_circling: f64,
    // 货物时效上下文
    pub priority: f64,
    pub priority_category: f64,
    // 余度上下文
    pub experimental_longitudinal_balance: f64,
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
