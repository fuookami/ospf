//! Yield 约束与目标管线 / Yield constraint and objective pipelines

use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    Csp1dShadowPriceKey, YieldOverProductionBoundShadowPriceKey, shadow_price_key_to_string, to_f64,
};
use crate::domain::wasting_minimization::WasteMinimizationConfig;
use crate::domain::r#yield::YieldSlackAggregation;

use super::super::aggregation::ProduceAggregation;
use super::{Csp1dCGPipeline, Csp1dShadowPriceExtractor, rest_material_value};

/// Yield 建模约束管线 / Yield modeling constraint pipeline
#[derive(Debug, Clone)]
pub struct YieldConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
    r#yield: YieldSlackAggregation<V>,
}

impl<V: SolveValue> YieldConstraintPipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(produce: ProduceAggregation<V>, r#yield: YieldSlackAggregation<V>) -> Self {
        Self {
            name: "yield_constraint".to_string(),
            group: Some(ConstraintGroup::new(10_004, "csp1d_yield_constraint")),
            produce,
            r#yield,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for YieldConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (demand_index, demand) in self.r#yield.demands.iter().enumerate() {
            let demand_key = YieldSlackAggregation::demand_shadow_price_key(demand);
            let mut balance_terms = self
                .produce
                .cutting_plans
                .iter()
                .enumerate()
                .filter_map(|(plan_index, plan)| {
                    if !self.produce.is_plan_active(plan_index) {
                        return None;
                    }
                    let variable = self.produce.plan_variable_index(plan_index)?;
                    let coefficient = plan
                        .demand_contributions
                        .iter()
                        .filter(|contribution| {
                            contribution.product.id == demand.product.id
                                && contribution.quantity.unit == demand.quantity.unit
                        })
                        .filter_map(|contribution| to_f64(&contribution.quantity.value))
                        .sum::<f64>();
                    (coefficient != 0.0).then_some((variable, coefficient))
                })
                .collect::<Vec<_>>();
            if let Some(under_variable) = self.r#yield.variables.under_index(demand_index) {
                balance_terms.push((under_variable, 1.0));
            }
            if let Some(over_variable) = self.r#yield.variables.over_index(demand_index) {
                balance_terms.push((over_variable, -1.0));
            }
            if balance_terms.len() > 1
                || self.r#yield.variables.under_index(demand_index).is_some()
                || self.r#yield.variables.over_index(demand_index).is_some()
            {
                if let Some(rhs) = to_f64(&demand.quantity.value) {
                    if let Err(error) = model.add_linear_constraint_with_metadata(
                        &balance_terms,
                        ConstraintRelation::Equal,
                        rhs,
                        &format!("yield_balance_{demand_index}"),
                        self.group.clone().map(Arc::new),
                        false,
                        0,
                        None,
                    ) {
                        log::warn!(
                            "Failed to register yield balance constraint {}: {:?}",
                            demand_index,
                            error
                        );
                    }
                }
            }
            let Some(upper_bound) = self
                .r#yield
                .config
                .over_production_upper_bound
                .get(&demand_key)
                .and_then(to_f64)
            else {
                continue;
            };
            let Some(over_variable) = self.r#yield.variables.over_index(demand_index) else {
                continue;
            };
            let key = Csp1dShadowPriceKey::YieldOverProductionBound(
                YieldOverProductionBoundShadowPriceKey {
                    product_id: demand_key.product_id,
                    unit_symbol: demand_key.unit_symbol,
                },
            );
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &[(over_variable, 1.0)],
                ConstraintRelation::LessEqual,
                upper_bound,
                &format!("over_production_bound_{demand_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!(
                    "Failed to register over-production bound {}: {:?}",
                    demand_index,
                    error
                );
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for YieldConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        Some(Arc::new(|_, _| 0.0))
    }
}

/// Yield 目标管线 / Yield objective pipeline
#[derive(Debug, Clone)]
pub struct YieldObjectivePipeline<V: SolveValue> {
    name: String,
    r#yield: YieldSlackAggregation<V>,
}

