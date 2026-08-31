//! 长度约束与目标管线 / Length constraint and objective pipelines

use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::to_f64;
use crate::domain::length_assignment::LengthSlackAggregation;

/// 长度建模约束管线 / Length modeling constraint pipeline
#[derive(Debug, Clone)]
pub struct LengthConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    length: LengthSlackAggregation<V>,
}

impl<V: SolveValue> LengthConstraintPipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(length: LengthSlackAggregation<V>) -> Self {
        Self {
            name: "length_constraint".to_string(),
            group: Some(ConstraintGroup::new(
                10_005,
                "csp1d_length_constraint",
            )),
            length,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for LengthConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (demand_index, demand) in self.length.demands.iter().enumerate() {
            let product_id = &demand.product.id;
            if !self.length.config.is_dynamic_product(&demand.product) {
                continue;
            }
            let assigned_variable = self.length.variables.assigned_index(demand_index);
            let over_variable = self.length.variables.over_index(demand_index);
            if let (Some(variable), Some(bound)) = (
                assigned_variable,
                self.length
                    .config
                    .assigned_length_lower_bound
                    .get(product_id)
                    .and_then(to_f64),
            ) {
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &[(variable, 1.0)],
                    ConstraintRelation::GreaterEqual,
                    bound,
                    &format!("assigned_length_lower_bound_{demand_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    None,
                ) {
                    log::warn!("Failed to register assigned length lower bound {}: {:?}", demand_index, error);
                }
            }
            if let (Some(variable), Some(bound)) = (
                assigned_variable,
                self.length
                    .config
                    .assigned_length_upper_bound
                    .get(product_id)
                    .and_then(to_f64),
            ) {
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &[(variable, 1.0)],
                    ConstraintRelation::LessEqual,
                    bound,
                    &format!("assigned_length_upper_bound_{demand_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    None,
                ) {
                    log::warn!("Failed to register assigned length upper bound {}: {:?}", demand_index, error);
                }
            }
            if let (Some(variable), Some(bound)) = (
                over_variable,
                self.length
                    .config
                    .over_length_upper_bound
                    .get(product_id)
                    .and_then(to_f64),
            ) {
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &[(variable, 1.0)],
                    ConstraintRelation::LessEqual,
                    bound,
                    &format!("over_length_bound_{demand_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    None,
                ) {
                    log::warn!("Failed to register over length bound {}: {:?}", demand_index, error);
                }
            }
            let Some(max_over_length) = demand
                .product
                .max_over_produce_length
                .as_ref()
                .and_then(|quantity| to_f64(&quantity.value))
            else {
                continue;
            };
            let (Some(assigned_variable), Some(over_variable)) = (assigned_variable, over_variable) else {
                continue;
            };
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &[(assigned_variable, 1.0), (over_variable, -1.0)],
                ConstraintRelation::LessEqual,
                max_over_length,
                &format!("assigned_over_length_link_{demand_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                None,
            ) {
                log::warn!("Failed to register assigned-over length link {}: {:?}", demand_index, error);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

/// 长度目标管线 / Length objective pipeline
#[derive(Debug, Clone)]
pub struct LengthObjectivePipeline<V: SolveValue> {
    name: String,
    length: LengthSlackAggregation<V>,
}

impl<V: SolveValue> LengthObjectivePipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(length: LengthSlackAggregation<V>) -> Self {
        Self {
            name: "length_objective".to_string(),
            length,
        }
    }

    /// 批次系数 / Batch coefficient
    pub fn batch_coefficient(&self) -> f64 {
        self.length
            .config
            .batch_min_penalty
            .as_ref()
            .and_then(to_f64)
            .map(|penalty| 1.0 + penalty)
            .unwrap_or(1.0)
    }

    /// 目标项 / Objective terms
    pub fn objective_terms(&self) -> Vec<(usize, f64)> {
        let mut terms = Vec::new();
        if let Some(penalty) = self.length.config.total_length_penalty.as_ref().and_then(to_f64) {
            for variable in self.length.variables.assigned_length().iter().filter_map(|value| *value) {
                terms.push((variable, penalty));
            }
        }
        for (demand_index, demand) in self.length.demands.iter().enumerate() {
            let Some(penalty) = self
                .length
                .config
                .over_length_penalty
                .get(&demand.product.id)
                .and_then(to_f64)
            else {
                continue;
            };
            let Some(variable) = self.length.variables.over_index(demand_index) else {
                continue;
            };
            terms.push((variable, penalty));
        }
        terms
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for LengthObjectivePipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}
