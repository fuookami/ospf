//! 切割方案生成模型 / Cutting plan generation models

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{
    to_f64, Costar, Csp1dQuantity, CuttingPlan, CuttingPlanSlice, Machine, Material, Product,
    ProductDemand, ShadowPriceMap, MaterialId,
};
use crate::domain::produce::{Csp1dDomainPolicy, Csp1dGenerationStrategy, Csp1dPricingPolicy};

/// 候选方案过滤器 / Candidate plan filter
pub type Csp1dCandidateFilter<V> =
    Arc<dyn Fn(&CuttingPlan<V>, &[CuttingPlan<V>]) -> bool + Send + Sync>;

/// 幅宽可行性检查 / Width feasibility check
pub type Csp1dWidthFeasibilityCheck<V> =
    Arc<dyn Fn(&Material<V>, &Product<V>, &Csp1dQuantity<V>) -> bool + Send + Sync>;

/// canonical key 覆盖器 / Canonical-key override
pub type Csp1dCanonicalKeyOverride<V> =
    Arc<dyn Fn(&CuttingPlan<V>) -> Option<String> + Send + Sync>;

/// dominance 接受判断 / Dominance acceptance judge
pub type Csp1dDominanceAcceptOverride<V> =
    Arc<dyn Fn(&CuttingPlan<V>, &[CuttingPlan<V>]) -> bool + Send + Sync>;

/// dominance 剪枝策略 / Dominance pruning strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DominanceStrategy {
    /// 相同贡献 / Same contribution
    SameContribution,
    /// 跨贡献 / Cross contribution
    CrossContribution,
}

impl Default for DominanceStrategy {
    fn default() -> Self {
        Self::SameContribution
    }
}

/// 生成约束配置 / Generation constraint configuration
#[derive(Debug, Clone)]
pub struct GenerationConstraints<V: SolveValue> {
    /// 最大刀数 / Maximum knife count
    pub max_knife_count: Option<u64>,
    /// 最小刀数 / Minimum knife count
    pub min_knife_count: Option<u64>,
    /// 最大超产长度 / Maximum over-production length
    pub max_over_produce_length: Option<Csp1dQuantity<V>>,
    /// 按物料并行度 / Material-level parallelism
    pub parallelism: u64,
    /// 是否启用 dominance 剪枝 / Whether dominance pruning is enabled
    pub enable_dominance_pruning: bool,
    /// dominance 策略 / Dominance strategy
    pub dominance_strategy: DominanceStrategy,
}

impl<V: SolveValue> Default for GenerationConstraints<V> {
    fn default() -> Self {
        Self {
            max_knife_count: None,
            min_knife_count: None,
            max_over_produce_length: None,
            parallelism: 1,
            enable_dominance_pruning: false,
            dominance_strategy: DominanceStrategy::default(),
        }
    }
}

impl<V: SolveValue> GenerationConstraints<V> {
    /// 无约束配置 / Unconstrained configuration
    pub fn unconstrained() -> Self {
        Self::default()
    }

    /// 转换为约束列表 / Convert to constraint list
    pub fn to_constraints(&self) -> Vec<Box<dyn CuttingPlanConstraint<V>>> {
        let mut constraints: Vec<Box<dyn CuttingPlanConstraint<V>>> = Vec::new();
        if let Some(max_knife_count) = self.max_knife_count {
            constraints.push(Box::new(MaxKnifeCountConstraint {
                value: max_knife_count,
            }));
        }
        if let Some(min_knife_count) = self.min_knife_count {
            constraints.push(Box::new(MinKnifeCountConstraint {
                value: min_knife_count,
            }));
        }
        if let Some(max_over_produce_length) = &self.max_over_produce_length {
            constraints.push(Box::new(MaxOverProduceLengthConstraint {
                value: max_over_produce_length.clone(),
            }));
        }
        constraints.push(Box::new(WidthUpperBoundConstraint));
        constraints
    }
}

