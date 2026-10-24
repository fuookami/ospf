//! 受管任务束主问题组件，增删列共用同一注册路径。 / Managed bunch master component sharing registration across column additions and removals.

use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint::{ConstraintRelation, LinearInequality};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use crate::domain::common::ExecutorIdTrait;
use crate::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::{GanttError, GanttResult};
use super::iterative::IterativeBunchCompilation;

/// 每个执行器可选择的任务束数量。 / Number of bunches selectable per executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BunchExecutorSelection {
    /// 至多一束，允许不用执行器。 / At most one bunch, allowing an unused executor.
    AtMostOne,
    /// 恰好一束或启用的空闲变量。 / Exactly one bunch or an enabled leisure variable.
    ExactlyOne,
}

/// 标准主问题配置；不启用时保留原有下游自定义建模入口。 / Standard master configuration; omitting it preserves downstream custom modeling.
#[derive(Debug, Clone)]
pub struct BunchMasterConfiguration {
    /// 每任务取消费用；None 表示必须覆盖的硬任务。 / Per-task cancellation cost; None denotes mandatory coverage.
    pub cancellation_costs: Vec<Option<f64>>,
    /// 执行器选择约束。 / Executor selection constraint.
    pub executor_selection: BunchExecutorSelection,
}

#[derive(Debug, Clone)]
pub(super) struct BunchMaster {
    configuration: BunchMasterConfiguration,
    group: Option<Arc<ConstraintGroup>>,
    objective_name: String,
}

impl BunchMaster {
    pub(super) fn new(configuration: BunchMasterConfiguration, tasks: usize) -> GanttResult<Self> {
        if configuration.cancellation_costs.len() != tasks
            || configuration.cancellation_costs.iter().flatten().any(|cost| !cost.is_finite())
        {
            return Err(GanttError::Calculation { message: "任务取消费用数量或数值无效 / invalid cancellation cost count or value".into() });
        }
        Ok(Self { configuration, group: None, objective_name: String::new() })
    }

    pub(super) fn refresh<I>(
        &mut self,
        compilation: &IterativeBunchCompilation<I>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()>
    where
        I: ExecutorIdTrait,
    {
        let result = (|| {
            if self.group.is_none() {
                let id = next_gantt_symbol_id();
                self.group = Some(model.create_constraint_group(id, "bunch_master")?);
                self.objective_name = format!("bunch_master_cost_{}", id);
                for (task, cost) in self.configuration.cancellation_costs.iter().enumerate() {
                    if cost.is_none() {
                        model.fix_variable_by_index(compilation.base.y_indices[task], 0.0)?;
                    }
                }
            }
            let group = self.group.clone().ok_or_else(|| ospf_rust_core::error::ModelError::ConstraintConflict("missing bunch master group".into()))?;
            model.remove_constraints_by_group_id(group.id);
            for (task, terms) in compilation.task_compilation_terms.iter().enumerate() {
                model.add_inequality_with_metadata(
                    LinearInequality::equal(polynomial(terms), 1.0),
                    &format!("task_compilation_{}", task),
                    Some(group.clone()),
                    false,
                    0,
                    None,
                )?;
            }
            for (executor, terms) in compilation.base.executor_ids.iter().zip(&compilation.executor_compilation_terms) {
                let relation = match self.configuration.executor_selection {
                    BunchExecutorSelection::AtMostOne => ConstraintRelation::LessEqual,
                    BunchExecutorSelection::ExactlyOne => ConstraintRelation::Equal,
                };
                model.add_inequality_with_metadata(
                    LinearInequality::new(polynomial(terms), relation, 1.0),
                    &format!("executor_compilation_{}", executor),
                    Some(group.clone()),
                    false,
                    0,
                    None,
                )?;
            }
            let mut terms = compilation.cost_terms.clone();
            terms.extend(self.configuration.cancellation_costs.iter().enumerate().filter_map(|(task, cost)| {
                cost.map(|cost| (compilation.base.y_indices[task], cost))
            }));
            model.replace_sub_objective(&self.objective_name, SubObjective::minimize(polynomial(&terms), &self.objective_name));
            ospf_rust_core::error::Result::Ok(())
        })();
        result.map_err(|error| GanttError::Calculation { message: format!("任务束主问题注册失败 / failed to refresh bunch master: {error}") })
    }
}

fn polynomial(terms: &[(usize, f64)]) -> Linear<f64> {
    Linear::new(terms.iter().map(|&(index, coefficient)| LinearMonomial::new(coefficient, index)).collect(), 0.0)
}

#[cfg(test)]
mod tests;
