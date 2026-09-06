//! 产出与消耗限制与目标族 / Produce and consumption limits and objective families
//!
//! 实现产出/消耗约束和松弛量最小化 Pipeline。
//! Implements produce/consumption constraint and slack minimization pipelines.

use ospf_rust_core::error::Result;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::symbol::LinearIntermediateSymbol;
use ospf_rust_framework::model::pipeline::Pipeline;

use crate::domain::produce::model::demand::{MaterialDemand, MaterialReserves};
use crate::domain::produce::model::usage::{ConsumptionUsage, ProduceUsage};

// ============================================================================
// 约束型 Pipeline / Constraint Pipelines
// ============================================================================

/// 产出量约束 / Produce quantity constraint
///
/// 对每个产品添加产出量上下界约束：
/// - 当 over slack 未启用时：`quantity[product] <= upper_bound`
/// - 当 less slack 未启用时：`quantity[product] >= lower_bound`
///
/// Adds produce quantity upper/lower bound constraints for each product.
/// When slack variables are enabled, capacity constraints are automatically enforced by SlackFunction.
#[derive(Debug)]
pub struct ProduceQuantityConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 产出使用量名称 / Produce usage name
    pub usage_name: String,
    /// 产品需求数量 / Product demands
    pub demands: Vec<MaterialDemand>,
    /// 是否全局启用 over slack / Whether over slack is globally enabled
    pub over_enabled: bool,
    /// 是否全局启用 less slack / Whether less slack is globally enabled
    pub less_enabled: bool,
}

