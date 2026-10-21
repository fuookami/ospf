//! 余料与良率上下文 / Waste and yield contexts
//!
//! 余料上下文按「余料面积与余料率」分析图案，并给出余料惩罚目标项；良率上下文按「欠产与超产」
//! 分析需求覆盖，并给出超产上界约束。两者都是主问题的附加口径，不改变图案本身。
//! The waste context analyses patterns by remnant area and ratio and contributes a waste penalty objective;
//! the yield context analyses demand coverage by under- and over-production and contributes an over-production
//! upper bound. Both are additional master-problem conventions and never change the pattern itself.

use crate::domain::material::{CuttingPlan2, Pattern2};

// ============================================================================
// WasteContext - 余料上下文 / Waste context
// ============================================================================

/// 单条余料指标 / One remnant metric
#[derive(Debug, Clone)]
pub struct Csp2dWasteMetric {
    /// 方案编号 / Plan identifier
    pub plan_id: String,
    /// 余料面积 / Remnant area
    pub waste_area: f64,
    /// 余料率 / Remnant ratio
    pub waste_ratio: f64,
}

/// 余料分析结果 / Waste analysis result
#[derive(Debug, Clone, Default)]
pub struct Csp2dWasteResult {
    /// 逐方案指标 / Per-plan metrics
    pub metrics: Vec<Csp2dWasteMetric>,
    /// 总余料面积 / Total remnant area
    pub total_waste_area: f64,
}

impl Csp2dWasteResult {
    /// 指标数量 / Metric count
    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }

    /// 平均余料率 / Average remnant ratio
    pub fn average_waste_ratio(&self) -> f64 {
        if self.metrics.is_empty() {
            return 0.0;
        }
        self.metrics
            .iter()
            .map(|metric| metric.waste_ratio)
            .sum::<f64>()
            / self.metrics.len() as f64
    }
}

/// 余料上下文 / Waste context
#[derive(Debug, Clone, Default)]
pub struct WasteContext;

impl WasteContext {
    /// 创建上下文 / Create the context
    pub fn new() -> Self {
        Self
    }

    /// 分析一组图案 / Analyse a set of patterns
    pub fn analyze(&self, plans: &[CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>]) -> Csp2dWasteResult {
        let mut metrics = Vec::new();
        let mut total = 0.0;
        for plan in plans {
            let waste = plan.waste_area().value;
            let region = plan.consumed_area().value;
            let ratio = if region > 0.0 { waste / region } else { 0.0 };
            total += waste;
            metrics.push(Csp2dWasteMetric {
                plan_id: plan.id.as_str().to_string(),
                waste_area: waste,
                waste_ratio: ratio,
            });
        }
        Csp2dWasteResult {
            metrics,
            total_waste_area: total,
        }
    }

    /// 余料惩罚目标项 / Waste penalty objective terms
    ///
    /// 按余料面积加权，与主问题的面积目标口径一致但可独立配权。
    /// Weighted by remnant area; the convention matches the master area objective while staying independently
    /// configurable.
    pub fn objective_terms(
        &self,
        plans: &[CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>],
        weight: f64,
    ) -> Vec<(usize, f64)> {
        plans
            .iter()
            .enumerate()
            .map(|(index, plan)| (index, plan.waste_area().value * weight))
            .collect()
    }
}

/// 图案的余料率 / The remnant ratio of a pattern
pub fn pattern_waste_ratio(pattern: &Pattern2<f64, ospf_rust_quantities::unit::derived::Meter>) -> f64 {
    let region = pattern.region.area().value;
    if region <= 0.0 {
        return 0.0;
    }
    pattern.waste_area().value / region
}
