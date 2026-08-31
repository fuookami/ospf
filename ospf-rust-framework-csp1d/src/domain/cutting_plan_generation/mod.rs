//! 切割方案生成 / Cutting plan generation

use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{
    Costar, Csp1dQuantity, Csp1dShadowPriceKey, CuttingPlan, CuttingPlanDemandContribution,
    CuttingPlanProduction, CuttingPlanSlice, MachineBatchShadowPriceKey,
    MachineCapacityShadowPriceKey, MaterialUsageShadowPriceKey, ProductDemandShadowPriceKey,
    from_f64, shadow_price_unit_symbol, to_f64,
};
use crate::domain::produce::{Csp1dDomainPolicy, SimpleDomainCalculationContext};

pub mod model;

pub use model::*;

/// 从领域策略构建幅宽可行性检查 / Build width feasibility check from domain policies
pub fn width_feasibility_check_from_policies<V: SolveValue>(
    domain_policies: &[std::sync::Arc<dyn Csp1dDomainPolicy<V>>],
    domain_value_sample: V,
) -> Option<Csp1dWidthFeasibilityCheck<V>> {
    let width_policies = domain_policies
        .iter()
        .filter(|policy| policy.overrides_width_feasibility())
        .cloned()
        .collect::<Vec<_>>();
    if width_policies.is_empty() {
        return None;
    }
    Some(std::sync::Arc::new(
        move |material, product, product_width| {
            let plan = CuttingPlan {
                id: format!("width-check-{}-{}", material.id, product.id).into(),
                material: material.clone(),
                machine_id: material.machine_id.clone(),
                slices: vec![CuttingPlanSlice {
                    production: CuttingPlanProduction::Product(product.clone()),
                    width: product_width.clone(),
                    amount: 1,
                }],
                demand_contributions: Vec::new(),
                capacity_consumption: None,
            };
            let context = SimpleDomainCalculationContext {
                plan,
                plan_index: usize::MAX,
                domain_value_sample: Some(domain_value_sample.clone()),
            };
            width_policies
                .iter()
                .all(|policy| policy.is_width_feasible(&context))
        },
    ))
}

/// 初始切割方案生成器 / Initial cutting plan generator
pub trait Csp1dInitialCuttingPlanGenerator<V: SolveValue>: Send + Sync {
    /// 生成切割方案 / Generate cutting plans
    fn generate(&self, input: &CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>>;

    /// 生成切割方案并返回报告 / Generate cutting plans with report
    fn generate_with_report(
        &self,
        input: &CuttingPlanGenerationInput<V>,
    ) -> CuttingPlanGenerationReport<V> {
        let plans = self.generate(input);
        CuttingPlanGenerationReport {
            plans: plans.clone(),
            statistics: CuttingPlanGenerationStatistics {
                generated_candidates: plans.len() as i64,
                accepted_plans: plans.len() as i64,
                ..CuttingPlanGenerationStatistics::default()
            },
        }
    }
}

/// 定价生成器 / Pricing generator
pub trait Csp1dPricingGenerator<V: SolveValue>: Send + Sync {
    /// 生成定价方案 / Generate pricing plans
    fn generate(&self, input: &Csp1dPricingInput<V>) -> Vec<CuttingPlan<V>>;

    /// 生成定价方案并返回报告 / Generate pricing plans with report
    fn generate_with_report(&self, input: &Csp1dPricingInput<V>) -> CuttingPlanGenerationReport<V> {
        let plans = self.generate(input);
        CuttingPlanGenerationReport {
            plans: plans.clone(),
            statistics: CuttingPlanGenerationStatistics {
                generated_candidates: plans.len() as i64,
                accepted_plans: plans.len() as i64,
                ..CuttingPlanGenerationStatistics::default()
            },
        }
    }
}

/// 简单初始生成器 / Simple initial cutting plan generator
///
/// 为每个物料和需求组合生成单产品方案。
/// Generates single-product plans for each material-demand combination.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleInitialCuttingPlanGenerator;

impl<V: SolveValue> Csp1dInitialCuttingPlanGenerator<V> for SimpleInitialCuttingPlanGenerator {
    fn generate(&self, input: &CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> {
        let mut plans = Vec::new();
        for material in &input.materials {
            for demand in &input.demands {
                let Some(width) = demand.product.width.iter().find(|width| {
                    input
                        .width_feasibility_check
                        .as_ref()
                        .map(|check| check(material, &demand.product, width))
                        .unwrap_or_else(|| material.width_range.can_cut(width))
                }) else {
                    continue;
                };
                let plan = CuttingPlan {
                    id: format!("init-{}-{}-{}", material.id, demand.product.id, plans.len())
                        .into(),
                    material: material.clone(),
                    machine_id: material.machine_id.clone(),
                    slices: vec![CuttingPlanSlice {
                        production: CuttingPlanProduction::Product(demand.product.clone()),
                        width: width.clone(),
                        amount: 1,
                    }],
                    demand_contributions: vec![CuttingPlanDemandContribution::from_demand(
                        demand, width, 1, None,
                    )],
                    capacity_consumption: None,
                };
                if !input
                    .width_feasibility_check
                    .as_ref()
                    .map(|_| {
                        material.enabled_without_width_check_with_machines(&plan, &input.machines)
                    })
                    .unwrap_or_else(|| material.enabled(&plan, &input.machines))
                {
                    continue;
                }
                if !accept_generated_plan(input, &plan, plans.len()) {
                    continue;
                }
                plans.push(plan);
            }
        }
        plans
    }
}

/// N-Same 生成器 / N-Same generator
///
/// 为每个物料和需求组合生成同产品多刀方案。
/// Generates same-product multi-knife plans for each material-demand combination.
#[derive(Debug, Clone)]
pub struct NSameGenerator<V: SolveValue> {
    /// 生成约束 / Generation constraints
    pub constraints: GenerationConstraints<V>,
    /// 是否生成全部可行数量 / Whether to generate all feasible amounts
    pub all_amount: bool,
    /// 最大方案数 / Maximum plan count
    pub max_plans: u64,
}

impl<V: SolveValue> Default for NSameGenerator<V> {
    fn default() -> Self {
        Self {
            constraints: GenerationConstraints::default(),
            all_amount: false,
            max_plans: 1000,
        }
    }
}

impl<V: SolveValue> NSameGenerator<V> {
    /// 使用约束创建 / Create with constraints
    pub fn with_constraints(constraints: GenerationConstraints<V>) -> Self {
        Self {
            constraints,
            ..Self::default()
        }
    }

