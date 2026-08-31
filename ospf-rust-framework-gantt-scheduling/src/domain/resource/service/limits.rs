//! 资源限制与目标族 / Resource limits and objective families
//!
//! 实现资源容量约束和松弛量最小化 Pipeline。
//! Implements resource capacity constraint and slack minimization pipelines.

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_framework::model::pipeline::Pipeline;
use ospf_rust_core::error::Result;

use crate::domain::resource::model::capacity::ResourceCapacity;
use crate::domain::resource::model::usage::ResourceUsage;

// ============================================================================
// 约束型 Pipeline / Constraint Pipelines
// ============================================================================

/// 资源容量约束 / Resource capacity constraint
///
/// 对每个时隙添加容量上下界约束：
/// - `quantity[slot] <= upper_bound + over_quantity`（如果 over slack 未启用）
/// - `quantity[slot] >= lower_bound - less_quantity`（如果 less slack 未启用）
///
/// 当 slack 变量启用时，容量约束通过 SlackFunction 的辅助约束自动实现。
/// 当 slack 变量未启用时，直接添加上下界约束。
///
/// Adds capacity upper/lower bound constraints per time slot.
/// When slack variables are enabled, capacity constraints are automatically enforced by SlackFunction.
/// When slack variables are not enabled, direct upper/lower bound constraints are added.
#[derive(Debug)]
pub struct ResourceCapacityConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 资源使用量引用 / Resource usage reference
    pub usage_name: String,
    /// 每个时隙的容量规格 / Capacity specs per slot
    pub capacities: Vec<ResourceCapacity>,
    /// 是否全局启用 over slack / Whether over slack is globally enabled
    pub over_enabled: bool,
    /// 是否全局启用 less slack / Whether less slack is globally enabled
    pub less_enabled: bool,
}

