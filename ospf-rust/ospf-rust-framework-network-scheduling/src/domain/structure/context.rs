//! 结构建模、事务回滚与下游 pipeline 入口。 / Structure modeling, transaction rollback and downstream pipeline entry point.

use std::fmt::Debug;
use std::sync::Arc;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::value::SolveValue;
use crate::error::{NetworkSchedulingError, Result};
use super::{NetworkStructureAggregation, NetworkStructureProblem, NetworkStructureSolution};

/// 每 context 的结构业务扩展，不跨模型共享注册态。 / Per-context structure extension without cross-model registration state.
pub trait NetworkStructurePipeline<V: SolveValue + UnitConversionValue>: Debug + Send + Sync {
    /// 用树、建设和分区表达式追加业务规则。 / Append business rules using tree, construction and partition expressions.
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkStructureAggregation<V>) -> Result<()>;
}

/// 同模型重复注册必须保持一致的选项。 / Options that must agree across repeated registration in one model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkStructureRegistration {
    /// 最小化纯建设费用；嵌入式可交给外层目标 / Minimize construction-only cost, optionally delegated to an outer objective.
    pub minimize_construction_cost: bool,
}

impl Default for NetworkStructureRegistration {
    fn default() -> Self { Self { minimize_construction_cost: true } }
}

/// 根树与设施资格归属的事务式建模入口。 / Transactional modeling entry point for rooted trees and eligible facility ownership.
#[derive(Debug, Clone)]
pub struct NetworkStructureContext<V: SolveValue + UnitConversionValue> {
    name: String,
    aggregation: NetworkStructureAggregation<V>,
    pipelines: Vec<Arc<dyn NetworkStructurePipeline<V>>>,
    binding: Option<(u64, NetworkStructureRegistration)>,
}

impl<V: SolveValue + UnitConversionValue> NetworkStructureContext<V> {
    /// 从不可变问题创建独立 context。 / Create an independent context from an immutable problem.
    pub fn new(problem: NetworkStructureProblem<V>, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        Ok(Self { aggregation: NetworkStructureAggregation::new(problem, &name)?, name, pipelines: Vec::new(), binding: None })
    }

    /// 公开只读聚合供下游追加模型组件。 / Expose a read-only aggregation for downstream model components.
    pub fn aggregation(&self) -> &NetworkStructureAggregation<V> { &self.aggregation }

    /// 在注册前追加业务 pipeline。 / Append a business pipeline before registration.
    pub fn add_pipeline(&mut self, pipeline: Arc<dyn NetworkStructurePipeline<V>>) -> Result<()> {
        if self.binding.is_some() {
            return Err(NetworkSchedulingError::contract("注册后不能追加结构管道 / cannot append a structure pipeline after registration"));
        }
        self.pipelines.push(pipeline);
        Ok(())
    }

    /// 原子注册建设、根树、覆盖、扩展与可选目标；不注册商品流。 / Atomically register construction, tree, coverage, extensions and optional objective, without commodity flows.
    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        options: NetworkStructureRegistration,
    ) -> Result<()> {
        if let Some(binding) = self.binding {
            return if binding == (model.model_identity(), options) { Ok(()) } else {
                Err(NetworkSchedulingError::contract("结构模型或重复注册选项不匹配 / structure model or repeated-registration options mismatch"))
            };
        }
        let mut next = self.aggregation.clone();
        model.transaction(|model| {
            next.register(model, &self.name)?;
            for pipeline in &self.pipelines { pipeline.register(model, &next)?; }
            if options.minimize_construction_cost {
                model.set_objective_category(ObjectiveCategory::Minimum);
                model.add_linear_objective(next.cost_expression(), &format!("{}_construction_cost", self.name));
            }
            Ok(())
        })?;
        self.aggregation = next;
        self.binding = Some((model.model_identity(), options));
        Ok(())
    }

    /// 生产入口独立复核二元性、深度、根路径、资格、配额与建设选择。 / Production entry independently validates binary choices, depths, root paths, eligibility, quotas and construction.
    pub fn extract_solution(&self, model: &MetaModel<f64>) -> Result<NetworkStructureSolution<V>> {
        if self.binding.map(|binding| binding.0) != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("结构模型未注册或身份不匹配 / unregistered structure model or identity mismatch"));
        }
        self.aggregation.extract(model)
    }
}
