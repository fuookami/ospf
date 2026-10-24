//! 检验数定价 / Reduced-cost pricing
//!
//! 列生成的定价判据：新图案的检验数必须为负才可能改进主问题。
//! The pricing criterion of column generation: a new pattern must have a negative reduced cost to possibly
//! improve the master problem.
//!
//! 检验数定义为
//! `rc = 消耗量 · 单价 − Σ_i 覆盖系数_i · π_i`
//! 其中 `π_i` 是需求约束的对偶（影子价格）。二维的「消耗量」是料域面积（定尺）或段高（卷材），
//! 「覆盖系数」是图案里该产品的件数（件数口径）或件数乘单件面积（面积口径）。
//! The reduced cost is `rc = consumption · unit price − Σ_i coefficient_i · π_i`, where `π_i` are the demand
//! constraint duals. In 2D the "consumption" is the region area (sheet) or the segment height (coil), and the
//! "coefficient" is the piece count of that product (amount mode) or the count times the unit area (area mode).
//!
//! 二维与一维的关键差异：一维按切片宽度累加，二维按图案使用次数累加，因此检验数必须先按「一次使用」
//! 归一，再与需求影子价格比较。
//! The key 2D difference: 1D accumulates slice widths while 2D accumulates pattern usage counts, so a reduced
//! cost must first be normalised to "one use" before comparing against the demand shadow prices.

use std::collections::BTreeMap;

use crate::domain::material::{CuttingPlan2, ProductId2};
use crate::domain::produce::{Csp2dMasterDemand, Csp2dPatternAssignment};

/// 默认检验数容差 / Default reduced-cost tolerance
///
/// 只有 `rc < -tolerance` 才算改进列；容差用于吸收浮点噪声，避免把「恰好为零」当成改进。
/// Only `rc < -tolerance` counts as an improving column; the tolerance absorbs floating-point noise so an exact
/// zero is never mistaken for an improvement.
pub const DEFAULT_REDUCED_COST_TOLERANCE: f64 = 1e-9;

/// 定价配置 / Pricing configuration
#[derive(Debug, Clone)]
pub struct Csp2dPricingConfig {
    /// 消耗量单价（面积或段高的单位成本）/ Unit price of consumption (area or segment height)
    pub consumption_price: f64,
    /// 检验数容差 / Reduced-cost tolerance
    pub tolerance: f64,
    /// 是否按段高（卷材）而非面积计价 / Whether to price by segment height (coil) instead of area
    pub price_by_height: bool,
}

impl Default for Csp2dPricingConfig {
    fn default() -> Self {
        Self {
            consumption_price: 1.0,
            tolerance: DEFAULT_REDUCED_COST_TOLERANCE,
            price_by_height: false,
        }
    }
}

/// 单列检验数结果 / Reduced cost of one column
#[derive(Debug, Clone)]
pub struct Csp2dReducedCost {
    /// 方案下标 / Plan index
    pub plan_index: usize,
    /// 消耗量 / Consumption
    pub consumption: f64,
    /// 覆盖收益 `Σ coefficient · π` / Coverage revenue
    pub revenue: f64,
    /// 检验数 / Reduced cost
    pub reduced_cost: f64,
}

impl Csp2dReducedCost {
    /// 是否为改进列 / Whether it is an improving column
    pub fn is_improving(&self, tolerance: f64) -> bool {
        self.reduced_cost < -tolerance
    }
}

/// 需求影子价格表 / Demand shadow price map
///
/// 按产品编号索引；面积口径的需求与件数口径的需求共用同一张表，因为覆盖系数已经把口径差异吸收掉了。
/// Indexed by product identifier; area-mode and amount-mode demands share one table because the coverage
/// coefficient already absorbs the convention difference.
pub type Csp2dShadowPrices = BTreeMap<ProductId2, f64>;

/// 计算单列检验数 / Compute the reduced cost of one column
pub fn reduced_cost_of(
    assignment: &Csp2dPatternAssignment,
    plan_index: usize,
    demands: &[Csp2dMasterDemand],
    shadow_prices: &Csp2dShadowPrices,
    config: &Csp2dPricingConfig,
) -> Csp2dReducedCost {
    let consumption = if config.price_by_height {
        assignment
            .consumed_height(plan_index)
            .unwrap_or_else(|| assignment.consumed_area(plan_index))
    } else {
        assignment.consumed_area(plan_index)
    };
    let mut revenue = 0.0;
    for demand in demands {
        let Some(price) = shadow_prices.get(&demand.product_id) else {
            continue;
        };
        revenue += assignment.coverage_coefficient(plan_index, demand) * price;
    }
    Csp2dReducedCost {
        plan_index,
        consumption,
        revenue,
        reduced_cost: consumption * config.consumption_price - revenue,
    }
}

