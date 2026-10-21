//! 良率上下文 / Yield context
//!
//! 按「欠产与超产」分析需求覆盖，并给出超产上界约束。
//! Analyses demand coverage by under- and over-production and contributes an over-production upper bound.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;

use crate::domain::material::ProductId2;

/// 单条良率指标 / One yield metric
#[derive(Debug, Clone)]
pub struct Csp2dYieldMetric {
    /// 产品编号 / Product identifier
    pub product_id: ProductId2,
    /// 需求量 / Demanded quantity
    pub demand: f64,
    /// 覆盖量 / Covered quantity
    pub covered: f64,
    /// 欠产量 / Under-production
    pub under_production: f64,
    /// 超产量 / Over-production
    pub over_production: f64,
}

/// 良率分析结果 / Yield analysis result
#[derive(Debug, Clone, Default)]
pub struct Csp2dYieldResult {
    /// 逐产品指标 / Per-product metrics
    pub metrics: Vec<Csp2dYieldMetric>,
    /// 总欠产量 / Total under-production
    pub total_under_production: f64,
    /// 总超产量 / Total over-production
    pub total_over_production: f64,
}

impl Csp2dYieldResult {
    /// 是否全部满足 / Whether everything is satisfied
    pub fn fully_satisfied(&self) -> bool {
        self.total_under_production <= 1e-9
    }

    /// 良率：覆盖量与需求量之比 / Yield: covered over demanded
    pub fn yield_ratio(&self) -> f64 {
        let demanded: f64 = self.metrics.iter().map(|metric| metric.demand).sum();
        if demanded <= 0.0 {
            return 1.0;
        }
        let covered: f64 = self
            .metrics
            .iter()
            .map(|metric| metric.covered.min(metric.demand))
            .sum();
        covered / demanded
    }
}

/// 良率上下文 / Yield context
#[derive(Debug, Clone, Default)]
pub struct YieldContext;

impl YieldContext {
    /// 创建上下文 / Create the context
    pub fn new() -> Self {
        Self
    }

    /// 分析需求覆盖 / Analyse the demand coverage
    ///
    /// `coverage` 与 `demands` 同序：前者是各产品的覆盖量，后者是需求量。
    /// `coverage` and `demands` share the same order: the former is the covered amount per product and the
    /// latter the demanded amount.
    pub fn analyze(
        &self,
        demands: &[(ProductId2, f64)],
        coverage: &[f64],
    ) -> Csp2dYieldResult {
        let mut metrics = Vec::new();
        let mut total_under = 0.0;
        let mut total_over = 0.0;
        for (index, (product_id, demand)) in demands.iter().enumerate() {
            let covered = coverage.get(index).copied().unwrap_or(0.0);
            let under = (demand - covered).max(0.0);
            let over = (covered - demand).max(0.0);
            total_under += under;
            total_over += over;
            metrics.push(Csp2dYieldMetric {
                product_id: product_id.clone(),
                demand: *demand,
                covered,
                under_production: under,
                over_production: over,
            });
        }
        Csp2dYieldResult {
            metrics,
            total_under_production: total_under,
            total_over_production: total_over,
        }
    }

    /// 超产上界约束检查 / Over-production upper bound check
    ///
    /// 任一产品超产超过给定上界即失败；未配置上界的产品不受约束。
    /// Fails when any product's over-production exceeds its bound; products without a bound are unconstrained.
    pub fn check_over_production_bounds(
        &self,
        result: &Csp2dYieldResult,
        bounds: &BTreeMap<ProductId2, f64>,
    ) -> Ret<()> {
        for metric in &result.metrics {
            if let Some(bound) = bounds.get(&metric.product_id)
                && metric.over_production > *bound + 1e-9
            {
                return Err(crate::infrastructure::error::validation_error(format!(
                    "产品 {} 的超产量 {} 超过上界 {} / Product {} over-production {} exceeds the bound {}",
                    metric.product_id,
                    metric.over_production,
                    bound,
                    metric.product_id,
                    metric.over_production,
                    bound
                )));
            }
        }
        Ok(())
    }
}
