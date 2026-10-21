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

// MetaModel 执行诊断已上移到 `ospf-rust-framework`，见 `executor/backend.rs` 的兼容 re-export。
// MetaModel execution diagnostics moved to `ospf-rust-framework`; see `executor/backend.rs` for the
// compatibility re-export.