/// 切割方案约束评估上下文 / Cutting plan constraint evaluation context
#[derive(Debug, Clone)]
pub struct CuttingPlanConstraintContext<V: SolveValue> {
    /// 当前切片 / Current slices
    pub slices: Vec<CuttingPlanSlice<V>>,
    /// 累计宽度 / Total width
    pub total_width: Csp1dQuantity<V>,
    /// 物料幅宽上界 / Material upper bound
    pub upper_bound: Csp1dQuantity<V>,
    /// 物料 / Material
    pub material: Material<V>,
}

/// 切割方案约束 / Cutting plan constraint
pub trait CuttingPlanConstraint<V: SolveValue>: Send + Sync {
    /// 约束名称 / Constraint name
    fn name(&self) -> &'static str;

    /// 是否为剪枝约束 / Whether this is a pruning constraint
    fn is_pruning(&self) -> bool {
        true
    }

    /// 判断是否满足 / Check whether satisfied
    fn is_satisfied(&self, context: &CuttingPlanConstraintContext<V>) -> bool;
}

/// 最大刀数约束 / Max knife count constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxKnifeCountConstraint {
    /// 最大刀数 / Max knife count
    pub value: u64,
}

impl<V: SolveValue> CuttingPlanConstraint<V> for MaxKnifeCountConstraint {
    fn name(&self) -> &'static str {
        "MaxKnifeCountConstraint"
    }

    fn is_satisfied(&self, context: &CuttingPlanConstraintContext<V>) -> bool {
        total_slice_amount(&context.slices) <= self.value
    }
}

/// 最小刀数约束 / Min knife count constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinKnifeCountConstraint {
    /// 最小刀数 / Min knife count
    pub value: u64,
}

impl<V: SolveValue> CuttingPlanConstraint<V> for MinKnifeCountConstraint {
    fn name(&self) -> &'static str {
        "MinKnifeCountConstraint"
    }

    fn is_pruning(&self) -> bool {
        false
    }

    fn is_satisfied(&self, context: &CuttingPlanConstraintContext<V>) -> bool {
        total_slice_amount(&context.slices) >= self.value
    }
}

/// 最大超产长度约束 / Max over-produce length constraint
#[derive(Debug, Clone, PartialEq)]
pub struct MaxOverProduceLengthConstraint<V: SolveValue> {
    /// 最大超产长度 / Max over-produce length
    pub value: Csp1dQuantity<V>,
}

impl<V: SolveValue> CuttingPlanConstraint<V> for MaxOverProduceLengthConstraint<V> {
    fn name(&self) -> &'static str {
        "MaxOverProduceLengthConstraint"
    }

    fn is_satisfied(&self, context: &CuttingPlanConstraintContext<V>) -> bool {
        context.slices.iter().all(|slice| {
            let Some(length) = slice.production.length() else {
                return true;
            };
            if length.unit != self.value.unit {
                return false;
            }
            let Some(length) = to_f64(&length.value) else {
                return false;
            };
            let Some(limit) = to_f64(&self.value.value) else {
                return false;
            };
            length <= limit
        })
    }
}

/// 幅宽上界约束 / Width upper bound constraint
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WidthUpperBoundConstraint;

impl<V: SolveValue> CuttingPlanConstraint<V> for WidthUpperBoundConstraint {
    fn name(&self) -> &'static str {
        "WidthUpperBoundConstraint"
    }

    fn is_satisfied(&self, context: &CuttingPlanConstraintContext<V>) -> bool {
        if context.total_width.unit != context.upper_bound.unit {
            return false;
        }
        let Some(total_width) = to_f64(&context.total_width.value) else {
            return false;
        };
        let Some(upper_bound) = to_f64(&context.upper_bound.value) else {
            return false;
        };
        total_width <= upper_bound
    }
}

fn total_slice_amount<V: SolveValue>(slices: &[CuttingPlanSlice<V>]) -> u64 {
    slices.iter().map(|slice| slice.amount).sum()
}

/// 定价成本修正器 / Pricing cost modifier
pub type Csp1dPricingCostModifier<V> = Arc<dyn Fn(&CuttingPlan<V>, V) -> V + Send + Sync>;

