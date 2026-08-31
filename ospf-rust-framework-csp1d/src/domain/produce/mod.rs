//! 主问题产出与扩展点 / Master problem output and extension points

pub mod aggregation;
pub mod builder;
pub mod context;
pub mod extraction;
pub mod model;
pub mod pipeline;
pub mod shadow_price;

use std::collections::BTreeMap;
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    from_f64, to_f64, CuttingPlan, Material, Machine, ProductDemand,
    Csp1dQuantity,
};

pub use aggregation::*;
pub use builder::*;
pub use context::*;
pub use pipeline::*;
pub use shadow_price::*;

/// 切割方案使用量 / Cutting plan usage
#[derive(Debug, Clone)]
pub struct CuttingPlanUsage<V: SolveValue> {
    pub plan: CuttingPlan<V>,
    pub amount: u64,
}

/// 物料使用量 / Material usage
#[derive(Debug, Clone)]
pub struct MaterialUsage<V: SolveValue> {
    pub material: Material<V>,
    pub amount: u64,
}

/// 设备产能使用 / Machine capacity usage
#[derive(Debug, Clone)]
pub struct MachineCapacityUsage<V: SolveValue> {
    pub machine: Machine<V>,
    pub used: Option<Csp1dQuantity<V>>,
}

/// 主问题求解产出 / Master problem output
#[derive(Debug, Clone)]
pub struct Produce<V: SolveValue> {
    pub cutting_plans: Vec<CuttingPlanUsage<V>>,
    pub material_usages: Vec<MaterialUsage<V>>,
    pub machine_usages: Vec<MachineCapacityUsage<V>>,
    pub unmet_demands: Vec<ProductDemand<V>>,
}

impl<V: SolveValue> Default for Produce<V> {
    fn default() -> Self {
        Self {
            cutting_plans: Vec::new(),
            material_usages: Vec::new(),
            machine_usages: Vec::new(),
            unmet_demands: Vec::new(),
        }
    }
}

/// 需求贡献聚合键 / Contribution aggregation key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContributionKey {
    pub product_id: String,
    pub unit_symbol: String,
}

/// 产出输入 / Produce input
#[derive(Debug, Clone)]
pub struct ProduceInput<V: SolveValue> {
    /// 切割方案池 / Cutting plan pool
    pub cutting_plans: Vec<CuttingPlan<V>>,
    /// 需求列表 / Demand list
    pub demands: Vec<ProductDemand<V>>,
    /// 物料列表 / Material list
    pub materials: Vec<Material<V>>,
    /// 设备列表 / Machine list
    pub machines: Vec<Machine<V>>,
    /// warm start 方案使用量 / Warm-start plan usages
    pub warm_start_plan_usages: Vec<CuttingPlanUsage<V>>,
}

impl<V: SolveValue> Default for ProduceInput<V> {
    fn default() -> Self {
        Self {
            cutting_plans: Vec::new(),
            demands: Vec::new(),
            materials: Vec::new(),
            machines: Vec::new(),
            warm_start_plan_usages: Vec::new(),
        }
    }
}

impl<V: SolveValue> Produce<V> {
    pub fn contributions(
        &self,
    ) -> BTreeMap<ContributionKey, Vec<crate::domain::material::CuttingPlanDemandContribution<V>>> {
        let mut contributions = BTreeMap::new();
        for usage in &self.cutting_plans {
            for contribution in &usage.plan.demand_contributions {
                let key = ContributionKey {
                    product_id: contribution.product.id.clone(),
                    unit_symbol: String::new(),
                };
                contributions.entry(key).or_insert_with(Vec::new).push(contribution.clone());
            }
        }
        contributions
    }
}

/// 扩展模式 / Extension mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dModelingMode {
    MILP,
    LP,
}

/// 扩展适用模式 / Extension applicable mode
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dExtensionMode {
    MILP,
    LP,
    FINAL_MILP,
    ALL,
}

impl Csp1dExtensionMode {
    pub fn matches(&self, mode: Csp1dModelingMode, is_final_milp: bool) -> bool {
        match self {
            Self::MILP => mode == Csp1dModelingMode::MILP && !is_final_milp,
            Self::LP => mode == Csp1dModelingMode::LP,
            Self::FINAL_MILP => is_final_milp,
            Self::ALL => true,
        }
    }
}

/// 建模扩展 / Modeling extension
#[derive(Clone)]
pub struct Csp1dModelingExtension<V: SolveValue> {
    pub pipeline: Option<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>>,
    pub mode: Csp1dExtensionMode,
    pub context_aware_pipeline: Option<Arc<dyn Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync> + Send + Sync>>,
}

