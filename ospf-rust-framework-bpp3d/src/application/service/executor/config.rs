/// MetaModel RMP executor 配置 / MetaModel RMP executor config
#[derive(Debug, Clone)]
pub struct MetaModelRmpExecutorConfig {
    /// 模型名称 / Model name
    pub model_name: String,
    /// no-op shadow price 向量 / No-op shadow price vector
    pub shadow_prices: Vec<f64>,
    /// no-op 原始解向量 / No-op primal solution vector
    pub primal_solution: Vec<f64>,
    /// no-op 目标值 / No-op objective
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
#[derive(Debug, Clone)]
pub struct MetaModelFinalExecutorConfig {
    /// 模型名称 / Model name
    pub model_name: String,
    /// no-op 原始解向量 / No-op primal solution vector
    pub primal_solution: Vec<f64>,
    /// no-op 目标值 / No-op objective
    pub objective: Option<f64>,
}

impl Default for MetaModelFinalExecutorConfig {
    fn default() -> Self {
        Self {
            model_name: "bpp3d_final_milp".to_string(),
            primal_solution: Vec::new(),
            objective: None,
        }
    }
}

/// MetaModel executor 求解结果 / MetaModel executor solve result
#[derive(Debug, Clone, Default)]
pub struct MetaModelExecutorSolveResult {
    /// 目标值 / Objective
    pub objective: Option<f64>,
    /// 原始解向量 / Primal solution vector
    pub primal_solution: Vec<f64>,
    /// 对偶解向量 / Dual solution vector
    pub dual_solution: Vec<f64>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