/// 定价收益修正器 / Pricing benefit modifier
pub type Csp1dPricingBenefitModifier<V> = Arc<dyn Fn(&CuttingPlan<V>, V) -> V + Send + Sync>;

/// 定价改善判断器 / Pricing improvement judge
pub type Csp1dIsImprovingJudge<V> =
    Arc<dyn Fn(&CuttingPlan<V>, &V, &V) -> Option<bool> + Send + Sync>;

/// 切割方案生成输入 / Cutting plan generation input
#[derive(Clone)]
pub struct CuttingPlanGenerationInput<V: SolveValue> {
    /// 产品列表 / Product list
    pub products: Vec<Product<V>>,
    /// 物料列表 / Material list
    pub materials: Vec<Material<V>>,
    /// 设备列表 / Machine list
    pub machines: Vec<Machine<V>>,
    /// 配规列表 / Costar list
    pub costars: Vec<Costar<V>>,
    /// 需求列表 / Demand list
    pub demands: Vec<ProductDemand<V>>,
    /// 已有方案 / Existing plans
    pub existing_plans: Vec<CuttingPlan<V>>,
    /// 领域策略 / Domain policies
    pub domain_policies: Vec<Arc<dyn Csp1dDomainPolicy<V>>>,
    /// 生成策略 / Generation strategies
    pub generation_strategies: Vec<Arc<dyn Csp1dGenerationStrategy<V>>>,
    /// 候选方案过滤器 / Candidate filters
    pub candidate_filters: Vec<Csp1dCandidateFilter<V>>,
    /// 幅宽可行性检查 / Width feasibility check
    pub width_feasibility_check: Option<Csp1dWidthFeasibilityCheck<V>>,
    /// canonical key 覆盖器 / Canonical-key overrides
    pub canonical_key_overrides: Vec<Csp1dCanonicalKeyOverride<V>>,
    /// dominance 接受判断 / Dominance acceptance overrides
    pub dominance_accept_overrides: Vec<Csp1dDominanceAcceptOverride<V>>,
}

impl<V: SolveValue> std::fmt::Debug for CuttingPlanGenerationInput<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CuttingPlanGenerationInput")
            .field("products", &self.products.len())
            .field("materials", &self.materials.len())
            .field("machines", &self.machines.len())
            .field("costars", &self.costars.len())
            .field("demands", &self.demands.len())
            .field("existing_plans", &self.existing_plans.len())
            .field("domain_policies", &self.domain_policies.len())
            .field("generation_strategies", &self.generation_strategies.len())
            .field("candidate_filters", &self.candidate_filters.len())
            .field("has_width_feasibility_check", &self.width_feasibility_check.is_some())
            .field("canonical_key_overrides", &self.canonical_key_overrides.len())
            .field("dominance_accept_overrides", &self.dominance_accept_overrides.len())
            .finish()
    }
}

impl<V: SolveValue> Default for CuttingPlanGenerationInput<V> {
    fn default() -> Self {
        Self {
            products: Vec::new(),
            materials: Vec::new(),
            machines: Vec::new(),
            costars: Vec::new(),
            demands: Vec::new(),
            existing_plans: Vec::new(),
            domain_policies: Vec::new(),
            generation_strategies: Vec::new(),
            candidate_filters: Vec::new(),
            width_feasibility_check: None,
            canonical_key_overrides: Vec::new(),
            dominance_accept_overrides: Vec::new(),
        }
    }
}

/// 生成终止原因 / Generation stop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuttingPlanGenerationStopReason {
    /// 搜索穷尽 / Search exhausted
    Exhausted,
    /// 达到最大方案数 / Reached maximum plan count
    MaxPlans,
    /// 超时 / Timed out
    Timeout,
}