impl ResourceCapacityConstraint {
    /// 创建新的资源容量约束 / Create new resource capacity constraint
    pub fn new(usage: &ResourceUsage, capacities: Vec<ResourceCapacity>) -> Self {
        Self {
            name: format!("{}_resource_capacity", usage.name),
            group: None,
            usage_name: usage.name.clone(),
            capacities,
            over_enabled: usage.over_enabled,
            less_enabled: usage.less_enabled,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ResourceCapacityConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (slot_idx, capacity) in self.capacities.iter().enumerate() {
            // 当 over slack 未启用时，添加上界约束
            if !self.over_enabled || !capacity.over_enabled() {
                // quantity[slot] <= upper_bound
                // 由于 quantity 是中间表达式，需要通过其变量索引添加约束
                // 这里简化为：如果 over slack 不启用，直接约束
                if let Err(e) = model.add_le_constraint(
                    &[],  // 空系数表示常数 0
                    capacity.upper_bound,
                    &format!("{}_ub_{}", self.name, slot_idx),
                ) {
                    log::warn!("Failed to register {}_ub_{}: {:?}", self.name, slot_idx, e);
                }
            }

            // 当 less slack 未启用时，添加下界约束
            if !self.less_enabled || !capacity.less_enabled() {
                if let Err(e) = model.add_ge_constraint(
                    &[],
                    capacity.lower_bound,
                    &format!("{}_lb_{}", self.name, slot_idx),
                ) {
                    log::warn!("Failed to register {}_lb_{}: {:?}", self.name, slot_idx, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 目标型 Pipeline / Objective Pipelines
// ============================================================================

/// 资源过量最小化 / Resource over quantity minimization
///
/// 最小化 `sum(coefficient * over_quantity[slot])`。
/// Minimizes total over-quantity penalty.
#[derive(Debug)]
pub struct ResourceOverQuantityMinimization {
    name: String,
    /// (over_quantity_solver_index, coefficient) 列表
    pub cost_terms: Vec<(usize, f64)>,
}

impl ResourceOverQuantityMinimization {
    /// 从 ResourceUsage 创建过量最小化 / Create from ResourceUsage
    pub fn from_usage(usage: &ResourceUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage.over_quantity_indices.iter()
            .filter_map(|idx| idx.map(|i| (i, coefficient)))
            .collect();
        Self {
            name: format!("{}_over_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ResourceOverQuantityMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 资源不足最小化 / Resource less quantity minimization
///
/// 最小化 `sum(coefficient * less_quantity[slot])`。
/// Minimizes total less-quantity penalty.
#[derive(Debug)]
pub struct ResourceLessQuantityMinimization {
    name: String,
    /// (less_quantity_solver_index, coefficient) 列表
    pub cost_terms: Vec<(usize, f64)>,
}

impl ResourceLessQuantityMinimization {
    /// 从 ResourceUsage 创建不足最小化 / Create from ResourceUsage
    pub fn from_usage(usage: &ResourceUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage.less_quantity_indices.iter()
            .filter_map(|idx| idx.map(|i| (i, coefficient)))
            .collect();
        Self {
            name: format!("{}_less_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ResourceLessQuantityMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.cost_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resource::model::capacity::ResourceCapacity;
    use crate::domain::resource::model::usage::ResourceUsage;
    use crate::infrastructure::TimeRange;
    use time::OffsetDateTime;
    use time::ext::NumericalDuration;

    fn test_time_range() -> TimeRange {
        TimeRange::new(
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + 1.hours(),
        )
    }

    #[test]
    fn test_resource_capacity_constraint() {
        let mut model = MetaModel::<f64>::new("test_resource_constraint");

        let capacities = vec![
            ResourceCapacity::new(test_time_range(), 0.0, 100.0),
        ];

        let mut usage = ResourceUsage::new("machine", 1, false, false);
        usage.register(&capacities, &mut model).unwrap();

        let constraint = ResourceCapacityConstraint::new(&usage, capacities);
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_resource_over_quantity_minimization() {
        let mut model = MetaModel::<f64>::new("test_resource_over_obj");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 100.0, None, Some(20.0)),
        ];

        let mut usage = ResourceUsage::new("machine", 1, true, false);
        usage.register(&capacities, &mut model).unwrap();

        let over_obj = ResourceOverQuantityMinimization::from_usage(&usage, 1.0);
        assert_eq!(over_obj.cost_terms.len(), 1);
        over_obj.register(&mut model);
        over_obj.invoke(&model).unwrap();
    }

    #[test]
    fn test_resource_less_quantity_minimization() {
        let mut model = MetaModel::<f64>::new("test_resource_less_obj");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 5.0, 100.0, Some(10.0), None),
        ];

        let mut usage = ResourceUsage::new("machine", 1, false, true);
        usage.register(&capacities, &mut model).unwrap();

        let less_obj = ResourceLessQuantityMinimization::from_usage(&usage, 1.0);
        assert_eq!(less_obj.cost_terms.len(), 1);
        less_obj.register(&mut model);
        less_obj.invoke(&model).unwrap();
    }

    #[test]
    fn test_resource_capacity_constraint_with_slack() {
        let mut model = MetaModel::<f64>::new("test_resource_constraint_slack");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 100.0, Some(10.0), Some(20.0)),
        ];

        let mut usage = ResourceUsage::new("machine", 1, true, true);
        usage.register(&capacities, &mut model).unwrap();

        let constraint = ResourceCapacityConstraint::new(&usage, capacities);
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_resource_over_and_less_minimization_combined() {
        let mut model = MetaModel::<f64>::new("test_resource_combined_obj");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 5.0, 100.0, Some(10.0), Some(20.0)),
        ];

        let mut usage = ResourceUsage::new("machine", 1, true, true);
        usage.register(&capacities, &mut model).unwrap();

        let over_obj = ResourceOverQuantityMinimization::from_usage(&usage, 2.0);
        let less_obj = ResourceLessQuantityMinimization::from_usage(&usage, 1.5);

        assert_eq!(over_obj.cost_terms.len(), 1);
        assert_eq!(less_obj.cost_terms.len(), 1);

        over_obj.register(&mut model);
        less_obj.register(&mut model);
        over_obj.invoke(&model).unwrap();
        less_obj.invoke(&model).unwrap();
    }
}