    /// 设置是否生成全部可行数量 / Set whether to generate all feasible amounts
    pub fn with_all_amount(mut self, all_amount: bool) -> Self {
        self.all_amount = all_amount;
        self
    }

    /// 设置最大方案数 / Set maximum plan count
    pub fn with_max_plans(mut self, max_plans: u64) -> Self {
        self.max_plans = max_plans;
        self
    }
}

impl<V: SolveValue> Csp1dInitialCuttingPlanGenerator<V> for NSameGenerator<V> {
    fn generate(&self, input: &CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> {
        self.generate_with_report(input).plans
    }

    fn generate_with_report(
        &self,
        input: &CuttingPlanGenerationInput<V>,
    ) -> CuttingPlanGenerationReport<V> {
        generate_n_same(input, &self.constraints, self.all_amount, self.max_plans)
    }
}

/// N-Sum 生成器 / N-Sum generator
///
/// 通过组合搜索生成多产品混合方案。
/// Generates multi-product mixed plans through combination search.
#[derive(Debug, Clone)]
pub struct NSumGenerator<V: SolveValue> {
    /// 生成约束 / Generation constraints
    pub constraints: GenerationConstraints<V>,
    /// 最大搜索深度 / Maximum search depth
    pub max_depth: u64,
    /// 最大方案数 / Maximum plan count
    pub max_plans: u64,
}

impl<V: SolveValue> Default for NSumGenerator<V> {
    fn default() -> Self {
        Self {
            constraints: GenerationConstraints::default(),
            max_depth: 7,
            max_plans: 1000,
        }
    }
}

impl<V: SolveValue> NSumGenerator<V> {
    /// 使用约束创建 / Create with constraints
    pub fn with_constraints(constraints: GenerationConstraints<V>) -> Self {
        Self {
            constraints,
            ..Self::default()
        }
    }

    /// 设置最大搜索深度 / Set maximum search depth
    pub fn with_max_depth(mut self, max_depth: u64) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// 设置最大方案数 / Set maximum plan count
    pub fn with_max_plans(mut self, max_plans: u64) -> Self {
        self.max_plans = max_plans;
        self
    }
}

impl<V: SolveValue> Csp1dInitialCuttingPlanGenerator<V> for NSumGenerator<V> {
    fn generate(&self, input: &CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> {
        self.generate_with_report(input).plans
    }

    fn generate_with_report(
        &self,
        input: &CuttingPlanGenerationInput<V>,
    ) -> CuttingPlanGenerationReport<V> {
        generate_combination_plans(
            input,
            &self.constraints,
            self.max_depth,
            self.max_plans,
            "nsum",
        )
    }
}

/// DFS 生成器 / DFS generator
///
/// 深度优先搜索生成组合方案。
/// Generates combination plans via depth-first search.
#[derive(Debug, Clone)]
pub struct DFSGenerator<V: SolveValue> {
    /// 生成约束 / Generation constraints
    pub constraints: GenerationConstraints<V>,
    /// 最大方案数 / Maximum plan count
    pub max_plans: u64,
}

impl<V: SolveValue> Default for DFSGenerator<V> {
    fn default() -> Self {
        Self {
            constraints: GenerationConstraints::default(),
            max_plans: 1000,
        }
    }
}

impl<V: SolveValue> DFSGenerator<V> {
    /// 使用约束创建 / Create with constraints
    pub fn with_constraints(constraints: GenerationConstraints<V>) -> Self {
        Self {
            constraints,
            ..Self::default()
        }
    }

    /// 设置最大方案数 / Set maximum plan count
    pub fn with_max_plans(mut self, max_plans: u64) -> Self {
        self.max_plans = max_plans;
        self
    }
}

impl<V: SolveValue> Csp1dInitialCuttingPlanGenerator<V> for DFSGenerator<V> {
    fn generate(&self, input: &CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> {
        self.generate_with_report(input).plans
    }

    fn generate_with_report(
        &self,
        input: &CuttingPlanGenerationInput<V>,
    ) -> CuttingPlanGenerationReport<V> {
        let max_depth = self
            .constraints
            .max_knife_count
            .unwrap_or(input.demands.len().max(1) as u64);
        generate_combination_plans(input, &self.constraints, max_depth, self.max_plans, "dfs")
    }
}

/// FullSum 生成器 / FullSum generator
///
/// 全组合搜索生成方案。
/// Generates plans through full combination search.
#[derive(Debug, Clone)]
pub struct FullSumGenerator<V: SolveValue> {
    /// 生成约束 / Generation constraints
    pub constraints: GenerationConstraints<V>,
    /// 最大方案数 / Maximum plan count
    pub max_plans: u64,
}

impl<V: SolveValue> Default for FullSumGenerator<V> {
    fn default() -> Self {
        Self {
            constraints: GenerationConstraints::default(),
            max_plans: 1000,
        }
    }
}

impl<V: SolveValue> FullSumGenerator<V> {
    /// 使用约束创建 / Create with constraints
    pub fn with_constraints(constraints: GenerationConstraints<V>) -> Self {
        Self {
            constraints,
            ..Self::default()
        }
    }

    /// 设置最大方案数 / Set maximum plan count
    pub fn with_max_plans(mut self, max_plans: u64) -> Self {
        self.max_plans = max_plans;
        self
    }
}

impl<V: SolveValue> Csp1dInitialCuttingPlanGenerator<V> for FullSumGenerator<V> {
    fn generate(&self, input: &CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> {
        self.generate_with_report(input).plans
    }

    fn generate_with_report(
        &self,
        input: &CuttingPlanGenerationInput<V>,
    ) -> CuttingPlanGenerationReport<V> {
        let max_depth = self.constraints.max_knife_count.unwrap_or(7);
        generate_combination_plans(
            input,
            &self.constraints,
            max_depth,
            self.max_plans,
            "fullsum",
        )
    }
}

/// 配规填充器 / Costar filler
///
/// 用配规切片填充切割方案的余宽。
/// Fills rest width of cutting plans with costar slices.
#[derive(Debug, Clone, Default)]
pub struct CostarFiller {
    /// 每个幅宽最大配规数量 / Maximum costar amount per width
    pub max_costar_amount_per_width: u64,
}

impl CostarFiller {
    /// 创建填充器 / Create filler
    pub fn new() -> Self {
        Self {
            max_costar_amount_per_width: 2,
        }
    }