/// 生成统计 / Generation statistics
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CuttingPlanGenerationStatistics {
    /// 访问节点数 / Visited node count
    pub visited_nodes: i64,
    /// 生成候选数 / Generated candidate count
    pub generated_candidates: i64,
    /// 接受方案数 / Accepted plan count
    pub accepted_plans: i64,
    /// 不可行候选数 / Infeasible candidate count
    pub infeasible_candidates: i64,
    /// 重复候选数 / Duplicate candidate count
    pub duplicate_candidates: i64,
    /// 被支配候选数 / Dominated candidate count
    pub dominated_candidates: i64,
    /// 幅宽上界剪枝节点数 / Width-bound pruned node count
    pub width_bound_pruned_nodes: i64,
    /// 刀数约束剪枝节点数 / Knife-bound pruned node count
    pub knife_bound_pruned_nodes: i64,
    /// 长度约束剪枝条目数 / Length-bound pruned entry count
    pub length_bound_pruned_entries: i64,
    /// 物料幅宽索引缓存命中数 / Material width index cache hit count
    pub material_width_index_cache_hits: i64,
    /// 物料切片模板缓存命中数 / Material slice template cache hit count
    pub material_slice_template_cache_hits: i64,
    /// 数量缓存命中数 / Quantity cache hit count
    pub quantity_cache_hits: i64,
    /// 数量缓存未命中数 / Quantity cache miss count
    pub quantity_cache_misses: i64,
    /// 物料切片模板缓存未命中数 / Material slice template cache miss count
    pub material_slice_template_cache_misses: i64,
    /// 跨工作线程重复候选数 / Cross-worker duplicate candidate count
    pub cross_worker_duplicate_candidates: i64,
    /// 跨贡献支配数 / Cross-contribution dominated count
    pub cross_contribution_dominated: i64,
    /// 耗时毫秒 / Elapsed milliseconds
    pub elapsed_milliseconds: i64,
    /// 停止原因 / Stop reason
    pub stop_reason: CuttingPlanGenerationStopReason,
}

impl Default for CuttingPlanGenerationStopReason {
    fn default() -> Self {
        Self::Exhausted
    }
}

impl CuttingPlanGenerationStopReason {
    /// 稳定名称 / Stable name
    pub fn stable_name(&self) -> &'static str {
        match self {
            Self::Exhausted => "Exhausted",
            Self::MaxPlans => "MaxPlans",
            Self::Timeout => "Timeout",
        }
    }
}

/// benchmark 快照 / Benchmark snapshot
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuttingPlanGenerationBenchmarkSnapshot {
    /// 生成器名称 / Generator name
    pub generator_name: String,
    /// 访问节点数 / Visited node count
    pub visited_nodes: i64,
    /// 生成候选数 / Generated candidate count
    pub generated_candidates: i64,
    /// 接受方案数 / Accepted plan count
    pub accepted_plans: i64,
    /// 不可行候选数 / Infeasible candidate count
    pub infeasible_candidates: i64,
    /// 重复候选数 / Duplicate candidate count
    pub duplicate_candidates: i64,
    /// 被支配候选数 / Dominated candidate count
    pub dominated_candidates: i64,
    /// 幅宽上界剪枝节点数 / Width-bound pruned node count
    pub width_bound_pruned_nodes: i64,
    /// 刀数约束剪枝节点数 / Knife-bound pruned node count
    pub knife_bound_pruned_nodes: i64,
    /// 长度约束剪枝条目数 / Length-bound pruned entry count
    pub length_bound_pruned_entries: i64,
    /// 物料幅宽索引缓存命中数 / Material width index cache hit count
    pub material_width_index_cache_hits: i64,
    /// 物料切片模板缓存命中数 / Material slice template cache hit count
    pub material_slice_template_cache_hits: i64,
    /// 数量缓存命中数 / Quantity cache hit count
    pub quantity_cache_hits: i64,
    /// 数量缓存未命中数 / Quantity cache miss count
    pub quantity_cache_misses: i64,
    /// 物料切片模板缓存未命中数 / Material slice template cache miss count
    pub material_slice_template_cache_misses: i64,
    /// 跨工作线程重复候选数 / Cross-worker duplicate candidate count
    pub cross_worker_duplicate_candidates: i64,
    /// 跨贡献支配数 / Cross-contribution dominated count
    pub cross_contribution_dominated: i64,
    /// 停止原因 / Stop reason
    pub stop_reason: CuttingPlanGenerationStopReason,
}

