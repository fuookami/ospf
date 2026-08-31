//! 主问题产出与扩展点 / Master problem output and extension points

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearObjectiveInput, MetaModel, ObjectiveCategory,
};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_core::variable::{Continuous, UContinuous, UInteger, VariableRange};
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    from_f64, shadow_price_key_from_string, shadow_price_key_to_string, shadow_price_unit_symbol,
    to_f64, Csp1dQuantity, Csp1dShadowPriceKey, CuttingPlan, Material,
    MaterialUsageShadowPriceKey, Machine, MachineBatchShadowPriceKey, MachineCapacityShadowPriceKey,
    ProductDemand, ProductDemandShadowPriceKey, ShadowPriceMap,
};
use crate::domain::length_assignment::{
    DefaultLengthDerivation, LengthAssignment, LengthAssignmentContext, LengthAssignmentInput,
    LengthAssignmentModelingConfig, LengthAssignmentResult, LengthSlackAggregation,
    OverLengthRecord,
};
use crate::domain::r#yield::{
    ModeledOverProduction, ModeledUnderProduction, ProductOutput, YieldAnalysis,
    YieldModelingConfig, YieldModelingResult, YieldSlackAggregation,
};
use crate::domain::wasting_minimization::{
    ModeledMaterialCost, RestMaterialMeasure, WasteAggregation, WasteAnalysis,
    WasteMinimizationConfig, WasteMinimizationResult,
};

pub mod model;

const DEMAND_CONSTRAINT_GROUP_ID: u64 = 10_001;
const MATERIAL_CONSTRAINT_GROUP_ID: u64 = 10_002;
const MACHINE_CONSTRAINT_GROUP_ID: u64 = 10_003;
const YIELD_CONSTRAINT_GROUP_ID: u64 = 10_004;
const LENGTH_CONSTRAINT_GROUP_ID: u64 = 10_005;
const BUILTIN_CONSTRAINT_GROUP_IDS: [u64; 5] = [
    DEMAND_CONSTRAINT_GROUP_ID,
    MATERIAL_CONSTRAINT_GROUP_ID,
    MACHINE_CONSTRAINT_GROUP_ID,
    YIELD_CONSTRAINT_GROUP_ID,
    LENGTH_CONSTRAINT_GROUP_ID,
];

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

/// 产出聚合 / Produce aggregation
#[derive(Debug, Clone)]
pub struct ProduceAggregation<V: SolveValue> {
    /// 当前所有切割方案 / Current cutting plans
    pub cutting_plans: Vec<CuttingPlan<V>>,
    /// 需求列表 / Demand list
    pub demands: Vec<ProductDemand<V>>,
    /// 物料列表 / Material list
    pub materials: Vec<Material<V>>,
    /// 设备列表 / Machine list
    pub machines: Vec<Machine<V>>,
    /// warm start 方案使用量 / Warm-start plan usages
    pub warm_start_plan_usages: Vec<CuttingPlanUsage<V>>,
    plans_iteration: Vec<Vec<CuttingPlan<V>>>,
    registered_ids: HashSet<String>,
    registered_keys: HashSet<String>,
    plan_variable_indices: Vec<usize>,
    plans_iteration_variable_indices: Vec<Vec<usize>>,
    retired_plan_indices: HashSet<usize>,
    lp_relaxation: bool,
}

impl<V: SolveValue> Default for ProduceAggregation<V> {
    fn default() -> Self {
        Self {
            cutting_plans: Vec::new(),
            demands: Vec::new(),
            materials: Vec::new(),
            machines: Vec::new(),
            warm_start_plan_usages: Vec::new(),
            plans_iteration: Vec::new(),
            registered_ids: HashSet::new(),
            registered_keys: HashSet::new(),
            plan_variable_indices: Vec::new(),
            plans_iteration_variable_indices: Vec::new(),
            retired_plan_indices: HashSet::new(),
            lp_relaxation: false,
        }
    }
}

impl<V: SolveValue> ProduceAggregation<V> {
    /// 创建产出聚合 / Create produce aggregation
    pub fn new(
        cutting_plans: Vec<CuttingPlan<V>>,
        demands: Vec<ProductDemand<V>>,
        materials: Vec<Material<V>>,
        machines: Vec<Machine<V>>,
        warm_start_plan_usages: Vec<CuttingPlanUsage<V>>,
    ) -> Self {
        let mut aggregation = Self {
            cutting_plans: Vec::new(),
            demands,
            materials,
            machines,
            warm_start_plan_usages,
            plans_iteration: Vec::new(),
            registered_ids: HashSet::new(),
            registered_keys: HashSet::new(),
            plan_variable_indices: Vec::new(),
            plans_iteration_variable_indices: Vec::new(),
            retired_plan_indices: HashSet::new(),
            lp_relaxation: false,
        };
        aggregation.add_initial_plans(cutting_plans);
        aggregation
    }

    /// 方案数量 / Plan count
    pub fn plan_count(&self) -> usize {
        self.cutting_plans.len()
    }

    /// 当前方案池 / Current cutting plans
    pub fn cutting_plans(&self) -> &[CuttingPlan<V>] {
        &self.cutting_plans
    }

    /// 每轮新增方案 / Plans added per iteration
    pub fn plans_iteration(&self) -> &[Vec<CuttingPlan<V>>] {
        &self.plans_iteration
    }

    /// 方案变量索引 / Plan variable indices
    pub fn plan_variable_indices(&self) -> &[usize] {
        &self.plan_variable_indices
    }

    /// 每轮新增方案变量索引 / Plan variable indices added per iteration
    pub fn plans_iteration_variable_indices(&self) -> &[Vec<usize>] {
        &self.plans_iteration_variable_indices
    }

    /// 方案变量索引 / Plan variable index
    pub fn plan_variable_index(&self, index: usize) -> Option<usize> {
        self.plan_variable_indices.get(index).copied()
    }

    /// 有效方案判断 / Active plan check
    pub fn is_plan_active(&self, index: usize) -> bool {
        index < self.cutting_plans.len() && !self.retired_plan_indices.contains(&index)
    }

    /// 退役方案索引 / Retired plan indices
    pub fn retired_plan_indices(&self) -> &HashSet<usize> {
        &self.retired_plan_indices
    }

