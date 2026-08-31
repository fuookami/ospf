//! 长度分配领域 / Length assignment domain

use std::collections::{BTreeMap, BTreeSet};

use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{from_f64, to_f64, Csp1dQuantity, ProductId};

pub mod model;

#[allow(unused_imports)]
pub use model::*;

/// 长度推导策略 / Length derivation strategy
pub trait LengthDerivation<V: SolveValue>: Send + Sync {
    /// 根据需求量和产品推导分配长度 / Derive assigned length from demand quantity and product
    fn derive(
        &self,
        demand_quantity: &Csp1dQuantity<V>,
        product: &crate::domain::material::Product<V>,
    ) -> Option<Csp1dQuantity<V>>;
}

/// 长度分配结果项 / Length assignment entry
#[derive(Debug, Clone)]
pub struct LengthAssignment<V: SolveValue> {
    /// 产品 / Product
    pub product: crate::domain::material::Product<V>,
    /// 分配长度 / Assigned length
    pub assigned_length: Csp1dQuantity<V>,
    /// 批次数 / Batch count
    pub batch_count: u64,
}

/// 超长记录 / Over-length record
#[derive(Debug, Clone)]
pub struct OverLengthRecord<V: SolveValue> {
    /// 产品 / Product
    pub product: crate::domain::material::Product<V>,
    /// 超长量 / Over-length quantity
    pub over_length: Csp1dQuantity<V>,
}

/// 长度分配输入 / Length assignment input
#[derive(Debug, Clone)]
pub struct LengthAssignmentInput<V: SolveValue> {
    /// 动态长度产品列表 / Dynamic-length product list
    pub dynamic_products: Vec<crate::domain::material::Product<V>>,
    /// 需求列表 / Demand list
    pub demands: Vec<crate::domain::material::ProductDemand<V>>,
    /// 约束列表 / Constraint list
    pub constraints: Vec<LengthAssignmentConstraint<V>>,
}

