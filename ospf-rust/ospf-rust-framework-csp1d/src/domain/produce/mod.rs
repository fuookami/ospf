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
    Csp1dQuantity, CuttingPlan, Machine, Material, ProductDemand, ProductId, from_f64, to_f64,
};

pub use aggregation::*;
pub use builder::*;
pub use context::*;
pub use pipeline::*;
pub use shadow_price::*;

/// 切割方案使用量 / Cutting plan usage
#[derive(Debug, Clone)]
pub struct CuttingPlanUsage<V: SolveValue> {
    /// 切割方案 / Cutting plan
    pub plan: CuttingPlan<V>,
    /// 使用份数 / Usage amount
    pub amount: u64,
}

/// 物料使用量 / Material usage
#[derive(Debug, Clone)]
pub struct MaterialUsage<V: SolveValue> {
    /// 物料 / Material
    pub material: Material<V>,
    /// 使用批次数 / Usage batch count
    pub amount: u64,
}

/// 设备产能使用 / Machine capacity usage
#[derive(Debug, Clone)]
pub struct MachineCapacityUsage<V: SolveValue> {
    /// 设备 / Machine
    pub machine: Machine<V>,
    /// 已使用产能 / Used capacity
    pub used: Option<Csp1dQuantity<V>>,
}

/// 主问题求解产出 / Master problem output
#[derive(Debug, Clone)]
pub struct Produce<V: SolveValue> {
    /// 选中的切割方案 / Selected cutting plans
    pub cutting_plans: Vec<CuttingPlanUsage<V>>,
    /// 物料使用量 / Material usages
    pub material_usages: Vec<MaterialUsage<V>>,
    /// 设备产能使用量 / Machine capacity usages
    pub machine_usages: Vec<MachineCapacityUsage<V>>,
    /// 未满足的需求 / Unmet demands
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
    /// 产品 ID / Product id
    pub product_id: ProductId,
    /// 单位符号 / Unit symbol
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
    /// 按贡献键聚合需求贡献 / Aggregate demand contributions by contribution key
    pub fn contributions(
        &self,
    ) -> BTreeMap<ContributionKey, Vec<crate::domain::material::CuttingPlanDemandContribution<V>>>
    {
        let mut contributions = BTreeMap::new();
        for usage in &self.cutting_plans {
            for contribution in &usage.plan.demand_contributions {
                let key = ContributionKey {
                    product_id: contribution.product.id.clone(),
                    unit_symbol: String::new(),
                };
                contributions
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(contribution.clone());
            }
        }
        contributions
    }
}

/// 建模模式 / Modeling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dModelingMode {
    /// 混合整数线性规划 / Mixed-integer linear programming
    MILP,
    /// 线性规划松弛 / Linear programming relaxation
    LP,
}

/// 扩展适用模式 / Extension applicable mode
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dExtensionMode {
    /// 仅 MILP 阶段 / MILP phase only
    MILP,
    /// 仅 LP 阶段 / LP phase only
    LP,
    /// 仅最终 MILP 阶段 / Final MILP phase only
    FINAL_MILP,
    /// 所有阶段 / All phases
    ALL,
}

impl Csp1dExtensionMode {
    /// 判断是否匹配当前模式 / Check whether this mode matches the current modeling mode
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
    /// 静态管线 / Static pipeline
    pub pipeline: Option<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>>,
    /// 适用模式 / Applicable mode
    pub mode: Csp1dExtensionMode,
    /// 上下文感知管线工厂 / Context-aware pipeline factory
    pub context_aware_pipeline: Option<
        Arc<
            dyn Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>
                + Send
                + Sync,
        >,
    >,
}

impl<V: SolveValue> Csp1dModelingExtension<V> {
    /// 创建全模式静态扩展 / Create a static extension for all modes
    pub fn new(pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>) -> Self {
        Self {
            pipeline: Some(pipeline),
            mode: Csp1dExtensionMode::ALL,
            context_aware_pipeline: None,
        }
    }

    /// 创建指定模式静态扩展 / Create a static extension with a specific mode
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

