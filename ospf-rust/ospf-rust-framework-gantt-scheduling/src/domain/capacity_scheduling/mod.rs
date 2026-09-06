//! 产能排程上下文 / Capacity scheduling context
//!
//! 映射 Kotlin `gantt-scheduling-domain-capacity-scheduling-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-capacity-scheduling-context` submodule.
//!
//! # 核心模块 / Core Modules
//!
//! - [`model`][]: 产能编译、生产动作、产能列和解组件
//! - [`service`]: 产能排程约束和目标 Pipeline

use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::model::pipeline::Pipeline;

use crate::GanttResult;

pub mod iterative;
pub mod model;
pub mod service;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use iterative::{IterativeCapacityColumn, IterativeCapacityCompilation};

pub use model::{
    ActionAllocation, BasicProductionAction, CapacityColumn, CapacityColumnAggregation,
    CapacityCompilation, CapacityOrderCompilation, CapacitySchedulingSolution,
    ExecutorCapacityResult, ProductionActionTrait,
};

pub use service::{
    CapacityColumnSelectionConstraint, CapacityCostMinimization, ExecutorCapacityConstraint,
    OrderConstraint,
};

// ========================================================================
// 产能排程聚合 / Capacity Scheduling Aggregation
// ========================================================================

/// 产能排程聚合 / Capacity scheduling aggregation
///
/// 编排产能编译和可选的带序编译注册到 MetaModel。
/// Orchstrates registration of capacity compilation and optional order compilation to MetaModel.
#[derive(Debug)]
pub struct CapacitySchedulingAggregation<A: ProductionActionTrait> {
    /// 产能编译（无序）/ Capacity compilation (no order)
    pub compilation: CapacityCompilation<A>,
    /// 可选的带序产能编译 / Optional ordered capacity compilation
    pub order_compilation: Option<CapacityOrderCompilation<A>>,
}

impl<A: ProductionActionTrait> CapacitySchedulingAggregation<A> {
    /// 创建无序产能聚合 / Create no-order capacity aggregation
    pub fn new_no_order(
        actions: Vec<A>,
        executor_ids: Vec<impl Into<A::ExecutorId>>,
        slot_count: usize,
    ) -> Self {
        Self {
            compilation: CapacityCompilation::new(actions, executor_ids, slot_count),
            order_compilation: None,
        }
    }

    /// 创建带序产能聚合 / Create ordered capacity aggregation
    pub fn new_with_order(
        actions: Vec<A>,
        executor_ids: Vec<impl Into<A::ExecutorId>>,
        slot_count: usize,
        max_order: usize,
    ) -> Self {
        let executor_ids = executor_ids.into_iter().map(Into::into).collect::<Vec<_>>();
        Self {
            compilation: CapacityCompilation::new(
                actions.clone(),
                executor_ids.clone(),
                slot_count,
            ),
            order_compilation: Some(CapacityOrderCompilation::new(
                actions,
                executor_ids,
                slot_count,
                max_order,
            )),
        }
    }

    /// 注册所有编译组件到模型 / Register all compilation components to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.compilation.register(model)?;
        if let Some(ref mut order_comp) = self.order_compilation {
            order_comp.register(model)?;
        }
        Ok(())
    }
}

// ========================================================================
// 产能排程上下文 / Capacity Scheduling Context
// ============================================================================

/// 产能排程上下文 / Capacity scheduling context
///
/// 作为应用层入口，组装聚合和限制 Pipeline。
/// Entry point for the application layer, assembling aggregation and limit pipelines.
#[allow(dead_code)]
pub struct CapacitySchedulingContext<A: ProductionActionTrait> {
    /// 产能排程聚合 / Capacity scheduling aggregation
    pub aggregation: CapacitySchedulingAggregation<A>,
    /// 限制 Pipeline 列表 / Limit pipeline list
    pub limits: Vec<Box<dyn Pipeline<MetaModel<f64>>>>,
}

impl<A: ProductionActionTrait> CapacitySchedulingContext<A> {
    /// 创建新的产能排程上下文 / Create new capacity scheduling context
    pub fn new(aggregation: CapacitySchedulingAggregation<A>) -> Self {
        Self {
            aggregation,
            limits: Vec::new(),
        }
    }

    /// 添加限制 Pipeline / Add limit pipeline
    pub fn add_limit(&mut self, limit: Box<dyn Pipeline<MetaModel<f64>>>) {
        self.limits.push(limit);
    }

    /// 注册到模型 / Register to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.aggregation.register(model)?;
        for limit in &self.limits {
            limit.register(model);
        }
        Ok(())
    }

    /// 调用验证 / Invoke validation
    pub fn invoke(&self, model: &MetaModel<f64>) -> crate::GanttResult<()> {
        for limit in &self.limits {
            limit
                .invoke(model)
                .map_err(|e| crate::GanttError::Calculation {
                    message: format!("Limit invoke failed: {:?}", e),
                })?;
        }
        Ok(())
    }
}
