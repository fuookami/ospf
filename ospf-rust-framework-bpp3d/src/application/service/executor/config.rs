/// MetaModel RMP executor 配置 / MetaModel RMP executor config
#[derive(Debug, Clone)]
pub struct MetaModelRmpExecutorConfig {
    /// 模型名称 / Model name
    pub model_name: String,
    /// 空操作影子价格向量 / No-op shadow price vector
    pub shadow_prices: Vec<f64>,
    /// 空操作原始解向量 / No-op primal solution vector
    pub primal_solution: Vec<f64>,
    /// 空操作目标值 / No-op objective
    pub objective: Option<f64>,
}

impl Default for MetaModelRmpExecutorConfig {
    fn default() -> Self {
        Self {
            model_name: "bpp3d_rmp".to_string(),
            shadow_prices: Vec::new(),
            primal_solution: Vec::new(),
            objective: None,
        }
    }
}

/// MetaModel final executor 配置 / MetaModel final executor config
///
/// 目标分层：体积消耗为主目标（等价于体积浪费最少），箱数与箱型成本为可配次级权重。
/// 默认值与 BPP2D 对齐。
/// Layered objectives: volume consumption is primary (equivalent to minimum volume waste), while bin amount
/// and per-bin-type cost are configurable secondary weights. Defaults are aligned with BPP2D.
#[derive(Debug, Clone)]
pub struct MetaModelFinalExecutorConfig {
    /// 模型名称 / Model name
    pub model_name: String,
    /// 空操作原始解向量 / No-op primal solution vector
    pub primal_solution: Vec<f64>,
    /// 空操作目标值 / No-op objective
    pub objective: Option<f64>,
    /// 箱型体积消耗权重（主目标）/ Bin volume consumption weight (primary)
    pub final_bin_volume_weight: f64,
    /// 箱数系数（默认极小，仅用于平局裁决）/ Bin amount coefficient (tiny by default, tie breaker only)
    pub final_bin_amount_coefficient: f64,
    /// 箱型成本权重 / Per-bin-type cost weight
    pub final_bin_type_cost_weight: f64,
    /// 箱型到成本的映射 / Bin type to cost mapping
    pub final_bin_type_costs: HashMap<BinTypeId, f64>,
}

impl Default for MetaModelFinalExecutorConfig {
    fn default() -> Self {
        Self {
            model_name: "bpp3d_final_milp".to_string(),
            primal_solution: Vec::new(),
            objective: None,
            final_bin_volume_weight: 1.0,
            // 体积为主目标；箱数系数极小，仅用于在「体积消耗相同」的解之间打破平局，避免多选零余料的整箱。
            // Volume is primary; the bin amount coefficient is tiny and only breaks ties between equal-volume
            // solutions, so the model does not add perfectly filled bins for free.
            final_bin_amount_coefficient: 1e-6,
            final_bin_type_cost_weight: 0.0,
            final_bin_type_costs: HashMap::new(),
        }
    }
}

// MetaModel executor 求解结果已上移到 `ospf-rust-framework`，见 `executor/backend.rs` 的兼容 re-export。
// MetaModel executor solve result moved to `ospf-rust-framework`; see `executor/backend.rs` for the
// compatibility re-export.