    /// 填充配规切片 / Fill costar slices
    pub fn fill<V: SolveValue>(
        &self,
        plan: &CuttingPlan<V>,
        costars: &[Costar<V>],
    ) -> Vec<CuttingPlan<V>> {
        if costars.is_empty() {
            return vec![plan.clone()];
        }
        let Some(rest_width) = plan.rest_width() else {
            return vec![plan.clone()];
        };
        let Some(rest_value) = to_f64(&rest_width.value) else {
            return vec![plan.clone()];
        };
        if rest_value <= 0.0 {
            return vec![plan.clone()];
        }
        let mut results = Vec::new();
        let mut slices = plan.slices.clone();
        self.fill_dfs(plan, costars, 0, rest_width, &mut slices, &mut results);
        if results.is_empty() {
            vec![plan.clone()]
        } else {
            results
        }
    }

    fn fill_dfs<V: SolveValue>(
        &self,
        plan: &CuttingPlan<V>,
        costars: &[Costar<V>],
        costar_index: usize,
        remaining_width: Csp1dQuantity<V>,
        current_slices: &mut Vec<CuttingPlanSlice<V>>,
        results: &mut Vec<CuttingPlan<V>>,
    ) {
        let remaining = to_f64(&remaining_width.value).unwrap_or(0.0);
        if remaining <= 0.0 || costar_index >= costars.len() {
            results.push(build_plan_with_slices(plan, current_slices.clone()));
            return;
        }
        let costar = &costars[costar_index];
        for costar_width in &costar.width {
            if costar_width.unit != remaining_width.unit {
                continue;
            }
            let Some(width_value) = to_f64(&costar_width.value) else {
                continue;
            };
            if width_value <= 0.0 || width_value > remaining {
                continue;
            }
            let max_amount = (remaining / width_value).floor() as u64;
            for amount in 1..=max_amount.min(self.max_costar_amount_per_width.max(1)) {
                let used_width = width_value * amount as f64;
                let Some(next_remaining_value) = from_f64(remaining - used_width) else {
                    continue;
                };
                current_slices.push(CuttingPlanSlice {
                    production: CuttingPlanProduction::Costar(costar.clone()),
                    width: costar_width.clone(),
                    amount,
                });
                self.fill_dfs(
                    plan,
                    costars,
                    costar_index + 1,
                    Csp1dQuantity {
                        value: next_remaining_value,
                        unit: remaining_width.unit.clone(),
                    },
                    current_slices,
                    results,
                );
                current_slices.pop();
            }
        }
        self.fill_dfs(
            plan,
            costars,
            costar_index + 1,
            remaining_width,
            current_slices,
            results,
        );
    }
}

/// 简单定价生成器 / Simple pricing generator
///
/// 为每个正影子价格需求生成单产品定价方案。
/// Generates single-product pricing plans for each demand with positive shadow price.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimplePricingGenerator;

impl<V: SolveValue> Csp1dPricingGenerator<V> for SimplePricingGenerator {
    fn generate(&self, input: &Csp1dPricingInput<V>) -> Vec<CuttingPlan<V>> {
        let Some(material) = input.generation_input.materials.first() else {
            return Vec::new();
        };
        let mut plans = Vec::new();
        for demand in &input.generation_input.demands {
            if plans.len() >= input.max_generated_plans as usize {
                break;
            }
            let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
                product_id: demand.product.id.clone(),
                unit_symbol: shadow_price_unit_symbol(&demand.quantity.unit),
            });
            let Some(shadow_price) = input.shadow_prices.get(&key).and_then(to_f64) else {
                continue;
            };
            if shadow_price <= 0.0 {
                continue;
            }
            let Some(width) = demand.product.width.iter().find(|width| {
                input
                    .generation_input
                    .width_feasibility_check
                    .as_ref()
                    .map(|check| check(material, &demand.product, width))
                    .unwrap_or_else(|| material.width_range.can_cut(width))
            }) else {
                continue;
            };
            let plan = CuttingPlan {
                id: format!(
                    "pricing-{}-{}-{}",
                    material.id,
                    demand.product.id,
                    plans.len()
                )
                .into(),
                material: material.clone(),
                machine_id: material.machine_id.clone(),
                slices: vec![CuttingPlanSlice {
                    production: CuttingPlanProduction::Product(demand.product.clone()),
                    width: width.clone(),
                    amount: 1,
                }],
                demand_contributions: vec![CuttingPlanDemandContribution::from_demand(
                    demand, width, 1, None,
                )],
                capacity_consumption: None,
            };
            if !input
                .generation_input
                .width_feasibility_check
                .as_ref()
                .map(|_| {
                    material.enabled_without_width_check_with_machines(
                        &plan,
                        &input.generation_input.machines,
                    )
                })
                .unwrap_or_else(|| material.enabled(&plan, &input.generation_input.machines))
            {
                continue;
            }
            if !accept_generated_plan(&input.generation_input, &plan, plans.len()) {
                continue;
            }
            plans.push(plan);
        }
        plans
    }
}

/// reduced cost 定价生成器 / Reduced cost pricing generator
///
/// 使用枚举器生成候选方案，按 reduced cost 排序筛选改善方案。
/// Uses an enumerator to generate candidates, then filters improving plans by reduced cost.
#[derive(Debug, Clone)]
pub struct ReducedCostPricingGenerator<G> {
    /// 候选方案枚举器 / Candidate plan enumerator
    pub enumerator: G,
}

impl<G> ReducedCostPricingGenerator<G> {
    /// 创建定价生成器 / Create pricing generator
    pub fn new(enumerator: G) -> Self {
        Self { enumerator }
    }
}

impl<V, G> Csp1dPricingGenerator<V> for ReducedCostPricingGenerator<G>
where
    V: SolveValue,
    G: Csp1dInitialCuttingPlanGenerator<V>,
{
    fn generate(&self, input: &Csp1dPricingInput<V>) -> Vec<CuttingPlan<V>> {
        self.generate_with_report(input).plans
    }

    fn generate_with_report(&self, input: &Csp1dPricingInput<V>) -> CuttingPlanGenerationReport<V> {
        let mut report = self
            .enumerator
            .generate_with_report(&input.generation_input);
        if report.plans.is_empty() || input.max_generated_plans == 0 {
            report.plans.clear();
            report.statistics.accepted_plans = 0;
            return report;
        }

        let mut existing_ids = std::collections::HashSet::new();
        let mut existing_keys = std::collections::HashSet::new();
        for plan in &input.generation_input.existing_plans {
            existing_ids.insert(plan.id.clone());
            existing_keys.insert(resolve_pricing_canonical_key(input, plan));
        }

        let mut seen_keys = std::collections::HashSet::new();
        let mut priced = report
            .plans
            .into_iter()
            .filter(|plan| !existing_ids.contains(&plan.id))
            .filter_map(|plan| {
                let key = resolve_pricing_canonical_key(input, &plan);
                if existing_keys.contains(&key) || !seen_keys.insert(key) {
                    return None;
                }
                let base_benefit = compute_dual_benefit(&plan, &input.shadow_prices)?;
                let benefit = input
                    .pricing_benefit_modifiers
                    .iter()
                    .fold(base_benefit, |benefit, modifier| modifier(&plan, benefit));
                let benefit = input
                    .pricing_policies
                    .iter()
                    .fold(benefit, |benefit, policy| {
                        policy.modify_benefit(&plan, benefit)
                    });
                let base_cost = compute_objective_cost(&plan, &input.objective_config)?;
                let cost = input
                    .pricing_cost_modifiers
                    .iter()
                    .fold(base_cost, |cost, modifier| modifier(&plan, cost));
                let cost = input
                    .pricing_policies
                    .iter()
                    .fold(cost, |cost, policy| policy.modify_cost(&plan, cost));
                is_pricing_improving(input, &plan, &benefit, &cost).then_some(PricedCandidate {
                    plan,
                    benefit,
                    cost,
                })
            })
            .collect::<Vec<_>>();
        priced.sort_by(|left, right| {
            let left_score = to_f64(&left.benefit).unwrap_or(f64::NEG_INFINITY)
                - to_f64(&left.cost).unwrap_or(f64::INFINITY);
            let right_score = to_f64(&right.benefit).unwrap_or(f64::NEG_INFINITY)
                - to_f64(&right.cost).unwrap_or(f64::INFINITY);
            right_score
                .partial_cmp(&left_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        report.plans = priced
            .into_iter()
            .take(input.max_generated_plans as usize)
            .map(|candidate| candidate.plan)
            .collect();
        report.statistics.accepted_plans = report.plans.len() as i64;
        report
    }
}

fn accept_generated_plan<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    plan: &CuttingPlan<V>,
    plan_index: usize,
) -> bool {
    let context = SimpleDomainCalculationContext {
        plan: plan.clone(),
        plan_index,
        domain_value_sample: resolve_domain_value_sample(input),
    };
    input
        .domain_policies
        .iter()
        .all(|policy| policy.is_feasible(&context) && policy.is_width_feasible(&context))
        && input
            .generation_strategies
            .iter()
            .all(|strategy| strategy.accept_candidate(plan, &input.existing_plans))
        && input
            .candidate_filters
            .iter()
            .all(|filter| filter(plan, &input.existing_plans))
}

fn accept_generated_dominance<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    plan: &CuttingPlan<V>,
    accepted_plans: &[CuttingPlan<V>],
) -> bool {
    input
        .generation_strategies
        .iter()
        .all(|strategy| strategy.accept_dominance(plan, accepted_plans))
        && input
            .dominance_accept_overrides
            .iter()
            .all(|judge| judge(plan, accepted_plans))
}

fn generate_n_same<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    constraints: &GenerationConstraints<V>,
    all_amount: bool,
    max_plans: u64,
) -> CuttingPlanGenerationReport<V> {
    let mut collector = PlanCollector::new(input, max_plans, Some(constraints));
    let mut quantity_cache = GenerationQuantityCache::new();
    let mut plan_index = 0usize;
    for material in &input.materials {
        for demand in &input.demands {
            for product_width in &demand.product.width {
                if collector.should_stop() {
                    break;
                }
                if !is_width_feasible(input, material, &demand.product, product_width) {
                    collector.statistics.infeasible_candidates += 1;
                    continue;
                }
                if exceeds_generation_length_bound(&demand.product, constraints) {
                    collector.statistics.length_bound_pruned_entries += 1;
                    continue;
                }
                let max_amount = max_amount_for_width(product_width, material, constraints);
                if max_amount == 0 {
                    continue;
                }
                let amounts = if all_amount {
                    (1..=max_amount).collect::<Vec<_>>()
                } else {
                    vec![max_amount]
                };
                for amount in amounts {
                    if collector.should_stop() {
                        break;
                    }
                    collector.statistics.visited_nodes += 1;
                    if !satisfies_knife_count(amount, constraints) {
                        if constraints
                            .min_knife_count
                            .is_some_and(|min_count| amount < min_count)
                        {
                            collector.statistics.knife_bound_pruned_nodes += 1;
                        }
                        continue;
                    }
                    let plan = build_plan(
                        "nsame",
                        material,
                        vec![CuttingPlanSlice {
                            production: CuttingPlanProduction::Product(demand.product.clone()),
                            width: product_width.clone(),
                            amount,
                        }],
                        vec![quantity_cache.contribution(demand, product_width, amount)],
                        plan_index,
                    );
                    plan_index += 1;
                    let feasible = if input.width_feasibility_check.is_some() {
                        material.enabled_without_width_check_with_machines(&plan, &input.machines)
                    } else {
                        material.enabled(&plan, &input.machines)
                    };
                    collector.record(plan, feasible);
                }
            }
        }
    }
    collector.statistics.quantity_cache_hits += quantity_cache.hits;
    collector.statistics.quantity_cache_misses += quantity_cache.misses;
    collector.finish()
}

fn generate_combination_plans<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    constraints: &GenerationConstraints<V>,
    max_depth: u64,
    max_plans: u64,
    id_prefix: &str,
) -> CuttingPlanGenerationReport<V> {
    let mut collector = PlanCollector::new(input, max_plans, Some(constraints));
    let entries = generation_entries(input, constraints, &mut collector.statistics);
    if entries.is_empty() || max_depth == 0 {
        return collector.finish();
    }
    let effective_depth = constraints
        .max_knife_count
        .map(|max_knife_count| max_knife_count.min(max_depth))
        .unwrap_or(max_depth);
    let mut plan_index = 0usize;
    let mut material_entry_cache = std::collections::HashMap::new();
    let mut material_template_cache = std::collections::HashMap::new();
    let mut quantity_cache = GenerationQuantityCache::new();
    for material in &input.materials {
        let material_entries = material_entries_for(
            input,
            material,
            &entries,
            &mut material_entry_cache,
            &mut collector,
        );
        if material_entries.is_empty() {
            continue;
        }
        let material_key = material_width_range_key(material);
        if can_reuse_material_slice_templates(input, constraints) {
            if let Some(templates) = material_template_cache.get(&material_key).cloned() {
                collector.statistics.material_slice_template_cache_hits += 1;
                emit_slice_templates(
                    material,
                    templates,
                    id_prefix,
                    &mut plan_index,
                    &mut collector,
                );
                if collector.should_stop() {
                    break;
                }
                continue;
            }
            collector.statistics.material_slice_template_cache_misses += 1;
        }
        let mut slices = Vec::new();
        let mut contributions = Vec::new();
        let mut template_recorder = Vec::new();
        search_combinations(
            input,
            constraints,
            material,
            &material_entries,
            0,
            effective_depth,
            id_prefix,
            &mut plan_index,
            &mut slices,
            &mut contributions,
            &mut quantity_cache,
            &mut template_recorder,
            &mut collector,
        );
        if can_reuse_material_slice_templates(input, constraints) && !collector.should_stop() {
            material_template_cache
                .entry(material_key)
                .or_insert(template_recorder);
        }
        if collector.should_stop() {
            break;
        }
    }
    collector.statistics.quantity_cache_hits += quantity_cache.hits;
    collector.statistics.quantity_cache_misses += quantity_cache.misses;
    collector.finish()
}

fn search_combinations<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    constraints: &GenerationConstraints<V>,
    material: &crate::domain::material::Material<V>,
    entries: &[GenerationEntry<V>],
    start_index: usize,
    remaining_depth: u64,
    id_prefix: &str,
    plan_index: &mut usize,
    slices: &mut Vec<CuttingPlanSlice<V>>,
    contributions: &mut Vec<CuttingPlanDemandContribution<V>>,
    quantity_cache: &mut GenerationQuantityCache<V>,
    template_recorder: &mut Vec<GenerationSliceTemplate<V>>,
    collector: &mut PlanCollector<V>,
) {
    if collector.should_stop() || remaining_depth == 0 {
        return;
    }
    for entry_index in start_index..entries.len() {
        if collector.should_stop() {
            break;
        }
        let entry = &entries[entry_index];
        if !is_width_feasible(input, material, &entry.demand.product, &entry.width) {
            collector.statistics.infeasible_candidates += 1;
            continue;
        }
        slices.push(CuttingPlanSlice {
            production: CuttingPlanProduction::Product(entry.demand.product.clone()),
            width: entry.width.clone(),
            amount: 1,
        });
        contributions.push(quantity_cache.contribution(&entry.demand, &entry.width, 1));
        collector.statistics.visited_nodes += 1;
        let used_width = used_width_of_slices(slices);
        let material_width = to_f64(&material.width_range.upper_bound.value).unwrap_or(0.0);
        if used_width > material_width {
            collector.statistics.width_bound_pruned_nodes += 1;
            slices.pop();
            contributions.pop();
            continue;
        }
        let amount = total_slice_amount(slices);
        let max_ok = constraints
            .max_knife_count
            .map(|max_count| amount <= max_count)
            .unwrap_or(true);
        if max_ok
            && constraints
                .min_knife_count
                .map(|min_count| amount >= min_count)
                .unwrap_or(true)
        {
            let plan = build_plan(
                id_prefix,
                material,
                slices.clone(),
                merge_contributions(contributions),
                *plan_index,
            );
            *plan_index += 1;
            let feasible = if input.width_feasibility_check.is_some() {
                material.enabled_without_width_check_with_machines(&plan, &input.machines)
            } else {
                material.enabled(&plan, &input.machines)
            };
            if feasible {
                template_recorder.push(GenerationSliceTemplate {
                    slices: slices.clone(),
                    demand_contributions: merge_contributions(contributions),
                });
            }
            collector.record(plan, feasible);
        } else if constraints
            .min_knife_count
            .is_some_and(|min_count| amount < min_count && remaining_depth == 1)
        {
            collector.statistics.knife_bound_pruned_nodes += 1;
        }
        search_combinations(
            input,
            constraints,
            material,
            entries,
            entry_index + 1,
            remaining_depth - 1,
            id_prefix,
            plan_index,
            slices,
            contributions,
            quantity_cache,
            template_recorder,
            collector,
        );
        slices.pop();
        contributions.pop();
    }
}

fn generation_entries<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    constraints: &GenerationConstraints<V>,
    statistics: &mut CuttingPlanGenerationStatistics,
) -> Vec<GenerationEntry<V>> {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for demand in &input.demands {
        if exceeds_generation_length_bound(&demand.product, constraints) {
            statistics.length_bound_pruned_entries += demand.product.width.len() as i64;
            continue;
        }
        for width in &demand.product.width {
            let key = format!(
                "{}|{:?}|{}|{}",
                demand.product.id,
                width.value,
                width.unit.symbol(),
                demand.quantity.unit.symbol()
            );
            if seen.insert(key) {
                entries.push(GenerationEntry {
                    demand: demand.clone(),
                    width: width.clone(),
                });
            } else {
                statistics.duplicate_candidates += 1;
            }
        }
    }
    entries
}

fn material_entries_for<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    material: &crate::domain::material::Material<V>,
    base_entries: &[GenerationEntry<V>],
    cache: &mut std::collections::HashMap<String, Vec<GenerationEntry<V>>>,
    collector: &mut PlanCollector<V>,
) -> Vec<GenerationEntry<V>> {
    let key = material_width_range_key(material);
    if let Some(entries) = cache.get(&key) {
        collector.statistics.material_width_index_cache_hits += 1;
        return entries.clone();
    }
    let entries = base_entries
        .iter()
        .filter(|entry| is_width_feasible(input, material, &entry.demand.product, &entry.width))
        .cloned()
        .collect::<Vec<_>>();
    cache.insert(key, entries.clone());
    entries
}

