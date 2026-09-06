//! Yield 领域 / Yield domain

use std::collections::BTreeMap;

use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{
    Csp1dQuantity, Product, ProductDemand, ProductDemandShadowPriceKey, ProductId,
    shadow_price_unit_symbol,
};

pub mod model;

#[allow(unused_imports)]
pub use model::*;

/// 需求聚合键 / Demand aggregation key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DemandAggregationKey {
    /// 产品 ID / Product id
    pub product_id: ProductId,
    /// 单位符号 / Unit symbol
    pub unit_symbol: String,
}

/// 欠产记录 / Under-production record
#[derive(Debug, Clone)]
pub struct ModeledUnderProduction<V: SolveValue> {
    /// 对应需求 / Corresponding demand
    pub demand: ProductDemand<V>,
    /// 欠产缺口量 / Shortfall quantity
    pub shortfall: Csp1dQuantity<V>,
}

/// 超产记录 / Over-production record
#[derive(Debug, Clone)]
pub struct ModeledOverProduction<V: SolveValue> {
    /// 对应需求 / Corresponding demand
    pub demand: ProductDemand<V>,
    /// 超产盈余量 / Surplus quantity
    pub surplus: Csp1dQuantity<V>,
}

/// 产品产出统计 / Product output statistics
#[derive(Debug, Clone)]
pub struct ProductOutput<V: SolveValue> {
    /// 产品 / Product
    pub product: Product<V>,
    /// 总产出量 / Total output quantity
    pub total_quantity: Csp1dQuantity<V>,
    /// 需求模式 / Demand mode
    pub mode: Option<crate::domain::material::DemandMode>,
}

/// 产出分析结果 / Yield analysis result
#[derive(Debug, Clone)]
pub struct YieldAnalysis<V: SolveValue> {
    /// 欠产列表 / Under-production list
    pub under_productions: Vec<ModeledUnderProduction<V>>,
    /// 超产列表 / Over-production list
    pub over_productions: Vec<ModeledOverProduction<V>>,
    /// 产出列表 / Output list
    pub outputs: Vec<ProductOutput<V>>,
}

/// Yield 建模配置 / Yield modeling configuration
#[derive(Debug, Clone, Default)]
pub struct YieldModelingConfig<V: SolveValue> {
    /// 欠产惩罚权重 / Under-production penalty weights
    pub under_production_penalty: BTreeMap<crate::domain::material::ProductDemandShadowPriceKey, V>,
    /// 超产惩罚权重 / Over-production penalty weights
    pub over_production_penalty: BTreeMap<crate::domain::material::ProductDemandShadowPriceKey, V>,
    /// 超产上界 / Over-production upper bounds
    pub over_production_upper_bound:
        BTreeMap<crate::domain::material::ProductDemandShadowPriceKey, V>,
}

/// Yield 建模结果 / Yield modeling result
#[derive(Debug, Clone)]
pub struct YieldModelingResult<V: SolveValue> {
    /// 产出分析 / Yield analysis
    pub analysis: YieldAnalysis<V>,
}

/// Yield 上下文 / Yield context
#[derive(Debug, Clone)]
pub struct YieldContext<V: SolveValue> {
    /// 建模配置 / Modeling configuration
    pub config: YieldModelingConfig<V>,
}

/// Yield 聚合 / Yield aggregation
#[derive(Debug, Clone, Default)]
pub struct YieldAggregation<V: SolveValue> {
    /// 产出列表 / Output list
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
