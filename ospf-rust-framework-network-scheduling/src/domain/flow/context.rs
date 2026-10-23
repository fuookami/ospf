//! flow context / Flow context.

use std::fmt::Debug;
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::error::Result;

use super::aggregation::FlowAggregation;
use super::model::{FlowProblem, FlowSolution};
use super::pipeline::{
    CapacityBoundsPipeline, FlowConservationPipeline, MinCostFlowObjectivePipeline,
};

/// flow context 扩展点 / Flow-context extension point.
pub trait FlowContextExtension<V>: Debug + Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 注册额外变量、约束或目标 / Register additional variables, constraints, or objectives.
    fn register(&self, model: &mut MetaModel<f64>, aggregation: &FlowAggregation<V>) -> Result<()>;

    /// 刷新扩展对偶或派生状态 / Refresh extension duals or derived state.
    fn refresh_shadow_price(
        &self,
        _model: &MetaModel<f64>,
        _dual_solution: &[f64],
        _aggregation: &FlowAggregation<V>,
    ) -> Result<()> {
        Ok(())
    }

    /// 提取扩展结果 / Extract extension results.
    fn extract_solution(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// flow 建模入口 / Flow modeling entry point.
#[derive(Debug, Clone)]
pub struct FlowContext<V: SolveValue + UnitConversionValue> {
    /// flow 聚合 / Flow aggregation.
    pub aggregation: FlowAggregation<V>,
    /// 注入的扩展 / Injected extensions.
    pub extensions: Vec<Arc<dyn FlowContextExtension<V>>>,
    /// 注册阶段 / Registration stage.
    registration: RegistrationStage,
    /// 已绑定模型的身份 / Identity of the model this context is bound to.
    registered_model: Option<u64>,
    name: String,
    options: Option<FlowRegistration>,
}

/// 独立或嵌入式流注册选项。 / Standalone or embedded flow-registration options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowRegistration {
    /// 注册总运行费用目标；场景嵌入时可关闭。 / Register total operating cost, disabled for embedded scenarios.
    pub minimize_cost: bool,
}

impl Default for FlowRegistration {
    fn default() -> Self { Self { minimize_cost: true } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegistrationStage {
    Empty,
    Variables,
    Complete,
}

impl<V> FlowContext<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建 flow context / Create a flow context.
    pub fn new(graph: FlowProblem<V>) -> Self {
        Self {
            aggregation: FlowAggregation::new(graph),
            extensions: Vec::new(),
            registration: RegistrationStage::Empty,
            registered_model: None,
            name: "flow".into(), options: None,
        }
    }

    /// 创建带扩展的 flow context / Create a flow context with extensions.
    pub fn with_extensions(
        graph: FlowProblem<V>,
        extensions: Vec<Arc<dyn FlowContextExtension<V>>>,
    ) -> Self {
        Self {
            aggregation: FlowAggregation::new(graph),
            extensions,
            registration: RegistrationStage::Empty,
            registered_model: None,
            name: "flow".into(), options: None,
        }
    }

    /// 使用额外 flow pipelines 创建 context / Create a context with extra flow pipelines.
    ///
    /// `FlowContextExtension` 是带聚合上下文的 pipeline facade，兼容旧的 extension 命名。
    /// `FlowContextExtension` is the pipeline facade with aggregation access and keeps the
    /// legacy extension naming for compatibility.
    pub fn with_extra_pipelines(
        graph: FlowProblem<V>,
        pipelines: Vec<Arc<dyn FlowContextExtension<V>>>,
    ) -> Self {
        Self::with_extensions(graph, pipelines)
    }

    /// 在模型注册前添加额外 flow pipeline / Add an extra flow pipeline before model registration.
    pub fn add_extra_pipeline(&mut self, pipeline: Arc<dyn FlowContextExtension<V>>) -> Result<()> {
        if self.registration == RegistrationStage::Complete {
            return Err(crate::error::NetworkSchedulingError::contract(
                "完整注册后不能添加 FlowContext pipeline / cannot add a FlowContext pipeline after registration is complete",
            ));
        }
        self.extensions.push(pipeline);
        Ok(())
    }

    /// 注册变量、守恒、容量和目标 / Register variables, conservation, capacities, and objective.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.register_with_options(model, Default::default())
    }

    /// 指定每模型命名空间；注册后不能修改。 / Set a per-model namespace; immutable after registration.
    pub fn with_name(mut self, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if self.registered_model.is_some() || name.is_empty() {
            return Err(crate::error::NetworkSchedulingError::contract("流名称非法或已注册 / invalid flow name or already registered"));
        }
        self.name = name;
        Ok(self)
    }