    /// 按模型模式注册初始变量 / Register initial variables by model mode
    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        lp_relaxation: bool,
    ) -> crate::Csp1dResult<()> {
        self.lp_relaxation = lp_relaxation;
        self.plan_variable_indices.clear();
        self.plans_iteration_variable_indices.clear();
        let plans = self.cutting_plans.clone();
        self.register_plan_variables(model, &plans)
            .map(|indices| {
                self.plan_variable_indices = indices;
            })
    }

    /// 添加初始方案 / Add initial plans
    pub fn add_initial_plans(&mut self, initial_plans: Vec<CuttingPlan<V>>) -> Vec<CuttingPlan<V>> {
        self.add_columns(0, initial_plans)
    }

    /// 添加列并按 id 与 canonical key 去重 / Add columns with id and canonical-key deduplication
    pub fn add_columns(
        &mut self,
        _iteration: u64,
        new_plans: Vec<CuttingPlan<V>>,
    ) -> Vec<CuttingPlan<V>> {
        let mut added = Vec::new();
        for plan in new_plans {
            let key = plan.canonical_key();
            if self.registered_ids.contains(&plan.id) || self.registered_keys.contains(&key) {
                continue;
            }
            self.registered_ids.insert(plan.id.clone());
            self.registered_keys.insert(key);
            self.cutting_plans.push(plan.clone());
            added.push(plan);
        }
        self.plans_iteration.push(added.clone());
        added
    }

    /// 添加列并同步注册变量 / Add columns and register variables
    pub fn add_columns_to_model(
        &mut self,
        iteration: u64,
        new_plans: Vec<CuttingPlan<V>>,
        model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>> {
        let added = self.add_columns(iteration, new_plans);
        let indices = self.register_plan_variables(model, &added)?;
        self.plan_variable_indices.extend(indices.iter().copied());
        self.plans_iteration_variable_indices.push(indices);
        Ok(added)
    }

    /// 移除列并固定变量为 0 / Remove columns and fix variables to zero
    pub fn remove_columns_from_model(
        &mut self,
        plan_indices: &[usize],
        model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>> {
        let mut removed = Vec::new();
        for &plan_index in plan_indices {
            if !self.is_plan_active(plan_index) {
                continue;
            }
            let Some(variable_index) = self.plan_variable_index(plan_index) else {
                continue;
            };
            model
                .set_variable_range_by_index(variable_index, VariableRange::fixed(0.0))
                .map_err(|error| crate::Csp1dError::Calculation {
                    message: format!("retire plan variable failed: {error}"),
                })?;
            self.retired_plan_indices.insert(plan_index);
            if let Some(plan) = self.cutting_plans.get(plan_index) {
                removed.push(plan.clone());
            }
        }
        Ok(removed)
    }

    fn register_plan_variables(
        &self,
        model: &mut MetaModel<f64>,
        plans: &[CuttingPlan<V>],
    ) -> crate::Csp1dResult<Vec<usize>> {
        let mut indices = Vec::with_capacity(plans.len());
        for plan in plans {
            let name = format!("csp1d_batch_{}", plan.id);
            let index = if self.lp_relaxation {
                model
                    .register_auto_variable_with_range::<Continuous>(
                        &name,
                        VariableRange::new(Some(0.0), None),
                    )
                    .map_err(|error| crate::Csp1dError::Calculation {
                        message: format!("register LP plan variable failed: {error}"),
                    })?
            } else {
                model
                    .register_auto_variable_with_range::<UInteger>(
                        &name,
                        VariableRange::new(Some(0.0), None),
                    )
                    .map_err(|error| crate::Csp1dError::Calculation {
                        message: format!("register MILP plan variable failed: {error}"),
                    })?
            };
            indices.push(index);
        }
        Ok(indices)
    }
}

/// 模型上下文 / Model context
pub trait Csp1dModelContext<V: SolveValue> {
    fn register(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()>;

    fn extract_solution(
        &self,
        model: &ospf_rust_core::model::MetaModel<f64>,
    ) -> crate::Csp1dResult<Produce<V>>;
}

/// 列生成上下文 / Iterative context
pub trait Csp1dIterativeContext<V: SolveValue>: Csp1dModelContext<V> {
    fn add_columns(
        &mut self,
        iteration: u64,
        new_plans: Vec<CuttingPlan<V>>,
        model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>>;

    fn remove_columns(
        &mut self,
        plan_indices: &[usize],
        model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>>;

    fn extract_shadow_price(
        &self,
        model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> crate::Csp1dResult<ShadowPriceMap<V>>;
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

/// 增量扩展管线 / Incremental extension pipeline
pub trait Csp1dIncrementalPipeline<V: SolveValue>: Pipeline<MetaModel<f64>> {
    fn add_columns(
        &self,
        _context: &dyn Csp1dModelingContext<V>,
        _iteration: u64,
        new_plans: Vec<CuttingPlan<V>>,
        _model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>> {
        Ok(new_plans)
    }
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

/// CSP1D 影子价格提取器 / CSP1D shadow price extractor
pub type Csp1dShadowPriceExtractor<V> =
    Arc<dyn Fn(&Csp1dDefaultShadowPriceMap, &CuttingPlan<V>) -> f64 + Send + Sync>;

/// CSP1D 列生成管线 / CSP1D column-generation pipeline
pub trait Csp1dCGPipeline<V: SolveValue>: Pipeline<MetaModel<f64>> {
    /// 刷新影子价格 / Refresh shadow prices
    fn refresh_shadow_price(
        &self,
        shadow_price_map: &mut Csp1dDefaultShadowPriceMap,
        model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> crate::Csp1dResult<()> {
        refresh_shadow_price_by_key_as_args(
            self.constraint_group(),
            shadow_price_map,
            model,
            shadow_prices,
        )
    }

    /// 获取方案级影子价格提取器 / Get plan-level shadow price extractor
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        None
    }
}

fn refresh_shadow_price_by_key_as_args(
    group: Option<&ConstraintGroup>,
    shadow_price_map: &mut Csp1dDefaultShadowPriceMap,
    model: &MetaModel<f64>,
    shadow_prices: &[f64],
) -> crate::Csp1dResult<()> {
    for (constraint_index, constraint) in model.constraints().iter().enumerate() {
        if let Some(group) = group {
            if constraint.group.as_ref().map(|value| value.id) != Some(group.id) {
                continue;
            }
        }
        let Some(args) = constraint.args.as_deref() else {
            continue;
        };
        let Some(key) = shadow_price_key_from_string(args) else {
            continue;
        };
        let Some(value) = shadow_prices.get(constraint_index).copied() else {
            continue;
        };
        shadow_price_map.put_or_add(key, value);
    }
    Ok(())
}

/// 默认需求约束管线 / Default demand constraint pipeline
#[derive(Debug, Clone)]
pub struct DemandConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
}

impl<V: SolveValue> DemandConstraintPipeline<V> {
    /// 创建需求约束管线 / Create demand constraint pipeline
    pub fn new(produce: ProduceAggregation<V>) -> Self {
        Self {
            name: "demand_constraint".to_string(),
            group: Some(ConstraintGroup::new(
                DEMAND_CONSTRAINT_GROUP_ID,
                "csp1d_demand_constraint",
            )),
            produce,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for DemandConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (demand_index, demand) in self.produce.demands.iter().enumerate() {
            let terms = self
                .produce
                .cutting_plans
                .iter()
                .enumerate()
                .filter_map(|(plan_index, plan)| {
                    if !self.produce.is_plan_active(plan_index) {
                        return None;
                    }
                    let variable = self.produce.plan_variable_index(plan_index)?;
                    let coefficient = plan
                        .demand_contributions
                        .iter()
                        .filter(|contribution| {
                            contribution.product.id == demand.product.id
                                && contribution.quantity.unit == demand.quantity.unit
                        })
                        .filter_map(|contribution| to_f64(&contribution.quantity.value))
                        .sum::<f64>();
                    (coefficient != 0.0).then_some((variable, coefficient))
                })
                .collect::<Vec<_>>();
            let Some(rhs) = to_f64(&demand.quantity.value) else {
                log::warn!("Skip demand constraint {} due to non-convertible rhs", demand_index);
                continue;
            };
            let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
                product_id: demand.product.id.clone(),
                unit_symbol: shadow_price_unit_symbol(&demand.quantity.unit),
            });
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &terms,
                ConstraintRelation::GreaterEqual,
                rhs,
                &format!("demand_{demand_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!("Failed to register demand constraint {}: {:?}", demand_index, error);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for DemandConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        let demands = self.produce.demands.clone();
        if demands.is_empty() {
            return None;
        }
        Some(Arc::new(move |map, plan| {
            demands
                .iter()
                .map(|demand| {
                    let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
                        product_id: demand.product.id.clone(),
                        unit_symbol: shadow_price_unit_symbol(&demand.quantity.unit),
                    });
                    let shadow_price = map.get(&key).unwrap_or(0.0);
                    let contribution = plan
                        .demand_contributions
                        .iter()
                        .filter(|contribution| {
                            contribution.product.id == demand.product.id
                                && contribution.quantity.unit == demand.quantity.unit
                        })
                        .filter_map(|contribution| to_f64(&contribution.quantity.value))
                        .sum::<f64>();
                    shadow_price * contribution
                })
                .sum()
        }))
    }
}

/// 默认物料约束管线 / Default material constraint pipeline
#[derive(Debug, Clone)]
pub struct MaterialConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
}

impl<V: SolveValue> MaterialConstraintPipeline<V> {
    /// 创建物料约束管线 / Create material constraint pipeline
    pub fn new(produce: ProduceAggregation<V>) -> Self {
        Self {
            name: "material_constraint".to_string(),
            group: Some(ConstraintGroup::new(
                MATERIAL_CONSTRAINT_GROUP_ID,
                "csp1d_material_constraint",
            )),
            produce,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for MaterialConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (material_index, material) in self.produce.materials.iter().enumerate() {
            if material.available_batches == u64::MAX {
                continue;
            }
            let terms = self
                .produce
                .cutting_plans
                .iter()
                .enumerate()
                .filter_map(|(plan_index, plan)| {
                    if !self.produce.is_plan_active(plan_index) {
                        return None;
                    }
                    (plan.material.id == material.id)
                        .then_some((self.produce.plan_variable_index(plan_index)?, 1.0))
                })
                .collect::<Vec<_>>();
            let key = Csp1dShadowPriceKey::MaterialUsage(MaterialUsageShadowPriceKey {
                material_id: material.id.clone(),
            });
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &terms,
                ConstraintRelation::LessEqual,
                material.available_batches as f64,
                &format!("material_{material_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!("Failed to register material constraint {}: {:?}", material_index, error);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for MaterialConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        if self.produce.materials.is_empty() {
            return None;
        }
        Some(Arc::new(move |map, plan| {
            let key = Csp1dShadowPriceKey::MaterialUsage(MaterialUsageShadowPriceKey {
                material_id: plan.material.id.clone(),
            });
            map.get(&key).unwrap_or(0.0)
        }))
    }
}

/// 默认设备约束管线 / Default machine constraint pipeline
#[derive(Debug, Clone)]
pub struct MachineConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
}

impl<V: SolveValue> MachineConstraintPipeline<V> {
    /// 创建设备约束管线 / Create machine constraint pipeline
    pub fn new(produce: ProduceAggregation<V>) -> Self {
        Self {
            name: "machine_constraint".to_string(),
            group: Some(ConstraintGroup::new(
                MACHINE_CONSTRAINT_GROUP_ID,
                "csp1d_machine_constraint",
            )),
            produce,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for MachineConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (machine_index, machine) in self.produce.machines.iter().enumerate() {
            if let Some(max_batch_count) = machine.max_batch_count {
                let terms = self
                    .produce
                    .cutting_plans
                    .iter()
                    .enumerate()
                    .filter_map(|(plan_index, plan)| {
                        if !self.produce.is_plan_active(plan_index) {
                            return None;
                        }
                        (plan.machine_id.as_deref() == Some(machine.id.as_str()))
                            .then_some((self.produce.plan_variable_index(plan_index)?, 1.0))
                    })
                    .collect::<Vec<_>>();
                let key = Csp1dShadowPriceKey::MachineBatch(MachineBatchShadowPriceKey {
                    machine_id: machine.id.clone(),
                });
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &terms,
                    ConstraintRelation::LessEqual,
                    max_batch_count as f64,
                    &format!("machine_batch_{machine_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    Some(shadow_price_key_to_string(&key)),
                ) {
                    log::warn!("Failed to register machine batch constraint {}: {:?}", machine_index, error);
                }
            }

            let Some(capacity) = &machine.capacity else {
                continue;
            };
            let Some(rhs) = to_f64(&capacity.value) else {
                continue;
            };
            let terms = self
                .produce
                .cutting_plans
                .iter()
                .enumerate()
                .filter_map(|(plan_index, plan)| {
                    if !self.produce.is_plan_active(plan_index) {
                        return None;
                    }
                    if plan.machine_id.as_deref() != Some(machine.id.as_str()) {
                        return None;
                    }
                    let consumption = plan.capacity_consumption.as_ref()?;
                    if consumption.unit != capacity.unit {
                        return None;
                    }
                    Some((self.produce.plan_variable_index(plan_index)?, to_f64(&consumption.value)?))
                })
                .collect::<Vec<_>>();
            if terms.is_empty() {
                continue;
            }
            let key = Csp1dShadowPriceKey::MachineCapacity(MachineCapacityShadowPriceKey {
                machine_id: machine.id.clone(),
            });
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &terms,
                ConstraintRelation::LessEqual,
                rhs,
                &format!("machine_capacity_{machine_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!("Failed to register machine capacity constraint {}: {:?}", machine_index, error);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for MachineConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        if self.produce.machines.is_empty() {
            return None;
        }
        Some(Arc::new(move |map, plan| {
            let Some(machine_id) = plan.machine_id.as_deref() else {
                return 0.0;
            };
            let batch_key = Csp1dShadowPriceKey::MachineBatch(MachineBatchShadowPriceKey {
                machine_id: machine_id.to_string(),
            });
            let capacity_key = Csp1dShadowPriceKey::MachineCapacity(MachineCapacityShadowPriceKey {
                machine_id: machine_id.to_string(),
            });
            let capacity_consumption = plan
                .capacity_consumption
                .as_ref()
                .and_then(|quantity| to_f64(&quantity.value))
                .unwrap_or(0.0);
            map.get(&batch_key).unwrap_or(0.0)
                + map.get(&capacity_key).unwrap_or(0.0) * capacity_consumption
        }))
    }
}

/// Yield 建模约束管线 / Yield modeling constraint pipeline
#[derive(Debug, Clone)]
pub struct YieldConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
    r#yield: YieldSlackAggregation<V>,
}

impl<V: SolveValue> YieldConstraintPipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(produce: ProduceAggregation<V>, r#yield: YieldSlackAggregation<V>) -> Self {
        Self {
            name: "yield_constraint".to_string(),
            group: Some(ConstraintGroup::new(
                YIELD_CONSTRAINT_GROUP_ID,
                "csp1d_yield_constraint",
            )),
            produce,
            r#yield,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for YieldConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (demand_index, demand) in self.r#yield.demands.iter().enumerate() {
            let demand_key = YieldSlackAggregation::demand_shadow_price_key(demand);
            let mut balance_terms = self
                .produce
                .cutting_plans
                .iter()
                .enumerate()
                .filter_map(|(plan_index, plan)| {
                    if !self.produce.is_plan_active(plan_index) {
                        return None;
                    }
                    let variable = self.produce.plan_variable_index(plan_index)?;
                    let coefficient = plan
                        .demand_contributions
                        .iter()
                        .filter(|contribution| {
                            contribution.product.id == demand.product.id
                                && contribution.quantity.unit == demand.quantity.unit
                        })
                        .filter_map(|contribution| to_f64(&contribution.quantity.value))
                        .sum::<f64>();
                    (coefficient != 0.0).then_some((variable, coefficient))
                })
                .collect::<Vec<_>>();
            if let Some(under_variable) = self
                .r#yield
                .under_production
                .get(demand_index)
                .copied()
                .flatten()
            {
                balance_terms.push((under_variable, 1.0));
            }
            if let Some(over_variable) = self
                .r#yield
                .over_production
                .get(demand_index)
                .copied()
                .flatten()
            {
                balance_terms.push((over_variable, -1.0));
            }
            if balance_terms.len() > 1
                || self
                    .r#yield
                    .under_production
                    .get(demand_index)
                    .and_then(|value| *value)
                    .is_some()
                || self
                    .r#yield
                    .over_production
                    .get(demand_index)
                    .and_then(|value| *value)
                    .is_some()
            {
                if let Some(rhs) = to_f64(&demand.quantity.value) {
                    if let Err(error) = model.add_linear_constraint_with_metadata(
                        &balance_terms,
                        ConstraintRelation::Equal,
                        rhs,
                        &format!("yield_balance_{demand_index}"),
                        self.group.clone().map(Arc::new),
                        false,
                        0,
                        None,
                    ) {
                        log::warn!("Failed to register yield balance constraint {}: {:?}", demand_index, error);
                    }
                }
            }
            let Some(upper_bound) = self
                .r#yield
                .config
                .over_production_upper_bound
                .get(&demand_key)
                .and_then(to_f64)
            else {
                continue;
            };
            let Some(over_variable) = self
                .r#yield
                .over_production
                .get(demand_index)
                .copied()
                .flatten()
            else {
                continue;
            };
            let key = Csp1dShadowPriceKey::YieldOverProductionBound(
                crate::domain::material::YieldOverProductionBoundShadowPriceKey {
                    product_id: demand_key.product_id,
                    unit_symbol: demand_key.unit_symbol,
                },
            );
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &[(over_variable, 1.0)],
                ConstraintRelation::LessEqual,
                upper_bound,
                &format!("over_production_bound_{demand_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!("Failed to register over-production bound {}: {:?}", demand_index, error);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for YieldConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        Some(Arc::new(|_, _| 0.0))
    }
}

/// 长度建模约束管线 / Length modeling constraint pipeline
#[derive(Debug, Clone)]
pub struct LengthConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    length: LengthSlackAggregation<V>,
}

impl<V: SolveValue> LengthConstraintPipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(length: LengthSlackAggregation<V>) -> Self {
        Self {
            name: "length_constraint".to_string(),
            group: Some(ConstraintGroup::new(
                LENGTH_CONSTRAINT_GROUP_ID,
                "csp1d_length_constraint",
            )),
            length,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for LengthConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (demand_index, demand) in self.length.demands.iter().enumerate() {
            let product_id = &demand.product.id;
            if !self.length.config.is_dynamic_product(&demand.product) {
                continue;
            }
            let assigned_variable = self
                .length
                .assigned_length
                .get(demand_index)
                .copied()
                .flatten();
            let over_variable = self
                .length
                .over_length
                .get(demand_index)
                .copied()
                .flatten();
            if let (Some(variable), Some(bound)) = (
                assigned_variable,
                self.length
                    .config
                    .assigned_length_lower_bound
                    .get(product_id)
                    .and_then(to_f64),
            ) {
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &[(variable, 1.0)],
                    ConstraintRelation::GreaterEqual,
                    bound,
                    &format!("assigned_length_lower_bound_{demand_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    None,
                ) {
                    log::warn!("Failed to register assigned length lower bound {}: {:?}", demand_index, error);
                }
            }
            if let (Some(variable), Some(bound)) = (
                assigned_variable,
                self.length
                    .config
                    .assigned_length_upper_bound
                    .get(product_id)
                    .and_then(to_f64),
            ) {
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &[(variable, 1.0)],
                    ConstraintRelation::LessEqual,
                    bound,
                    &format!("assigned_length_upper_bound_{demand_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    None,
                ) {
                    log::warn!("Failed to register assigned length upper bound {}: {:?}", demand_index, error);
                }
            }
            if let (Some(variable), Some(bound)) = (
                over_variable,
                self.length
                    .config
                    .over_length_upper_bound
                    .get(product_id)
                    .and_then(to_f64),
            ) {
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &[(variable, 1.0)],
                    ConstraintRelation::LessEqual,
                    bound,
                    &format!("over_length_bound_{demand_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    None,
                ) {
                    log::warn!("Failed to register over length bound {}: {:?}", demand_index, error);
                }
            }
            let Some(max_over_length) = demand
                .product
                .max_over_produce_length
                .as_ref()
                .and_then(|quantity| to_f64(&quantity.value))
            else {
                continue;
            };
            let (Some(assigned_variable), Some(over_variable)) = (assigned_variable, over_variable) else {
                continue;
            };
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &[(assigned_variable, 1.0), (over_variable, -1.0)],
                ConstraintRelation::LessEqual,
                max_over_length,
                &format!("assigned_over_length_link_{demand_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                None,
            ) {
                log::warn!("Failed to register assigned-over length link {}: {:?}", demand_index, error);
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

/// Yield 目标管线 / Yield objective pipeline
#[derive(Debug, Clone)]
pub struct YieldObjectivePipeline<V: SolveValue> {
    name: String,
    r#yield: YieldSlackAggregation<V>,
}

impl<V: SolveValue> YieldObjectivePipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(r#yield: YieldSlackAggregation<V>) -> Self {
        Self {
            name: "yield_objective".to_string(),
            r#yield,
        }
    }

    /// 目标项 / Objective terms
    pub fn objective_terms(&self) -> Vec<(usize, f64)> {
        let mut terms = Vec::new();
        for (demand_index, demand) in self.r#yield.demands.iter().enumerate() {
            let key = YieldSlackAggregation::demand_shadow_price_key(demand);
            if let (Some(variable), Some(penalty)) = (
                self.r#yield
                    .under_production
                    .get(demand_index)
                    .copied()
                    .flatten(),
                self.r#yield
                    .config
                    .under_production_penalty
                    .get(&key)
                    .and_then(to_f64),
            ) {
                terms.push((variable, penalty));
            }
            if let (Some(variable), Some(penalty)) = (
                self.r#yield
                    .over_production
                    .get(demand_index)
                    .copied()
                    .flatten(),
                self.r#yield
                    .config
                    .over_production_penalty
                    .get(&key)
                    .and_then(to_f64),
            ) {
                terms.push((variable, penalty));
            }
        }
        terms
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for YieldObjectivePipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

/// Waste 目标管线 / Waste objective pipeline
#[derive(Debug, Clone)]
pub struct WasteObjectivePipeline<V: SolveValue> {
    name: String,
    produce: ProduceAggregation<V>,
    config: WasteMinimizationConfig<V>,
    r#yield: Option<YieldSlackAggregation<V>>,
}

impl<V: SolveValue> WasteObjectivePipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(
        produce: ProduceAggregation<V>,
        config: WasteMinimizationConfig<V>,
        r#yield: Option<YieldSlackAggregation<V>>,
    ) -> Self {
        Self {
            name: "waste_objective".to_string(),
            produce,
            config,
            r#yield,
        }
    }

    /// 方案目标项 / Plan objective terms
    pub fn plan_objective_terms<'a, I>(&self, plans: I, offset: usize) -> Vec<(usize, f64)>
    where
        I: IntoIterator<Item = (usize, &'a CuttingPlan<V>)>,
        V: 'a,
    {
        plans
            .into_iter()
            .filter_map(|(local_index, plan)| {
                let plan_index = offset + local_index;
                if !self.produce.is_plan_active(plan_index) {
                    return None;
                }
                let variable_index = self.produce.plan_variable_index(plan_index)?;
                let mut coefficient = 0.0;
                if let Some(trim_penalty) = self.config.trim_width_penalty.as_ref().and_then(to_f64) {
                    let rest_width = plan
                        .rest_width()
                        .and_then(|width| to_f64(&width.value))
                        .unwrap_or(0.0);
                    coefficient += rest_width * trim_penalty;
                }
                if let Some(rest_penalty) = self.config.rest_material_penalty.as_ref().and_then(to_f64) {
                    if let Some(rest_material) = rest_material_value(plan, self.config.rest_material_measure) {
                        coefficient += rest_material * rest_penalty;
                    }
                }
                if let Some(material_penalty) = self
                    .config
                    .material_cost_penalty
                    .get(&plan.material.id)
                    .and_then(to_f64)
                {
                    coefficient += material_penalty;
                }
                (coefficient != 0.0).then_some((variable_index, coefficient))
            })
            .collect()
    }

    /// 超产面积目标项 / Over-production area objective terms
    pub fn over_production_area_objective_terms(&self) -> Vec<(usize, f64)> {
        let Some(area_penalty) = self.config.over_production_area_penalty.as_ref().and_then(to_f64) else {
            return Vec::new();
        };
        let Some(r#yield) = &self.r#yield else {
            return Vec::new();
        };
        r#yield
            .demands
            .iter()
            .enumerate()
            .filter_map(|(demand_index, demand)| {
                let variable = r#yield
                    .over_production
                    .get(demand_index)
                    .copied()
                    .flatten()?;
                let width = demand
                    .product
                    .max_width()
                    .and_then(|width| to_f64(&width.value))?;
                Some((variable, width * area_penalty))
            })
            .collect()
    }

    /// 全量目标项 / Full objective terms
    pub fn objective_terms(&self) -> Vec<(usize, f64)> {
        let mut terms = self.plan_objective_terms(self.produce.cutting_plans.iter().enumerate(), 0);
        terms.extend(self.over_production_area_objective_terms());
        terms
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for WasteObjectivePipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

/// 长度目标管线 / Length objective pipeline
#[derive(Debug, Clone)]
pub struct LengthObjectivePipeline<V: SolveValue> {
    name: String,
    length: LengthSlackAggregation<V>,
}

impl<V: SolveValue> LengthObjectivePipeline<V> {
    /// 创建管线 / Create pipeline
    pub fn new(length: LengthSlackAggregation<V>) -> Self {
        Self {
            name: "length_objective".to_string(),
            length,
        }
    }

    /// 批次系数 / Batch coefficient
    pub fn batch_coefficient(&self) -> f64 {
        self.length
            .config
            .batch_min_penalty
            .as_ref()
            .and_then(to_f64)
            .map(|penalty| 1.0 + penalty)
            .unwrap_or(1.0)
    }

    /// 目标项 / Objective terms
    pub fn objective_terms(&self) -> Vec<(usize, f64)> {
        let mut terms = Vec::new();
        if let Some(penalty) = self.length.config.total_length_penalty.as_ref().and_then(to_f64) {
            for variable in self.length.assigned_length.iter().filter_map(|value| *value) {
                terms.push((variable, penalty));
            }
        }
        for (demand_index, demand) in self.length.demands.iter().enumerate() {
            let Some(penalty) = self
                .length
                .config
                .over_length_penalty
                .get(&demand.product.id)
                .and_then(to_f64)
            else {
                continue;
            };
            let Some(variable) = self
                .length
                .over_length
                .get(demand_index)
                .copied()
                .flatten()
            else {
                continue;
            };
            terms.push((variable, penalty));
        }
        terms
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for LengthObjectivePipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

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
    cg_pipelines: Vec<Arc<dyn Csp1dCGPipeline<V>>>,
    extractors: Vec<Csp1dShadowPriceExtractor<V>>,
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
        cg_pipelines: Vec<Arc<dyn Csp1dCGPipeline<V>>>,
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
    pub fn plan_shadow_price(&self, plan: &CuttingPlan<V>) -> Option<V> {
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

/// CSP1D 产出模型上下文 / CSP1D produce model context
#[derive(Clone)]
pub struct Csp1dProduceContext<V: SolveValue> {
    /// 产出聚合 / Produce aggregation
    pub produce: ProduceAggregation<V>,
    /// Yield 聚合 / Yield aggregation
    pub r#yield: Option<YieldSlackAggregation<V>>,
    /// Waste 聚合 / Waste aggregation
    pub waste: Option<WasteAggregation<V>>,
    /// Length 聚合 / Length aggregation
    pub length: Option<LengthSlackAggregation<V>>,
    /// 约束管线 / Constraint pipelines
    pub constraint_pipelines: Vec<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>>,
    /// 列生成影子价格管线 / Column-generation shadow price pipelines
    pub cg_pipelines: Vec<Arc<dyn Csp1dCGPipeline<V>>>,
    /// 额外管线 / Extra pipelines
    pub extra_pipelines: Vec<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>>,
    /// 增量扩展管线 / Incremental extension pipelines
    pub incremental_pipelines: Vec<Arc<dyn Csp1dIncrementalPipeline<V>>>,
    /// 建模模式 / Modeling mode
    pub mode: Csp1dModelingMode,
    /// 是否最终 MILP / Whether final MILP
    pub is_final_milp: bool,
    /// warm start 方案使用量 / Warm-start plan usages
    pub warm_start_plan_usages: Vec<CuttingPlanUsage<V>>,
    /// 目标策略 / Objective policies
    pub objective_policies: Vec<Arc<dyn Csp1dObjectivePolicy<V>>>,
    /// 领域数值样本 / Domain value sample
    pub domain_value_sample: V,
    yield_config: Option<YieldModelingConfig<V>>,
    waste_config: Option<WasteMinimizationConfig<V>>,
    length_config: Option<LengthAssignmentModelingConfig<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dProduceContext<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dProduceContext")
            .field("produce", &self.produce)
            .field("has_yield", &self.r#yield.is_some())
            .field("has_waste", &self.waste.is_some())
            .field("has_length", &self.length.is_some())
            .field("constraint_pipelines", &self.constraint_pipelines.len())
            .field("cg_pipelines", &self.cg_pipelines.len())
            .field("extra_pipelines", &self.extra_pipelines.len())
            .field("incremental_pipelines", &self.incremental_pipelines.len())
            .field("mode", &self.mode)
            .field("is_final_milp", &self.is_final_milp)
            .finish()
    }
}

impl<V: SolveValue> Csp1dModelingContext<V> for Csp1dProduceContext<V> {
    fn mode(&self) -> Csp1dModelingMode {
        self.mode
    }

    fn is_final_milp(&self) -> bool {
        self.is_final_milp
    }

    fn produce(&self) -> &ProduceAggregation<V> {
        &self.produce
    }

    fn demands(&self) -> &[ProductDemand<V>] {
        &self.produce.demands
    }

    fn materials(&self) -> &[Material<V>] {
        &self.produce.materials
    }

    fn machines(&self) -> &[Machine<V>] {
        &self.produce.machines
    }

    fn domain_value_sample(&self) -> Option<V> {
        Some(self.domain_value_sample.clone())
    }

    fn to_domain_value(&self, value: f64) -> V {
        from_f64(value).unwrap_or_else(|| self.domain_value_sample.clone())
    }
}

impl<V: SolveValue> Csp1dModelContext<V> for Csp1dProduceContext<V> {
    fn register(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        self.produce
            .register(model, self.mode == Csp1dModelingMode::LP)?;
        self.register_yield_variables(model)?;
        self.register_length_variables(model)?;
        self.rebuild_builtin_constraint_pipelines();
        Self::register_constraint_pipelines(model, &self.constraint_pipelines)?;
        for pipeline in &self.extra_pipelines {
            pipeline.register(model);
            pipeline.invoke(model).map_err(|error| crate::Csp1dError::Calculation {
                message: format!("invoke extra pipeline {} failed: {error}", pipeline.name()),
            })?;
        }
        for pipeline in &self.incremental_pipelines {
            pipeline.register(model);
            pipeline.invoke(model).map_err(|error| crate::Csp1dError::Calculation {
                message: format!("invoke incremental pipeline {} failed: {error}", pipeline.name()),
            })?;
        }
        self.set_objective(model);
        if self.mode == Csp1dModelingMode::MILP && !self.warm_start_plan_usages.is_empty() {
            self.apply_warm_start(model);
        }
        Ok(())
    }

    fn extract_solution(&self, model: &MetaModel<f64>) -> crate::Csp1dResult<Produce<V>> {
        let mut selected_plans = Vec::new();
        let mut material_usage_map: BTreeMap<String, u64> = BTreeMap::new();
        for (plan_index, plan) in self.produce.cutting_plans.iter().enumerate() {
            if !self.produce.is_plan_active(plan_index) {
                continue;
            }
            let Some(variable_index) = self.produce.plan_variable_index(plan_index) else {
                continue;
            };
            let value = model
                .tokens()
                .get(variable_index)
                .and_then(|token| token.get_result())
                .unwrap_or(0.0);
            if value <= 0.0 {
                continue;
            }
            let amount = value.round().max(0.0) as u64;
            if amount == 0 {
                continue;
            }
            selected_plans.push(CuttingPlanUsage {
                plan: plan.clone(),
                amount,
            });
            material_usage_map
                .entry(plan.material.id.clone())
                .and_modify(|current| *current = current.saturating_add(amount))
                .or_insert(amount);
        }
        let material_usages = self
            .produce
            .materials
            .iter()
            .filter_map(|material| {
                material_usage_map
                    .get(&material.id)
                    .copied()
                    .map(|amount| MaterialUsage {
                        material: material.clone(),
                        amount,
                    })
            })
            .collect();
        let machine_usages = self.extract_machine_usages(&selected_plans);
        let unmet_demands = self.extract_unmet_demands(&selected_plans);
        Ok(Produce {
            cutting_plans: selected_plans,
            material_usages,
            machine_usages,
            unmet_demands,
        })
    }
}

impl<V: SolveValue> Csp1dIterativeContext<V> for Csp1dProduceContext<V> {
    fn add_columns(
        &mut self,
        iteration: u64,
        new_plans: Vec<CuttingPlan<V>>,
        model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>> {
        let added = self
            .produce
            .add_columns_to_model(iteration, new_plans, model)?;
        if !added.is_empty() {
            self.refresh_builtin_constraints(model)?;
            self.set_objective(model);
            let mut refreshed = added;
            for pipeline in &self.incremental_pipelines {
                refreshed = pipeline.add_columns(self, iteration, refreshed, model)?;
                if refreshed.is_empty() {
                    break;
                }
            }
            return Ok(refreshed);
        }
        Ok(added)
    }

    fn remove_columns(
        &mut self,
        plan_indices: &[usize],
        model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>> {
        let removed = self
            .produce
            .remove_columns_from_model(plan_indices, model)?;
        if !removed.is_empty() {
            self.refresh_builtin_constraints(model)?;
            self.set_objective(model);
        }
        Ok(removed)
    }

    fn extract_shadow_price(
        &self,
        model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> crate::Csp1dResult<ShadowPriceMap<V>> {
        let mut lifecycle = Csp1dShadowPriceLifecycle::with_pipelines(
            self.domain_value_sample.clone(),
            self.cg_pipelines.clone(),
        );
        lifecycle.try_extract_from_dual_solution(model, shadow_prices)
    }
}

impl<V: SolveValue> Csp1dProduceContext<V> {
    fn rebuild_builtin_constraint_pipelines(&mut self) {
        self.constraint_pipelines.clear();
        self.cg_pipelines.clear();
        let demand_pipeline = DemandConstraintPipeline::new(self.produce.clone());
        self.constraint_pipelines.push(Arc::new(demand_pipeline.clone()));
        self.cg_pipelines.push(Arc::new(demand_pipeline));
        let material_pipeline = MaterialConstraintPipeline::new(self.produce.clone());
        self.constraint_pipelines.push(Arc::new(material_pipeline.clone()));
        self.cg_pipelines.push(Arc::new(material_pipeline));
        let machine_pipeline = MachineConstraintPipeline::new(self.produce.clone());
        self.constraint_pipelines.push(Arc::new(machine_pipeline.clone()));
        self.cg_pipelines.push(Arc::new(machine_pipeline));
        if let Some(r#yield) = &self.r#yield {
            let yield_pipeline = YieldConstraintPipeline::new(
                self.produce.clone(),
                r#yield.clone(),
            );
            self.constraint_pipelines.push(Arc::new(yield_pipeline.clone()));
            self.cg_pipelines.push(Arc::new(yield_pipeline));
        }
        if let Some(length) = &self.length {
            self.constraint_pipelines
                .push(Arc::new(LengthConstraintPipeline::new(length.clone())));
        }
    }

    fn register_constraint_pipelines(
        model: &mut MetaModel<f64>,
        pipelines: &[Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>],
    ) -> crate::Csp1dResult<()> {
        for pipeline in pipelines {
            pipeline.register(model);
            pipeline.invoke(model).map_err(|error| crate::Csp1dError::Calculation {
                message: format!("invoke pipeline {} failed: {error}", pipeline.name()),
            })?;
        }
        Ok(())
    }

    fn refresh_builtin_constraints(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        for group_id in BUILTIN_CONSTRAINT_GROUP_IDS {
            model.remove_constraints_by_group_id(group_id);
        }
        self.rebuild_builtin_constraint_pipelines();
        Self::register_constraint_pipelines(model, &self.constraint_pipelines)
    }

    fn register_yield_variables(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        let Some(r#yield) = self.r#yield.as_mut() else {
            return Ok(());
        };
        r#yield.under_production.clear();
        r#yield.over_production.clear();
        for demand_index in 0..r#yield.demands.len() {
            let demand = &r#yield.demands[demand_index];
            let under_variable = if r#yield.needs_under_production(demand) {
                Some(
                    model
                        .register_auto_variable_with_range::<UContinuous>(
                            &format!("under_production_{demand_index}"),
                            VariableRange::new(Some(0.0), None),
                        )
                        .map_err(|error| crate::Csp1dError::Calculation {
                            message: format!("register under-production variable failed: {error}"),
                        })?,
                )
            } else {
                None
            };
            let over_variable = if r#yield.needs_over_production(demand) {
                Some(
                    model
                        .register_auto_variable_with_range::<UContinuous>(
                            &format!("over_production_{demand_index}"),
                            VariableRange::new(Some(0.0), None),
                        )
                        .map_err(|error| crate::Csp1dError::Calculation {
                            message: format!("register over-production variable failed: {error}"),
                        })?,
                )
            } else {
                None
            };
            r#yield.under_production.push(under_variable);
            r#yield.over_production.push(over_variable);
        }
        Ok(())
    }

    fn register_length_variables(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        let Some(length) = self.length.as_mut() else {
            return Ok(());
        };
        length.assigned_length.clear();
        length.over_length.clear();
        for demand_index in 0..length.demands.len() {
            let demand = &length.demands[demand_index];
            let assigned_variable = if length.needs_assigned_length(demand) {
                Some(
                    model
                        .register_auto_variable_with_range::<UContinuous>(
                            &format!("assigned_length_{demand_index}"),
                            VariableRange::new(Some(0.0), None),
                        )
                        .map_err(|error| crate::Csp1dError::Calculation {
                            message: format!("register assigned-length variable failed: {error}"),
                        })?,
                )
            } else {
                None
            };
            let over_variable = if length.needs_over_length(demand) {
                Some(
                    model
                        .register_auto_variable_with_range::<UContinuous>(
                            &format!("over_length_{demand_index}"),
                            VariableRange::new(Some(0.0), None),
                        )
                        .map_err(|error| crate::Csp1dError::Calculation {
                            message: format!("register over-length variable failed: {error}"),
                        })?,
                )
            } else {
                None
            };
            length.assigned_length.push(assigned_variable);
            length.over_length.push(over_variable);
        }
        Ok(())
    }

    fn set_objective(&self, model: &mut MetaModel<f64>) {
        let mut terms = self.objective_terms_for_plans(
            self.produce.cutting_plans.iter().enumerate(),
            0,
        );
        if let Some(r#yield) = &self.r#yield {
            terms.extend(YieldObjectivePipeline::new(r#yield.clone()).objective_terms());
        }
        if let Some(config) = &self.waste_config {
            terms.extend(
                WasteObjectivePipeline::new(
                    self.produce.clone(),
                    config.clone(),
                    self.r#yield.clone(),
                )
                .objective_terms(),
            );
        }
        if let Some(length) = &self.length {
            terms.extend(LengthObjectivePipeline::new(length.clone()).objective_terms());
        }
        model.set_linear_objective_input(
            LinearObjectiveInput::minimize("csp1d_objective")
                .category(ObjectiveCategory::Minimum)
                .terms(terms),
        );
    }

    fn objective_terms_for_plans<'a, I>(&self, plans: I, offset: usize) -> Vec<(usize, f64)>
    where
        I: IntoIterator<Item = (usize, &'a CuttingPlan<V>)>,
        V: 'a,
    {
        plans
            .into_iter()
            .filter_map(|(local_index, plan)| {
                let plan_index = offset + local_index;
                if !self.produce.is_plan_active(plan_index) {
                    return None;
                }
                let variable_index = self.produce.plan_variable_index(plan_index)?;
                let base = self
                    .length
                    .as_ref()
                    .map(|length| LengthObjectivePipeline::new(length.clone()).batch_coefficient())
                    .unwrap_or(1.0);
                let coefficient = if self.objective_policies.is_empty() {
                    base
                } else {
                    let context = SimpleDomainCalculationContext {
                        plan: plan.clone(),
                        plan_index,
                        domain_value_sample: Some(self.domain_value_sample.clone()),
                    };
                    self.objective_policies
                        .iter()
                        .fold(base, |coefficient, policy| {
                            policy.modify_batch_coefficient(&context, coefficient)
                        })
                };
                Some((variable_index, coefficient))
            })
            .collect()
    }

    fn apply_warm_start(&self, model: &mut MetaModel<f64>) {
        let mut usage_by_key = BTreeMap::new();
        for usage in &self.warm_start_plan_usages {
            if usage.amount == 0 {
                continue;
            }
            usage_by_key
                .entry(usage.plan.canonical_key())
                .and_modify(|amount| *amount += usage.amount)
                .or_insert(usage.amount);
        }
        if usage_by_key.is_empty() {
            return;
        }
        let mut solution = HashMap::new();
        for (plan_index, plan) in self.produce.cutting_plans.iter().enumerate() {
            if !self.produce.is_plan_active(plan_index) {
                continue;
            }
            let Some(amount) = usage_by_key.get(&plan.canonical_key()).copied() else {
                continue;
            };
            let Some(variable_index) = self.produce.plan_variable_index(plan_index) else {
                continue;
            };
            if let Some(token) = model.tokens().get(variable_index) {
                solution.insert(token.id(), amount as f64);
            }
        }
        model.set_solution_by_id(&solution);
    }

    fn extract_machine_usages(
        &self,
        selected_plans: &[CuttingPlanUsage<V>],
    ) -> Vec<MachineCapacityUsage<V>> {
        self.produce
            .machines
            .iter()
            .filter_map(|machine| {
                let capacity = machine.capacity.as_ref();
                let mut unit = capacity.map(|capacity| capacity.unit.clone());
                let mut total = 0.0;
                for usage in selected_plans
                    .iter()
                    .filter(|usage| usage.plan.machine_id.as_deref() == Some(machine.id.as_str()))
                {
                    let Some(consumption) = &usage.plan.capacity_consumption else {
                        continue;
                    };
                    if let Some(unit) = &unit {
                        if *unit != consumption.unit {
                            continue;
                        }
                    } else {
                        unit = Some(consumption.unit.clone());
                    }
                    let Some(value) = to_f64(&consumption.value) else {
                        continue;
                    };
                    total += value * usage.amount as f64;
                }
                let unit = unit?;
                (total > 0.0).then(|| MachineCapacityUsage {
                    machine: machine.clone(),
                    used: from_f64(total).map(|value| Csp1dQuantity { value, unit }),
                })
            })
            .collect()
    }

    fn extract_unmet_demands(&self, selected_plans: &[CuttingPlanUsage<V>]) -> Vec<ProductDemand<V>> {
        self.produce
            .demands
            .iter()
            .filter(|demand| {
                let supplied = selected_plans
                    .iter()
                    .flat_map(|usage| {
                        usage.plan.demand_contributions.iter().filter_map(|contribution| {
                            if contribution.product.id == demand.product.id
                                && contribution.quantity.unit == demand.quantity.unit
                            {
                                Some(to_f64(&contribution.quantity.value)? * usage.amount as f64)
                            } else {
                                None
                            }
                        })
                    })
                    .sum::<f64>();
                let required = to_f64(&demand.quantity.value).unwrap_or(f64::INFINITY);
                supplied + f64::EPSILON < required
            })
            .cloned()
            .collect()
    }

    /// 提取 yield 结果 / Extract yield result
    pub fn extract_yield_result(&self, model: &MetaModel<f64>) -> Option<YieldModelingResult<V>> {
        self.yield_config.as_ref()?;
        let produce = self.extract_solution(model).ok()?;
        let analysis = self
            .extract_modeled_yield_analysis(model, &produce)
            .unwrap_or_else(|| analyze_yield(&produce, &self.produce.demands));
        Some(YieldModelingResult {
            analysis,
        })
    }

    /// 提取 waste 结果 / Extract waste result
    pub fn extract_waste_result(&self, model: &MetaModel<f64>) -> Option<WasteMinimizationResult<V>> {
        let config = self.waste_config.as_ref()?;
        let produce = self.extract_solution(model).ok()?;
        let yield_analysis = analyze_yield(&produce, &self.produce.demands);
        let mut total_trim_width = 0.0;
        let mut total_rest_material = 0.0;
        let mut material_costs = BTreeMap::<String, f64>::new();
        for usage in &produce.cutting_plans {
            let amount = usage.amount as f64;
            let rest_width = usage
                .plan
                .rest_width()
                .and_then(|width| to_f64(&width.value))
                .unwrap_or(0.0);
            total_trim_width += rest_width * amount;
            if let Some(length) = usage
                .plan
                .material
                .length
                .as_ref()
                .and_then(|length| to_f64(&length.value))
            {
                total_rest_material += rest_width * length * amount;
            }
            if let Some(cost_penalty) = config
                .material_cost_penalty
                .get(&usage.plan.material.id)
                .and_then(to_f64)
            {
                material_costs
                    .entry(usage.plan.material.id.clone())
                    .and_modify(|cost| *cost += cost_penalty * amount)
                    .or_insert(cost_penalty * amount);
            }
        }
        let over_production_area = yield_analysis
            .over_productions
            .iter()
            .filter_map(|over_production| {
                let surplus = to_f64(&over_production.surplus.value)?;
                let width = over_production
                    .demand
                    .product
                    .max_width()
                    .and_then(|width| to_f64(&width.value))?;
                Some(surplus * width)
            })
            .sum::<f64>();
        let material_costs = material_costs
            .into_iter()
            .filter_map(|(material_id, cost)| {
                Some(ModeledMaterialCost {
                    material_id,
                    cost: from_f64::<V>(cost)?,
                })
            })
            .collect::<Vec<_>>();
        let mut metrics = BTreeMap::new();
        if let Some(value) = from_f64(total_trim_width) {
            metrics.insert("totalTrimWidth".to_string(), value);
        }
        if let Some(value) = from_f64(total_rest_material) {
            metrics.insert("totalRestMaterial".to_string(), value);
        }
        if let Some(value) = from_f64(over_production_area) {
            metrics.insert("overProductionArea".to_string(), value);
        }
        for cost in &material_costs {
            metrics.insert(format!("materialCost.{}", cost.material_id), cost.cost.clone());
        }
        Some(WasteMinimizationResult {
            total_trim_width: from_f64(total_trim_width),
            material_costs,
            over_production_area: from_f64(over_production_area),
            total_rest_material: from_f64(total_rest_material),
            over_production_area_measure: config.over_production_area_measure,
            rest_material_measure: config.rest_material_measure,
            analysis: WasteAnalysis {
                metrics,
            },
        })
    }

    /// 提取 length 结果 / Extract length result
    pub fn extract_length_result(&self, model: &MetaModel<f64>) -> Option<LengthAssignmentResult<V>> {
        let config = self.length_config.as_ref()?;
        if !config.enabled {
            return None;
        }
        let mut result = self.extract_analytical_length_result(config);
        if let Some(modeled) = self.extract_modeled_length_result(model) {
            if modeled.has_assigned_values {
                result.assignments = modeled.result.assignments;
            }
            if modeled.has_over_values {
                result.over_length_records = modeled.result.over_length_records;
            }
        }
        Some(result)
    }

    fn extract_analytical_length_result(
        &self,
        config: &LengthAssignmentModelingConfig<V>,
    ) -> LengthAssignmentResult<V> {
        let context = LengthAssignmentContext {
            derivation: DefaultLengthDerivation,
            phantom: std::marker::PhantomData,
        };
        context.assign(LengthAssignmentInput {
            dynamic_products: self
                .produce
                .demands
                .iter()
                .map(|demand| demand.product.clone())
                .filter(|product| config.is_dynamic_product(product))
                .collect(),
            demands: self.produce.demands.clone(),
            constraints: Vec::new(),
        })
    }

    fn extract_modeled_yield_analysis(
        &self,
        model: &MetaModel<f64>,
        produce: &Produce<V>,
    ) -> Option<YieldAnalysis<V>> {
        let r#yield = self.r#yield.as_ref()?;
        if !r#yield.has_any() {
            return None;
        }
        let mut under_productions = Vec::new();
        let mut over_productions = Vec::new();
        let mut has_solver_value = false;
        for (demand_index, demand) in r#yield.demands.iter().enumerate() {
            let under_value = r#yield
                .under_production
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
                .and_then(|token| token.get_result());
            has_solver_value |= under_value.is_some();
            if let Some(value) = under_value
                .filter(|value| *value > 0.0)
                .and_then(from_f64)
            {
                under_productions.push(ModeledUnderProduction {
                    demand: demand.clone(),
                    shortfall: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
            let over_value = r#yield
                .over_production
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
                .and_then(|token| token.get_result());
            has_solver_value |= over_value.is_some();
            if let Some(value) = over_value
                .filter(|value| *value > 0.0)
                .and_then(from_f64)
            {
                over_productions.push(ModeledOverProduction {
                    demand: demand.clone(),
                    surplus: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
        }
        if !has_solver_value {
            return None;
        }
        if under_productions.is_empty() && over_productions.is_empty() {
            return None;
        }
        Some(YieldAnalysis {
            under_productions,
            over_productions,
            outputs: analyze_yield(produce, &self.produce.demands).outputs,
        })
    }

    fn extract_modeled_length_result(&self, model: &MetaModel<f64>) -> Option<ModeledLengthExtraction<V>> {
        let length = self.length.as_ref()?;
        if !length.has_any() {
            return None;
        }
        let mut assignments = Vec::new();
        let mut over_length_records = Vec::new();
        let mut has_assigned_values = false;
        let mut has_over_values = false;
        for (demand_index, demand) in length.demands.iter().enumerate() {
            let assigned_value = length
                .assigned_length
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
                .and_then(|token| token.get_result());
            has_assigned_values |= assigned_value.is_some();
            if let Some(value) = assigned_value
                .filter(|value| *value >= 0.0)
                .and_then(from_f64)
            {
                assignments.push(LengthAssignment {
                    product: demand.product.clone(),
                    assigned_length: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                    batch_count: 1,
                });
            }
            let over_value = length
                .over_length
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
                .and_then(|token| token.get_result());
            has_over_values |= over_value.is_some();
            if let Some(value) = over_value
                .filter(|value| *value > 0.0)
                .and_then(from_f64)
            {
                over_length_records.push(OverLengthRecord {
                    product: demand.product.clone(),
                    over_length: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
        }
        if !has_assigned_values && !has_over_values {
            return None;
        }
        Some(ModeledLengthExtraction {
            result: LengthAssignmentResult {
                assignments,
                over_length_records,
            },
            has_assigned_values,
            has_over_values,
        })
    }
}

struct ModeledLengthExtraction<V: SolveValue> {
    result: LengthAssignmentResult<V>,
    has_assigned_values: bool,
    has_over_values: bool,
}

fn rest_material_value<V: SolveValue>(
    plan: &CuttingPlan<V>,
    measure: RestMaterialMeasure,
) -> Option<f64> {
    match measure {
        RestMaterialMeasure::RestWidthByMaterialLengthProxy => {
            let rest_width = plan.rest_width().and_then(|width| to_f64(&width.value))?;
            let material_length = plan
                .material
                .length
                .as_ref()
                .and_then(|length| to_f64(&length.value))?;
            Some(rest_width * material_length)
        }
    }
}

fn analyze_yield<V: SolveValue>(
    produce: &Produce<V>,
    demands: &[ProductDemand<V>],
) -> YieldAnalysis<V> {
    let mut supplied = BTreeMap::<(String, String), (ProductDemand<V>, f64)>::new();
    for usage in &produce.cutting_plans {
        for contribution in &usage.plan.demand_contributions {
            let key = (
                contribution.product.id.clone(),
                shadow_price_unit_symbol(&contribution.quantity.unit),
            );
            let Some(value) = to_f64(&contribution.quantity.value) else {
                continue;
            };
            let demand_like = ProductDemand {
                product: contribution.product.clone(),
                quantity: contribution.quantity.clone(),
                mode: None,
            };
            supplied
                .entry(key)
                .and_modify(|(_, total)| *total += value * usage.amount as f64)
                .or_insert((demand_like, value * usage.amount as f64));
        }
    }
    let mut under_productions = Vec::new();
    let mut over_productions = Vec::new();
    let mut outputs = Vec::new();
    for demand in demands {
        let key = (
            demand.product.id.clone(),
            shadow_price_unit_symbol(&demand.quantity.unit),
        );
        let supplied_value = supplied.get(&key).map(|(_, total)| *total).unwrap_or(0.0);
        let required = to_f64(&demand.quantity.value).unwrap_or(f64::INFINITY);
        if supplied_value > 0.0 {
            if let Some(value) = from_f64(supplied_value) {
                outputs.push(ProductOutput {
                    product: demand.product.clone(),
                    total_quantity: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                    mode: demand.mode,
                });
            }
        }
        if supplied_value + f64::EPSILON < required {
            if let Some(value) = from_f64(required - supplied_value) {
                under_productions.push(ModeledUnderProduction {
                    demand: demand.clone(),
                    shortfall: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
        } else if supplied_value > required + f64::EPSILON {
            if let Some(value) = from_f64(supplied_value - required) {
                over_productions.push(ModeledOverProduction {
                    demand: demand.clone(),
                    surplus: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
        }
    }
    YieldAnalysis {
        under_productions,
        over_productions,
        outputs,
    }
}

/// CSP1D 产出上下文 builder / CSP1D produce context builder
#[derive(Clone)]
pub struct Csp1dProduceContextBuilder<V: SolveValue> {
    input: ProduceInput<V>,
    yield_config: Option<YieldModelingConfig<V>>,
    waste_config: Option<WasteMinimizationConfig<V>>,
    length_config: Option<LengthAssignmentModelingConfig<V>>,
    mode: Csp1dModelingMode,
    is_final_milp: bool,
    extra_pipelines: Vec<Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>>,
    incremental_pipelines: Vec<Arc<dyn Csp1dIncrementalPipeline<V>>>,
    objective_policies: Vec<Arc<dyn Csp1dObjectivePolicy<V>>>,
    extensions: Vec<Csp1dModelingExtension<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dProduceContextBuilder<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dProduceContextBuilder")
            .field("input", &self.input)
            .field("has_yield_config", &self.yield_config.is_some())
            .field("has_waste_config", &self.waste_config.is_some())
            .field("has_length_config", &self.length_config.is_some())
            .field("mode", &self.mode)
            .field("is_final_milp", &self.is_final_milp)
            .field("extra_pipelines", &self.extra_pipelines.len())
            .field("incremental_pipelines", &self.incremental_pipelines.len())
            .field("objective_policies", &self.objective_policies.len())
            .field("extensions", &self.extensions.len())
            .finish()
    }
}

impl<V: SolveValue> Csp1dProduceContextBuilder<V> {
    /// 创建 builder / Create builder
    pub fn new(input: ProduceInput<V>) -> Self {
        Self {
            input,
            yield_config: None,
            waste_config: None,
            length_config: None,
            mode: Csp1dModelingMode::MILP,
            is_final_milp: false,
            extra_pipelines: Vec::new(),
            incremental_pipelines: Vec::new(),
            objective_policies: Vec::new(),
            extensions: Vec::new(),
        }
    }

    /// 设置 yield 配置 / Set yield config
    pub fn yield_config(&mut self, config: YieldModelingConfig<V>) -> &mut Self {
        self.yield_config = Some(config);
        self
    }

    /// 设置 waste 配置 / Set waste config
    pub fn waste_config(&mut self, config: WasteMinimizationConfig<V>) -> &mut Self {
        self.waste_config = Some(config);
        self
    }

    /// 设置 length 配置 / Set length config
    pub fn length_config(&mut self, config: LengthAssignmentModelingConfig<V>) -> &mut Self {
        self.length_config = Some(config);
        self
    }

    /// 设置建模模式 / Set modeling mode
    pub fn mode(&mut self, mode: Csp1dModelingMode) -> &mut Self {
        self.mode = mode;
        self
    }

    /// 设置最终 MILP 标记 / Set final MILP flag
    pub fn is_final_milp(&mut self, is_final_milp: bool) -> &mut Self {
        self.is_final_milp = is_final_milp;
        self
    }

    /// 追加额外管线 / Add extra pipeline
    pub fn extra_pipeline(
        &mut self,
        pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>,
    ) -> &mut Self {
        self.extra_pipelines.push(pipeline);
        self
    }

    /// 追加增量管线 / Add incremental pipeline
    pub fn incremental_pipeline(
        &mut self,
        pipeline: Arc<dyn Csp1dIncrementalPipeline<V>>,
    ) -> &mut Self {
        self.incremental_pipelines.push(pipeline);
        self
    }

    /// 追加目标策略 / Add objective policy
    pub fn objective_policy(&mut self, policy: Arc<dyn Csp1dObjectivePolicy<V>>) -> &mut Self {
        self.objective_policies.push(policy);
        self
    }

    /// 追加建模扩展 / Add modeling extension
    pub fn extension(&mut self, extension: Csp1dModelingExtension<V>) -> &mut Self {
        self.extensions.push(extension);
        self
    }

    /// 构建上下文 / Build context
    pub fn build(&self) -> crate::Csp1dResult<Csp1dProduceContext<V>> {
        let domain_value_sample = resolve_domain_value_sample(
            &self.input,
            self.yield_config.as_ref(),
        )
        .ok_or_else(|| crate::Csp1dError::InvalidInput {
            message: "Cannot derive domain value sample from ProduceInput".into(),
        })?;
        let produce = ProduceAggregation::new(
            self.input.cutting_plans.clone(),
            self.input.demands.clone(),
            self.input.materials.clone(),
            self.input.machines.clone(),
            self.input.warm_start_plan_usages.clone(),
        );
        let needs_over_slack_for_over_area = self
            .waste_config
            .as_ref()
            .and_then(|config| config.over_production_area_penalty.as_ref())
            .is_some();
        let mut context = Csp1dProduceContext {
            produce,
            r#yield: self
                .yield_config
                .as_ref()
                .filter(|_| self.mode == Csp1dModelingMode::MILP)
                .map(|config| {
                    YieldSlackAggregation::new(
                        config.clone(),
                        self.input.demands.clone(),
                        needs_over_slack_for_over_area,
                    )
                }),
            waste: if self.mode == Csp1dModelingMode::MILP && self.waste_config.is_some() {
                Some(WasteAggregation {
                    analysis: None,
                })
            } else {
                None
            },
            length: self
                .length_config
                .as_ref()
                .filter(|config| self.mode == Csp1dModelingMode::MILP && config.enabled)
                .map(|config| {
                    LengthSlackAggregation::new(config.clone(), self.input.demands.clone())
                }),
            constraint_pipelines: Vec::new(),
            cg_pipelines: Vec::new(),
            extra_pipelines: self.extra_pipelines.clone(),
            incremental_pipelines: self.incremental_pipelines.clone(),
            mode: self.mode,
            is_final_milp: self.is_final_milp,
            warm_start_plan_usages: self.input.warm_start_plan_usages.clone(),
            objective_policies: self.objective_policies.clone(),
            domain_value_sample,
            yield_config: self.yield_config.clone(),
            waste_config: self.waste_config.clone(),
            length_config: self.length_config.clone(),
        };
        for extension in &self.extensions {
            if extension.mode.matches(self.mode, self.is_final_milp) {
                if let Some(pipeline) = extension.resolve_pipeline(Some(&context)) {
                    context.extra_pipelines.push(pipeline);
                }
            }
        }
        Ok(context)
    }
}

fn resolve_domain_value_sample<V: SolveValue>(
    input: &ProduceInput<V>,
    yield_config: Option<&YieldModelingConfig<V>>,
) -> Option<V> {
    input
        .demands
        .first()
        .map(|demand| demand.quantity.value.clone())
        .or_else(|| {
            input
                .materials
                .first()
                .map(|material| material.width_range.lower_bound.value.clone())
        })
        .or_else(|| {
            input
                .cutting_plans
                .first()
                .and_then(|plan| plan.rest_width())
                .map(|width| width.value)
        })
        .or_else(|| {
            yield_config
                .and_then(|config| config.under_production_penalty.values().next().cloned())
        })
}
