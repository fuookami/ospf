//! CSP1D 产出上下文 builder / CSP1D produce context builder

use std::sync::Arc;

use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::length_assignment::{LengthAssignmentModelingConfig, LengthSlackAggregation};
use crate::domain::wasting_minimization::{WasteAggregation, WasteMinimizationConfig};
use crate::domain::r#yield::{YieldModelingConfig, YieldSlackAggregation};
use ospf_rust_core::model::MetaModel;

use super::aggregation::ProduceAggregation;
use super::context::Csp1dProduceContext;
use super::pipeline::Csp1dIncrementalPipeline;
use super::{Csp1dModelingExtension, Csp1dModelingMode, Csp1dObjectivePolicy, ProduceInput};

/// CSP1D 产出上下文 builder / CSP1D produce context builder
#[derive(Clone)]
pub struct Csp1dProduceContextBuilder<V: SolveValue> {
    /// 产出输入 / Produce input
    input: ProduceInput<V>,
    /// Yield 配置 / Yield configuration
    yield_config: Option<YieldModelingConfig<V>>,
    /// Waste 配置 / Waste minimization configuration
    waste_config: Option<WasteMinimizationConfig<V>>,
    /// Length 配置 / Length assignment configuration
    length_config: Option<LengthAssignmentModelingConfig<V>>,
    /// 建模模式 / Modeling mode
    mode: Csp1dModelingMode,
    /// 是否最终 MILP / Whether final MILP
    is_final_milp: bool,
    /// 额外管线 / Extra pipelines
    extra_pipelines: Vec<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>>,
    /// 增量扩展管线 / Incremental extension pipelines
    incremental_pipelines: Vec<Arc<dyn Csp1dIncrementalPipeline<V>>>,
    /// 目标策略 / Objective policies
    objective_policies: Vec<Arc<dyn Csp1dObjectivePolicy<V>>>,
    /// 建模扩展 / Modeling extensions
    extensions: Vec<Csp1dModelingExtension<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dProduceContextBuilder<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dProduceContextBuilder")
            .field("input", &self.input)
            .field("has_yield_config", &self.yield_config.is_some())
            .field("has_waste_config", &self.waste_config.is_some())
            .field("has_length_config", &self.length_config.is_some())
            .field("mode", &self.mode)
            .field("is_final_milp", &self.is_final_milp)
            .field("extra_pipelines", &self.extra_pipelines.len())
            .field("incremental_pipelines", &self.incremental_pipelines.len())
            .field("objective_policies", &self.objective_policies.len())
            .field("extensions", &self.extensions.len())
            .finish()
    }
}

impl<V: SolveValue> Csp1dProduceContextBuilder<V> {
    /// 创建 builder / Create builder
    pub fn new(input: ProduceInput<V>) -> Self {
        Self {
            input,
            yield_config: None,
            waste_config: None,
            length_config: None,
            mode: Csp1dModelingMode::MILP,
            is_final_milp: false,
            extra_pipelines: Vec::new(),
            incremental_pipelines: Vec::new(),
            objective_policies: Vec::new(),
            extensions: Vec::new(),
        }
    }

    /// 设置 yield 配置 / Set yield config
    pub fn yield_config(&mut self, config: YieldModelingConfig<V>) -> &mut Self {
        self.yield_config = Some(config);
        self
    }

    /// 设置 waste 配置 / Set waste config
    pub fn waste_config(&mut self, config: WasteMinimizationConfig<V>) -> &mut Self {
        self.waste_config = Some(config);
        self
    }

    /// 设置 length 配置 / Set length config
    pub fn length_config(&mut self, config: LengthAssignmentModelingConfig<V>) -> &mut Self {
        self.length_config = Some(config);
        self
    }

    /// 设置建模模式 / Set modeling mode
    pub fn mode(&mut self, mode: Csp1dModelingMode) -> &mut Self {
        self.mode = mode;
        self
    }

    /// 设置最终 MILP 标记 / Set final MILP flag
    pub fn is_final_milp(&mut self, is_final_milp: bool) -> &mut Self {
        self.is_final_milp = is_final_milp;
        self
    }

