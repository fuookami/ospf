//! 韧性领域 context 与事务式管道入口。 / Resilience context and transactional pipeline entry point.

use std::fmt::Debug;
use std::sync::Arc;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::error::{NetworkSchedulingError, Result};
use super::{NetworkResilienceAggregation, NetworkResilienceProblem, NetworkResilienceSolution};

/// 每模型独立的场景扩展。 / Per-model independent scenario extension.
pub trait NetworkResiliencePipeline<V: SolveValue + UnitConversionValue>: Debug + Send + Sync {
    /// 追加场景约束、目标或业务组件。 / Append scenario constraints, objectives or business components.
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkResilienceAggregation<V>) -> Result<()>;
}

/// 同一建设集与多个独立场景流的建模入口。 / Modeling entry point for one construction set and independent scenario flows.
#[derive(Debug, Clone)]
pub struct NetworkResilienceContext<V: SolveValue + UnitConversionValue> {
    name: String,
    aggregation: NetworkResilienceAggregation<V>,
    pipelines: Vec<Arc<dyn NetworkResiliencePipeline<V>>>,
    binding: Option<u64>,
}

impl<V: SolveValue + UnitConversionValue> NetworkResilienceContext<V> {
    /// 在修改模型前准备所有场景。 / Prepare all scenarios before modifying a model.
    pub fn new(problem: NetworkResilienceProblem<V>, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        Ok(Self { aggregation: NetworkResilienceAggregation::new(problem, &name)?, name, pipelines: Vec::new(), binding: None })
    }

    /// 下游只读建模边界 / Read-only downstream modeling boundary.
    pub fn aggregation(&self) -> &NetworkResilienceAggregation<V> { &self.aggregation }

    /// 注册前追加场景管道。 / Append a scenario pipeline before registration.
    pub fn add_pipeline(&mut self, pipeline: Arc<dyn NetworkResiliencePipeline<V>>) -> Result<()> {
        if self.binding.is_some() { return Err(NetworkSchedulingError::contract("注册后不能追加韧性管道 / cannot append a resilience pipeline after registration")); }
        self.pipelines.push(pipeline);
        Ok(())
    }

    /// 原子注册共享建设、全部场景、辅助连通流与最坏费用目标。 / Atomically register shared construction, all scenarios, connectivity witnesses and worst-cost objective.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        if let Some(binding) = self.binding {
            return if binding == model.model_identity() { Ok(()) } else {
                Err(NetworkSchedulingError::contract("韧性 context 不能跨模型复用 / resilience context cannot cross models"))
            };
        }
        let mut next = self.aggregation.clone();
        model.transaction(|model| {
            next.register(model, &self.name)?;
            for pipeline in &self.pipelines { pipeline.register(model, &next)?; }
            model.set_objective_category(ObjectiveCategory::Minimum);
            model.add_linear_objective(&next.cost_expression()?, &format!("{}_cost", self.name));
            Ok(())
        })?;
        self.aggregation = next;
        self.binding = Some(model.model_identity());
        Ok(())
    }

    /// 在生产入口复核所有建设、场景、连通与派生费用。 / Validate all construction, scenarios, connectivity and derived costs in the production entry point.
    pub fn extract_solution(&mut self, model: &MetaModel<f64>) -> Result<NetworkResilienceSolution<V>> {
        if self.binding != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("韧性模型未注册或身份不匹配 / unregistered resilience model or identity mismatch"));
        }
        self.aggregation.extract(model)
    }
}