impl<V: SolveValue> Csp1dModelingExtension<V> {
    pub fn new(pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>) -> Self {
        Self {
            pipeline: Some(pipeline),
            mode: Csp1dExtensionMode::ALL,
            context_aware_pipeline: None,
        }
    }

    pub fn with_mode(
        pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>,
        mode: Csp1dExtensionMode,
    ) -> Self {
        Self {
            pipeline: Some(pipeline),
            mode,
            context_aware_pipeline: None,
        }
    }

    pub fn context_aware(
        context_aware_pipeline: Arc<dyn Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync> + Send + Sync>,
    ) -> Self {
        Self {
            pipeline: None,
            mode: Csp1dExtensionMode::ALL,
            context_aware_pipeline: Some(context_aware_pipeline),
        }
    }

    pub fn context_aware_with_mode(
        context_aware_pipeline: Arc<dyn Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync> + Send + Sync>,
        mode: Csp1dExtensionMode,
    ) -> Self {
        Self {
            pipeline: None,
            mode,
            context_aware_pipeline: Some(context_aware_pipeline),
        }
    }

    pub fn resolve_pipeline(
        &self,
        context: Option<&dyn Csp1dModelingContext<V>>,
    ) -> Option<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>> {
        if let (Some(factory), Some(context)) = (&self.context_aware_pipeline, context) {
            Some(factory(context))
        } else {
            self.pipeline.clone()
        }
    }
}

impl<V: SolveValue> std::fmt::Debug for Csp1dModelingExtension<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dModelingExtension")
            .field("has_pipeline", &self.pipeline.is_some())
            .field("mode", &self.mode)
            .field("has_context_aware_pipeline", &self.context_aware_pipeline.is_some())
            .finish()
    }
}

/// 建模上下文 / Modeling context
pub trait Csp1dModelingContext<V: SolveValue> {
    fn mode(&self) -> Csp1dModelingMode;
    fn is_final_milp(&self) -> bool;
    fn produce(&self) -> &ProduceAggregation<V>;
    fn demands(&self) -> &[ProductDemand<V>];
    fn materials(&self) -> &[Material<V>];
    fn machines(&self) -> &[Machine<V>];
    fn cutting_plans(&self) -> &[CuttingPlan<V>] {
        self.produce().cutting_plans()
    }
    fn domain_value_sample(&self) -> Option<V>;
    fn to_domain_value(&self, value: f64) -> V;
}

/// 领域计算上下文 / Domain calculation context
pub trait Csp1dDomainCalculationContext<V: SolveValue> {
    fn plan(&self) -> &CuttingPlan<V>;

    fn plan_index(&self) -> usize {
        0
    }

    fn material(&self) -> &Material<V> {
        &self.plan().material
    }

    fn machine_id(&self) -> Option<&str> {
        self.plan().machine_id.as_deref()
    }

    fn slices(&self) -> &[crate::domain::material::CuttingPlanSlice<V>] {
        &self.plan().slices
    }

    fn demand_contributions(
        &self,
    ) -> &[crate::domain::material::CuttingPlanDemandContribution<V>] {
        &self.plan().demand_contributions
    }

    fn domain_value_sample(&self) -> Option<V> {
        None
    }

    fn to_domain_value(&self, value: f64) -> Option<V> {
        let _ = value;
        None
    }

    fn contribution_for(&self, product_id: &str) -> Option<f64> {
        self.demand_contributions()
            .iter()
            .find(|contribution| contribution.product.id == product_id)
            .and_then(|contribution| to_f64(&contribution.quantity.value))
    }
}

/// 方案判断上下文 / Plan judgment context
pub trait Csp1dPlanJudgmentContext<V: SolveValue>: Csp1dDomainCalculationContext<V> {
    fn same_material_plan_indices(&self) -> &[usize];
    fn same_machine_plan_indices(&self) -> &[usize];
    fn all_plans(&self) -> &[CuttingPlan<V>];
}

/// 领域策略 / Domain policy
pub trait Csp1dDomainPolicy<V: SolveValue>: Send + Sync {
    fn name(&self) -> &str;

    fn overrides_width_feasibility(&self) -> bool {
        false
    }

    fn is_feasible(&self, _context: &dyn Csp1dDomainCalculationContext<V>) -> bool {
        true
    }

    fn is_width_feasible(&self, _context: &dyn Csp1dDomainCalculationContext<V>) -> bool {
        true
    }
}

/// 目标策略 / Objective policy
pub trait Csp1dObjectivePolicy<V: SolveValue>: Send + Sync {
    fn name(&self) -> &str;