impl CuttingPlanGenerationBenchmarkSnapshot {
    /// 从统计构建快照 / Build snapshot from statistics
    pub fn from_statistics(
        generator_name: impl Into<String>,
        statistics: &CuttingPlanGenerationStatistics,
    ) -> Self {
        Self {
            generator_name: generator_name.into(),
            visited_nodes: statistics.visited_nodes,
            generated_candidates: statistics.generated_candidates,
            accepted_plans: statistics.accepted_plans,
            infeasible_candidates: statistics.infeasible_candidates,
            duplicate_candidates: statistics.duplicate_candidates,
            dominated_candidates: statistics.dominated_candidates,
            width_bound_pruned_nodes: statistics.width_bound_pruned_nodes,
            knife_bound_pruned_nodes: statistics.knife_bound_pruned_nodes,
            length_bound_pruned_entries: statistics.length_bound_pruned_entries,
            material_width_index_cache_hits: statistics.material_width_index_cache_hits,
            material_slice_template_cache_hits: statistics.material_slice_template_cache_hits,
            quantity_cache_hits: statistics.quantity_cache_hits,
            quantity_cache_misses: statistics.quantity_cache_misses,
            material_slice_template_cache_misses: statistics.material_slice_template_cache_misses,
            cross_worker_duplicate_candidates: statistics.cross_worker_duplicate_candidates,
            cross_contribution_dominated: statistics.cross_contribution_dominated,
            stop_reason: statistics.stop_reason,
        }
    }

    /// 输出稳定文本行 / Render stable text line
    pub fn to_stable_line(&self) -> String {
        format!(
            "generator={};visitedNodes={};generatedCandidates={};acceptedPlans={};infeasibleCandidates={};duplicateCandidates={};dominatedCandidates={};widthBoundPrunedNodes={};knifeBoundPrunedNodes={};lengthBoundPrunedEntries={};materialWidthIndexCacheHits={};materialSliceTemplateCacheHits={};quantityCacheHits={};quantityCacheMisses={};materialSliceTemplateCacheMisses={};crossWorkerDuplicateCandidates={};crossContributionDominated={};stopReason={}",
            self.generator_name,
            self.visited_nodes,
            self.generated_candidates,
            self.accepted_plans,
            self.infeasible_candidates,
            self.duplicate_candidates,
            self.dominated_candidates,
            self.width_bound_pruned_nodes,
            self.knife_bound_pruned_nodes,
            self.length_bound_pruned_entries,
            self.material_width_index_cache_hits,
            self.material_slice_template_cache_hits,
            self.quantity_cache_hits,
            self.quantity_cache_misses,
            self.material_slice_template_cache_misses,
            self.cross_worker_duplicate_candidates,
            self.cross_contribution_dominated,
            self.stop_reason.stable_name(),
        )
    }
}

/// 生成报告 / Generation report
#[derive(Debug, Clone)]
pub struct CuttingPlanGenerationReport<V: SolveValue> {
    /// 生成的切割方案 / Generated cutting plans
    pub plans: Vec<CuttingPlan<V>>,
    /// 生成统计 / Generation statistics
    pub statistics: CuttingPlanGenerationStatistics,
}

/// 生成报告合并配置 / Generation report merge options
#[derive(Clone)]
pub struct GenerationReportMergeOptions<V: SolveValue> {
    /// 最大方案数 / Maximum plan count
    pub max_plans: u64,
    /// 起始时间 / Start time
    pub started_at: Instant,
    /// 截止时间 / Deadline
    pub deadline: Option<Instant>,
    /// canonical key 覆盖器 / Canonical-key override
    pub canonical_key_override: Option<Csp1dCanonicalKeyOverride<V>>,
}

impl<V: SolveValue> std::fmt::Debug for GenerationReportMergeOptions<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerationReportMergeOptions")
            .field("max_plans", &self.max_plans)
            .field("deadline", &self.deadline)
            .field(
                "has_canonical_key_override",
                &self.canonical_key_override.is_some(),
            )
            .finish()
    }
}

