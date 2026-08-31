//! 调度成本策略 / Scheduling cost policy
//!
//! 为任务成本、执行器成本、连接成本、产能成本和软约束惩罚提供标准注入入口。
//! Provides standard injection points for task cost, executor cost, connection cost,
//! capacity cost, and soft-constraint penalties.

use std::collections::HashMap;

use crate::domain::bunch_compilation::model::BunchEntry;
use crate::domain::common::{ExecutorId, ExecutorIdTrait};

/// 成本分解 / Cost breakdown
#[derive(Debug, Clone, PartialEq)]
pub struct CostBreakdown {
    /// 任务成本 / Task cost
    pub task_cost: f64,
    /// 执行器成本 / Executor cost
    pub executor_cost: f64,
    /// 连接成本 / Connection cost
    pub connection_cost: f64,
    /// 产能成本 / Capacity cost
    pub capacity_cost: f64,
    /// 软约束惩罚 / Soft-constraint penalty
    pub penalty_cost: f64,
}

impl CostBreakdown {
    /// 创建空成本分解 / Create empty cost breakdown
    pub fn zero() -> Self {
        Self {
            task_cost: 0.0,
            executor_cost: 0.0,
            connection_cost: 0.0,
            capacity_cost: 0.0,
            penalty_cost: 0.0,
        }
    }

    /// 总成本 / Total cost
    pub fn total(&self) -> f64 {
        self.task_cost
            + self.executor_cost
            + self.connection_cost
            + self.capacity_cost
            + self.penalty_cost
    }
}

impl Default for CostBreakdown {
    fn default() -> Self {
        Self::zero()
    }
}

/// 任务束成本策略 / Bunch cost policy
pub trait BunchCostPolicy<I = ExecutorId>: Send + Sync
where
    I: ExecutorIdTrait,
{
    /// 计算成本分解 / Calculate cost breakdown
    fn cost_breakdown(
        &self,
        bunch: &BunchEntry<I>,
        shadow_prices: &HashMap<usize, f64>,
    ) -> CostBreakdown;

    /// 计算 reduced cost / Calculate reduced cost
    fn reduced_cost(&self, bunch: &BunchEntry<I>, shadow_prices: &HashMap<usize, f64>) -> f64 {
        self.cost_breakdown(bunch, shadow_prices).total()
            - bunch
                .task_indices
                .iter()
                .filter_map(|task_index| shadow_prices.get(task_index))
                .sum::<f64>()
    }
}

/// 默认任务束成本策略 / Default bunch cost policy
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultBunchCostPolicy;

impl<I> BunchCostPolicy<I> for DefaultBunchCostPolicy
where
    I: ExecutorIdTrait,
{
    fn cost_breakdown(
        &self,
        bunch: &BunchEntry<I>,
        _shadow_prices: &HashMap<usize, f64>,
    ) -> CostBreakdown {
        CostBreakdown {
            task_cost: bunch.cost,
            ..Default::default()
        }
    }
}

/// 函数式任务束成本策略 / Functional bunch cost policy
pub struct FunctionalBunchCostPolicy<F, I = ExecutorId>
where
    F: Fn(&BunchEntry<I>, &HashMap<usize, f64>) -> CostBreakdown + Send + Sync,
    I: ExecutorIdTrait,
{
    calculate: F,
    _id: std::marker::PhantomData<I>,
}

impl<F, I> FunctionalBunchCostPolicy<F, I>
where
    F: Fn(&BunchEntry<I>, &HashMap<usize, f64>) -> CostBreakdown + Send + Sync,
    I: ExecutorIdTrait,
{
    /// 创建函数式成本策略 / Create functional cost policy
    pub fn new(calculate: F) -> Self {
        Self {
            calculate,
            _id: std::marker::PhantomData,
        }
    }
}

impl<F, I> BunchCostPolicy<I> for FunctionalBunchCostPolicy<F, I>
where
    F: Fn(&BunchEntry<I>, &HashMap<usize, f64>) -> CostBreakdown + Send + Sync,
    I: ExecutorIdTrait,
{
    fn cost_breakdown(
        &self,
        bunch: &BunchEntry<I>,
        shadow_prices: &HashMap<usize, f64>,
    ) -> CostBreakdown {
        (self.calculate)(bunch, shadow_prices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bunch_cost_policy_uses_bunch_cost() {
        let bunch: BunchEntry = BunchEntry {
            index: 0,
            executor_id: "exec_1".into(),
            task_indices: vec![0, 1],
            cost: 10.0,
            iteration: 0,
            slot_index: None,
        };
        let policy = DefaultBunchCostPolicy;
        let reduced = policy.reduced_cost(&bunch, &HashMap::from([(0, 2.0), (1, 3.0)]));

        assert!((reduced - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_functional_bunch_cost_policy_injects_business_penalty() {
        let bunch: BunchEntry = BunchEntry {
            index: 0,
            executor_id: "exec_1".into(),
            task_indices: vec![0],
            cost: 1.0,
            iteration: 0,
            slot_index: None,
        };
        let policy = FunctionalBunchCostPolicy::new(|bunch, _shadow_prices| CostBreakdown {
            task_cost: bunch.cost,
            executor_cost: 2.0,
            connection_cost: 3.0,
            capacity_cost: 4.0,
            penalty_cost: 5.0,
        });

        assert_eq!(policy.cost_breakdown(&bunch, &HashMap::new()).total(), 15.0);
    }
}