impl<V: SolveValue> Default for LengthAssignmentInput<V> {
    fn default() -> Self {
        Self {
            dynamic_products: Vec::new(),
            demands: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

/// 长度分配约束 / Length assignment constraint
#[derive(Debug, Clone)]
pub enum LengthAssignmentConstraint<V: SolveValue> {
    /// 最大超长约束 / Maximum over-length constraint
    MaxOverLength(crate::domain::material::Product<V>),
    /// 最小批次约束 / Minimum batch count constraint
    MinBatchCount(crate::domain::material::Product<V>, u64),
}

/// 长度分配结果 / Length assignment result
#[derive(Debug, Clone)]
pub struct LengthAssignmentResult<V: SolveValue> {
    /// 分配列表 / Assignment list
    pub assignments: Vec<LengthAssignment<V>>,
    /// 超长记录列表 / Over-length record list
    pub over_length_records: Vec<OverLengthRecord<V>>,
}

/// 长度建模变量聚合 / Length modeling variable aggregation
#[derive(Debug, Clone)]
pub struct LengthSlackAggregation<V: SolveValue> {
    /// 配置 / Configuration
    pub config: LengthAssignmentModelingConfig<V>,
    /// 需求列表 / Demand list
    pub demands: Vec<crate::domain::material::ProductDemand<V>>,
    /// 松弛变量 / Slack variables backed by OptionalIndexedVariableArray
    pub variables: LengthSlackVariables,
}

impl<V: SolveValue> LengthSlackAggregation<V> {
    /// 创建聚合 / Create aggregation
    pub fn new(
        config: LengthAssignmentModelingConfig<V>,
        demands: Vec<crate::domain::material::ProductDemand<V>>,
    ) -> Self {
        Self {
            config,
            demands,
            variables: LengthSlackVariables::new(),
        }
    }

    /// 是否需要分配长度变量 / Whether assigned-length variable is needed
    pub fn needs_assigned_length(
        &self,
        demand: &crate::domain::material::ProductDemand<V>,
    ) -> bool {
        let product_id = &demand.product.id;
        self.config.is_dynamic_product(&demand.product)
            && (self.config.assigned_length_lower_bound.contains_key(product_id)
                || self.config.assigned_length_upper_bound.contains_key(product_id)
                || self.config.total_length_penalty.is_some()
                || self.config.over_length_penalty.contains_key(product_id))
    }

    /// 是否需要超长变量 / Whether over-length variable is needed
    pub fn needs_over_length(
        &self,
        demand: &crate::domain::material::ProductDemand<V>,
    ) -> bool {
        let product_id = &demand.product.id;
        self.config.is_dynamic_product(&demand.product)
            && (self.config.over_length_upper_bound.contains_key(product_id)
                || self.config.over_length_penalty.contains_key(product_id)
                || demand.product.max_over_produce_length.is_some())
    }

    /// 是否存在变量 / Whether any variables exist
    pub fn has_any(&self) -> bool {
        self.variables.has_any()
    }
}

/// 长度分配建模配置 / Length assignment modeling config
#[derive(Debug, Clone)]
pub struct LengthAssignmentModelingConfig<V: SolveValue> {
    /// 是否启用 / Whether enabled
    pub enabled: bool,
    /// 动态长度产品 ID / Dynamic-length product IDs
    pub dynamic_product_ids: BTreeSet<ProductId>,
    /// 分配长度下界 / Assigned length lower bounds
    pub assigned_length_lower_bound: BTreeMap<ProductId, V>,
    /// 分配长度上界 / Assigned length upper bounds
    pub assigned_length_upper_bound: BTreeMap<ProductId, V>,
    /// 超长惩罚 / Over-length penalties
    pub over_length_penalty: BTreeMap<ProductId, V>,
    /// 超长上界 / Over-length upper bounds
    pub over_length_upper_bound: BTreeMap<ProductId, V>,
    /// 总长度惩罚 / Total assigned length penalty
    pub total_length_penalty: Option<V>,
    /// 批次最小化惩罚 / Batch minimization penalty
    pub batch_min_penalty: Option<V>,
    /// 幽灵类型标记 / Phantom type marker
    pub phantom: std::marker::PhantomData<V>,
}

impl<V: SolveValue> Default for LengthAssignmentModelingConfig<V> {
    fn default() -> Self {
        Self {
            enabled: true,
            dynamic_product_ids: BTreeSet::new(),
            assigned_length_lower_bound: BTreeMap::new(),
            assigned_length_upper_bound: BTreeMap::new(),
            over_length_penalty: BTreeMap::new(),
            over_length_upper_bound: BTreeMap::new(),
            total_length_penalty: None,
            batch_min_penalty: None,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<V: SolveValue> LengthAssignmentModelingConfig<V> {
    /// 判断产品是否动态长度 / Check whether product requires dynamic length
    pub fn is_dynamic_product(&self, product: &crate::domain::material::Product<V>) -> bool {
        product.dynamic_length || self.dynamic_product_ids.contains(&product.id)
    }
}

/// 长度分配上下文 / Length assignment context
#[derive(Debug, Clone)]
pub struct LengthAssignmentContext<V: SolveValue, D: LengthDerivation<V>> {
    /// 长度推导策略 / Length derivation strategy
    pub derivation: D,
    /// 幽灵类型标记 / Phantom type marker
    pub phantom: std::marker::PhantomData<V>,
}

impl<V: SolveValue, D: LengthDerivation<V>> LengthAssignmentContext<V, D> {
    /// 执行长度分配 / Perform length assignment
    pub fn assign(&self, input: LengthAssignmentInput<V>) -> LengthAssignmentResult<V> {
        let mut assignments = Vec::new();
        let mut over_length_records = Vec::new();
        for product in input.dynamic_products {
            let Some(demand) = input
                .demands
                .iter()
                .find(|demand| demand.product.id == product.id)
            else {
                continue;
            };
            let Some(assigned_length) = self.derivation.derive(&demand.quantity, &product) else {
                continue;
            };
            assignments.push(LengthAssignment {
                product: product.clone(),
                assigned_length: assigned_length.clone(),
                batch_count: 1,
            });
            if let Some(max_length) = &product.max_over_produce_length {
                let assigned_value = to_f64(&assigned_length.value);
                let max_value = to_f64(&max_length.value);
                if max_length.unit == assigned_length.unit
                    && assigned_value > max_value
                {
                    let Some(value) = assigned_value
                        .zip(max_value)
                        .and_then(|(assigned, max)| from_f64::<V>(assigned - max))
                    else {
                        continue;
                    };
                    over_length_records.push(OverLengthRecord {
                        product,
                        over_length: Csp1dQuantity {
                            value,
                            unit: assigned_length.unit,
                        },
                    });
                }
            }
        }
        LengthAssignmentResult {
            assignments,
            over_length_records,
        }
    }
}

/// 默认长度推导策略 / Default length derivation strategy
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultLengthDerivation;

impl<V: SolveValue> LengthDerivation<V> for DefaultLengthDerivation {
    /// 优先使用产品长度，否则使用需求量 / Prefer product length, fallback to demand quantity
    fn derive(
        &self,
        demand_quantity: &Csp1dQuantity<V>,
        product: &crate::domain::material::Product<V>,
    ) -> Option<Csp1dQuantity<V>> {
        product
            .length
            .clone()
            .or_else(|| Some(demand_quantity.clone()))
    }
}