impl<V: SolveValue> GenerationReportMergeOptions<V> {
    /// 创建合并配置 / Create merge options
    pub fn new(max_plans: u64) -> Self {
        Self {
            max_plans,
            started_at: Instant::now(),
            deadline: None,
            canonical_key_override: None,
        }
    }

    /// 设置起始时间 / Set start time
    pub fn with_started_at(mut self, started_at: Instant) -> Self {
        self.started_at = started_at;
        self
    }

    /// 设置截止时间 / Set deadline
    pub fn with_deadline(mut self, deadline: Option<Instant>) -> Self {
        self.deadline = deadline;
        self
    }

    /// 设置最大运行时长 / Set maximum elapsed duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.deadline = Some(self.started_at + timeout);
        self
    }

    /// 设置 canonical key 覆盖器 / Set canonical-key override
    pub fn with_canonical_key_override(
        mut self,
        canonical_key_override: Csp1dCanonicalKeyOverride<V>,
    ) -> Self {
        self.canonical_key_override = Some(canonical_key_override);
        self
    }
}

/// 合并生成报告 / Merge generation reports
pub fn merge_generation_reports<V: SolveValue>(
    reports: Vec<CuttingPlanGenerationReport<V>>,
    options: GenerationReportMergeOptions<V>,
) -> CuttingPlanGenerationReport<V> {
    let mut canonical_keys = HashSet::new();
    let mut plans = Vec::new();
    let mut statistics = sum_generation_statistics(&reports);
    statistics.duplicate_candidates = reports
        .iter()
        .map(|report| report.statistics.duplicate_candidates)
        .sum();
    statistics.cross_worker_duplicate_candidates = 0;

    for report in &reports {
        for plan in &report.plans {
            if plans.len() as u64 >= options.max_plans {
                break;
            }
            let key = options
                .canonical_key_override
                .as_ref()
                .and_then(|override_fn| override_fn(plan))
                .unwrap_or_else(|| plan.canonical_key());
            if !canonical_keys.insert(key) {
                statistics.duplicate_candidates += 1;
                statistics.cross_worker_duplicate_candidates += 1;
                continue;
            }
            plans.push(plan.clone());
        }
        if plans.len() as u64 >= options.max_plans {
            break;
        }
    }

    let timed_out = reports
        .iter()
        .any(|report| report.statistics.stop_reason == CuttingPlanGenerationStopReason::Timeout)
        || options.deadline.is_some_and(|deadline| Instant::now() > deadline);
    statistics.accepted_plans = plans.len() as i64;
    statistics.elapsed_milliseconds = options.started_at.elapsed().as_millis() as i64;
    statistics.stop_reason = if timed_out {
        CuttingPlanGenerationStopReason::Timeout
    } else if plans.len() as u64 >= options.max_plans {
        CuttingPlanGenerationStopReason::MaxPlans
    } else {
        CuttingPlanGenerationStopReason::Exhausted
    };

    CuttingPlanGenerationReport { plans, statistics }
}

fn sum_generation_statistics<V: SolveValue>(
    reports: &[CuttingPlanGenerationReport<V>],
) -> CuttingPlanGenerationStatistics {
    reports.iter().fold(
        CuttingPlanGenerationStatistics::default(),
        |mut acc, report| {
            let statistics = &report.statistics;
            acc.visited_nodes += statistics.visited_nodes;
            acc.generated_candidates += statistics.generated_candidates;
            acc.infeasible_candidates += statistics.infeasible_candidates;
            acc.duplicate_candidates += statistics.duplicate_candidates;
            acc.dominated_candidates += statistics.dominated_candidates;
            acc.width_bound_pruned_nodes += statistics.width_bound_pruned_nodes;
            acc.knife_bound_pruned_nodes += statistics.knife_bound_pruned_nodes;
            acc.length_bound_pruned_entries += statistics.length_bound_pruned_entries;
            acc.material_width_index_cache_hits += statistics.material_width_index_cache_hits;
            acc.material_slice_template_cache_hits += statistics.material_slice_template_cache_hits;
            acc.quantity_cache_hits += statistics.quantity_cache_hits;
            acc.quantity_cache_misses += statistics.quantity_cache_misses;
            acc.material_slice_template_cache_misses +=
                statistics.material_slice_template_cache_misses;
            acc.cross_worker_duplicate_candidates +=
                statistics.cross_worker_duplicate_candidates;
            acc.cross_contribution_dominated += statistics.cross_contribution_dominated;
            acc
        },
    )
}

