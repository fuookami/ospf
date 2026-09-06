//! 成本 / Cost
//!
//! 表示调度任务的成本项，支持泛型数值和物理量。
//! Represents cost items for scheduling tasks, supporting generic numeric values and quantities.

use crate::infrastructure::GanttValueAdapter;
use ospf_rust_core::solver::value::SolveValue;

/// 成本项 / Cost item
///
/// 单个成本项，包含标签、成本量和消息。
/// A single cost item containing tag, cost quantity, and message.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct CostItem<V: SolveValue> {
    /// 成本标签 / Cost tag
    pub tag: String,
    /// 成本量 / Cost quantity (nullable)
    pub cost_quantity: Option<V>,
    /// 消息 / Message (nullable)
    pub message: Option<String>,
}

impl<V: SolveValue> CostItem<V> {
    /// 创建新的成本项 / Create new cost item
    pub fn new(tag: impl Into<String>, cost_quantity: Option<V>, message: Option<String>) -> Self {
        Self {
            tag: tag.into(),
            cost_quantity,
            message,
        }
    }

    /// 创建带成本量的成本项 / Create cost item with quantity
    pub fn with_quantity(tag: impl Into<String>, cost_quantity: V) -> Self {
        Self {
            tag: tag.into(),
            cost_quantity: Some(cost_quantity),
            message: None,
        }
    }

    /// 是否有效 / Whether valid
    ///
    /// 当 `cost_quantity` 不为 `None` 时有效。
    /// Valid when `cost_quantity` is not `None`.
    pub fn valid(&self) -> bool {
        self.cost_quantity.is_some()
    }
}

/// 成本 / Cost
///
/// 包含多个成本项的不可变成本容器。
/// Immutable cost container with multiple cost items.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Cost<V: SolveValue> {
    /// 成本项列表 / Cost items list
    pub items: Vec<CostItem<V>>,
    /// 成本总和 / Cost sum (nullable)
    pub cost_sum: Option<V>,
}

impl<V: SolveValue> Cost<V> {
    /// 创建空成本 / Create empty cost
    pub fn empty() -> Self {
        Self {
            items: vec![],
            cost_sum: None,
        }
    }

    /// 创建新的成本 / Create new cost
    ///
    /// 自动计算 `cost_sum`（当所有项都有效时）。
    /// Automatically computes `cost_sum` (when all items are valid).
    pub fn new(items: Vec<CostItem<V>>) -> Self {
        let cost_sum = if items.iter().all(|i| i.valid()) {
            let sum_f64: f64 = items
                .iter()
                .map(|i| GanttValueAdapter::<V>::to_f64(i.cost_quantity.as_ref().unwrap()))
                .sum();
            Some(GanttValueAdapter::<V>::from_f64(sum_f64))
        } else {
            None
        };
        Self { items, cost_sum }
    }

    /// 是否有效 / Whether valid
    ///
    /// 当 `cost_sum` 不为 `None` 时有效。
    /// Valid when `cost_sum` is not `None`.
    pub fn valid(&self) -> bool {
        self.cost_sum.is_some()
    }

    /// 求解器成本 / Solver cost
    ///
    /// 返回 `f64` 形式的成本值，用于求解器目标函数。
    /// Returns cost value as `f64` for solver objective function.
    ///
    /// - `default`: 当 `cost_sum` 为 `None` 时使用的默认值
    /// - `default`: default value when `cost_sum` is `None`
    pub fn solver_cost(&self, default: f64) -> f64 {
        self.cost_sum
            .as_ref()
            .map(|v| GanttValueAdapter::<V>::to_f64(v))
            .unwrap_or(default)
    }

    /// 合并另一个成本项 / Add another cost item
    pub fn add_item(&self, item: CostItem<V>) -> Self {
        let mut items = self.items.clone();
        items.push(item);
        Self::new(items)
    }

    /// 合并另一个成本 / Merge another cost
    pub fn merge(&self, other: &Cost<V>) -> Self {
        let mut items = self.items.clone();
        items.extend(other.items.iter().cloned());
        Self::new(items)
    }
}

impl<V: SolveValue> Default for Cost<V> {
    fn default() -> Self {
        Self::empty()
    }
}

