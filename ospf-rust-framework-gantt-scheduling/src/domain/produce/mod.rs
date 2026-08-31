//! 产出与消耗上下文 / Produce and consumption context
//!
//! 映射 Kotlin `gantt-scheduling-domain-produce-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-produce-context` submodule.
//!
//! # 核心模块 / Core Modules
//!
//! - [`model`][]: 物料类型、需求/储备、生产任务和使用量组件
//! - [`service`]: 产出/消耗限制和目标 Pipeline

use crate::GanttResult;
use ospf_rust_core::model::MetaModel;

pub mod model;
pub mod service;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use model::{
    ConsumptionUsage, MaterialDemand, MaterialReserves, MaterialTrait, ProduceUsage, Product,
    ProductionTaskTrait, RawMaterial, SemiProduct,
};

pub use service::{
    ConsumptionLessQuantityMinimization, ConsumptionOverQuantityMinimization,
    ConsumptionQuantityConstraint, ConsumptionQuantityMaximization,
    ConsumptionQuantityMinimization, ProduceLessQuantityMinimization,
    ProduceOverQuantityMinimization, ProduceQuantityConstraint, ProduceQuantityMaximization,
    ProduceQuantityMinimization,
};

// ========================================================================
// 产出与消耗聚合 / Produce and Consumption Aggregation
// ========================================================================

/// 产出与消耗聚合 / Produce and consumption aggregation
///
/// 编排产出和消耗使用量注册到 MetaModel。
/// Orchstrates registration of produce and consumption usages to MetaModel.
#[derive(Debug, Default)]
pub struct ProduceAggregation {
    /// 产出使用量列表 / Produce usage list
    pub produce_usages: Vec<ProduceUsage>,
    /// 消耗使用量列表 / Consumption usage list
    pub consumption_usages: Vec<ConsumptionUsage>,
}

impl ProduceAggregation {
    /// 创建新的产出聚合 / Create new produce aggregation
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册所有使用量到模型 / Register all usages to model
    ///
    /// 注意：各 Usage 的 register() 需要在创建时单独调用，
    /// 因为它们需要不同的 demands/reserves 参数。
    /// 此方法保留用于未来统一注册场景。
    ///
    /// Note: Each Usage's register() must be called individually during creation,
    /// as they require different demands/reserves parameters.
    /// This method is reserved for future unified registration scenarios.
    pub fn register(&mut self, _model: &mut MetaModel<f64>) -> GanttResult<()> {
        Ok(())
    }
}