    /// 追加额外管线 / Add extra pipeline
    pub fn extra_pipeline(
        &mut self,
        pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>,
    ) -> &mut Self {
        self.extra_pipelines.push(pipeline);
        self
    }

    /// 追加增量管线 / Add incremental pipeline
    pub fn incremental_pipeline(
        &mut self,
        pipeline: Arc<dyn Csp1dIncrementalPipeline<V>>,
    ) -> &mut Self {
        self.incremental_pipelines.push(pipeline);
        self
    }

    /// 追加目标策略 / Add objective policy
    pub fn objective_policy(&mut self, policy: Arc<dyn Csp1dObjectivePolicy<V>>) -> &mut Self {
        self.objective_policies.push(policy);
        self
    }

    /// 追加建模扩展 / Add modeling extension
    pub fn extension(&mut self, extension: Csp1dModelingExtension<V>) -> &mut Self {
        self.extensions.push(extension);
        self
    }

    /// 构建上下文 / Build context
    pub fn build(&self) -> crate::Csp1dResult<Csp1dProduceContext<V>> {
        let domain_value_sample =
            resolve_domain_value_sample(&self.input, self.yield_config.as_ref()).ok_or_else(
                || crate::Csp1dError::InvalidInput {
                    message: "Cannot derive domain value sample from ProduceInput".into(),
                },
            )?;
        let produce = ProduceAggregation::new(
            self.input.cutting_plans.clone(),
            self.input.demands.clone(),
            self.input.materials.clone(),
            self.input.machines.clone(),
            self.input.warm_start_plan_usages.clone(),
        );
        let needs_over_slack_for_over_area = self
            .waste_config
            .as_ref()
            .and_then(|config| config.over_production_area_penalty.as_ref())
            .is_some();
        let mut context = Csp1dProduceContext {
            produce,
            r#yield: self
                .yield_config
                .as_ref()
                .filter(|_| self.mode == Csp1dModelingMode::MILP)
                .map(|config| {
                    YieldSlackAggregation::new(
                        config.clone(),
                        self.input.demands.clone(),
                        needs_over_slack_for_over_area,
                    )
                }),
            waste: if self.mode == Csp1dModelingMode::MILP && self.waste_config.is_some() {
                Some(WasteAggregation { analysis: None })
            } else {
                None
            },
            length: self
                .length_config
                .as_ref()
                .filter(|config| self.mode == Csp1dModelingMode::MILP && config.enabled)
                .map(|config| {
                    LengthSlackAggregation::new(config.clone(), self.input.demands.clone())
                }),
            constraint_pipelines: Vec::new(),
            cg_pipelines: Vec::new(),
            extra_pipelines: self.extra_pipelines.clone(),
            incremental_pipelines: self.incremental_pipelines.clone(),
            mode: self.mode,
            is_final_milp: self.is_final_milp,
            warm_start_plan_usages: self.input.warm_start_plan_usages.clone(),
            objective_policies: self.objective_policies.clone(),
            domain_value_sample,
            yield_config: self.yield_config.clone(),
            waste_config: self.waste_config.clone(),
            length_config: self.length_config.clone(),
        };
        for extension in &self.extensions {
            if extension.mode.matches(self.mode, self.is_final_milp) {
                if let Some(pipeline) = extension.resolve_pipeline(Some(&context)) {
                    context.extra_pipelines.push(pipeline);
                }
            }
        }
        Ok(context)
    }
}

fn resolve_domain_value_sample<V: SolveValue>(
    input: &ProduceInput<V>,
    yield_config: Option<&YieldModelingConfig<V>>,
) -> Option<V> {
    input
        .demands
        .first()
        .map(|demand| demand.quantity.value.clone())
        .or_else(|| {
            input
                .materials
                .first()
                .map(|material| material.width_range.lower_bound.value.clone())
        })
        .or_else(|| {
            input
                .cutting_plans
                .first()
                .and_then(|plan| plan.rest_width())
                .map(|width| width.value)
        })
        .or_else(|| {
            yield_config.and_then(|config| config.under_production_penalty.values().next().cloned())
        })
}
