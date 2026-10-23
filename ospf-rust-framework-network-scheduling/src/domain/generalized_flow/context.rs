//! 广义流事务式注册入口。 / Transactional generalized-flow registration entry point.

use std::fmt::Debug;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::error::{NetworkSchedulingError, Result};
use super::aggregation::{GeneralizedFlowAggregation, GeneralizedFlowExpressions};
use super::model::{GeneralizedFlowProblem, GeneralizedFlowSolution, GeneralizedFlowSolverTolerance};
use super::pipeline::{register_balances, register_cost};

/// 每 context 的下游管道；不得跨模型共享可变注册状态。 / Per-context downstream pipeline; mutable registration state must not be shared across models.
pub trait GeneralizedFlowPipeline: Debug + Send + Sync {
    /// 用已归一化表达式追加模型组件。 / Append model components using normalized expressions.
    fn register(&self, model: &mut MetaModel<f64>, expressions: &GeneralizedFlowExpressions) -> Result<()>;
}

/// 同模型重复注册必须保持一致的选项。 / Options that must agree across repeated registration in one model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneralizedFlowRegistration {
    /// 是否追加内置费用目标 / Whether to append the built-in cost objective.
    pub minimize_cost: bool,
}

impl Default for GeneralizedFlowRegistration {
    fn default() -> Self { Self { minimize_cost: true } }
}

/// 每模型独立的广义流 context，失败回滚后可重试。 / Per-model generalized-flow context supporting retry after rollback.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowContext<V> {
    name: String,
    aggregation: GeneralizedFlowAggregation<V>,
    pipelines: Vec<Arc<dyn GeneralizedFlowPipeline>>,
    binding: Option<(u64, GeneralizedFlowRegistration)>,
    tolerance: GeneralizedFlowSolverTolerance,
}

impl<V> GeneralizedFlowContext<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建无注册态的独立 context。 / Create an independent unregistered context.
    pub fn new(problem: GeneralizedFlowProblem<V>, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(NetworkSchedulingError::validation("广义流名称不能为空 / generalized-flow name cannot be empty"));
        }
        Ok(Self {
            name, aggregation: GeneralizedFlowAggregation::new(problem)?, pipelines: Vec::new(),
            binding: None, tolerance: Default::default(),
        })
    }

    /// 注册前追加下游管道。 / Append a downstream pipeline before registration.
    pub fn add_pipeline(&mut self, pipeline: Arc<dyn GeneralizedFlowPipeline>) -> Result<()> {
        if self.binding.is_some() {
            return Err(NetworkSchedulingError::contract("注册后不能追加管道 / cannot append a pipeline after registration"));
        }
        self.pipelines.push(pipeline);
        Ok(())
    }

    /// 设置并校验提取容差。 / Set and validate extraction tolerances.
    pub fn set_solver_tolerance(&mut self, tolerance: GeneralizedFlowSolverTolerance) -> Result<()> {
        tolerance.for_values(&[])?;
        self.tolerance = tolerance;
        Ok(())
    }

    /// 访问只读聚合及归一化表达式。 / Access the read-only aggregation and normalized expressions.
    pub fn aggregation(&self) -> &GeneralizedFlowAggregation<V> { &self.aggregation }

    /// 原子注册变量、有限容量、守恒、扩展与可选费用目标。 / Atomically register variables, finite capacities, conservation, extensions and optional cost.
    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        options: GeneralizedFlowRegistration,
    ) -> Result<()> {
        if let Some(binding) = self.binding {
            return if binding == (model.model_identity(), options) { Ok(()) } else {
                Err(NetworkSchedulingError::contract("广义流注册模型或选项不匹配 / generalized-flow registration model or options mismatch"))
            };
        }
        let mut aggregation = self.aggregation.clone();
        model.transaction(|model| {
            aggregation.register_variables(model, &self.name)?;
            register_balances(model, &aggregation, &self.name)?;
            for pipeline in &self.pipelines {
                pipeline.register(model, aggregation.expressions())?;
            }
            if options.minimize_cost { register_cost(model, &aggregation, &self.name); }
            Ok(())
        })?;
        self.aggregation = aggregation;
        self.binding = Some((model.model_identity(), options));
        Ok(())
    }

    /// 独立验证投入、容量、守恒与全部派生值后返回领域解。 / Return a domain solution only after independently validating inputs, capacities, conservation and all derived values.
    pub fn extract_solution(&self, model: &MetaModel<f64>) -> Result<GeneralizedFlowSolution<V>> {
        if self.binding.as_ref().map(|binding| binding.0) != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("广义流模型未注册或身份不匹配 / generalized-flow model is unregistered or has a different identity"));
        }
        self.aggregation.extract(model, self.tolerance)
    }
}