fn material_width_range_key<V: SolveValue>(
    material: &crate::domain::material::Material<V>,
) -> String {
    format!(
        "{:?}:{}|{:?}:{}",
        material.width_range.lower_bound.value,
        material.width_range.lower_bound.unit.symbol(),
        material.width_range.upper_bound.value,
        material.width_range.upper_bound.unit.symbol()
    )
}

fn can_reuse_material_slice_templates<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    _constraints: &GenerationConstraints<V>,
) -> bool {
    input.domain_policies.is_empty()
        && input.generation_strategies.is_empty()
        && input.candidate_filters.is_empty()
        && input.width_feasibility_check.is_none()
        && input.canonical_key_overrides.is_empty()
        && input.dominance_accept_overrides.is_empty()
}

fn emit_slice_templates<V: SolveValue>(
    material: &crate::domain::material::Material<V>,
    templates: Vec<GenerationSliceTemplate<V>>,
    id_prefix: &str,
    plan_index: &mut usize,
    collector: &mut PlanCollector<V>,
) {
    for template in templates {
        if collector.should_stop() {
            break;
        }
        let plan = build_plan(
            id_prefix,
            material,
            template.slices,
            template.demand_contributions,
            *plan_index,
        );
        *plan_index += 1;
        collector.record(plan, true);
    }
}