    /// 创建全模式上下文感知扩展 / Create a context-aware extension for all modes
    pub fn context_aware(
        context_aware_pipeline: Arc<
            dyn Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>
                + Send
                + Sync,
        >,
    ) -> Self {
        Self {
            pipeline: None,
            mode: Csp1dExtensionMode::ALL,
            context_aware_pipeline: Some(context_aware_pipeline),
        }
    }

    /// 创建指定模式上下文感知扩展 / Create a context-aware extension with a specific mode
    pub fn context_aware_with_mode(
        context_aware_pipeline: Arc<
            dyn Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>
                + Send
                + Sync,
        >,
        mode: Csp1dExtensionMode,
    ) -> Self {
        Self {
            pipeline: None,
            mode,
            context_aware_pipeline: Some(context_aware_pipeline),
        }
    }

    /// 根据上下文解析管线 / Resolve pipeline from context
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
            .field(
                "has_context_aware_pipeline",
                &self.context_aware_pipeline.is_some(),
            )
            .finish()
    }
}

/// 建模上下文 / Modeling context
pub trait Csp1dModelingContext<V: SolveValue> {
    /// 获取建模模式 / Get modeling mode
    fn mode(&self) -> Csp1dModelingMode;
    /// 是否最终 MILP / Whether this is the final MILP phase
    fn is_final_milp(&self) -> bool;
    /// 获取产出聚合 / Get produce aggregation
    fn produce(&self) -> &ProduceAggregation<V>;
    /// 获取需求列表 / Get demand list
    fn demands(&self) -> &[ProductDemand<V>];
    /// 获取物料列表 / Get material list
    fn materials(&self) -> &[Material<V>];
    /// 获取设备列表 / Get machine list
    fn machines(&self) -> &[Machine<V>];
    /// 获取切割方案列表 / Get cutting plan list
    fn cutting_plans(&self) -> &[CuttingPlan<V>] {
        self.produce().cutting_plans()
    }
    /// 获取领域数值样本 / Get domain value sample
    fn domain_value_sample(&self) -> Option<V>;
    /// 将 f64 转换为领域数值 / Convert f64 to domain value
    fn to_domain_value(&self, value: f64) -> V;
}

/// 领域计算上下文 / Domain calculation context
pub trait Csp1dDomainCalculationContext<V: SolveValue> {
    /// 获取切割方案 / Get cutting plan
    fn plan(&self) -> &CuttingPlan<V>;

    /// 获取方案索引 / Get plan index
    fn plan_index(&self) -> usize {
        0
    }

    /// 获取物料 / Get material
    fn material(&self) -> &Material<V> {
        &self.plan().material
    }

    /// 获取设备 ID / Get machine id
    fn machine_id(&self) -> Option<&str> {
        self.plan().machine_id.as_deref()
    }

    /// 获取切片列表 / Get slices
    fn slices(&self) -> &[crate::domain::material::CuttingPlanSlice<V>] {
        &self.plan().slices
    }

    /// 获取需求贡献列表 / Get demand contributions
    fn demand_contributions(&self) -> &[crate::domain::material::CuttingPlanDemandContribution<V>] {
        &self.plan().demand_contributions
    }

    /// 获取领域数值样本 / Get domain value sample
    fn domain_value_sample(&self) -> Option<V> {
        None
    }

    /// 将 f64 转换为领域数值 / Convert f64 to domain value
    fn to_domain_value(&self, value: f64) -> Option<V> {
        let _ = value;
        None
    }

    /// 获取指定产品的贡献量 / Get contribution for a specific product
    fn contribution_for(&self, product_id: &str) -> Option<f64> {
        self.demand_contributions()
            .iter()
            .find(|contribution| contribution.product.id == product_id)
            .and_then(|contribution| to_f64(&contribution.quantity.value))
    }
}

/// 方案判断上下文 / Plan judgment context
pub trait Csp1dPlanJudgmentContext<V: SolveValue>: Csp1dDomainCalculationContext<V> {
    /// 获取同物料方案索引 / Get indices of plans using the same material
    fn same_material_plan_indices(&self) -> &[usize];
    /// 获取同设备方案索引 / Get indices of plans using the same machine
    fn same_machine_plan_indices(&self) -> &[usize];
    /// 获取所有方案 / Get all plans
    fn all_plans(&self) -> &[CuttingPlan<V>];
}