impl ProduceQuantityConstraint {
    /// 从 ProduceUsage 创建产出量约束 / Create produce quantity constraint from ProduceUsage
    pub fn new(usage: &ProduceUsage, demands: Vec<MaterialDemand>) -> Self {
        Self {
            name: format!("{}_quantity_constraint", usage.name),
            group: None,
            usage_name: usage.name.clone(),
            demands,
            over_enabled: usage.over_enabled,
            less_enabled: usage.less_enabled,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ProduceQuantityConstraint {
    fn name(&self) -> &str {
        &self.name
    }
    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (product_idx, demand) in self.demands.iter().enumerate() {
            // 当 over slack 未启用时，添加上界约束
            if (!self.over_enabled || !demand.over_enabled())
                && let Err(e) = model.add_le_constraint(
                    &[],
                    demand.upper_bound,
                    &format!("{}_ub_{}", self.name, product_idx),
                )
            {
                log::warn!(
                    "Failed to register {}_ub_{}: {:?}",
                    self.name,
                    product_idx,
                    e
                );
            }

            // 当 less slack 未启用时，添加下界约束
            if (!self.less_enabled || !demand.less_enabled())
                && let Err(e) = model.add_ge_constraint(
                    &[],
                    demand.lower_bound,
                    &format!("{}_lb_{}", self.name, product_idx),
                )
            {
                log::warn!(
                    "Failed to register {}_lb_{}: {:?}",
                    self.name,
                    product_idx,
                    e
                );
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 消耗量约束 / Consumption quantity constraint
///
/// 对每个物料添加消耗量上下界约束。
/// Adds consumption quantity upper/lower bound constraints for each material.
#[derive(Debug)]
pub struct ConsumptionQuantityConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 消耗使用量名称 / Consumption usage name
    pub usage_name: String,
    /// 物料储备数量 / Material reserves
    pub reserves: Vec<MaterialReserves>,
    /// 是否全局启用 over slack / Whether over slack is globally enabled
    pub over_enabled: bool,
    /// 是否全局启用 less slack / Whether less slack is globally enabled
    pub less_enabled: bool,
}

impl ConsumptionQuantityConstraint {
    /// 从 ConsumptionUsage 创建消耗量约束 / Create consumption quantity constraint from ConsumptionUsage
    pub fn new(usage: &ConsumptionUsage, reserves: Vec<MaterialReserves>) -> Self {
        Self {
            name: format!("{}_quantity_constraint", usage.name),
            group: None,
            usage_name: usage.name.clone(),
            reserves,
            over_enabled: usage.over_enabled,
            less_enabled: usage.less_enabled,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ConsumptionQuantityConstraint {
    fn name(&self) -> &str {
        &self.name
    }
    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (material_idx, reserve) in self.reserves.iter().enumerate() {
            if (!self.over_enabled || !reserve.over_enabled())
                && let Err(e) = model.add_le_constraint(
                    &[],
                    reserve.upper_bound,
                    &format!("{}_ub_{}", self.name, material_idx),
                )
            {
                log::warn!(
                    "Failed to register {}_ub_{}: {:?}",
                    self.name,
                    material_idx,
                    e
                );
            }

            if (!self.less_enabled || !reserve.less_enabled())
                && let Err(e) = model.add_ge_constraint(
                    &[],
                    reserve.lower_bound,
                    &format!("{}_lb_{}", self.name, material_idx),
                )
            {
                log::warn!(
                    "Failed to register {}_lb_{}: {:?}",
                    self.name,
                    material_idx,
                    e
                );
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

/// 产出过量最小化 / Produce over quantity minimization
///
/// 最小化 `sum(coefficient * over_quantity[product])`。
/// Minimizes total over-quantity penalty for produce.
#[derive(Debug)]
pub struct ProduceOverQuantityMinimization {
    name: String,
    /// (over_quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ProduceOverQuantityMinimization {
    /// 从 ProduceUsage 创建产出过量最小化 / Create from ProduceUsage
    pub fn from_usage(usage: &ProduceUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .over_quantity_indices
            .iter()
            .filter_map(|idx| idx.map(|i| (i, coefficient)))
            .collect();
        Self {
            name: format!("{}_over_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ProduceOverQuantityMinimization {
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

/// 产出不足最小化 / Produce less quantity minimization
#[derive(Debug)]
pub struct ProduceLessQuantityMinimization {
    name: String,
    /// (less_quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ProduceLessQuantityMinimization {
    /// 从 ProduceUsage 创建产出不足最小化 / Create from ProduceUsage
    pub fn from_usage(usage: &ProduceUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .less_quantity_indices
            .iter()
            .filter_map(|idx| idx.map(|i| (i, coefficient)))
            .collect();
        Self {
            name: format!("{}_less_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ProduceLessQuantityMinimization {
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

/// 产出量最大化 / Produce quantity maximization
///
/// 最大化 `sum(coefficient * quantity[product])`。
/// Maximizes total produce quantity.
#[derive(Debug)]
pub struct ProduceQuantityMaximization {
    name: String,
    /// (quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ProduceQuantityMaximization {
    /// 从 ProduceUsage 创建产出量最大化 / Create from ProduceUsage
    ///
    /// 产出量最大化通过 quantity 中间表达式的变量项实现。
    /// 当中间表达式有 monomials 时，使用其变量索引；
    /// 否则回退到 over_quantity slack 变量索引。
    ///
    /// Produce quantity maximization uses variable terms from the quantity intermediate expression.
    /// When the expression has monomials, their variable indices are used;
    /// otherwise, falls back to over_quantity slack variable indices.
    pub fn from_usage(usage: &ProduceUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .quantity_symbols
            .iter()
            .enumerate()
            .filter_map(|(idx, sym)| {
                let poly = sym.as_ref().to_linear_polynomial();
                if let Some(first) = poly.monomials().first() {
                    Some((first.var_index(), coefficient))
                } else if idx < usage.over_quantity_indices.len() {
                    usage.over_quantity_indices[idx].map(|i| (i, coefficient))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: format!("{}_quantity_maximization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ProduceQuantityMaximization {
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
        let sub_obj = SubObjective::maximize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 产出量最小化 / Produce quantity minimization
#[derive(Debug)]
pub struct ProduceQuantityMinimization {
    name: String,
    /// (quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ProduceQuantityMinimization {
    /// 从 ProduceUsage 创建产出量最小化 / Create from ProduceUsage
    ///
    /// 产出量最小化通过 quantity 中间表达式的变量项实现。
    /// 当中间表达式有 monomials 时，使用其变量索引；
    /// 否则回退到 less_quantity slack 变量索引。
    pub fn from_usage(usage: &ProduceUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .quantity_symbols
            .iter()
            .enumerate()
            .filter_map(|(idx, sym)| {
                let poly = sym.as_ref().to_linear_polynomial();
                if let Some(first) = poly.monomials().first() {
                    Some((first.var_index(), coefficient))
                } else if idx < usage.less_quantity_indices.len() {
                    usage.less_quantity_indices[idx].map(|i| (i, coefficient))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: format!("{}_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ProduceQuantityMinimization {
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

/// 消耗过量最小化 / Consumption over quantity minimization
#[derive(Debug)]
pub struct ConsumptionOverQuantityMinimization {
    name: String,
    /// (over_quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ConsumptionOverQuantityMinimization {
    /// 从 ConsumptionUsage 创建消耗过量最小化 / Create from ConsumptionUsage
    pub fn from_usage(usage: &ConsumptionUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .over_quantity_indices
            .iter()
            .filter_map(|idx| idx.map(|i| (i, coefficient)))
            .collect();
        Self {
            name: format!("{}_over_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ConsumptionOverQuantityMinimization {
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

/// 消耗不足最小化 / Consumption less quantity minimization
#[derive(Debug)]
pub struct ConsumptionLessQuantityMinimization {
    name: String,
    /// (less_quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ConsumptionLessQuantityMinimization {
    /// 从 ConsumptionUsage 创建消耗不足最小化 / Create from ConsumptionUsage
    pub fn from_usage(usage: &ConsumptionUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .less_quantity_indices
            .iter()
            .filter_map(|idx| idx.map(|i| (i, coefficient)))
            .collect();
        Self {
            name: format!("{}_less_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ConsumptionLessQuantityMinimization {
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

/// 消耗量最大化 / Consumption quantity maximization
#[derive(Debug)]
pub struct ConsumptionQuantityMaximization {
    name: String,
    /// (quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ConsumptionQuantityMaximization {
    /// 从 ConsumptionUsage 创建消耗量最大化 / Create from ConsumptionUsage
    ///
    /// 消耗量最大化通过 quantity 中间表达式的变量项实现。
    /// 当中间表达式有 monomials 时，使用其变量索引；
    /// 否则回退到 over_quantity slack 变量索引。
    pub fn from_usage(usage: &ConsumptionUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .quantity_symbols
            .iter()
            .enumerate()
            .filter_map(|(idx, sym)| {
                let poly = sym.as_ref().to_linear_polynomial();
                if let Some(first) = poly.monomials().first() {
                    Some((first.var_index(), coefficient))
                } else if idx < usage.over_quantity_indices.len() {
                    usage.over_quantity_indices[idx].map(|i| (i, coefficient))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: format!("{}_quantity_maximization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ConsumptionQuantityMaximization {
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
        let sub_obj = SubObjective::maximize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 消耗量最小化 / Consumption quantity minimization
#[derive(Debug)]
pub struct ConsumptionQuantityMinimization {
    name: String,
    /// (quantity_solver_index, coefficient) 列表 / List
    pub cost_terms: Vec<(usize, f64)>,
}

impl ConsumptionQuantityMinimization {
    /// 从 ConsumptionUsage 创建消耗量最小化 / Create from ConsumptionUsage
    ///
    /// 消耗量最小化通过 quantity 中间表达式的变量项实现。
    /// 当中间表达式有 monomials 时，使用其变量索引；
    /// 否则回退到 less_quantity slack 变量索引。
    pub fn from_usage(usage: &ConsumptionUsage, coefficient: f64) -> Self {
        let cost_terms: Vec<(usize, f64)> = usage
            .quantity_symbols
            .iter()
            .enumerate()
            .filter_map(|(idx, sym)| {
                let poly = sym.as_ref().to_linear_polynomial();
                if let Some(first) = poly.monomials().first() {
                    Some((first.var_index(), coefficient))
                } else if idx < usage.less_quantity_indices.len() {
                    usage.less_quantity_indices[idx].map(|i| (i, coefficient))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: format!("{}_quantity_minimization", usage.name),
            cost_terms,
        }
    }
}

impl Pipeline<MetaModel<f64>> for ConsumptionQuantityMinimization {
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

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_produce_over_quantity_minimization() {
        let mut model = MetaModel::<f64>::new("test_produce_over_obj");

        let demands = vec![MaterialDemand::with_slack(
            "product_a",
            10.0,
            100.0,
            Some(5.0),
            Some(20.0),
        )];

        let mut usage = ProduceUsage::new("produce", 1, true, false);
        usage.register(&demands, &mut model).unwrap();

        let obj = ProduceOverQuantityMinimization::from_usage(&usage, 1.0);
        assert_eq!(obj.cost_terms.len(), 1);
        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn test_consumption_less_quantity_minimization() {
        let mut model = MetaModel::<f64>::new("test_consumption_less_obj");

        let reserves = vec![MaterialReserves::with_slack(
            "raw_1",
            0.0,
            200.0,
            Some(10.0),
            None,
        )];

        let mut usage = ConsumptionUsage::new("consumption", 1, false, true);
        usage.register(&reserves, &mut model).unwrap();

        let obj = ConsumptionLessQuantityMinimization::from_usage(&usage, 2.0);
        assert_eq!(obj.cost_terms.len(), 1);
        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn test_produce_quantity_constraint() {
        let mut model = MetaModel::<f64>::new("test_produce_constraint");

        let demands = vec![
            MaterialDemand::new("product_a", 10.0, 100.0),
            MaterialDemand::with_slack("product_b", 5.0, 50.0, Some(2.0), Some(10.0)),
        ];

        let mut usage = ProduceUsage::new("produce", 2, true, true);
        usage.register(&demands, &mut model).unwrap();

        let constraint = ProduceQuantityConstraint::new(&usage, demands);
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_consumption_quantity_constraint() {
        let mut model = MetaModel::<f64>::new("test_consumption_constraint");

        let reserves = vec![
            MaterialReserves::new("raw_1", 0.0, 100.0),
            MaterialReserves::with_slack("raw_2", 10.0, 200.0, Some(5.0), None),
        ];

        let mut usage = ConsumptionUsage::new("consumption", 2, true, true);
        usage.register(&reserves, &mut model).unwrap();

        let constraint = ConsumptionQuantityConstraint::new(&usage, reserves);
        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_produce_less_quantity_minimization() {
        let mut model = MetaModel::<f64>::new("test_produce_less_obj");

        let demands = vec![MaterialDemand::with_slack(
            "product_a",
            10.0,
            100.0,
            Some(5.0),
            Some(20.0),
        )];

        let mut usage = ProduceUsage::new("produce", 1, true, true);
        usage.register(&demands, &mut model).unwrap();

        let obj = ProduceLessQuantityMinimization::from_usage(&usage, 1.0);
        assert_eq!(obj.cost_terms.len(), 1);
        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn test_consumption_over_quantity_minimization() {
        let mut model = MetaModel::<f64>::new("test_consumption_over_obj");

        let reserves = vec![MaterialReserves::with_slack(
            "raw_1",
            0.0,
            200.0,
            None,
            Some(30.0),
        )];

        let mut usage = ConsumptionUsage::new("consumption", 1, true, false);
        usage.register(&reserves, &mut model).unwrap();

        let obj = ConsumptionOverQuantityMinimization::from_usage(&usage, 1.5);
        assert_eq!(obj.cost_terms.len(), 1);
        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }
}