fn is_width_feasible<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    material: &crate::domain::material::Material<V>,
    product: &crate::domain::material::Product<V>,
    width: &Csp1dQuantity<V>,
) -> bool {
    input
        .width_feasibility_check
        .as_ref()
        .map(|check| check(material, product, width))
        .unwrap_or_else(|| material.width_range.can_cut(width))
}

fn exceeds_generation_length_bound<V: SolveValue>(
    product: &crate::domain::material::Product<V>,
    constraints: &GenerationConstraints<V>,
) -> bool {
    let Some(max_length) = &constraints.max_over_produce_length else {
        return false;
    };
    let Some(product_length) = &product.length else {
        return false;
    };
    if product_length.unit != max_length.unit {
        return false;
    }
    product_length.value > max_length.value
}

fn max_amount_for_width<V: SolveValue>(
    width: &Csp1dQuantity<V>,
    material: &crate::domain::material::Material<V>,
    constraints: &GenerationConstraints<V>,
) -> u64 {
    if width.unit != material.width_range.upper_bound.unit {
        return 0;
    }
    let Some(width_value) = to_f64(&width.value) else {
        return 0;
    };
    let Some(upper_bound) = to_f64(&material.width_range.upper_bound.value) else {
        return 0;
    };
    if width_value <= 0.0 || width_value > upper_bound {
        return 0;
    }
    let mut amount = (upper_bound / width_value).floor() as u64;
    if let Some(max_knife_count) = constraints.max_knife_count {
        amount = amount.min(max_knife_count);
    }
    amount
}

