//! 迭代产能列编译 / Iterative capacity-column compilation
//!
//! 以稳定单列变量维护产能列生成的加列、删列和表达式重建生命周期。
//! Uses stable per-column variables to maintain the add/remove and expression
//! rebuild lifecycle of capacity-column generation.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{UInteger, VariableRange};

use crate::domain::capacity_scheduling::model::{
    CapacityColumn, CapacityColumnAggregation, ProductionActionTrait,
};
use crate::{GanttError, GanttResult};

/// 已注册的迭代产能列 / Registered iterative capacity column
#[derive(Debug, Clone)]
pub struct IterativeCapacityColumn<A: ProductionActionTrait> {
    /// 稳定列编号 / Stable column index
    pub index: usize,
    /// 生成迭代 / Generation iteration
    pub iteration: usize,
    /// 领域列 / Domain column
    pub column: CapacityColumn<A>,
    /// 模型变量索引 / Model variable index
    pub model_index: usize,
}

/// 迭代产能编译 / Iterative capacity compilation
///
/// 逻辑变量为 `x[executor, iteration, column]`。每个新列注册独立模型变量，
/// 因而不会在扩容时替换已注册 token 或破坏已有求解器索引。
///
/// The logical variable is `x[executor, iteration, column]`. Every new column
/// registers an independent model variable, so growth never replaces registered
/// tokens or invalidates existing solver indexes.
#[derive(Debug)]
pub struct IterativeCapacityCompilation<A: ProductionActionTrait> {
    /// 生产动作 / Production actions
    pub actions: Vec<A>,
    /// 执行器 / Executors
    pub executor_ids: Vec<A::ExecutorId>,
    /// 时隙数量 / Slot count
    pub slot_count: usize,
    /// 按执行器聚合的列 / Columns aggregated by executor
    pub columns_by_executor: HashMap<A::ExecutorId, CapacityColumnAggregation<A>>,
    /// 已注册列 / Registered columns
    pub columns: Vec<IterativeCapacityColumn<A>>,
    /// 已移除列编号 / Removed column indexes
    pub removed_columns: HashSet<usize>,
    next_column_index: usize,
}

impl<A: ProductionActionTrait> IterativeCapacityCompilation<A> {
    /// 创建迭代产能编译 / Create iterative capacity compilation
    pub fn new(
        actions: Vec<A>,
        executor_ids: Vec<impl Into<A::ExecutorId>>,
        slot_count: usize,
    ) -> Self {
        let executor_ids = executor_ids.into_iter().map(Into::into).collect::<Vec<_>>();
        let columns_by_executor = executor_ids
            .iter()
            .cloned()
            .map(|executor_id| (executor_id, CapacityColumnAggregation::new()))
            .collect();
        Self {
            actions,
            executor_ids,
            slot_count,
            columns_by_executor,
            columns: Vec::new(),
            removed_columns: HashSet::new(),
            next_column_index: 0,
        }
    }

    /// 注册迭代编译 / Register iterative compilation
    ///
    /// 变量由 `add_columns` 注册，本入口与其他 context 保持统一生命周期。
    /// Variables are registered by `add_columns`; this entry point keeps the
    /// lifecycle uniform with other contexts.
    pub fn register(&mut self, _model: &mut MetaModel<f64>) -> GanttResult<()> {
        Ok(())
    }

