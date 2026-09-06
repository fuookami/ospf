//! 模型构建状态 / Model building status

use crate::error::Result;
use std::sync::Arc;

/// 模型阶段 / Model stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBuildingStage {
    /// 注册 token / Register tokens
    RegisterTokens,
    /// 注册线性约束 / Register linear constraints
    RegisterLinearConstraints,
    /// 注册二次约束 / Register quadratic constraints
    RegisterQuadraticConstraints,
    /// 注册符号约束 / Register symbol constraints
    RegisterSymbols,
    /// 平展线性模型 / Flatten linear model
    FlattenLinearModel,
    /// 平展二次模型 / Flatten quadratic model
    FlattenQuadraticModel,
    /// 组装目标函数 / Assemble objective
    BuildObjective,
}

/// 模型构建状态快照 / Model building status snapshot
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBuildingStatus {
    /// 模型名 / Model name
    pub model_name: String,
    /// 当前阶段 / Current stage
    pub stage: ModelBuildingStage,
    /// 已完成数量 / Ready amount
    pub ready: usize,
    /// 总数量 / Total amount
    pub total: usize,
}

impl ModelBuildingStatus {
    /// 创建状态 / Create status
    pub fn new(
        model_name: impl Into<String>,
        stage: ModelBuildingStage,
        ready: usize,
        total: usize,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            stage,
            ready,
            total,
        }
    }
}

/// 模型构建状态回调 / Model building status callback
pub type ModelBuildingStatusCallback =
    Arc<dyn Fn(&ModelBuildingStatus) -> Result<()> + Send + Sync>;