fn satisfies_knife_count<V: SolveValue>(
    amount: u64,
    constraints: &GenerationConstraints<V>,
) -> bool {
    constraints
        .max_knife_count
        .map(|max_count| amount <= max_count)
        .unwrap_or(true)
        && constraints
            .min_knife_count
            .map(|min_count| amount >= min_count)
            .unwrap_or(true)
}

fn build_plan<V: SolveValue>(
    prefix: &str,
    material: &crate::domain::material::Material<V>,
    slices: Vec<CuttingPlanSlice<V>>,
    demand_contributions: Vec<CuttingPlanDemandContribution<V>>,
    plan_index: usize,
) -> CuttingPlan<V> {
    CuttingPlan {
        id: format!("{prefix}-{}-{plan_index}", material.id).into(),
        material: material.clone(),
        machine_id: material.machine_id.clone(),
        slices,
        demand_contributions,
        capacity_consumption: None,
    }
}

fn build_plan_with_slices<V: SolveValue>(
    original: &CuttingPlan<V>,
    slices: Vec<CuttingPlanSlice<V>>,
) -> CuttingPlan<V> {
    CuttingPlan {
        id: original.id.clone(),
        material: original.material.clone(),
        machine_id: original.machine_id.clone(),
        slices,
        demand_contributions: original.demand_contributions.clone(),
        capacity_consumption: original.capacity_consumption.clone(),
    }
}

fn build_contribution<V: SolveValue>(
    demand: &crate::domain::material::ProductDemand<V>,
    width: &Csp1dQuantity<V>,
    amount: u64,
) -> CuttingPlanDemandContribution<V> {
    CuttingPlanDemandContribution::from_demand(demand, width, amount, None)
}

struct GenerationQuantityCache<V: SolveValue> {
    cache: std::collections::HashMap<String, CuttingPlanDemandContribution<V>>,
    hits: i64,
    misses: i64,
}

impl<V: SolveValue> GenerationQuantityCache<V> {
    fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    fn contribution(
        &mut self,
        demand: &crate::domain::material::ProductDemand<V>,
        width: &Csp1dQuantity<V>,
        amount: u64,
    ) -> CuttingPlanDemandContribution<V> {
        let key = format!(
            "{}|{:?}|{}|{:?}|{}|{}",
            demand.product.id,
            demand.quantity.value,
            demand.quantity.unit.symbol(),
            width.value,
            width.unit.symbol(),
            amount
        );
        if let Some(value) = self.cache.get(&key) {
            self.hits += 1;
            return value.clone();
        }
        self.misses += 1;
        let contribution = build_contribution(demand, width, amount);
        self.cache.insert(key, contribution.clone());
        contribution
    }
}

fn used_width_of_slices<V: SolveValue>(slices: &[CuttingPlanSlice<V>]) -> f64 {
    slices
        .iter()
        .filter_map(|slice| to_f64(&slice.width.value).map(|width| width * slice.amount as f64))
        .sum()
}

fn total_slice_amount<V: SolveValue>(slices: &[CuttingPlanSlice<V>]) -> u64 {
    slices.iter().map(|slice| slice.amount).sum()
}

fn contribution_quantity_key<V: SolveValue>(
    contribution: &CuttingPlanDemandContribution<V>,
) -> String {
    format!(
        "{}:{}:{:?}",
        contribution.product.id,
        contribution.quantity.unit.symbol(),
        contribution.quantity.value
    )
}

fn plan_dominance_key<V: SolveValue>(plan: &CuttingPlan<V>) -> String {
    let mut contributions = plan
        .demand_contributions
        .iter()
        .map(contribution_quantity_key)
        .collect::<Vec<_>>();
    contributions.sort();
    format!(
        "{}|{}|{}|{}",
        plan.material.id,
        plan.machine_id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default(),
        plan.capacity_consumption
            .as_ref()
            .map(|quantity| format!("{:?}:{}", quantity.value, quantity.unit.symbol()))
            .unwrap_or_default(),
        contributions.join(",")
    )
}

fn plan_relaxed_dominance_key<V: SolveValue>(plan: &CuttingPlan<V>) -> String {
    let mut product_ids = plan
        .demand_contributions
        .iter()
        .map(|contribution| contribution.product.id.to_string())
        .collect::<Vec<_>>();
    product_ids.sort();
    product_ids.dedup();
    format!(
        "{}|{}|{}",
        plan.material.id,
        plan.machine_id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default(),
        product_ids.join(",")
    )
}

fn compare_rest_width<V: SolveValue>(
    new_plan: &CuttingPlan<V>,
    existing_plan: &CuttingPlan<V>,
) -> DominanceComparison {
    let Some(new_rest_width) = new_plan.rest_width() else {
        return DominanceComparison::Incomparable;
    };
    let Some(existing_rest_width) = existing_plan.rest_width() else {
        return DominanceComparison::Incomparable;
    };
    if new_rest_width.unit != existing_rest_width.unit {
        return DominanceComparison::Incomparable;
    }
    match (
        to_f64(&new_rest_width.value),
        to_f64(&existing_rest_width.value),
    ) {
        (Some(new_value), Some(existing_value)) if new_value < existing_value => {
            DominanceComparison::NewDominates
        }
        (Some(_), Some(_)) => DominanceComparison::ExistingDominates,
        _ => DominanceComparison::Incomparable,
    }
}

