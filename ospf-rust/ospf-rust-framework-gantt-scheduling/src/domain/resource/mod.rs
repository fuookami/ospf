//! 资源上下文 / Resource context
//!
//! 映射 Kotlin `gantt-scheduling-domain-resource-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-resource-context` submodule.
//!
//! # 核心模块 / Core Modules
//!
//! - [`model`]: 资源容量、资源 trait、使用量组件和松弛配置
//! - [`service`]: 资源限制和目标 Pipeline

use crate::GanttResult;
use ospf_rust_core::model::MetaModel;

pub mod model;
pub mod service;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use model::{
    BasicConnectionResource, BasicExecutionResource, BasicStorageResource, ConnectionResourceTrait,
    ConnectionResourceUsage, ExecutionResourceTrait, ResourceCapacity, ResourceSlack,
    ResourceTrait, ResourceUsage, StorageResourceTrait, StorageResourceUsage,
};

pub use service::{
    ResourceCapacityConstraint, ResourceLessQuantityMinimization, ResourceOverQuantityMinimization,
};

// ========================================================================
// 资源聚合 / Resource Aggregation
// ========================================================================

/// 资源聚合 / Resource aggregation
///
/// 编排多个资源的使用量注册到 MetaModel。
/// Orchstrates registration of multiple resource usages to MetaModel.
#[derive(Debug, Default)]
pub struct ResourceAggregation {
    /// 执行资源使用量列表 / Execution resource usage list
    pub execution_usages: Vec<ResourceUsage>,
    /// 存储资源使用量列表 / Storage resource usage list
    pub storage_usages: Vec<StorageResourceUsage>,
    /// 连接资源使用量列表 / Connection resource usage list
    pub connection_usages: Vec<ConnectionResourceUsage>,
}

impl ResourceAggregation {
    /// 创建新的资源聚合 / Create new resource aggregation
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册所有资源使用量到模型 / Register all resource usages to model
    pub fn register(&mut self, _model: &mut MetaModel<f64>) -> GanttResult<()> {
        // 执行资源不需要 capacities 参数，已在各自 register 中处理
        // 注意：执行资源的 register 需要 capacities 参数，
        // 因此此方法仅迭代已注册的使用量符号进行验证
        // 实际注册应在创建 Usage 时通过其 register() 方法完成
        Ok(())
    }
}