/// 领域策略 / Domain policy
pub trait Csp1dDomainPolicy<V: SolveValue>: Send + Sync {
    /// 策略名称 / Policy name
    fn name(&self) -> &str;

    /// 是否覆盖幅宽可行性判断 / Whether to override width feasibility check
    fn overrides_width_feasibility(&self) -> bool {
        false
    }

    /// 判断方案是否可行 / Check whether plan is feasible
    fn is_feasible(&self, _context: &dyn Csp1dDomainCalculationContext<V>) -> bool {
        true
    }

    /// 判断方案幅宽是否可行 / Check whether plan width is feasible
    fn is_width_feasible(&self, _context: &dyn Csp1dDomainCalculationContext<V>) -> bool {
        true
    }
}

/// 目标策略 / Objective policy
pub trait Csp1dObjectivePolicy<V: SolveValue>: Send + Sync {
    /// 策略名称 / Policy name
    fn name(&self) -> &str;

    /// 修改批次系数 / Modify batch coefficient
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
    /// 策略名称 / Strategy name
    fn name(&self) -> &str;

    /// 是否接受候选方案 / Whether to accept the candidate plan
    fn accept_candidate(
        &self,
        _candidate: &CuttingPlan<V>,
        _existing_plans: &[CuttingPlan<V>],
    ) -> bool {
        true
    }

    /// 获取候选方案的规范键 / Get canonical key for the candidate plan
    fn canonical_key_for(&self, _candidate: &CuttingPlan<V>) -> Option<String> {
        None
    }

    /// 是否接受支配关系 / Whether to accept dominance
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
    /// 策略名称 / Policy name
    fn name(&self) -> &str;

    /// 修改定价成本 / Modify pricing cost
    fn modify_cost(&self, _candidate: &CuttingPlan<V>, base_cost: V) -> V {
        base_cost
    }

    /// 修改定价收益 / Modify pricing benefit
    fn modify_benefit(&self, _candidate: &CuttingPlan<V>, base_benefit: V) -> V {
        base_benefit
    }

    /// 判断定价是否改善 / Check whether pricing is improving
    fn is_improving(&self, _candidate: &CuttingPlan<V>, _benefit: &V, _cost: &V) -> Option<bool> {
        None
    }
}

/// 流程上下文 / Flow context
pub trait Csp1dFlowContext<V: SolveValue> {
    /// 当前迭代次数 / Current iteration count
    fn iteration(&self) -> u64;

    /// 当前方案列表 / Current plan list
    fn current_plans(&self) -> &[CuttingPlan<V>];

    /// 迭代上限 / Iteration limit
    fn iteration_limit(&self) -> u64;

    /// 是否允许部分解 / Whether partial solution is allowed
    fn allow_partial_solution(&self) -> bool;

    /// 新增方案列表 / New plans list
    fn new_plans(&self) -> &[CuttingPlan<V>] {
        &[]
    }

    /// 获取定价统计 / Get pricing statistics
    fn pricing_statistics(
        &self,
    ) -> Option<&crate::domain::cutting_plan_generation::CuttingPlanGenerationStatistics> {
        None
    }

    /// 是否有有效 LP 结果 / Whether a valid LP result exists
    fn has_valid_lp_result(&self) -> bool {
        false
    }

    /// warm start 方案数量 / Warm-start plan count
    fn warm_start_plan_count(&self) -> u64 {
        0
    }

    /// warm start 是否需要回退 / Whether warm-start requires fallback
    fn warm_start_requires_fallback(&self) -> bool {
        false
    }
}

/// 流程策略 / Flow policy
pub trait Csp1dFlowPolicy<V: SolveValue>: Send + Sync {
    /// 策略名称 / Policy name
    fn name(&self) -> &str;

    /// 过滤初始方案 / Filter initial plans
    fn filter_initial_plans(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        plans: Vec<CuttingPlan<V>>,
    ) -> Vec<CuttingPlan<V>> {
        plans
    }