/// 可变成本 / Mutable cost
///
/// 支持动态添加成本项的可变成本容器。
/// Mutable cost container supporting dynamic addition of cost items.
#[derive(Debug, Clone)]
pub struct MutableCost<V: SolveValue> {
    /// 成本项列表 / Cost items list
    pub items: Vec<CostItem<V>>,
    /// 成本总和 / Cost sum (nullable)
    pub cost_sum: Option<V>,
}

impl<V: SolveValue> MutableCost<V> {
    /// 创建空的可变成本 / Create empty mutable cost
    pub fn new() -> Self {
        Self {
            items: vec![],
            cost_sum: None,
        }
    }

    /// 添加成本项 / Add cost item
    ///
    /// 动态更新 `cost_sum`。
    /// Dynamically updates `cost_sum`.
    pub fn add_item(&mut self, item: CostItem<V>) {
        if item.valid() {
            let item_val = GanttValueAdapter::<V>::to_f64(item.cost_quantity.as_ref().unwrap());
            let new_sum = match &self.cost_sum {
                Some(current) => GanttValueAdapter::<V>::to_f64(current) + item_val,
                None => item_val,
            };
            self.cost_sum = Some(GanttValueAdapter::<V>::from_f64(new_sum));
        }
        self.items.push(item);
    }

    /// 转换为不可变成本 / Convert to immutable cost
    pub fn to_cost(&self) -> Cost<V> {
        Cost {
            items: self.items.clone(),
            cost_sum: self.cost_sum.clone(),
        }
    }
}

impl<V: SolveValue> Default for MutableCost<V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_item_with_quantity() {
        let item = CostItem::with_quantity("test", 12.5_f64);
        assert!(item.valid());
        assert_eq!(item.tag, "test");
        assert_eq!(item.cost_quantity, Some(12.5));
    }

    #[test]
    fn test_cost_item_null_quantity() {
        let item: CostItem<f64> = CostItem::new("test", None, None);
        assert!(!item.valid());
    }

    #[test]
    fn test_cost_sum_computation() {
        let cost = Cost::new(vec![
            CostItem::with_quantity("a", 1.25_f64),
            CostItem::with_quantity("b", 2.75_f64),
        ]);
        assert!(cost.valid());
        let sum = cost.cost_sum.unwrap();
        assert!((sum - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_cost_null_sum() {
        let cost = Cost::new(vec![
            CostItem::with_quantity("a", 1.0_f64),
            CostItem::new("b", None, None),
        ]);
        assert!(!cost.valid());
        assert!(cost.cost_sum.is_none());
    }

    #[test]
    fn test_mutable_cost() {
        let mut cost = MutableCost::<f64>::new();
        cost.add_item(CostItem::with_quantity("a", 1.0));
        cost.add_item(CostItem::with_quantity("b", 2.0));
        assert!(cost.cost_sum.is_some());
        assert!((cost.cost_sum.unwrap() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_mutable_cost_with_invalid_item() {
        let mut cost = MutableCost::<f64>::new();
        cost.add_item(CostItem::with_quantity("a", 1.0));
        cost.add_item(CostItem::new("b", None, None));
        // cost_sum should still be updated for valid items
        assert_eq!(cost.items.len(), 2);
        assert!(cost.cost_sum.is_some());
    }

    #[test]
    fn test_cost_solver_cost() {
        let cost = Cost::new(vec![
            CostItem::with_quantity("a", 1.5_f64),
            CostItem::with_quantity("b", 2.5_f64),
        ]);
        assert!((cost.solver_cost(0.0) - 4.0).abs() < 1e-10);

        let empty_cost = Cost::<f64>::empty();
        assert_eq!(empty_cost.solver_cost(0.0), 0.0);
    }

    #[test]
    fn test_cost_merge() {
        let cost1 = Cost::new(vec![CostItem::with_quantity("a", 1.0_f64)]);
        let cost2 = Cost::new(vec![CostItem::with_quantity("b", 2.0_f64)]);
        let merged = cost1.merge(&cost2);
        assert_eq!(merged.items.len(), 2);
        assert!((merged.solver_cost(0.0) - 3.0).abs() < 1e-10);
    }
}