/// 定价输入 / Pricing input
#[derive(Clone)]
pub struct Csp1dPricingInput<V: SolveValue> {
    /// 生成输入 / Generation input
    pub generation_input: CuttingPlanGenerationInput<V>,
    /// 影子价格 / Shadow prices
    pub shadow_prices: ShadowPriceMap<V>,
    /// 最大生成方案数 / Maximum generated plan count
    pub max_generated_plans: u64,
    /// 定价目标配置 / Pricing objective configuration
    pub objective_config: Csp1dPricingObjectiveConfig<V>,
    /// 定价成本修正器 / Pricing cost modifiers
    pub pricing_cost_modifiers: Vec<Csp1dPricingCostModifier<V>>,
    /// 定价收益修正器 / Pricing benefit modifiers
    pub pricing_benefit_modifiers: Vec<Csp1dPricingBenefitModifier<V>>,
    /// 改善判断器 / Improvement judges
    pub is_improving_judges: Vec<Csp1dIsImprovingJudge<V>>,
    /// pricing 级 canonical key 覆盖器 / Pricing-level canonical-key overrides
    pub canonical_key_overrides: Vec<Csp1dCanonicalKeyOverride<V>>,
    /// 定价策略 / Pricing policies
    pub pricing_policies: Vec<Arc<dyn Csp1dPricingPolicy<V>>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dPricingInput<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dPricingInput")
            .field("generation_input", &self.generation_input)
            .field("shadow_prices", &self.shadow_prices.len())
            .field("max_generated_plans", &self.max_generated_plans)
            .field("objective_config", &self.objective_config)
            .field("pricing_cost_modifiers", &self.pricing_cost_modifiers.len())
            .field("pricing_benefit_modifiers", &self.pricing_benefit_modifiers.len())
            .field("is_improving_judges", &self.is_improving_judges.len())
            .field("canonical_key_overrides", &self.canonical_key_overrides.len())
            .field("pricing_policies", &self.pricing_policies.len())
            .finish()
    }
}

impl<V: SolveValue> Default for Csp1dPricingInput<V> {
    fn default() -> Self {
        Self {
            generation_input: CuttingPlanGenerationInput::default(),
            shadow_prices: ShadowPriceMap::new(),
            max_generated_plans: 1,
            objective_config: Csp1dPricingObjectiveConfig::default(),
            pricing_cost_modifiers: Vec::new(),
            pricing_benefit_modifiers: Vec::new(),
            is_improving_judges: Vec::new(),
            canonical_key_overrides: Vec::new(),
            pricing_policies: Vec::new(),
        }
    }
}

/// 定价目标配置 / Pricing objective configuration
#[derive(Debug, Clone)]
pub struct Csp1dPricingObjectiveConfig<V: SolveValue> {
    /// 方案使用惩罚 / Plan usage penalty
    pub plan_usage_penalty: Option<V>,
    /// 余宽惩罚 / Trim width penalty
    pub trim_width_penalty: Option<V>,
    /// 余料惩罚 / Rest material penalty
    pub rest_material_penalty: Option<V>,
    /// 物料成本惩罚 / Material cost penalties
    pub material_cost_penalty: std::collections::BTreeMap<MaterialId, V>,
}

impl<V: SolveValue> Default for Csp1dPricingObjectiveConfig<V> {
    fn default() -> Self {
        Self {
            plan_usage_penalty: None,
            trim_width_penalty: None,
            rest_material_penalty: None,
            material_cost_penalty: std::collections::BTreeMap::new(),
        }
    }
}