    /// 原子注册同一领域组件，可将运行费用交给外层聚合。 / Atomically register the same components, optionally delegating operating cost to an outer aggregation.
    pub fn register_with_options(
        &mut self,
        model: &mut MetaModel<f64>,
        options: FlowRegistration,
    ) -> Result<()> {
        let aggregation_before = self.aggregation.clone();
        let registration_before = self.registration;
        let registered_model_before = self.registered_model;
        let options_before = self.options;
        let result = model.transaction(|model| {
            self.bind_model(model)?;
            if self.registration == RegistrationStage::Complete {
                return if self.options == Some(options) { Ok(()) } else {
                    Err(crate::error::NetworkSchedulingError::contract("流重复注册选项不一致 / inconsistent repeated flow-registration options"))
                };
            }
            if self.registration == RegistrationStage::Empty {
                self.aggregation.register_named(model, &self.name)?;
                self.registration = RegistrationStage::Variables;
            }
            let mut conservation = FlowConservationPipeline::new(self.aggregation.graph.clone());
            conservation.name = format!("{}_conservation", self.name);
            conservation.apply(model, &self.aggregation)?;
            let mut capacity = CapacityBoundsPipeline::new(self.aggregation.graph.clone());
            capacity.name = format!("{}_capacity", self.name);
            capacity.apply(model, &self.aggregation)?;
            if options.minimize_cost {
                let mut objective = MinCostFlowObjectivePipeline::new(self.aggregation.graph.clone());
                objective.name = format!("{}_cost", self.name);
                objective.apply(model, &self.aggregation)?;
            }
            for extension in &self.extensions {
                extension.register(model, &self.aggregation)?;
            }
            self.registration = RegistrationStage::Complete;
            self.options = Some(options);
            Ok(())
        });
        if result.is_err() {
            self.aggregation = aggregation_before;
            self.registration = registration_before;
            self.registered_model = registered_model_before;
            self.options = options_before;
        }
        result
    }

    /// 仅注册变量和基础聚合 / Register only variables and the base aggregation.
    pub fn register_variables(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        let aggregation_before = self.aggregation.clone();
        let registration_before = self.registration;
        let registered_model_before = self.registered_model;
        let result = model.transaction(|model| {
            self.bind_model(model)?;
            if self.registration == RegistrationStage::Empty {
                self.aggregation.register_named(model, &self.name)?;
                self.registration = RegistrationStage::Variables;
            }
            Ok(())
        });
        if result.is_err() {
            self.aggregation = aggregation_before;
            self.registration = registration_before;
            self.registered_model = registered_model_before;
        }
        result
    }

    /// 刷新扩展对偶生命周期 / Refresh the extension dual lifecycle.
    pub fn refresh_shadow_price(
        &mut self,
        model: &MetaModel<f64>,
        dual_solution: &[f64],
    ) -> Result<()> {
        self.bind_model(model)?;
        for extension in &self.extensions {
            extension.refresh_shadow_price(model, dual_solution, &self.aggregation)?;
        }
        Ok(())
    }

    /// 提取扩展结果生命周期 / Extract the extension result lifecycle.
    pub fn extract_solution(&mut self, model: &MetaModel<f64>) -> Result<FlowSolution<V>> {
        if self.registration != RegistrationStage::Complete || self.registered_model != Some(model.model_identity()) {
            return Err(crate::error::NetworkSchedulingError::contract(
                "流模型未完整注册或模型身份不匹配 / flow model is not fully registered or has a different identity",
            ));
        }
        let solution = super::extraction::extract(&self.aggregation, model)?;
        for extension in &self.extensions {
            extension.extract_solution(model)?;
        }
        Ok(solution)
    }

    /// 获取图 / Get the immutable graph.
    pub fn problem(&self) -> &FlowProblem<V> {
        &self.aggregation.graph
    }

    fn bind_model(&mut self, model: &MetaModel<f64>) -> Result<()> {
        let model_identity = model.model_identity();
        if let Some(registered_model) = self.registered_model
            && registered_model != model_identity
        {
            return Err(crate::error::NetworkSchedulingError::contract(
                "FlowContext 不能跨 MetaModel 复用 / FlowContext cannot be reused across MetaModels",
            ));
        }
        self.registered_model = Some(model_identity);
        Ok(())
    }
}
