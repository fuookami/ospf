//! 建设领域的事务式注册和下游扩展。 / Transactional registration and downstream extensions for network design.

use std::fmt::Debug;
use std::sync::Arc;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::error::{NetworkSchedulingError, Result};
use super::{NetworkDesignAggregation, NetworkDesignProblem, NetworkDesignSolution};

/// 每 context 独立的下游扩展。 / Per-context downstream extension.
pub trait NetworkDesignPipeline<V: SolveValue + UnitConversionValue>: Debug + Send + Sync {
    /// 用公开建设、流和费用表达式追加规则。 / Append rules using public construction, flow and cost expressions.
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &NetworkDesignAggregation<V>) -> Result<()>;
}

/// 同模型重复注册时必须一致。 / Must agree across repeated registrations in one model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkDesignRegistration {
    /// 是否最小化建设加名义运行费用 / Minimize construction plus nominal operating cost.
    pub minimize_cost: bool,
}

impl Default for NetworkDesignRegistration {
    fn default() -> Self { Self { minimize_cost: true } }
}

/// 建设领域入口；不持有全局求解状态。 / Design domain entry point without global solve state.
#[derive(Debug, Clone)]
pub struct NetworkDesignContext<V: SolveValue + UnitConversionValue> {
    name: String,
    aggregation: NetworkDesignAggregation<V>,
    pipelines: Vec<Arc<dyn NetworkDesignPipeline<V>>>,
    binding: Option<(u64, NetworkDesignRegistration)>,
}

impl<V: SolveValue + UnitConversionValue> NetworkDesignContext<V> {
    /// 从不可变问题创建每模型独立的 context。 / Create a per-model context from an immutable problem.
    pub fn new(problem: NetworkDesignProblem<V>, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        Ok(Self { aggregation: NetworkDesignAggregation::new(problem, &name)?, name, pipelines: Vec::new(), binding: None })
    }

    /// 只读聚合及下游表达式 / Read-only aggregation and downstream expressions.
    pub fn aggregation(&self) -> &NetworkDesignAggregation<V> { &self.aggregation }

    /// 注册前追加扩展。 / Append an extension before registration.
    pub fn add_pipeline(&mut self, pipeline: Arc<dyn NetworkDesignPipeline<V>>) -> Result<()> {
        if self.binding.is_some() { return Err(NetworkSchedulingError::contract("注册后不能追加建设管道 / cannot append a design pipeline after registration")); }
        self.pipelines.push(pipeline);
        Ok(())
    }

    /// 失败时同时回滚模型和内部注册状态。 / Roll back both model and internal registration state on failure.
    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        options: NetworkDesignRegistration,
    ) -> Result<()> {
        if let Some(binding) = self.binding {
            return if binding == (model.model_identity(), options) { Ok(()) } else {
                Err(NetworkSchedulingError::contract("建设模型或注册选项不匹配 / design model or registration options mismatch"))
            };
        }
        let mut next = self.aggregation.clone();
        model.transaction(|model| {
            next.register(model, &self.name)?;
            for pipeline in &self.pipelines { pipeline.register(model, &next)?; }
            if options.minimize_cost {
                model.set_objective_category(ObjectiveCategory::Minimum);
                model.add_linear_objective(&next.cost_expression()?, &format!("{}_cost", self.name));
            }
            Ok(())
        })?;
        self.aggregation = next;
        self.binding = Some((model.model_identity(), options));
        Ok(())
    }

    /// 独立复核建设、容量联动与流守恒后提取。 / Extract after independently validating construction, capacity linking and flow conservation.
    pub fn extract_solution(&mut self, model: &MetaModel<f64>) -> Result<NetworkDesignSolution<V>> {
        if self.binding.map(|item| item.0) != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("建设模型未注册或身份不匹配 / unregistered design model or identity mismatch"));
        }
        self.aggregation.extract(model)
    }
}