fn can_cross_contribution_dominate<V: SolveValue>(
    new_plan: &CuttingPlan<V>,
    existing_plan: &CuttingPlan<V>,
) -> bool {
    let Some(new_rest_width) = new_plan.rest_width() else {
        return false;
    };
    let Some(existing_rest_width) = existing_plan.rest_width() else {
        return false;
    };
    if new_rest_width.unit != existing_rest_width.unit {
        return false;
    }
    let Some(new_rest_value) = to_f64(&new_rest_width.value) else {
        return false;
    };
    let Some(existing_rest_value) = to_f64(&existing_rest_width.value) else {
        return false;
    };
    if new_rest_value > existing_rest_value {
        return false;
    }
    existing_plan.demand_contributions.iter().all(|existing| {
        new_plan
            .demand_contributions
            .iter()
            .find(|new| {
                new.product.id == existing.product.id && new.quantity.unit == existing.quantity.unit
            })
            .and_then(|new| {
                Some((
                    to_f64(&new.quantity.value)?,
                    to_f64(&existing.quantity.value)?,
                ))
            })
            .map(|(new_value, existing_value)| new_value >= existing_value)
            .unwrap_or(false)
    })
}

fn merge_contributions<V: SolveValue>(
    contributions: &[CuttingPlanDemandContribution<V>],
) -> Vec<CuttingPlanDemandContribution<V>> {
    let mut merged: Vec<CuttingPlanDemandContribution<V>> = Vec::new();
    for contribution in contributions {
        if let Some(existing) = merged.iter_mut().find(|existing| {
            existing.product.id == contribution.product.id
                && existing.quantity.unit == contribution.quantity.unit
        }) {
            if let (Some(lhs), Some(rhs)) = (
                to_f64(&existing.quantity.value),
                to_f64(&contribution.quantity.value),
            ) {
                if let Some(value) = from_f64(lhs + rhs) {
                    existing.quantity.value = value;
                }
            }
        } else {
            merged.push(contribution.clone());
        }
    }
    merged
}

fn resolve_domain_value_sample<V: SolveValue>(input: &CuttingPlanGenerationInput<V>) -> Option<V> {
    input
        .demands
        .first()
        .map(|demand| demand.quantity.value.clone())
        .or_else(|| {
            input
                .materials
                .first()
                .map(|material| material.width_range.upper_bound.value.clone())
        })
}

fn resolve_generation_canonical_key<V: SolveValue>(
    input: &CuttingPlanGenerationInput<V>,
    plan: &CuttingPlan<V>,
) -> String {
    input
        .canonical_key_overrides
        .iter()
        .find_map(|override_fn| override_fn(plan))
        .or_else(|| {
            input
                .generation_strategies
                .iter()
                .find_map(|strategy| strategy.canonical_key_for(plan))
        })
        .unwrap_or_else(|| plan.canonical_key())
}

fn resolve_pricing_canonical_key<V: SolveValue>(
    input: &Csp1dPricingInput<V>,
    plan: &CuttingPlan<V>,
) -> String {
    input
        .canonical_key_overrides
        .iter()
        .find_map(|override_fn| override_fn(plan))
        .unwrap_or_else(|| resolve_generation_canonical_key(&input.generation_input, plan))
}

fn compute_dual_benefit<V: SolveValue>(
    plan: &CuttingPlan<V>,
    shadow_prices: &crate::domain::material::ShadowPriceMap<V>,
) -> Option<V> {
    let mut benefit = 0.0;
    for contribution in &plan.demand_contributions {
        let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
            product_id: contribution.product.id.clone(),
            unit_symbol: shadow_price_unit_symbol(&contribution.quantity.unit),
        });
        let shadow_price = shadow_prices.get(&key).and_then(to_f64).unwrap_or(0.0);
        let quantity = to_f64(&contribution.quantity.value)?;
        benefit += quantity * shadow_price;
    }
    let material_key = Csp1dShadowPriceKey::MaterialUsage(MaterialUsageShadowPriceKey {
        material_id: plan.material.id.clone(),
    });
    benefit += shadow_prices
        .get(&material_key)
        .and_then(to_f64)
        .unwrap_or(0.0);
    if let Some(machine_id) = &plan.machine_id {
        let batch_key = Csp1dShadowPriceKey::MachineBatch(MachineBatchShadowPriceKey {
            machine_id: machine_id.clone(),
        });
        benefit += shadow_prices
            .get(&batch_key)
            .and_then(to_f64)
            .unwrap_or(0.0);
        let capacity_key = Csp1dShadowPriceKey::MachineCapacity(MachineCapacityShadowPriceKey {
            machine_id: machine_id.clone(),
        });
        let capacity_shadow_price = shadow_prices
            .get(&capacity_key)
            .and_then(to_f64)
            .unwrap_or(0.0);
        if let Some(consumption) = &plan.capacity_consumption {
            benefit += to_f64(&consumption.value)? * capacity_shadow_price;
        }
    }
    from_f64(benefit)
}

fn compute_objective_cost<V: SolveValue>(
    plan: &CuttingPlan<V>,
    objective_config: &Csp1dPricingObjectiveConfig<V>,
) -> Option<V> {
    let mut cost = 1.0;
    cost += objective_config
        .plan_usage_penalty
        .as_ref()
        .and_then(to_f64)
        .unwrap_or(0.0);
    let rest_width = plan
        .rest_width()
        .and_then(|rest| to_f64(&rest.value))
        .unwrap_or(0.0);
    let trim_penalty = objective_config
        .trim_width_penalty
        .as_ref()
        .and_then(to_f64)
        .unwrap_or(0.0);
    if rest_width > 0.0 {
        cost += rest_width * trim_penalty;
    }
    let material_length = plan
        .material
        .length
        .as_ref()
        .and_then(|length| to_f64(&length.value))
        .unwrap_or(0.0);
    let rest_material_penalty = objective_config
        .rest_material_penalty
        .as_ref()
        .and_then(to_f64)
        .unwrap_or(0.0);
    if rest_width > 0.0 && material_length > 0.0 {
        cost += rest_width * material_length * rest_material_penalty;
    }
    cost += objective_config
        .material_cost_penalty
        .get(&plan.material.id)
        .and_then(to_f64)
        .unwrap_or(0.0);
    from_f64(cost)
}

