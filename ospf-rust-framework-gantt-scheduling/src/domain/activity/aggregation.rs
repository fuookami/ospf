//! 受管二元变量、完整费用和已准备约束的每模型聚合。 / Per-model aggregation of managed binaries, complete costs and prepared constraints.

use std::collections::BTreeMap;
use std::sync::Arc;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{Linear, LinearMonomial, MetaModel, ObjectiveCategory, SubObjective};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::domain::common::TaskId;
use crate::{GanttError, GanttResult};
use super::compilation::ActivityPreparation;
use super::model::ActivityChoiceKey;
use super::pipeline::ActivityDecision;

/// 活动变量组件，按稳定决策身份访问源 token 索引。 / Activity variable component accessing source-token indices by stable decision identities.
#[derive(Debug, Clone, Default)]
pub struct ActivityCompilation {
    variables: BTreeMap<ActivityDecision, usize>,
}

impl ActivityCompilation {
    /// 完整候选变量，未知或尚未注册时为 None。 / Complete-choice variable, None when unknown or unregistered.
    pub fn variable(&self, key: &ActivityChoiceKey) -> Option<usize> { self.variables.get(&ActivityDecision::Execute(key.clone())).copied() }

    /// 显式取消变量，未允许取消时为 None。 / Explicit cancellation variable, None when cancellation is disallowed.
    pub fn cancellation_variable(&self, task: &TaskId) -> Option<usize> { self.variables.get(&ActivityDecision::Cancel(task.clone())).copied() }

    /// 稳定决策到源 token 索引；不是 solver 列映射。 / Stable decisions to source-token indices, not solver-column mappings.
    pub fn variables(&self) -> &BTreeMap<ActivityDecision, usize> { &self.variables }
}

/// 每模型独立的活动域聚合。 / Per-model independent activity-domain aggregation.
#[derive(Debug, Clone)]
pub struct ActivitySchedulingAggregation<V> {
    prepared: Arc<ActivityPreparation<V>>,
    compilation: ActivityCompilation,
    binding: Option<u64>,
}

impl<V: SolveValue + UnitConversionValue> ActivitySchedulingAggregation<V> {
    pub(super) fn new(prepared: ActivityPreparation<V>) -> Self {
        Self { prepared: Arc::new(prepared), compilation: ActivityCompilation::default(), binding: None }
    }

    /// 不可变完整候选与双数值行。 / Immutable complete choices and dual-numeric rows.
    pub fn prepared(&self) -> &ActivityPreparation<V> { &self.prepared }

    /// 受管变量组件。 / Managed variable component.
    pub fn compilation(&self) -> &ActivityCompilation { &self.compilation }

    /// 是否完成成功注册。 / Whether registration has successfully completed.
    pub fn is_registered(&self) -> bool { self.binding.is_some() }

    /// 验证本 context 的模型归属，不禁止合法下游扩展。 / Validate context ownership without forbidding legitimate downstream extensions.
    pub fn ensure_model(&self, model: &MetaModel<f64>) -> GanttResult<()> {
        if self.binding != Some(model.model_identity()) { return Err(lifecycle("unregistered activity context or wrong model")); }
        Ok(())
    }

    /// 待排活动的完整领域费用表达式，不包含下游费用或执行沉没费用。
    /// Complete pending-domain cost expression, excluding downstream and sunk execution costs.
    pub fn cost_expression(&self) -> GanttResult<Vec<(usize, f64)>> {
        if !self.is_registered() { return Err(lifecycle("activity context is not registered")); }
        Ok(self.compilation.variables.iter().map(|(decision, index)| {
            let cost = match decision {
                ActivityDecision::Execute(key) => self.prepared.choices()[key].cost.solver,
                ActivityDecision::Cancel(task) => self.prepared.cancellation_costs()[task].solver,
            };
            (*index, cost)
        }).collect())
    }

    pub(super) fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        namespace: &str,
        minimize_cost: bool,
    ) -> GanttResult<()> {
        if self.is_registered() { return Err(lifecycle("activity context cannot be registered twice")); }
        if minimize_cost && model.objective().category != ObjectiveCategory::Minimum {
            return Err(lifecycle("activity cost minimization is incompatible with the model objective direction"));
        }
        let decisions = self.prepared.choices().keys().cloned().map(ActivityDecision::Execute)
            .chain(self.prepared.cancellation_costs().keys().cloned().map(ActivityDecision::Cancel));
        for (ordinal, decision) in decisions.enumerate() {
            let index = model.register_auto_variable_with_range::<Binary>(
                &format!("{namespace}_decision_{ordinal}"), VariableRange::bounded(0.0, 1.0),
            ).map_err(model_error)?;
            self.compilation.variables.insert(decision, index);
        }
        for (ordinal, row) in self.prepared.rows().iter().enumerate() {
            let terms = row.terms.iter().map(|(decision, coefficient)| (self.compilation.variables[decision], coefficient.solver)).collect::<Vec<_>>();
            if let Some(lower) = &row.lower {
                model.add_ge_constraint(&terms, lower.solver, &format!("{namespace}_row_{ordinal}_lower")).map_err(model_error)?;
            }
            if let Some(upper) = &row.upper {
                model.add_le_constraint(&terms, upper.solver, &format!("{namespace}_row_{ordinal}_upper")).map_err(model_error)?;
            }
        }
        self.binding = Some(model.model_identity());
        if minimize_cost {
            let cost = Linear::new(self.cost_expression()?.into_iter().map(|(index, cost)| LinearMonomial::new(cost, index)).collect(), 0.0);
            model.add_sub_objective(SubObjective::minimize(cost, &format!("{namespace}_cost")));
        }
        Ok(())
    }
}

pub(super) fn lifecycle(message: impl Into<String>) -> GanttError { GanttError::Lifecycle { message: message.into() } }

pub(super) fn model_error(error: impl std::fmt::Display) -> GanttError { GanttError::Calculation { message: error.to_string() } }