    /// 判断两个方案是否等价 / Check whether two plans are equivalent
    fn is_equivalent(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        _existing: &CuttingPlan<V>,
        _candidate: &CuttingPlan<V>,
    ) -> bool {
        false
    }

    /// 是否应停止迭代 / Whether iteration should stop
    fn should_stop_iteration(&self, _context: &dyn Csp1dFlowContext<V>) -> bool {
        false
    }

    /// 选择终止原因 / Select termination reason
    fn select_termination(
        &self,
        _context: &dyn Csp1dFlowContext<V>,
        default_reason: String,
        default_message: Option<String>,
    ) -> (String, Option<String>) {
        (default_reason, default_message)
    }

    /// 是否接受部分解 / Whether to accept partial solution
    fn accept_partial(&self, _context: &dyn Csp1dFlowContext<V>, default_decision: bool) -> bool {
        default_decision
    }

    /// 是否允许恢复回退 / Whether to allow recovery fallback
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
    /// 策略名称 / Policy name
    fn name(&self) -> &str;

    /// 丰富输出信息 / Enrich output information
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
        _pricing_statistics: Option<
            &crate::domain::cutting_plan_generation::CuttingPlanGenerationStatistics,
        >,
    ) {
    }
}

/// 扩展集合 / Extension set
#[derive(Clone)]
pub struct Csp1dExtensionSet<V: SolveValue> {
    /// 建模扩展列表 / Modeling extensions
    pub modeling_extensions: Vec<Csp1dModelingExtension<V>>,
    /// 领域策略列表 / Domain policies
    pub domain_policies: Vec<Arc<dyn Csp1dDomainPolicy<V>>>,
    /// 目标策略列表 / Objective policies
    pub objective_policies: Vec<Arc<dyn Csp1dObjectivePolicy<V>>>,
    /// 生成策略列表 / Generation strategies
    pub generation_strategies: Vec<Arc<dyn Csp1dGenerationStrategy<V>>>,
    /// 定价策略列表 / Pricing policies
    pub pricing_policies: Vec<Arc<dyn Csp1dPricingPolicy<V>>>,
    /// 流程策略列表 / Flow policies
    pub flow_policies: Vec<Arc<dyn Csp1dFlowPolicy<V>>>,
    /// 提取策略列表 / Extraction policies
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

/// 按策略过滤初始方案 / Filter initial plans by flow policies
pub fn filter_initial_plans_by_policies<V: SolveValue>(
    plans: Vec<CuttingPlan<V>>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> Vec<CuttingPlan<V>> {
    let context = BasicPolicyFlowContext {
        current_plans: plans.clone(),
    };
    filter_initial_plans_by_policies_with_context(plans, policies, &context)
}

/// 按策略和上下文过滤初始方案 / Filter initial plans by flow policies with context
pub fn filter_initial_plans_by_policies_with_context<V: SolveValue>(
    plans: Vec<CuttingPlan<V>>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
    context: &dyn Csp1dFlowContext<V>,
) -> Vec<CuttingPlan<V>> {
    policies.iter().fold(plans, |plans, policy| {
        policy.filter_initial_plans(context, plans)
    })
}

/// 按策略判断方案等价性 / Check plan equivalence by policies
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

/// 按策略判断是否应停止迭代 / Check whether iteration should stop by policies
pub fn should_stop_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> bool {
    policies
        .iter()
        .any(|policy| policy.should_stop_iteration(context))
}

/// 按策略选择终止原因 / Select termination reason by policies
pub fn select_termination_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> (String, Option<String>) {
    policies
        .iter()
        .fold((String::new(), None), |(reason, message), policy| {
            policy.select_termination(context, reason, message)
        })
}

/// 按策略选择终止原因（带默认值）/ Select termination reason by policies with default
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

/// 按策略判断是否接受部分解 / Check whether to accept partial solution by policies
pub fn accept_partial_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[Arc<dyn Csp1dFlowPolicy<V>>],
) -> bool {
    policies.iter().fold(true, |decision, policy| {
        policy.accept_partial(context, decision)
    })
}

/// 按策略判断是否允许恢复回退 / Check whether to allow recovery fallback by policies
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