fn is_pricing_improving<V: SolveValue>(
    input: &Csp1dPricingInput<V>,
    plan: &CuttingPlan<V>,
    benefit: &V,
    cost: &V,
) -> bool {
    for judge in &input.is_improving_judges {
        if let Some(result) = judge(plan, benefit, cost) {
            return result;
        }
    }
    for policy in &input.pricing_policies {
        if let Some(result) = policy.is_improving(plan, benefit, cost) {
            return result;
        }
    }
    match (to_f64(benefit), to_f64(cost)) {
        (Some(benefit), Some(cost)) => benefit > cost,
        _ => false,
    }
}

struct PricedCandidate<V: SolveValue> {
    plan: CuttingPlan<V>,
    benefit: V,
    cost: V,
}

#[derive(Debug, Clone)]
struct GenerationEntry<V: SolveValue> {
    demand: crate::domain::material::ProductDemand<V>,
    width: Csp1dQuantity<V>,
}

#[derive(Debug, Clone)]
struct GenerationSliceTemplate<V: SolveValue> {
    slices: Vec<CuttingPlanSlice<V>>,
    demand_contributions: Vec<CuttingPlanDemandContribution<V>>,
}

struct PlanCollector<'a, V: SolveValue> {
    input: &'a CuttingPlanGenerationInput<V>,
    plans: Vec<CuttingPlan<V>>,
    seen_keys: std::collections::HashSet<String>,
    dominance_index: std::collections::HashMap<String, usize>,
    relaxed_dominance_index: std::collections::HashMap<String, usize>,
    max_plans: u64,
    enable_dominance_pruning: bool,
    dominance_strategy: DominanceStrategy,
    statistics: CuttingPlanGenerationStatistics,
}

impl<'a, V: SolveValue> PlanCollector<'a, V> {
    fn new(
        input: &'a CuttingPlanGenerationInput<V>,
        max_plans: u64,
        constraints: Option<&GenerationConstraints<V>>,
    ) -> Self {
        let seen_keys = input
            .existing_plans
            .iter()
            .map(|plan| resolve_generation_canonical_key(input, plan))
            .collect();
        Self {
            input,
            plans: Vec::new(),
            seen_keys,
            dominance_index: std::collections::HashMap::new(),
            relaxed_dominance_index: std::collections::HashMap::new(),
            max_plans,
            enable_dominance_pruning: constraints
                .map(|constraints| constraints.enable_dominance_pruning)
                .unwrap_or(false),
            dominance_strategy: constraints
                .map(|constraints| constraints.dominance_strategy)
                .unwrap_or_default(),
            statistics: CuttingPlanGenerationStatistics::default(),
        }
    }

    fn should_stop(&self) -> bool {
        self.max_plans > 0 && self.plans.len() >= self.max_plans as usize
    }

    fn record(&mut self, plan: CuttingPlan<V>, feasible: bool) {
        self.statistics.generated_candidates += 1;
        if !feasible {
            self.statistics.infeasible_candidates += 1;
            return;
        }
        let key = resolve_generation_canonical_key(self.input, &plan);
        if !self.seen_keys.insert(key) {
            self.statistics.duplicate_candidates += 1;
            return;
        }
        if !accept_generated_plan(self.input, &plan, self.plans.len()) {
            self.seen_keys
                .remove(&resolve_generation_canonical_key(self.input, &plan));
            self.statistics.dominated_candidates += 1;
            return;
        }
        if !accept_generated_dominance(self.input, &plan, &self.plans) {
            self.seen_keys
                .remove(&resolve_generation_canonical_key(self.input, &plan));
            self.statistics.dominated_candidates += 1;
            return;
        }
        if self.enable_dominance_pruning {
            match self.apply_dominance_pruning(&plan) {
                DominanceAction::Accept => {}
                DominanceAction::Reject => {
                    self.seen_keys
                        .remove(&resolve_generation_canonical_key(self.input, &plan));
                    self.statistics.dominated_candidates += 1;
                    return;
                }
                DominanceAction::Replace(index) => {
                    let existing_key =
                        resolve_generation_canonical_key(self.input, &self.plans[index]);
                    self.seen_keys.remove(&existing_key);
                    self.plans[index] = plan;
                    self.statistics.dominated_candidates += 1;
                    self.index_plan_for_dominance(index);
                    return;
                }
            }
        }
        if self.should_stop() {
            self.seen_keys
                .remove(&resolve_generation_canonical_key(self.input, &plan));
            return;
        }
        self.statistics.accepted_plans += 1;
        self.plans.push(plan);
        self.index_plan_for_dominance(self.plans.len() - 1);
        if self.should_stop() {
            self.statistics.stop_reason = CuttingPlanGenerationStopReason::MaxPlans;
        }
    }

    fn apply_dominance_pruning(&mut self, plan: &CuttingPlan<V>) -> DominanceAction {
        let dominance_key = plan_dominance_key(plan);
        if let Some(existing_index) = self.dominance_index.get(&dominance_key).copied() {
            let Some(existing_plan) = self.plans.get(existing_index) else {
                return DominanceAction::Accept;
            };
            return match compare_rest_width(plan, existing_plan) {
                DominanceComparison::NewDominates => DominanceAction::Replace(existing_index),
                DominanceComparison::ExistingDominates => DominanceAction::Reject,
                DominanceComparison::Incomparable => DominanceAction::Accept,
            };
        }
        if self.dominance_strategy == DominanceStrategy::CrossContribution {
            let relaxed_key = plan_relaxed_dominance_key(plan);
            if let Some(existing_index) = self.relaxed_dominance_index.get(&relaxed_key).copied() {
                if let Some(existing_plan) = self.plans.get(existing_index) {
                    if can_cross_contribution_dominate(plan, existing_plan) {
                        self.statistics.cross_contribution_dominated += 1;
                        return DominanceAction::Reject;
                    }
                }
            }
        }
        DominanceAction::Accept
    }

    fn index_plan_for_dominance(&mut self, index: usize) {
        if !self.enable_dominance_pruning {
            return;
        }
        let Some(plan) = self.plans.get(index) else {
            return;
        };
        self.dominance_index.insert(plan_dominance_key(plan), index);
        if self.dominance_strategy == DominanceStrategy::CrossContribution {
            self.relaxed_dominance_index
                .insert(plan_relaxed_dominance_key(plan), index);
        }
    }

    fn finish(self) -> CuttingPlanGenerationReport<V> {
        CuttingPlanGenerationReport {
            plans: self.plans,
            statistics: self.statistics,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DominanceComparison {
    NewDominates,
    ExistingDominates,
    Incomparable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DominanceAction {
    Accept,
    Reject,
    Replace(usize),
}
