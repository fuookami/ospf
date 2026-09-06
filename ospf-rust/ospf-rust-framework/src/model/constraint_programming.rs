//! CP framework 上下文与 pipeline 合同 / CP framework context and pipeline contracts.
//!
//! 本模块只负责 CP builder 的 framework 装配；CP AST、snapshot 和求解报告仍由 core 提供。
//! This module assembles CP builders at the framework boundary; the core crate owns the CP AST,
//! snapshots, and unified solve reports.

use std::sync::Arc;

use ospf_rust_core::error::Result;
use ospf_rust_core::model::constraint_programming::ConstraintProgrammingModel;
use ospf_rust_core::model::mechanism::ConstraintGroup;

use super::Pipeline;

/// CP 模型组件 / CP model component.
pub trait ConstraintProgrammingModelComponent: Send + Sync {
    /// 组件名称 / Component name.
    fn name(&self) -> &str;

    /// 可选约束组 / Optional constraint group.
    fn constraint_group(&self) -> Option<ConstraintGroup> {
        None
    }

    /// 将组件注册到可变 CP builder / Register the component into a mutable CP builder.
    fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()>;
}

/// CP 专用 pipeline 合同 / CP-specific pipeline contract.
///
/// 现有 [`Pipeline`] trait 继续服务于通用 framework 代码；CP 注册还需要可失败的可变 builder
/// 步骤，因此本 trait 显式暴露该步骤，不需要将模型向下转型为 `Any`。
/// The existing [`Pipeline`] trait remains available for common framework code. CP registration
/// additionally needs a fallible mutable-builder step, so this trait makes that step explicit
/// without downcasting the model to `Any`.
pub trait ConstraintProgrammingPipeline: Pipeline<ConstraintProgrammingModel> {
    /// 将 CP 状态注册到可变 builder / Register CP state into the mutable builder.
    fn register_cp(&self, model: &mut ConstraintProgrammingModel) -> Result<()>;
}

/// 模型组件到 CP pipeline 的适配器 / Adapter from a model component to a CP pipeline.
pub struct ConstraintProgrammingComponentPipeline {
    component: Arc<dyn ConstraintProgrammingModelComponent>,
    name: String,
    group: Option<ConstraintGroup>,
}

impl ConstraintProgrammingComponentPipeline {
    /// 创建组件 pipeline / Create a component pipeline.
    pub fn new(component: Arc<dyn ConstraintProgrammingModelComponent>) -> Self {
        let name = component.name().to_owned();
        let group = component.constraint_group();
        Self {
            component,
            name,
            group,
        }
    }

    /// 从具体组件创建 pipeline / Create a component pipeline from a concrete component.
    pub fn from_component<C>(component: C) -> Self
    where
        C: ConstraintProgrammingModelComponent + 'static,
    {
        Self::new(Arc::new(component))
    }
}

impl Pipeline<ConstraintProgrammingModel> for ConstraintProgrammingComponentPipeline {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn invoke(&self, _model: &ConstraintProgrammingModel) -> Result<()> {
        Ok(())
    }
}

impl ConstraintProgrammingPipeline for ConstraintProgrammingComponentPipeline {
    fn register_cp(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
        self.component.register(model)
    }
}

/// CP 聚合 / CP aggregation.
#[derive(Default)]
pub struct ConstraintProgrammingAggregation {
    pipelines: Vec<Arc<dyn ConstraintProgrammingPipeline>>,
}

impl ConstraintProgrammingAggregation {
    /// 创建空聚合 / Create an empty aggregation.
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加 pipeline / Add a pipeline.
    pub fn add_pipeline<P>(&mut self, pipeline: P)
    where
        P: ConstraintProgrammingPipeline + 'static,
    {
        self.pipelines.push(Arc::new(pipeline));
    }

    /// 将模型组件作为 pipeline 添加 / Add a model component as a pipeline.
    pub fn add_component<C>(&mut self, component: C)
    where
        C: ConstraintProgrammingModelComponent + 'static,
    {
        self.add_pipeline(ConstraintProgrammingComponentPipeline::from_component(
            component,
        ));
    }

    /// 注册全部 pipeline 及其约束组 / Register every pipeline and its group.
    pub fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
        let mut staged = model.clone();
        self.register_into(&mut staged)?;
        *model = staged;
        Ok(())
    }

    fn register_into(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
        for pipeline in &self.pipelines {
            if let Some(group) = pipeline.constraint_group() {
                super::ConstraintGroupRegistrar::ensure_constraint_group(model, group)?;
            }
            pipeline.register_cp(model)?;
        }
        Ok(())
    }

    /// 返回已注册 pipeline 数量 / Return the number of registered pipelines.
    pub fn len(&self) -> usize {
        self.pipelines.len()
    }

    /// 检查聚合是否为空 / Check whether the aggregation is empty.
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

/// CP 上下文 / CP context.
pub struct ConstraintProgrammingContext {
    aggregation: ConstraintProgrammingAggregation,
}

impl ConstraintProgrammingContext {
    /// 创建空 CP 上下文 / Create an empty CP context.
    pub fn new() -> Self {
        Self {
            aggregation: ConstraintProgrammingAggregation::new(),
        }
    }

    /// 从聚合创建上下文 / Create a context from an aggregation.
    pub fn from_aggregation(aggregation: ConstraintProgrammingAggregation) -> Self {
        Self { aggregation }
    }