/// 计算全部列的检验数 / Compute the reduced cost of every column
pub fn reduced_costs(
    assignment: &Csp2dPatternAssignment,
    demands: &[Csp2dMasterDemand],
    shadow_prices: &Csp2dShadowPrices,
    config: &Csp2dPricingConfig,
) -> Vec<Csp2dReducedCost> {
    (0..assignment.len())
        .map(|index| reduced_cost_of(assignment, index, demands, shadow_prices, config))
        .collect()
}

/// 筛出改进列，按检验数升序（最负的在前）/ Keep improving columns sorted by ascending reduced cost
pub fn improving_columns(
    assignment: &Csp2dPatternAssignment,
    demands: &[Csp2dMasterDemand],
    shadow_prices: &Csp2dShadowPrices,
    config: &Csp2dPricingConfig,
) -> Vec<Csp2dReducedCost> {
    let mut improving: Vec<Csp2dReducedCost> = reduced_costs(assignment, demands, shadow_prices, config)
        .into_iter()
        .filter(|entry| entry.is_improving(config.tolerance))
        .collect();
    improving.sort_by(|lhs, rhs| {
        lhs.reduced_cost
            .partial_cmp(&rhs.reduced_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    improving
}

/// 按图案覆盖量与消耗量估算检验数 / Estimate a reduced cost from coverage and consumption
///
/// 用于**尚未加入主问题**的新图案：它没有列下标，只能直接用图案自身的件数与面积估算。
/// 这是定价阶段对候选图案排序的正确口径——先估检验数，再决定是否加入。
/// Used for a pattern that is **not yet in the master**: it has no column index, so the estimate comes directly
/// from the pattern's own piece counts and area. This is the right criterion for ordering candidate patterns
/// during pricing, deciding which ones are worth adding.
pub fn estimate_reduced_cost(
    plan: &CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>,
    demands: &[Csp2dMasterDemand],
    shadow_prices: &Csp2dShadowPrices,
    config: &Csp2dPricingConfig,
) -> Csp2dReducedCost {
    let consumption = if config.price_by_height {
        plan.consumed_height()
            .map(|height| height.value)
            .unwrap_or_else(|| plan.consumed_area().value)
    } else {
        plan.consumed_area().value
    };
    let amounts = plan.product_amounts();
    let mut revenue = 0.0;
    for demand in demands {
        let Some(count) = amounts.get(&demand.product_id).copied() else {
            continue;
        };
        let Some(price) = shadow_prices.get(&demand.product_id) else {
            continue;
        };
        let coefficient = match demand.mode {
            crate::domain::material::DemandMode2::Amount => count as f64,
            crate::domain::material::DemandMode2::Area => {
                let unit_area = plan
                    .pattern
                    .placements
                    .iter()
                    .find(|placement| {
                        placement
                            .item_id
                            .as_deref()
                            .map(|id| id == demand.product_id.as_str())
                            .unwrap_or(false)
                    })
                    .map(|placement| placement.shape.area().value)
                    .unwrap_or(0.0);
                unit_area * count as f64
            }
        };
        revenue += coefficient * price;
    }
    Csp2dReducedCost {
        // 未入列的图案没有下标，用 `usize::MAX` 明确标记，避免被误当成有效列。
        // A pattern outside the master has no index; `usize::MAX` marks that explicitly so it is never mistaken
        // for a valid column.
        plan_index: usize::MAX,
        consumption,
        revenue,
        reduced_cost: consumption * config.consumption_price - revenue,
    }
}

/// 由需求覆盖量与消耗量构造影子价格估计 / Build shadow price estimates from coverage and consumption
///
/// 在没有真实 LP 对偶时，用「需求紧迫度」作为影子价格的替代口径：覆盖率越低的需求，其影子价格越高。
/// 这只是启发式估计，**不是**对偶值；调用方必须清楚这一点，不能把它当成定价证书。
/// Without real LP duals, "demand urgency" stands in for the shadow prices: the lower the coverage, the higher
/// the price. This is a heuristic estimate, **not** a dual value; callers must be aware and may not treat it as a
/// pricing certificate.
pub fn estimated_shadow_prices(
    assignment: &Csp2dPatternAssignment,
    usage: &[f64],
    demands: &[Csp2dMasterDemand],
) -> Csp2dShadowPrices {
    let coverage = assignment.coverage(usage, demands);
    let mut prices = BTreeMap::new();
    for (demand, covered) in demands.iter().zip(coverage.iter()) {
        let required = demand.demand_value();
        let price = if required <= 0.0 {
            0.0
        } else {
            // 缺口比例 × 单位消耗面积：缺口越大、单件越占地方，影子价格越高。
            // Deficit ratio times the unit consumption area: the bigger the deficit and the more space one piece
            // takes, the higher the shadow price.
            let deficit_ratio = ((required - covered) / required).clamp(0.0, 1.0);
            deficit_ratio * required
        };
        prices.insert(demand.product_id.clone(), price);
    }
    prices
}