    fn modify_batch_coefficient(
        &self,
        _context: &dyn Csp1dDomainCalculationContext<V>,
        base_coefficient: f64,
    ) -> f64 {
        base_coefficient
    }
}

/// 生成策略 / Generation strategy
pub trait Csp1dGenerationStrategy<V: SolveValue>: Send + Sync {
    fn name(&self) -> &str;

    fn accept_candidate(
        &self,
        _candidate: &CuttingPlan<V>,
        _existing_plans: &[CuttingPlan<V>],
    ) -> bool {
        true
    }

    fn canonical_key_for(&self, _candidate: &CuttingPlan<V>) -> Option<String> {
        None
    }

    fn accept_dominance(
        &self,
        _candidate: &CuttingPlan<V>,
        _existing_plans: &[CuttingPlan<V>],
    ) -> bool {
        true
    }
}

/// 定价策略 / Pricing policy
pub trait Csp1dPricingPolicy<V: SolveValue>: Send + Sync {
    fn name(&self) -> &str;

    fn modify_cost(&self, _candidate: &CuttingPlan<V>, base_cost: V) -> V {
        base_cost
    }

    fn modify_benefit(&self, _candidate: &CuttingPlan<V>, base_benefit: V) -> V {
        base_benefit
    }

    fn is_improving(
        &self,
        _candidate: &CuttingPlan<V>,
        _benefit: &V,
        _cost: &V,
    ) -> Option<bool> {
        None
    }
}

/// 流程上下文 / Flow context
pub trait Csp1dFlowContext<V: SolveValue> {
    fn iteration(&self) -> u64;

    fn current_plans(&self) -> &[CuttingPlan<V>];

    fn iteration_limit(&self) -> u64;

    fn allow_partial_solution(&self) -> bool;

    fn new_plans(&self) -> &[CuttingPlan<V>] {
        &[]
    }

    fn pricing_statistics(&self) -> Option<&crate::domain::cutting_plan_generation::CuttingPlanGenerationStatistics> {
        None
    }

    fn has_valid_lp_result(&self) -> bool {
        false
    }

    fn warm_start_plan_count(&self) -> u64 {
        0
    }

    fn warm_start_requires_fallback(&self) -> bool {
        false
    }
}

/// 流程策略 / Flow policy
pub trait Csp1dFlowPolicy<V: SolveValue>: Send + Sync {
    fn name(&self) -> &str;

    fn filter_initial_plans(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        plans: Vec<CuttingPlan<V>>,
    ) -> Vec<CuttingPlan<V>> {
        plans
    }

    fn is_equivalent(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        _existing: &CuttingPlan<V>,
        _candidate: &CuttingPlan<V>,
    ) -> bool {
        false
    }

    fn should_stop_iteration(&self, _context: &dyn Csp1dFlowContext<V>) -> bool {
        false
    }

    fn select_termination(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        default_reason: String,
        default_message: Option<String>,
    ) -> (String, Option<String>) {
        (default_reason, default_message)
    }

    fn accept_partial(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        default_decision: bool,
    ) -> bool {
        default_decision
    }

    fn allow_recovery_fallback(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        default_decision: bool,
    ) -> bool {
        default_decision
    }
}

/// 提取策略 / Extraction policy
pub trait Csp1dExtractionPolicy<V: SolveValue>: Send + Sync {
    fn name(&self) -> &str;

    fn enrich_output(
        &self,
        _details: &mut BTreeMap<String, String>,
        _render_kpi: &mut BTreeMap<String, String>,
        _produce: &Produce<V>,
        _demands: &[ProductDemand<V>],
        _materials: &[Material<V>],
        _machines: &[Machine<V>],
        _generated_plans: &[CuttingPlan<V>],
        _iteration_count: u64,
        _termination_reason: Option<&str>,
        _final_milp_status: Option<&str>,
        _pricing_statistics: Option<&crate::domain::cutting_plan_generation::CuttingPlanGenerationStatistics>,
    ) {
    }
}

/// 扩展集合 / Extension set
#[derive(Clone)]
pub struct Csp1dExtensionSet<V: SolveValue> {
    pub modeling_extensions: Vec<Csp1dModelingExtension<V>>,
    pub domain_policies: Vec<Arc<dyn Csp1dDomainPolicy<V>>>,
    pub objective_policies: Vec<Arc<dyn Csp1dObjectivePolicy<V>>>,
    pub generation_strategies: Vec<Arc<dyn Csp1dGenerationStrategy<V>>>,
    pub pricing_policies: Vec<Arc<dyn Csp1dPricingPolicy<V>>>,
    pub flow_policies: Vec<Arc<dyn Csp1dFlowPolicy<V>>>,
    pub extraction_policies: Vec<Arc<dyn Csp1dExtractionPolicy<V>>>,
}