    /// 添加 pipeline 扩展 / Add a pipeline extension.
    pub fn add_pipeline<P>(&mut self, pipeline: P)
    where
        P: ConstraintProgrammingPipeline + 'static,
    {
        self.aggregation.add_pipeline(pipeline);
    }

    /// 添加额外组件扩展 / Add an extra component extension.
    pub fn add_extra_component<C>(&mut self, component: C)
    where
        C: ConstraintProgrammingModelComponent + 'static,
    {
        self.aggregation.add_component(component);
    }

    /// 注册完整上下文 / Register the complete context.
    pub fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
        self.aggregation.register(model)
    }

    /// 将已注册上下文冻结为不可变快照 / Freeze the registered context to an immutable snapshot.
    pub fn freeze(
        &self,
        model: &mut ConstraintProgrammingModel,
    ) -> Result<ospf_rust_core::model::constraint_programming::ConstraintProgrammingSnapshot> {
        let mut staged = model.clone();
        self.aggregation.register_into(&mut staged)?;
        let snapshot = staged.freeze()?;
        *model = staged;
        Ok(snapshot)
    }

    /// 返回上下文 pipeline 数量 / Return the number of context pipelines.
    pub fn pipeline_count(&self) -> usize {
        self.aggregation.len()
    }
}

impl Default for ConstraintProgrammingContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, IntegerDomain, IntegerExpression,
        IntegerRelation, IntegerVariable,
    };
    use ospf_rust_core::solver::StableVariableId;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    struct VariableComponent {
        variable: IntegerVariable,
        group: ConstraintGroup,
    }

    impl ConstraintProgrammingModelComponent for VariableComponent {
        fn name(&self) -> &str {
            "variable-component"
        }

        fn constraint_group(&self) -> Option<ConstraintGroup> {
            Some(self.group.clone())
        }

        fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
            model.register_variable(self.variable.clone(), IntegerDomain::boolean())
        }
    }

    struct ExtraConstraintComponent {
        variable: IntegerVariable,
    }

    impl ConstraintProgrammingModelComponent for ExtraConstraintComponent {
        fn name(&self) -> &str {
            "extra-constraint"
        }

        fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
            model.add_constraint(ConstraintDefinition::new(
                "extra/constraint",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(self.variable.clone()),
                    IntegerRelation::GreaterOrEqual,
                    0,
                ),
            ))
        }
    }

    struct FailOnceComponent {
        variable: IntegerVariable,
        failed: Arc<AtomicBool>,
    }

    impl ConstraintProgrammingModelComponent for FailOnceComponent {
        fn name(&self) -> &str {
            "fail-once"
        }

        fn register(&self, model: &mut ConstraintProgrammingModel) -> Result<()> {
            model.register_variable(self.variable.clone(), IntegerDomain::boolean())?;
            if !self.failed.swap(true, Ordering::SeqCst) {
                return Err(ospf_rust_core::error::CoreError::contract_error(
                    "test component failed after mutating the staged model",
                ));
            }
            Ok(())
        }
    }

    #[test]
    fn context_registers_components_groups_and_extra_extensions() {
        let variable = IntegerVariable::new("x");
        let mut context = ConstraintProgrammingContext::new();
        context.add_extra_component(VariableComponent {
            variable: variable.clone(),
            group: ConstraintGroup::new(7, "variables"),
        });
        context.add_extra_component(ExtraConstraintComponent { variable });

        let mut model = ConstraintProgrammingModel::new("context-test");
        let snapshot = context.freeze(&mut model).expect("snapshot");
        assert_eq!(context.pipeline_count(), 2);
        assert_eq!(snapshot.constraints.len(), 1);
        assert_eq!(
            snapshot.constraint_groups.get(&7).map(String::as_str),
            Some("variables")
        );
    }

    #[test]
    fn conflicting_group_names_are_rejected() {
        let group = ConstraintGroup::new(7, "first");
        let mut model = ConstraintProgrammingModel::new("group-test");
        model
            .ensure_constraint_group(group.id, group.name.clone())
            .expect("first group");
        assert!(model.ensure_constraint_group(group.id, "second").is_err());
    }

    #[test]
    fn failed_registration_restores_the_original_model_and_allows_a_new_model_retry() {
        let failed = Arc::new(AtomicBool::new(false));
        let variable = IntegerVariable::new("retry");
        let mut context = ConstraintProgrammingContext::new();
        context.add_extra_component(FailOnceComponent {
            variable: variable.clone(),
            failed: Arc::clone(&failed),
        });

        let mut first_model = ConstraintProgrammingModel::new("first-model");
        let before = first_model.freeze().expect("baseline snapshot");
        assert!(context.register(&mut first_model).is_err());
        assert_eq!(first_model.variable_count(), 0);
        assert_eq!(first_model.constraint_count(), 0);
        assert_eq!(
            first_model.freeze().expect("restored snapshot").fingerprint,
            before.fingerprint
        );

        let mut retry_model = ConstraintProgrammingModel::new("retry-model");
        context
            .register(&mut retry_model)
            .expect("retry should use the next staged attempt");
        assert_eq!(retry_model.variable_count(), 1);
        let stable_id = StableVariableId::from("retry");
        assert_eq!(
            retry_model
                .freeze()
                .expect("retry snapshot")
                .variable(&stable_id)
                .map(|entry| entry.variable.stable_id.0.as_str()),
            Some("retry")
        );
    }
}
