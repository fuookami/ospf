/// 列生成失败阶段 / Column-generation failure stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnGenerationFailureStage {
    /// 初始列 / Initial columns
    InitialColumns,
    /// RMP 求解 / Restricted master problem
    RestrictedMasterProblem,
    /// 最终 MILP 求解 / Final MILP
    FinalMilp,
    /// 候选过滤 / Candidate filter
    CandidateFilter,
    /// 解分析 / Solution analysis
    SolutionAnalysis,
}

/// 执行器失败 / Executor failure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnGenerationExecutionError {
    /// 失败阶段 / Failure stage
    pub stage: ColumnGenerationFailureStage,
    /// 原始错误 / Original error
    pub message: String,
}

/// RMP 模型扩展 / RMP model extension
pub trait ColumnGenerationRmpModelExtension: Debug + Send + Sync {
    /// 注册额外模型行 / Register additional model rows
    fn register(
        &self,
        state: &ColumnGenerationApplicationState,
        model: &mut MetaModel<f64>,
    ) -> Result<(), String>;

    /// 提取类型化附加对偶值 / Extract typed additional dual values
    fn additional_shadow_prices(
        &self,
        _state: &ColumnGenerationApplicationState,
        _model: &MetaModel<f64>,
        _solve: &MetaModelExecutorSolveResult,
    ) -> HashMap<String, f64> {
        HashMap::new()
    }
}

/// final 模型扩展 / Final model extension
pub trait ColumnGenerationFinalModelExtension: Debug + Send + Sync {
    /// 注册额外最终模型内容 / Register additional final-model content
    fn register(
        &self,
        state: &ColumnGenerationApplicationState,
        model: &mut MetaModel<f64>,
    ) -> Result<(), String>;
}

/// RMP 执行器 / RMP executor
pub trait ColumnGenerationRmpExecutor: Debug + Send + Sync {
    /// 执行 RMP / Execute RMP
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution;

    /// 以 Result 传播失败 / Execute with typed failure propagation
    fn execute_result(
        &self,
        state: &ColumnGenerationApplicationState,
    ) -> Result<ColumnGenerationRmpExecution, ColumnGenerationExecutionError> {
        let execution = self.execute(state);
        if matches!(execution.info.get("status").map(String::as_str),
            Some("registration_failed" | "solve_failed")) {
            return Err(ColumnGenerationExecutionError {
                stage: ColumnGenerationFailureStage::RestrictedMasterProblem,
                message: execution
                    .info
                    .get("error")
                    .cloned()
                    .unwrap_or_else(|| "RMP execution failed".to_string()),
            });
        }
        Ok(execution)
    }
}

/// 最终 MILP 执行器 / Final MILP executor
pub trait ColumnGenerationFinalExecutor: Debug + Send + Sync {
    /// 执行 final MILP / Execute final MILP
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution;

    /// 以 Result 传播失败 / Execute with typed failure propagation
    fn execute_result(
        &self,
        state: &ColumnGenerationApplicationState,
    ) -> Result<ColumnGenerationFinalExecution, ColumnGenerationExecutionError> {
        let execution = self.execute(state);
        if matches!(execution.info.get("status").map(String::as_str),
            Some("registration_failed" | "solve_failed")) {
            return Err(ColumnGenerationExecutionError {
                stage: ColumnGenerationFailureStage::FinalMilp,
                message: execution
                    .info
                    .get("error")
                    .cloned()
                    .unwrap_or_else(|| "final MILP execution failed".to_string()),
            });
        }
        Ok(execution)
    }
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