impl<V: SolveValue> YieldObjectivePipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(r#yield: YieldSlackAggregation<V>) -> Self {
        Self {
            name: "yield_objective".to_string(),
            r#yield,
        }
    }

    /// 目标项 / Objective terms
    pub fn objective_terms(&self) -> Vec<(usize, f64)> {
        let mut terms = Vec::new();
        for (demand_index, demand) in self.r#yield.demands.iter().enumerate() {
            let key = YieldSlackAggregation::demand_shadow_price_key(demand);
            if let (Some(variable), Some(penalty)) = (
                self.r#yield.variables.under_index(demand_index),
                self.r#yield
                    .config
                    .under_production_penalty
                    .get(&key)
                    .and_then(to_f64),
            ) {
                terms.push((variable, penalty));
            }
            if let (Some(variable), Some(penalty)) = (
                self.r#yield.variables.over_index(demand_index),
                self.r#yield
                    .config
                    .over_production_penalty
                    .get(&key)
                    .and_then(to_f64),
            ) {
                terms.push((variable, penalty));
            }
        }
        terms
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for YieldObjectivePipeline<V> {
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

/// Waste 目标管线 / Waste objective pipeline
#[derive(Debug, Clone)]
pub struct WasteObjectivePipeline<V: SolveValue> {
    name: String,
    produce: ProduceAggregation<V>,
    config: WasteMinimizationConfig<V>,
    r#yield: Option<YieldSlackAggregation<V>>,
}

impl<V: SolveValue> WasteObjectivePipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(
        produce: ProduceAggregation<V>,
        config: WasteMinimizationConfig<V>,
        r#yield: Option<YieldSlackAggregation<V>>,
    ) -> Self {
        Self {
            name: "waste_objective".to_string(),
            produce,
            config,
            r#yield,
        }
    }

    /// 方案目标项 / Plan objective terms
    pub fn plan_objective_terms<'a, I>(&self, plans: I, offset: usize) -> Vec<(usize, f64)>
    where
        I: IntoIterator<Item = (usize, &'a crate::domain::material::CuttingPlan<V>)>,
        V: 'a,
    {
        plans
            .into_iter()
            .filter_map(|(local_index, plan)| {
                let plan_index = offset + local_index;
                if !self.produce.is_plan_active(plan_index) {
                    return None;
                }
                let variable_index = self.produce.plan_variable_index(plan_index)?;
                let mut coefficient = 0.0;
                if let Some(trim_penalty) = self.config.trim_width_penalty.as_ref().and_then(to_f64)
                {
                    let rest_width = plan
                        .rest_width()
                        .and_then(|width| to_f64(&width.value))
                        .unwrap_or(0.0);
                    coefficient += rest_width * trim_penalty;
                }
                if let Some(rest_penalty) =
                    self.config.rest_material_penalty.as_ref().and_then(to_f64)
                {
                    if let Some(rest_material) =
                        rest_material_value(plan, self.config.rest_material_measure)
                    {
                        coefficient += rest_material * rest_penalty;
                    }
                }
                if let Some(material_penalty) = self
                    .config
                    .material_cost_penalty
                    .get(&plan.material.id)
                    .and_then(to_f64)
                {
                    coefficient += material_penalty;
                }
                (coefficient != 0.0).then_some((variable_index, coefficient))
            })
            .collect()
    }

    /// 超产面积目标项 / Over-production area objective terms
    pub fn over_production_area_objective_terms(&self) -> Vec<(usize, f64)> {
        let Some(area_penalty) = self
            .config
            .over_production_area_penalty
            .as_ref()
            .and_then(to_f64)
        else {
            return Vec::new();
        };
        let Some(r#yield) = &self.r#yield else {
            return Vec::new();
        };
        r#yield
            .demands
            .iter()
            .enumerate()
            .filter_map(|(demand_index, demand)| {
                let variable = r#yield.variables.over_index(demand_index)?;
                let width = demand
                    .product
                    .max_width()
                    .and_then(|width| to_f64(&width.value))?;
                Some((variable, width * area_penalty))
            })
            .collect()
    }

    /// 全量目标项 / Full objective terms
    pub fn objective_terms(&self) -> Vec<(usize, f64)> {
        let mut terms = self.plan_objective_terms(self.produce.cutting_plans.iter().enumerate(), 0);
        terms.extend(self.over_production_area_objective_terms());
        terms
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for WasteObjectivePipeline<V> {
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