    /// 添加产能列 / Add capacity columns
    pub fn add_columns(
        &mut self,
        iteration: usize,
        new_columns: Vec<CapacityColumn<A>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<IterativeCapacityColumn<A>>> {
        let mut grouped = HashMap::<A::ExecutorId, Vec<CapacityColumn<A>>>::new();
        for column in new_columns {
            self.validate_column(&column)?;
            grouped
                .entry(column.executor_id.clone())
                .or_default()
                .push(column);
        }

        let mut added = Vec::new();
        for (executor_id, columns) in grouped {
            let aggregation = self
                .columns_by_executor
                .get_mut(&executor_id)
                .ok_or_else(|| GanttError::Calculation {
                    message: format!("missing capacity aggregation for executor {}", executor_id),
                })?;
            for column in aggregation.add_columns(iteration, columns) {
                let index = self.next_column_index;
                self.next_column_index += 1;
                let name = format!("capacity_column_{}_{}_{}", executor_id, iteration, index);
                let model_index = model
                    .register_auto_variable_with_range::<UInteger>(
                        &name,
                        VariableRange::bounded(0.0, self.column_upper_bound(&column) as f64),
                    )
                    .map_err(|error| GanttError::Calculation {
                        message: format!(
                            "failed to register capacity column {}: {:?}",
                            index, error
                        ),
                    })?;
                let record = IterativeCapacityColumn {
                    index,
                    iteration,
                    column,
                    model_index,
                };
                self.columns.push(record.clone());
                added.push(record);
            }
        }
        Ok(added)
    }

    /// 移除产能列 / Remove capacity columns
    ///
    /// 移除操作将变量固定为零，并更新活跃列和聚合状态。
    /// Removal fixes the variable to zero and updates active-column and aggregation state.
    pub fn remove_columns(
        &mut self,
        column_indices: &[usize],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        for &index in column_indices {
            let Some(record) = self.columns.iter().find(|record| record.index == index) else {
                continue;
            };
            if !self.removed_columns.insert(index) {
                continue;
            }
            model
                .fix_variable_by_index(record.model_index, 0.0)
                .map_err(|error| GanttError::Calculation {
                    message: format!("failed to remove capacity column {}: {:?}", index, error),
                })?;
            if let Some(aggregation) = self.columns_by_executor.get_mut(&record.column.executor_id)
            {
                aggregation.remove_column(&record.column);
            }
        }
        Ok(())
    }

    /// 活跃列到变量的权威映射 / Authoritative active-column to variable mapping
    pub fn active_column_variables(&self) -> Vec<(&CapacityColumn<A>, usize)> {
        self.columns
            .iter()
            .filter(|record| !self.removed_columns.contains(&record.index))
            .map(|record| (&record.column, record.model_index))
            .collect()
    }

    /// 执行器-时隙选列项 / Executor-slot selection terms
    #[allow(clippy::type_complexity)]
    pub fn executor_slot_terms(&self) -> HashMap<(A::ExecutorId, usize), Vec<(usize, f64)>> {
        let mut terms = HashMap::new();
        for executor_id in &self.executor_ids {
            for slot_index in 0..self.slot_count {
                terms.insert((executor_id.clone(), slot_index), Vec::new());
            }
        }
        for record in self.active_records() {
            terms
                .entry((record.column.executor_id.clone(), record.column.slot_index))
                .or_default()
                .push((record.model_index, 1.0));
        }
        terms
    }

    /// 动作-时隙操作时间项 / Action-slot operation-time terms
    pub fn operation_time_terms(&self) -> Vec<Vec<(usize, f64)>> {
        let mut terms = vec![Vec::new(); self.actions.len() * self.slot_count];
        for record in self.active_records() {
            for (action, amount) in &record.column.allocations {
                if *amount == 0 {
                    continue;
                }
                if let Some(action_index) = self
                    .actions
                    .iter()
                    .position(|candidate| candidate.id() == action.id())
                {
                    terms[action_index * self.slot_count + record.column.slot_index]
                        .push((record.model_index, action.unit_capacity() * *amount as f64));
                }
            }
        }
        terms
    }

    /// 执行器-时隙产能项 / Executor-slot capacity terms
    #[allow(clippy::type_complexity)]
    pub fn capacity_terms(&self) -> HashMap<(A::ExecutorId, usize), Vec<(usize, f64)>> {
        let mut terms = self.executor_slot_terms();
        for values in terms.values_mut() {
            values.clear();
        }
        for record in self.active_records() {
            let coefficient = record
                .column
                .allocations
                .iter()
                .map(|(action, amount)| action.unit_capacity() * *amount as f64)
                .sum();
            terms
                .entry((record.column.executor_id.clone(), record.column.slot_index))
                .or_default()
                .push((record.model_index, coefficient));
        }
        terms
    }

    /// 产能列成本项 / Capacity-column cost terms
    pub fn cost_terms(&self) -> Vec<(usize, f64)> {
        self.active_records()
            .map(|record| (record.model_index, record.column.cost))
            .collect()
    }

    /// 从解中提取产能排程结果 / Extract capacity scheduling solution
    pub fn extract_solution(
        &self,
        solution: &[f64],
    ) -> super::model::CapacitySchedulingSolution<A> {
        let mut action_allocations = Vec::new();
        let mut executor_capacities = Vec::new();
        for record in self.active_records() {
            let multiplier = solution
                .get(record.model_index)
                .copied()
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| value.round() as u64)
                .unwrap_or(0);
            if multiplier == 0 {
                continue;
            }
            let total_capacity = record
                .column
                .allocations
                .iter()
                .map(|(action, amount)| action.unit_capacity() * *amount as f64 * multiplier as f64)
                .sum();
            if total_capacity > 0.0 {
                executor_capacities.push(super::model::ExecutorCapacityResult {
                    executor_id: record.column.executor_id.clone(),
                    slot_index: record.column.slot_index,
                    total_capacity,
                });
            }
            for (action, amount) in &record.column.allocations {
                if *amount > 0 {
                    action_allocations.push(super::model::ActionAllocation {
                        action: action.clone(),
                        slot_index: record.column.slot_index,
                        amount: amount.saturating_mul(multiplier),
                        order: record.column.order,
                    });
                }
            }
        }
        super::model::CapacitySchedulingSolution {
            actions: self.actions.clone(),
            action_allocations,
            executor_capacities,
        }
    }

