//! 影子价格提取逻辑 / Shadow price extraction logic

use std::collections::BTreeMap;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{
    from_f64, shadow_price_key_from_string, Csp1dShadowPriceKey, ShadowPriceMap,
};

use super::pipeline::Csp1dCGPipeline;

/// CSP1D 默认影子价格映射 / CSP1D default shadow price map
#[derive(Debug, Clone, Default)]
pub struct Csp1dDefaultShadowPriceMap {
    /// 对偶值 / Dual values
    pub prices: BTreeMap<Csp1dShadowPriceKey, f64>,
}

impl Csp1dDefaultShadowPriceMap {
    /// 获取影子价格 / Get shadow price
    pub fn get(&self, key: &Csp1dShadowPriceKey) -> Option<f64> {
        self.prices.get(key).copied()
    }

    /// 写入影子价格 / Put shadow price
    pub fn put(&mut self, key: Csp1dShadowPriceKey, value: f64) {
        self.prices.insert(key, value);
    }

    /// 写入或累加 / Put or add
    pub fn put_or_add(&mut self, key: Csp1dShadowPriceKey, value: f64) {
        self.prices
            .entry(key)
            .and_modify(|price| *price += value)
            .or_insert(value);
    }

    /// 转换为领域影子价格映射 / Convert to domain shadow price map
    pub fn to_shadow_price_map<V: SolveValue>(&self) -> ShadowPriceMap<V> {
        self.prices
            .iter()
            .filter_map(|(key, value)| Some((key.clone(), from_f64(*value)?)))
            .collect()
    }
}

/// CSP1D 影子价格生命周期 / CSP1D shadow price lifecycle
#[derive(Clone)]
pub struct Csp1dShadowPriceLifecycle<V: SolveValue> {
    /// 领域数值样本 / Domain value sample
    pub domain_value_sample: V,
    /// 框架兼容影子价格映射 / Framework-compatible shadow price map
    pub framework_shadow_price_map: Csp1dDefaultShadowPriceMap,
    cg_pipelines: Vec<std::sync::Arc<dyn Csp1dCGPipeline<V>>>,
    extractors: Vec<super::pipeline::Csp1dShadowPriceExtractor<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dShadowPriceLifecycle<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dShadowPriceLifecycle")
            .field("domain_value_sample", &self.domain_value_sample)
            .field("framework_shadow_price_map", &self.framework_shadow_price_map)
            .field("cg_pipelines", &self.cg_pipelines.len())
            .field("extractors", &self.extractors.len())
            .finish()
    }
}

impl<V: SolveValue> Csp1dShadowPriceLifecycle<V> {
    /// 创建生命周期 / Create lifecycle
    pub fn new(domain_value_sample: V) -> Self {
        Self {
            domain_value_sample,
            framework_shadow_price_map: Csp1dDefaultShadowPriceMap::default(),
            cg_pipelines: Vec::new(),
            extractors: Vec::new(),
        }
    }

    /// 使用 CG 管线创建生命周期 / Create lifecycle with CG pipelines
    pub fn with_pipelines(
        domain_value_sample: V,
        cg_pipelines: Vec<std::sync::Arc<dyn Csp1dCGPipeline<V>>>,
    ) -> Self {
        Self {
            domain_value_sample,
            framework_shadow_price_map: Csp1dDefaultShadowPriceMap::default(),
            cg_pipelines,
            extractors: Vec::new(),
        }
    }

    /// 从对偶解提取 / Extract from dual solution
    pub fn extract_from_dual_solution(
        &mut self,
        model: &MetaModel<f64>,
        dual_solution: &[f64],
    ) -> ShadowPriceMap<V> {
        self.try_extract_from_dual_solution(model, dual_solution)
            .unwrap_or_else(|error| {
                log::warn!("Failed to extract CSP1D shadow price: {:?}", error);
                self.framework_shadow_price_map.to_shadow_price_map()
            })
    }

    /// 尝试从对偶解提取 / Try to extract from dual solution
    pub fn try_extract_from_dual_solution(
        &mut self,
        model: &MetaModel<f64>,
        dual_solution: &[f64],
    ) -> crate::Csp1dResult<ShadowPriceMap<V>> {
        if !self.cg_pipelines.is_empty() {
            self.extractors.clear();
            for pipeline in &self.cg_pipelines {
                pipeline.refresh_shadow_price(
                    &mut self.framework_shadow_price_map,
                    model,
                    dual_solution,
                )?;
                if let Some(extractor) = pipeline.shadow_price_extractor() {
                    self.extractors.push(extractor);
                }
            }
            return Ok(self.framework_shadow_price_map.to_shadow_price_map());
        }
        for (constraint_index, constraint) in model.constraints().iter().enumerate() {
            let Some(args) = constraint.args.as_deref() else {
                continue;
            };
            let Some(key) = shadow_price_key_from_string(args) else {
                continue;
            };
            let Some(value) = dual_solution.get(constraint_index).copied() else {
                continue;
            };
            self.framework_shadow_price_map.put_or_add(key, value);
        }
        Ok(self.framework_shadow_price_map.to_shadow_price_map())
    }

    /// 计算方案影子价格贡献 / Compute plan shadow price contribution
    pub fn plan_shadow_price(&self, plan: &crate::domain::material::CuttingPlan<V>) -> Option<V> {
        let value = self
            .extractors
            .iter()
            .map(|extractor| extractor(&self.framework_shadow_price_map, plan))
            .sum::<f64>();
        from_f64(value)
    }

    /// 转换对偶值 / Convert dual value
    pub fn convert_dual_value(&self, dual_value: f64) -> Option<V> {
        let _ = &self.domain_value_sample;
        from_f64(dual_value)
    }
}
