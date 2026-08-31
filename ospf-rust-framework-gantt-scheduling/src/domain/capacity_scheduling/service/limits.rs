//! 产能排程限制与目标族 / Capacity scheduling limits and objective families
//!
//! 实现产能排程约束和目标 Pipeline。
//! Implements capacity scheduling constraint and objective pipelines.

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_framework::model::pipeline::Pipeline;
use ospf_rust_core::error::Result;
use ospf_rust_core::symbol::LinearIntermediateSymbol;

use crate::domain::capacity_scheduling::model::{
    CapacityCompilation, CapacityOrderCompilation, ProductionActionTrait,
};

// ============================================================================
// 约束型 Pipeline / Constraint Pipelines
// ============================================================================

/// 执行器产能约束 / Executor capacity constraint
///
/// 对每个执行器在每个时隙添加产能上界约束：
/// `capacity[executor, slot] <= available_capacity`
///
/// Adds capacity upper bound constraint per executor per time slot:
/// `capacity[executor, slot] <= available_capacity`
#[derive(Debug)]
pub struct ExecutorCapacityConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// (capacity_usage_model_index, capacity_limit) 列表
    /// 每项对应一个 executor-slot 对
    pub constraints: Vec<(usize, f64)>,
}

impl ExecutorCapacityConstraint {
    /// 从 CapacityCompilation 创建执行器产能约束 / Create from CapacityCompilation
    pub fn from_compilation<A: ProductionActionTrait>(
        compilation: &CapacityCompilation<A>,
        slot_capacity: f64,
    ) -> Self {
        let constraints: Vec<(usize, f64)> = compilation.capacity_symbols.iter()
            .enumerate()
            .filter_map(|(idx, sym)| {
                let poly = sym.as_ref().to_linear_polynomial();
                if let Some(first) = poly.monomials().first() {
                    Some((first.var_index(), slot_capacity))
                } else {
                    let _ = idx;
                    None
                }
            })
            .collect();

        Self {
            name: format!("{}_executor_capacity", compilation.actions.first().map(|a| a.executor_id()).unwrap_or("cap")),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ExecutorCapacityConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (idx, (cap_idx, limit)) in self.constraints.iter().enumerate() {
            let terms = vec![LinearMonomial::new(1.0, *cap_idx)];
            if let Err(e) = model.add_le_constraint(
                &terms.iter().map(|m| (m.var_index(), *m.coefficient())).collect::<Vec<_>>(),
                *limit,
                &format!("{}_{}", self.name, idx),
            ) {
                log::warn!("Failed to register {}_{}: {:?}", self.name, idx, e);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 订单约束 / Order constraint
///
/// 用于 CapacityOrderCompilation，确保：
/// 1. 每个订单位置最多选择一个动作：`sum(b[action, slot, order] for action) <= 1`
/// 2. x 与 b 的关联：`x[action, slot, order] >= b[action, slot, order]`
/// 3. x 的上界链接：`x[action, slot, order] <= M * b[action, slot, order]`
///
/// Used for CapacityOrderCompilation, ensures:
/// 1. At most one action per order position: `sum(b[action, slot, order] for action) <= 1`
/// 2. x-b linking: `x[action, slot, order] >= b[action, slot, order]`
/// 3. Upper bound linking: `x[action, slot, order] <= M * b[action, slot, order]`
#[derive(Debug)]
pub struct OrderConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 订单编译约束多项式 / Order compilation constraint polynomials
    /// 每项为 (terms, rhs) 表示 sum(terms) <= rhs
    pub order_polynomials: Vec<(Vec<(usize, f64)>, f64)>,
    /// x >= b 约束 / x >= b constraints
    pub x_ge_b_constraints: Vec<(usize, usize)>,  // (x_model_index, b_model_index)
    /// x <= M * b 约束 / x <= M * b constraints
    pub x_le_mb_constraints: Vec<(usize, usize, f64)>,  // (x_model_index, b_model_index, M)
}

impl OrderConstraint {
    /// 从 CapacityOrderCompilation 创建订单约束 / Create from CapacityOrderCompilation
    pub fn from_compilation<A: ProductionActionTrait>(
        compilation: &CapacityOrderCompilation<A>,
        big_m: f64,
    ) -> Self {
        let n_actions = compilation.actions.len();
        let n_slots = compilation.slot_count;
        let n_orders = compilation.max_order;

        let mut order_polynomials = Vec::new();
        let mut x_ge_b_constraints = Vec::new();
        let mut x_le_mb_constraints = Vec::new();

        // 1. sum(b[action, slot, order] for action) <= 1
        for si in 0..n_slots {
            for oi in 0..n_orders {
                let mut terms = Vec::new();
                for ai in 0..n_actions {
                    if let Some(b_idx) = compilation.b.as_ref().and_then(|b| b.model_index(&ai, &si, &oi)) {
                        terms.push((b_idx, 1.0));
                    }
                }
                if !terms.is_empty() {
                    order_polynomials.push((terms, 1.0));
                }
            }
        }

        // 2. x[action, slot, order] >= b[action, slot, order]
        //    等价于 b - x <= 0
        // 3. x[action, slot, order] <= M * b[action, slot, order]
        //    等价于 x - M*b <= 0
        for ai in 0..n_actions {
            for si in 0..n_slots {
                for oi in 0..n_orders {
                    if let (Some(x_idx), Some(b_idx)) = (
                        compilation.x.as_ref().and_then(|x| x.model_index(&ai, &si, &oi)),
                        compilation.b.as_ref().and_then(|b| b.model_index(&ai, &si, &oi)),
                    ) {
                        x_ge_b_constraints.push((x_idx, b_idx));
                        x_le_mb_constraints.push((x_idx, b_idx, big_m));
                    }
                }
            }
        }

        Self {
            name: "order_constraint".to_string(),
            group: None,
            order_polynomials,
            x_ge_b_constraints,
            x_le_mb_constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for OrderConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        // 1. sum(b[action, slot, order]) <= 1
        for (idx, (terms, rhs)) in self.order_polynomials.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(terms, *rhs, &format!("{}_order_{}", self.name, idx)) {
                log::warn!("Failed to register {}_order_{}: {:?}", self.name, idx, e);
            }
        }

        // 2. x >= b  =>  b - x <= 0
        for (idx, &(x_idx, b_idx)) in self.x_ge_b_constraints.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(
                &[(b_idx, 1.0), (x_idx, -1.0)],
                0.0,
                &format!("{}_x_ge_b_{}", self.name, idx),
            ) {
                log::warn!("Failed to register {}_x_ge_b_{}: {:?}", self.name, idx, e);
            }
        }

        // 3. x <= M * b  =>  x - M*b <= 0
        for (idx, &(x_idx, b_idx, m)) in self.x_le_mb_constraints.iter().enumerate() {
            if let Err(e) = model.add_le_constraint(
                &[(x_idx, 1.0), (b_idx, -m)],
                0.0,
                &format!("{}_x_le_mb_{}", self.name, idx),
            ) {
                log::warn!("Failed to register {}_x_le_mb_{}: {:?}", self.name, idx, e);
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

/// 产能成本最小化 / Capacity cost minimization
///
/// 最小化 `sum(action.unit_cost * x[action, slot])`。
/// Minimizes total capacity cost.
#[derive(Debug)]
pub struct CapacityCostMinimization {
    name: String,
    /// 成本变量 solver_index / Cost variable solver_index
    pub cost_index: Option<usize>,
    /// 系数 / Coefficient
    pub coefficient: f64,
    /// 直接成本项：x 变量索引列表和系数 / Direct cost terms: x variable indices and coefficients
    pub cost_terms: Vec<(usize, f64)>,
}

impl CapacityCostMinimization {
    /// 从 CapacityCompilation 创建产能成本最小化 / Create from CapacityCompilation
    pub fn from_compilation<A: ProductionActionTrait>(
        compilation: &CapacityCompilation<A>,
        coefficient: f64,
    ) -> Self {
        let mut cost_terms = Vec::new();
        if let Some(ref x) = compilation.x {
            for (ai, action) in compilation.actions.iter().enumerate() {
                if action.unit_cost() != 0.0 {
                    for si in 0..compilation.slot_count {
                        if let Some(x_idx) = x.model_index(&ai, &si) {
                            cost_terms.push((x_idx, action.unit_cost() * coefficient));
                        }
                    }
                }
            }
        }

        Self {
            name: "capacity_cost_minimization".to_string(),
            cost_index: compilation.cost_index,
            coefficient,
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for CapacityCostMinimization {
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
    use crate::domain::capacity_scheduling::model::BasicProductionAction;

    #[test]
    fn test_executor_capacity_constraint() {
        let mut model = MetaModel::<f64>::new("test_executor_cap");

        let actions = vec![
            BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();

        let constraint = ExecutorCapacityConstraint::from_compilation(&compilation, 8.0);
        assert!(!constraint.constraints.is_empty());
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_order_constraint() {
        let mut model = MetaModel::<f64>::new("test_order_constraint");

        let actions = vec![
            BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityOrderCompilation::new(actions, executor_ids, 2, 3);
        compilation.register(&mut model).unwrap();

        let constraint = OrderConstraint::from_compilation(&compilation, 100.0);
        assert!(!constraint.order_polynomials.is_empty());
        assert!(!constraint.x_ge_b_constraints.is_empty());
        assert!(!constraint.x_le_mb_constraints.is_empty());
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_capacity_cost_minimization() {
        let mut model = MetaModel::<f64>::new("test_cap_cost");

        let actions = vec![
            BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();

        let obj = CapacityCostMinimization::from_compilation(&compilation, 1.0);
        assert!(!obj.cost_terms.is_empty());
        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }
}
