/// RMP executor / RMP executor
pub trait ColumnGenerationRmpExecutor: Debug + Send + Sync {
    /// 执行 RMP / Execute RMP
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution;
}

/// final MILP executor / Final MILP executor
pub trait ColumnGenerationFinalExecutor: Debug + Send + Sync {
    /// 执行 final MILP / Execute final MILP
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution;
}

/// MetaModel 执行诊断 / MetaModel execution diagnostics
#[derive(Debug, Clone, Default)]
pub struct MetaModelExecutionDiagnostics {
    /// 模型名称 / Model name
    pub model_name: String,
    /// 变量数量 / Variable count
    pub variable_count: usize,
    /// 约束数量 / Constraint count
    pub constraint_count: usize,
    /// 注册的需求数量 / Registered demand count
    pub demand_count: usize,
    /// 注册的层数量 / Registered layer count
    pub layer_count: usize,
    /// 注册的箱数量 / Registered bin count
    pub bin_count: usize,
}

impl MetaModelExecutionDiagnostics {
    /// 从模型创建诊断 / Create diagnostics from model
    pub fn from_model(
        model_name: impl Into<String>,
        model: &MetaModel<f64>,
        demand_count: usize,
        layer_count: usize,
        bin_count: usize,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            variable_count: model.num_tokens(),
            constraint_count: model.num_constraints(),
            demand_count,
            layer_count,
            bin_count,
        }
    }

    /// 写入 info map / Write into info map
    pub fn write_info(&self, info: &mut HashMap<String, String>) {
        info.insert("model_name".to_string(), self.model_name.clone());
        info.insert("variable_count".to_string(), self.variable_count.to_string());
        info.insert("constraint_count".to_string(), self.constraint_count.to_string());
        info.insert("demand_count".to_string(), self.demand_count.to_string());
        info.insert("layer_count".to_string(), self.layer_count.to_string());
        info.insert("bin_count".to_string(), self.bin_count.to_string());
    }
}

