//! 产能排程限制与目标族 / Capacity scheduling limits and objective families
//!
//! 实现产能排程约束和目标 Pipeline。
//! Implements capacity scheduling constraint and objective pipelines.

use std::collections::HashMap;

use ospf_rust_core::error::Result;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearIntermediateSymbol;
use ospf_rust_framework::model::pipeline::Pipeline;
use ospf_rust_framework::solver::column_generation_solver::LinearDualSolution;

use crate::domain::capacity_scheduling::iterative::IterativeCapacityCompilation;
use crate::domain::capacity_scheduling::model::{
    CapacityColumn, CapacityCompilation, CapacityOrderCompilation, ProductionActionTrait,
};
use crate::domain::common::{ConstraintIndexKey, ConstraintIndexMap, ExecutorId, ExecutorIdTrait};

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
    /// (capacity_usage_model_index, capacity_limit) 列表 / List
    /// 每项对应一个 executor-slot 对 / Each item corresponds to an executor-slot pair
    pub constraints: Vec<(usize, f64)>,
}

impl ExecutorCapacityConstraint {
    /// 从 CapacityCompilation 创建执行器产能约束 / Create from CapacityCompilation
    pub fn from_compilation<A: ProductionActionTrait>(
        compilation: &CapacityCompilation<A>,
        slot_capacity: f64,
    ) -> Self {
        let constraints: Vec<(usize, f64)> = compilation
            .capacity_symbols
            .iter()
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
            name: format!(
                "{}_executor_capacity",
                compilation
                    .actions
                    .first()
                    .map(|action| action.executor_id().to_string())
                    .unwrap_or_else(|| "cap".to_string()),
            ),
            group: None,
            constraints,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ExecutorCapacityConstraint {
    fn name(&self) -> &str {
        &self.name
    }
    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (idx, (cap_idx, limit)) in self.constraints.iter().enumerate() {
            let terms = vec![LinearMonomial::new(1.0, *cap_idx)];
            if let Err(e) = model.add_le_constraint(
                &terms
                    .iter()
                    .map(|m| (m.var_index(), *m.coefficient()))
                    .collect::<Vec<_>>(),
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

/// 产能列选择约束 / Capacity-column selection constraint
///
/// 对每个活跃 `(executor, slot)` 组添加恰选一条产能列的约束：
/// `sum(column_selection) == 1`。
///
/// Adds an exactly-one capacity-column constraint for every active
/// `(executor, slot)` group: `sum(column_selection) == 1`.
#[derive(Debug)]
pub struct CapacityColumnSelectionConstraint<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    name: String,
    group: Option<ConstraintGroup>,
    /// 执行器-时隙选列项 / Executor-slot column-selection terms
    pub selection_polynomials: Vec<(I, usize, Vec<(usize, f64)>)>,
}

impl<I> CapacityColumnSelectionConstraint<I>
where
    I: ExecutorIdTrait,
{
    /// 从迭代产能编译创建约束 / Create from iterative capacity compilation
    ///
    /// 使用编译当前活跃的列变量快照，覆盖全部执行器-时隙组。
    /// Uses the compilation's current active-column-variable snapshot and
    /// covers every executor-slot group.
    pub fn from_iterative_compilation<A>(compilation: &IterativeCapacityCompilation<A>) -> Self
    where
        A: ProductionActionTrait<ExecutorId = I>,
    {
        let active_groups = compilation
            .executor_ids
            .iter()
            .flat_map(|executor_id| {
                (0..compilation.slot_count).map(move |slot_index| (executor_id.clone(), slot_index))
            })
            .collect::<Vec<_>>();
        Self::from_columns(active_groups, compilation.active_column_variables())
    }

    /// 从活跃列变量映射创建约束 / Create from active column-variable mappings
    pub fn from_columns<'a, A, Groups, Columns>(
        active_groups: Groups,
        column_variables: Columns,
    ) -> Self
    where
        A: ProductionActionTrait<ExecutorId = I> + 'a,
        Groups: IntoIterator<Item = (I, usize)>,
        Columns: IntoIterator<Item = (&'a CapacityColumn<A>, usize)>,
    {
        let mut selection_polynomials = active_groups
            .into_iter()
            .map(|(executor_id, slot_index)| (executor_id, slot_index, Vec::new()))
            .collect::<Vec<_>>();

        for (column, variable_index) in column_variables {
            if let Some((_, _, terms)) =
                selection_polynomials
                    .iter_mut()
                    .find(|(executor_id, slot_index, _)| {
                        *executor_id == column.executor_id && *slot_index == column.slot_index
                    })
            {
                terms.push((variable_index, 1.0));
            }
        }

        selection_polynomials.sort_by(|lhs, rhs| {
            lhs.0
                .to_string()
                .cmp(&rhs.0.to_string())
                .then_with(|| lhs.1.cmp(&rhs.1))
        });
        Self {
            name: "capacity_column_selection".to_string(),
            group: None,
            selection_polynomials,
        }
    }

    /// 约束名称 / Constraint name
    pub fn constraint_name(executor_id: &I, slot_index: usize) -> String {
        format!("capacity_column_selection_{}_{}", executor_id, slot_index)
    }

    /// 注册约束索引映射 / Register constraint-index mappings
    pub fn register_constraint_indexes(
        &self,
        constraint_index_map: &mut ConstraintIndexMap,
        constraint_name_to_index: &HashMap<String, usize>,
    ) {
        for (executor_id, slot_index, _) in &self.selection_polynomials {
            let name = Self::constraint_name(executor_id, *slot_index);
            if let Some(&dual_index) = constraint_name_to_index.get(&name) {
                constraint_index_map.register(
                    ConstraintIndexKey::capacity_column_selection(
                        executor_id.to_string(),
                        *slot_index,
                    ),
                    name,
                    dual_index,
                );
            }
        }
    }

    /// 提取产能列选择影子价格 / Extract capacity-column selection shadow prices
    pub fn extract_shadow_prices(
        &self,
        dual_solution: &LinearDualSolution,
        constraint_index_map: &ConstraintIndexMap,
    ) -> HashMap<(I, usize), f64> {
        let mut prices = HashMap::new();
        for (executor_id, slot_index, _) in &self.selection_polynomials {
            let key =
                ConstraintIndexKey::capacity_column_selection(executor_id.to_string(), *slot_index);
            if let Some(price) = constraint_index_map.dual_value(&key, &dual_solution.constraints) {
                prices.insert((executor_id.clone(), *slot_index), price);
            }
        }
        prices
    }
}

impl<I> Pipeline<MetaModel<f64>> for CapacityColumnSelectionConstraint<I>
where
    I: ExecutorIdTrait,
{
    fn name(&self) -> &str {
        &self.name
    }
    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (executor_id, slot_index, terms) in &self.selection_polynomials {
            let name = Self::constraint_name(executor_id, *slot_index);
            if let Err(error) = model.add_eq_constraint(terms, 1.0, &name) {
                log::warn!("Failed to register {}: {:?}", name, error);
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
    /// 每项为 (terms, rhs) 表示 sum(terms) <= rhs / Each item is (terms, rhs) representing sum(terms) <= rhs
    pub order_polynomials: Vec<(Vec<(usize, f64)>, f64)>,
    /// x >= b 约束 / x >= b constraints
    pub x_ge_b_constraints: Vec<(usize, usize)>, // (x_model_index, b_model_index)
    /// x <= M * b 约束 / x <= M * b constraints
    pub x_le_mb_constraints: Vec<(usize, usize, f64)>, // (x_model_index, b_model_index, M)
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
                    if let Some(b_idx) = compilation
                        .b
                        .as_ref()
                        .and_then(|b| b.model_index(&ai, &si, &oi))
                    {
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
                        compilation
                            .x
                            .as_ref()
                            .and_then(|x| x.model_index(&ai, &si, &oi)),
                        compilation
                            .b
                            .as_ref()
                            .and_then(|b| b.model_index(&ai, &si, &oi)),
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
    fn name(&self) -> &str {
        &self.name
    }
    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        // 1. sum(b[action, slot, order]) <= 1
        for (idx, (terms, rhs)) in self.order_polynomials.iter().enumerate() {
            if let Err(e) =
                model.add_le_constraint(terms, *rhs, &format!("{}_order_{}", self.name, idx))
            {
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
    fn name(&self) -> &str {
        &self.name
    }
    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.cost_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self
            .cost_terms
            .iter()
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
    use std::collections::HashMap;

    use super::*;
    use crate::domain::capacity_scheduling::model::BasicProductionAction;

    #[test]
    fn test_executor_capacity_constraint() {
        let mut model = MetaModel::<f64>::new("test_executor_cap");

        let actions = vec![BasicProductionAction::new(
            "a1", "Action 1", "exec_1", 1.0, 10.0,
        )];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();

        let constraint = ExecutorCapacityConstraint::from_compilation(&compilation, 8.0);
        assert!(!constraint.constraints.is_empty());
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_capacity_column_selection_registers_and_preserves_zero_dual() {
        let mut model = MetaModel::<f64>::new("test_capacity_column_selection");
        let actions = vec![BasicProductionAction::new(
            "a1", "Action 1", "exec_1", 1.0, 10.0,
        )];
        let executor_ids = vec!["exec_1".to_string()];
        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();
        let x = compilation.x.as_ref().unwrap();

        let first = CapacityColumn::<BasicProductionAction>::new("exec_1", 0, 0, 0.0);
        let second = CapacityColumn::<BasicProductionAction>::new("exec_1", 1, 0, 0.0);
        let constraint = CapacityColumnSelectionConstraint::from_columns(
            vec![("exec_1".into(), 0), ("exec_1".into(), 1)],
            vec![
                (&first, x.model_index(&0, &0).unwrap()),
                (&second, x.model_index(&0, &1).unwrap()),
            ],
        );
        constraint.register(&mut model);

        assert_eq!(constraint.selection_polynomials.len(), 2);
        assert_eq!(model.num_constraints(), 2);

        let names = HashMap::from([
            ("capacity_column_selection_exec_1_0".to_string(), 0),
            ("capacity_column_selection_exec_1_1".to_string(), 1),
        ]);
        let mut indexes = ConstraintIndexMap::new();
        constraint.register_constraint_indexes(&mut indexes, &names);
        let prices = constraint.extract_shadow_prices(
            &LinearDualSolution::new(vec![0.0, 3.0], Vec::new()),
            &indexes,
        );

        assert_eq!(prices.get(&("exec_1".into(), 0)), Some(&0.0));
        assert_eq!(prices.get(&("exec_1".into(), 1)), Some(&3.0));
    }

    #[test]
    fn test_capacity_column_selection_uses_iterative_active_snapshot() {
        let actions = vec![BasicProductionAction::new(
            "a1", "Action 1", "exec_1", 1.0, 10.0,
        )];
        let mut compilation = IterativeCapacityCompilation::new(actions, vec!["exec_1"], 2);
        let mut model = MetaModel::<f64>::new("test_iterative_capacity_column_selection");
        compilation.register(&mut model).unwrap();
        let added = compilation
            .add_columns(
                0,
                vec![
                    CapacityColumn::new("exec_1", 0, 0, 0.0),
                    CapacityColumn::new("exec_1", 0, 1, 0.0),
                ],
                &mut model,
            )
            .unwrap();

        let before = CapacityColumnSelectionConstraint::from_iterative_compilation(&compilation);
        assert_eq!(before.selection_polynomials[0].2.len(), 2);
        assert!(before.selection_polynomials[1].2.is_empty());

        compilation
            .remove_columns(&[added[0].index], &mut model)
            .unwrap();
        let after = CapacityColumnSelectionConstraint::from_iterative_compilation(&compilation);
        assert_eq!(
            after.selection_polynomials[0].2,
            vec![(added[1].model_index, 1.0)]
        );
        assert!(after.selection_polynomials[1].2.is_empty());
    }

    #[test]
    fn test_order_constraint() {
        let mut model = MetaModel::<f64>::new("test_order_constraint");

        let actions = vec![BasicProductionAction::new(
            "a1", "Action 1", "exec_1", 1.0, 10.0,
        )];
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

        let actions = vec![BasicProductionAction::new(
            "a1", "Action 1", "exec_1", 1.0, 10.0,
        )];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();

        let obj = CapacityCostMinimization::from_compilation(&compilation, 1.0);
        assert!(!obj.cost_terms.is_empty());
        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }
}