impl<V: SolveValue> Default for Csp1dExtensionSet<V> {
    fn default() -> Self {
        Self {
            modeling_extensions: Vec::new(),
            domain_policies: Vec::new(),
            objective_policies: Vec::new(),
            generation_strategies: Vec::new(),
            pricing_policies: Vec::new(),
            flow_policies: Vec::new(),
            extraction_policies: Vec::new(),
        }
    }
}

impl<V: SolveValue> std::fmt::Debug for Csp1dExtensionSet<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dExtensionSet")
            .field("modeling_extensions", &self.modeling_extensions.len())
            .field("domain_policies", &self.domain_policies.len())
            .field("objective_policies", &self.objective_policies.len())
            .field("generation_strategies", &self.generation_strategies.len())
            .field("pricing_policies", &self.pricing_policies.len())
            .field("flow_policies", &self.flow_policies.len())
            .field("extraction_policies", &self.extraction_policies.len())
            .finish()
    }
}

pub fn filter_initial_plans_by_policies<V: SolveValue>(
    plans: Vec<CuttingPlan<V>>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> Vec<CuttingPlan<V>> {
    let context = BasicPolicyFlowContext {
        current_plans: plans.clone(),
    };
    filter_initial_plans_by_policies_with_context(plans, policies, &context)
}

pub fn filter_initial_plans_by_policies_with_context<V: SolveValue>(
    plans: Vec<CuttingPlan<V>>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
    context: &dyn Csp1dFlowContext<V>,
) -> Vec<CuttingPlan<V>> {
    policies
        .iter()
        .fold(plans, |plans, policy| policy.filter_initial_plans(context, plans))
}

pub fn is_equivalent_by_policies<V: SolveValue>(
    lhs: &CuttingPlan<V>,
    rhs: &CuttingPlan<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> bool {
    let context = BasicPolicyFlowContext {
        current_plans: vec![lhs.clone()],
    };
    lhs.canonical_key() == rhs.canonical_key()
        || policies
            .iter()
            .any(|policy| policy.is_equivalent(&context, lhs, rhs))
}

pub fn should_stop_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> bool {
    policies
        .iter()
        .any(|policy| policy.should_stop_iteration(context))
}

pub fn select_termination_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> (String, Option<String>) {
    policies.iter().fold(
        (String::new(), None),
        |(reason, message), policy| policy.select_termination(context, reason, message),
    )
}

pub fn select_termination_by_policies_with_default<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
    default_reason: String,
    default_message: Option<String>,
) -> (String, Option<String>) {
    policies.iter().fold(
        (default_reason, default_message),
        |(reason, message), policy| policy.select_termination(context, reason, message),
    )
}

pub fn accept_partial_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> bool {
    policies
        .iter()
        .fold(true, |decision, policy| policy.accept_partial(context, decision))
}

pub fn allow_recovery_fallback_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
    default_decision: bool,
) -> bool {
    policies.iter().fold(default_decision, |decision, policy| {
        policy.allow_recovery_fallback(context, decision)
    })
}

struct BasicPolicyFlowContext<V: SolveValue> {
    current_plans: Vec<CuttingPlan<V>>,
}

impl<V: SolveValue> Csp1dFlowContext<V> for BasicPolicyFlowContext<V> {
    fn iteration(&self) -> u64 {
        0
    }

    fn current_plans(&self) -> &[CuttingPlan<V>] {
        &self.current_plans
    }

    fn iteration_limit(&self) -> u64 {
        0
    }

    fn allow_partial_solution(&self) -> bool {
        true
    }
}

/// 简单领域计算上下文 / Simple domain calculation context
#[derive(Debug, Clone)]
pub struct SimpleDomainCalculationContext<V: SolveValue> {
    /// 切割方案 / Cutting plan
    pub plan: CuttingPlan<V>,
    /// 方案索引 / Plan index
    pub plan_index: usize,
    /// 领域数值样本 / Domain value sample
    pub domain_value_sample: Option<V>,
}

impl<V: SolveValue> Csp1dDomainCalculationContext<V> for SimpleDomainCalculationContext<V> {
    fn plan(&self) -> &CuttingPlan<V> {
        &self.plan
    }

    fn plan_index(&self) -> usize {
        self.plan_index
    }

    fn domain_value_sample(&self) -> Option<V> {
        self.domain_value_sample.clone()
    }

    fn to_domain_value(&self, value: f64) -> Option<V> {
        from_f64(value)
    }
}