    fn active_records(&self) -> impl Iterator<Item = &IterativeCapacityColumn<A>> {
        self.columns
            .iter()
            .filter(|record| !self.removed_columns.contains(&record.index))
    }

    fn validate_column(&self, column: &CapacityColumn<A>) -> GanttResult<()> {
        if !self.executor_ids.contains(&column.executor_id) || column.slot_index >= self.slot_count
        {
            return Err(GanttError::Calculation {
                message: "capacity column references an unknown executor or slot".to_string(),
            });
        }
        if column.allocations.iter().any(|(action, _)| {
            action.executor_id() != &column.executor_id
                || !self
                    .actions
                    .iter()
                    .any(|candidate| candidate.id() == action.id())
        }) {
            return Err(GanttError::Calculation {
                message: "capacity column contains an invalid production action".to_string(),
            });
        }
        Ok(())
    }

    fn column_upper_bound(&self, column: &CapacityColumn<A>) -> u64 {
        column
            .allocations
            .iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(action, amount)| action.upper_bound_at(column.slot_index) / *amount)
            .min()
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::capacity_scheduling::model::BasicProductionAction;

    #[test]
    fn iterative_capacity_compilation_tracks_add_remove_and_terms() {
        let actions = vec![BasicProductionAction::new(
            "a1", "Action 1", "exec_1", 2.0, 3.0,
        )];
        let mut compilation = IterativeCapacityCompilation::new(actions.clone(), vec!["exec_1"], 2);
        let mut model = MetaModel::<f64>::new("iterative_capacity");
        compilation.register(&mut model).unwrap();
        let mut productive = CapacityColumn::new("exec_1", 0, 0, 7.0).with_key("productive");
        productive.allocations.push((actions[0].clone(), 2));
        let idle = CapacityColumn::new("exec_1", 1, 0, 0.0).with_key("idle");
        let added = compilation
            .add_columns(0, vec![productive, idle], &mut model)
            .unwrap();

        assert_eq!(added.len(), 2);
        assert_eq!(compilation.active_column_variables().len(), 2);
        assert_eq!(
            compilation.operation_time_terms()[0],
            vec![(added[0].model_index, 4.0)]
        );
        assert_eq!(
            model
                .variable_range_by_index(added[1].model_index)
                .unwrap()
                .upper_bound,
            Some(1.0)
        );

        compilation
            .remove_columns(&[added[0].index], &mut model)
            .unwrap();
        assert_eq!(compilation.active_column_variables().len(), 1);
        assert!(compilation.operation_time_terms()[0].is_empty());
        assert_eq!(
            model.variable_range_by_index(added[0].model_index),
            Some(VariableRange::fixed(0.0))
        );
    }
}
