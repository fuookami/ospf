//! Yield 领域 / Yield domain

use std::collections::BTreeMap;

use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{
    shadow_price_unit_symbol, Csp1dQuantity, Product, ProductDemand,
    ProductDemandShadowPriceKey,
};

pub mod model;

#[allow(unused_imports)]
pub use model::*;

/// 需求聚合键 / Demand aggregation key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DemandAggregationKey {
    pub product_id: String,
    pub unit_symbol: String,
}

/// 欠产 / Under-production
#[derive(Debug, Clone)]
pub struct ModeledUnderProduction<V: SolveValue> {
    pub demand: ProductDemand<V>,
    pub shortfall: Csp1dQuantity<V>,
}

/// 超产 / Over-production
#[derive(Debug, Clone)]
pub struct ModeledOverProduction<V: SolveValue> {
    pub demand: ProductDemand<V>,
    pub surplus: Csp1dQuantity<V>,
}

/// 产品产出 / Product output
#[derive(Debug, Clone)]
pub struct ProductOutput<V: SolveValue> {
    pub product: Product<V>,
    pub total_quantity: Csp1dQuantity<V>,
    pub mode: Option<crate::domain::material::DemandMode>,
}

/// 产出分析 / Yield analysis
#[derive(Debug, Clone)]
pub struct YieldAnalysis<V: SolveValue> {
    pub under_productions: Vec<ModeledUnderProduction<V>>,
    pub over_productions: Vec<ModeledOverProduction<V>>,
    pub outputs: Vec<ProductOutput<V>>,
}

/// Yield 配置 / Yield modeling config
#[derive(Debug, Clone, Default)]
pub struct YieldModelingConfig<V: SolveValue> {
    pub under_production_penalty: BTreeMap<crate::domain::material::ProductDemandShadowPriceKey, V>,
    pub over_production_penalty: BTreeMap<crate::domain::material::ProductDemandShadowPriceKey, V>,
    pub over_production_upper_bound: BTreeMap<crate::domain::material::ProductDemandShadowPriceKey, V>,
}

/// Yield 结果 / Yield modeling result
#[derive(Debug, Clone)]
pub struct YieldModelingResult<V: SolveValue> {
    pub analysis: YieldAnalysis<V>,
}

/// Yield 上下文 / Yield context
#[derive(Debug, Clone)]
pub struct YieldContext<V: SolveValue> {
    pub config: YieldModelingConfig<V>,
}

/// Yield 聚合 / Yield aggregation
#[derive(Debug, Clone, Default)]
pub struct YieldAggregation<V: SolveValue> {
    pub outputs: Vec<ProductOutput<V>>,
}

/// Yield 建模变量聚合 / Yield modeling variable aggregation
#[derive(Debug, Clone)]
pub struct YieldSlackAggregation<V: SolveValue> {
    /// 配置 / Configuration
    pub config: YieldModelingConfig<V>,
    /// 需求列表 / Demand list
    pub demands: Vec<ProductDemand<V>>,
    /// 松弛变量 / Slack variables backed by OptionalIndexedVariableArray
    pub variables: YieldSlackVariables,
    /// 超产面积是否需要超产变量 / Whether over-area objective needs over slack
    pub needs_over_slack_for_over_area: bool,
}

impl<V: SolveValue> YieldSlackAggregation<V> {
    /// 创建聚合 / Create aggregation
    pub fn new(
        config: YieldModelingConfig<V>,
        demands: Vec<ProductDemand<V>>,
        needs_over_slack_for_over_area: bool,
    ) -> Self {
        Self {
            config,
            demands,
            variables: YieldSlackVariables::new(),
            needs_over_slack_for_over_area,
        }
    }

    /// 是否需要欠产变量 / Whether under-production variable is needed
    pub fn needs_under_production(&self, demand: &ProductDemand<V>) -> bool {
        self.config
            .under_production_penalty
            .contains_key(&Self::demand_shadow_price_key(demand))
    }

    /// 是否需要超产变量 / Whether over-production variable is needed
    pub fn needs_over_production(&self, demand: &ProductDemand<V>) -> bool {
        let key = Self::demand_shadow_price_key(demand);
        self.config.over_production_penalty.contains_key(&key)
            || self.config.over_production_upper_bound.contains_key(&key)
            || self.needs_over_slack_for_over_area
    }

    /// 是否存在变量 / Whether any variables exist
    pub fn has_any(&self) -> bool {
        self.variables.has_any()
    }

    /// 需求 key / Demand key
    pub fn demand_shadow_price_key(demand: &ProductDemand<V>) -> ProductDemandShadowPriceKey {
        ProductDemandShadowPriceKey {
            product_id: demand.product.id.clone(),
            unit_symbol: shadow_price_unit_symbol(&demand.quantity.unit),
        }
    }
}
