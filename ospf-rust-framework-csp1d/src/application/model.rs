//! CSP1D 应用模型 / CSP1D application models
//!
//! 保持公开问题、配置、结果和 builder 入口。
//! Keeps public problem, configuration, result, and builder entry points.

use std::collections::BTreeMap;
use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::SolveValue;
use ospf_rust_core::variable::{UInteger, VariableRange};
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    render_cutting_plan, Costar, CuttingPlan, Machine, Material, Product, ProductDemand,
};
use crate::domain::produce::{
    Csp1dDomainPolicy, Csp1dExtensionMode, Csp1dExtensionSet, Csp1dExtractionPolicy,
    Csp1dFlowPolicy, Csp1dGenerationStrategy, Csp1dModelingContext, Csp1dModelingExtension,
    Csp1dObjectivePolicy, Csp1dPricingPolicy, Produce,
};
use crate::domain::r#yield::YieldModelingConfig;
use crate::domain::length_assignment::LengthAssignmentResult;
use crate::domain::wasting_minimization::WasteMinimizationConfig;
use crate::infrastructure::dto::RenderSchemaDTO;

/// CSP1D 解状态 / CSP1D solution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dSolutionStatus {
    /// 已得到最终 MILP 解 / Final MILP solution is available
    Feasible,
    /// 只有方案池或中间结果可用 / Only plan pool or intermediate result is available
    Partial,
    /// 无初始方案 / No initial plans
    NoInitialPlans,
    /// 求解失败且无可用部分结果 / Solve failed without usable partial result
    Failed,
}

/// CSP1D 方案使用量变量组 / CSP1D plan-assignment variable group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csp1dAssignment {
    /// 方案使用量变量索引 / Plan-usage variable indices
    pub x: Vec<usize>,
    /// 方案数量 / Plan count
    pub plan_count: usize,
}

impl Csp1dAssignment {
    /// 创建空 assignment helper / Create an empty assignment helper
    pub fn create(plan_count: usize) -> Self {
        Self {
            x: Vec::with_capacity(plan_count),
            plan_count,
        }
    }

    /// 获取变量索引 / Get variable index
    pub fn get(&self, index: usize) -> Option<usize> {
        self.x.get(index).copied()
    }

    /// 注册方案使用量变量 / Register plan-usage variables
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        self.x.clear();
        for index in 0..self.plan_count {
            let variable_index = model
                .register_auto_variable_with_range::<UInteger>(
                    &format!("x_{index}"),
                    VariableRange::new(Some(0.0), None),
                )
                .map_err(|error| crate::Csp1dError::Calculation {
                    message: format!("register CSP1D assignment variable failed: {error}"),
                })?;
            self.x.push(variable_index);
        }
        Ok(())
    }
}

/// CSP1D KPI 稳定字段名 / Stable CSP1D KPI keys
pub struct Csp1dKpiKeys;

#[allow(non_upper_case_globals, non_snake_case)]
impl Csp1dKpiKeys {
    pub const SelectedPlanCount: &'static str = "selectedPlanCount";
    pub const SelectedBatchCount: &'static str = "selectedBatchCount";
    pub const SatisfiedDemandCount: &'static str = "satisfiedDemandCount";
    pub const UnmetDemandCount: &'static str = "unmetDemandCount";
    pub const MaterialUsageCount: &'static str = "materialUsageCount";
    pub const MachineUsageCount: &'static str = "machineUsageCount";
    pub const GeneratedPlanCount: &'static str = "generatedPlanCount";
    pub const TopPlanCount: &'static str = "topPlanCount";
    pub const YieldMetricCount: &'static str = "yieldMetricCount";
    pub const WasteMetricCount: &'static str = "wasteMetricCount";
    pub const LengthMetricCount: &'static str = "lengthMetricCount";
    pub const SolutionStatus: &'static str = "solutionStatus";
    pub const TerminationReason: &'static str = "terminationReason";
    pub const FinalMilpStatus: &'static str = "finalMilpStatus";
    pub const PartialSolutionAvailable: &'static str = "partialSolutionAvailable";
    pub const FailureMessage: &'static str = "failureMessage";
    pub const ColumnGenerationTerminationReason: &'static str = "columnGeneration.terminationReason";
    pub const ColumnGenerationIterationCount: &'static str = "columnGeneration.iterationCount";
    pub const ColumnGenerationPricedPlanCount: &'static str = "columnGeneration.pricedPlanCount";
    pub const ColumnGenerationLastLpObjective: &'static str = "columnGeneration.lastLpObjective";
    pub const ColumnGenerationLastPlanCount: &'static str = "columnGeneration.lastPlanCount";
    pub const InitialGenerationVisitedNodes: &'static str = "initialGeneration.visitedNodes";
    pub const InitialGenerationGeneratedCandidates: &'static str = "initialGeneration.generatedCandidates";
    pub const InitialGenerationAcceptedPlans: &'static str = "initialGeneration.acceptedPlans";
    pub const InitialGenerationInfeasibleCandidates: &'static str = "initialGeneration.infeasibleCandidates";
    pub const InitialGenerationDuplicateCandidates: &'static str = "initialGeneration.duplicateCandidates";
    pub const InitialGenerationDominatedCandidates: &'static str = "initialGeneration.dominatedCandidates";
    pub const InitialGenerationWidthBoundPrunedNodes: &'static str = "initialGeneration.widthBoundPrunedNodes";
    pub const InitialGenerationKnifeBoundPrunedNodes: &'static str = "initialGeneration.knifeBoundPrunedNodes";
    pub const InitialGenerationLengthBoundPrunedEntries: &'static str = "initialGeneration.lengthBoundPrunedEntries";
    pub const InitialGenerationMaterialWidthIndexCacheHits: &'static str = "initialGeneration.materialWidthIndexCacheHits";
    pub const InitialGenerationMaterialSliceTemplateCacheHits: &'static str = "initialGeneration.materialSliceTemplateCacheHits";
    pub const InitialGenerationQuantityCacheHits: &'static str = "initialGeneration.quantityCacheHits";
    pub const InitialGenerationQuantityCacheMisses: &'static str = "initialGeneration.quantityCacheMisses";
    pub const InitialGenerationMaterialSliceTemplateCacheMisses: &'static str = "initialGeneration.materialSliceTemplateCacheMisses";
    pub const InitialGenerationCrossWorkerDuplicateCandidates: &'static str = "initialGeneration.crossWorkerDuplicateCandidates";
    pub const InitialGenerationCrossContributionDominated: &'static str = "initialGeneration.crossContributionDominated";
    pub const InitialGenerationElapsedMilliseconds: &'static str = "initialGeneration.elapsedMilliseconds";
    pub const InitialGenerationStopReason: &'static str = "initialGeneration.stopReason";
    pub const InitialVisitedNodes: &'static str = "initialVisitedNodes";
    pub const InitialGeneratedCandidates: &'static str = "initialGeneratedCandidates";
    pub const InitialAcceptedPlans: &'static str = "initialAcceptedPlans";
    pub const InitialInfeasibleCandidates: &'static str = "initialInfeasibleCandidates";
    pub const InitialDuplicateCandidates: &'static str = "initialDuplicateCandidates";
    pub const InitialDominatedCandidates: &'static str = "initialDominatedCandidates";
    pub const InitialWidthBoundPrunedNodes: &'static str = "initialWidthBoundPrunedNodes";
    pub const InitialKnifeBoundPrunedNodes: &'static str = "initialKnifeBoundPrunedNodes";
    pub const InitialLengthBoundPrunedEntries: &'static str = "initialLengthBoundPrunedEntries";
    pub const InitialMaterialWidthIndexCacheHits: &'static str = "initialMaterialWidthIndexCacheHits";
    pub const InitialMaterialSliceTemplateCacheHits: &'static str = "initialMaterialSliceTemplateCacheHits";
    pub const InitialGenerationElapsedMillisecondsRender: &'static str = "initialGenerationElapsedMilliseconds";
    pub const InitialGenerationStopReasonRender: &'static str = "initialGenerationStopReason";
    pub const PricingVisitedNodes: &'static str = "pricingGeneration.visitedNodes";
    pub const PricingGeneratedCandidates: &'static str = "pricingGeneration.generatedCandidates";
    pub const PricingAcceptedPlans: &'static str = "pricingGeneration.acceptedPlans";
    pub const PricingInfeasibleCandidates: &'static str = "pricingGeneration.infeasibleCandidates";
    pub const PricingDuplicateCandidates: &'static str = "pricingGeneration.duplicateCandidates";
    pub const PricingDominatedCandidates: &'static str = "pricingGeneration.dominatedCandidates";
    pub const PricingElapsedMilliseconds: &'static str = "pricingGeneration.elapsedMilliseconds";
    pub const PricingStopReason: &'static str = "pricingGeneration.stopReason";
    pub const LpFailureMessage: &'static str = "lpFailureMessage";
    pub const TotalTrimWidth: &'static str = "totalTrimWidth";
    pub const TotalRestMaterial: &'static str = "totalRestMaterial";
    pub const OverProductionArea: &'static str = "overProductionArea";
    pub const OverProductionAreaMeasure: &'static str = "overProductionAreaMeasure";
    pub const RestMaterialMeasure: &'static str = "restMaterialMeasure";

    pub fn materialUsageBatchCount(material_id: &str) -> String {
        format!("materialUsage.{material_id}.batchCount")
    }

    pub fn machineCapacityUsed(machine_id: &str) -> String {
        format!("machineCapacityUsed.{machine_id}")
    }

    pub fn underProduction(product_id: &str, unit_symbol: &str) -> String {
        format!("underProduction.{product_id}.{unit_symbol}")
    }

    pub fn overProduction(product_id: &str, unit_symbol: &str) -> String {
        format!("overProduction.{product_id}.{unit_symbol}")
    }

    pub fn materialCost(material_id: &str) -> String {
        format!("materialCost.{material_id}")
    }

    pub fn assignedLength(product_id: &str) -> String {
        format!("assignedLength.{product_id}")
    }

    pub fn overLength(product_id: &str) -> String {
        format!("overLength.{product_id}")
    }
}

/// CSP1D KPI / CSP1D KPI
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Csp1dKpi {
    pub selected_plan_count: u64,
    pub selected_batch_count: u64,
    pub satisfied_demand_count: u64,
    pub unmet_demand_count: u64,
    pub material_usage_count: u64,
    pub machine_usage_count: u64,
    pub generated_plan_count: u64,
    pub top_plan_count: u64,
    pub yield_metric_count: u64,
    pub waste_metric_count: u64,
    pub length_metric_count: u64,
    pub details: BTreeMap<String, String>,
}

/// CSP1D 问题定义 / CSP1D problem definition
#[derive(Debug, Clone)]
pub struct Csp1dProblem<V: SolveValue> {
    pub products: Vec<Product<V>>,
    pub materials: Vec<Material<V>>,
    pub machines: Vec<Machine<V>>,
    pub costars: Vec<Costar<V>>,
    pub demands: Vec<ProductDemand<V>>,
    pub configuration: Csp1dConfiguration,
    pub solve_config: Option<Csp1dSolveConfig<V>>,
}

impl<V: SolveValue> Csp1dProblem<V> {
    pub fn new(
        products: Vec<Product<V>>,
        materials: Vec<Material<V>>,
        machines: Vec<Machine<V>>,
        demands: Vec<ProductDemand<V>>,
    ) -> Self {
        Self {
            products,
            materials,
            machines,
            costars: Vec::new(),
            demands,
            configuration: Csp1dConfiguration::default(),
            solve_config: None,
        }
    }
}

/// CSP1D 问题 builder / CSP1D problem builder
#[derive(Debug, Clone)]
pub struct Csp1dProblemBuilder<V: SolveValue> {
    products: Vec<Product<V>>,
    materials: Vec<Material<V>>,
    machines: Vec<Machine<V>>,
    costars: Vec<Costar<V>>,
    demands: Vec<ProductDemand<V>>,
    configuration: Csp1dConfiguration,
    solve_config: Option<Csp1dSolveConfig<V>>,
}

impl<V: SolveValue> Default for Csp1dProblemBuilder<V> {
    fn default() -> Self {
        Self {
            products: Vec::new(),
            materials: Vec::new(),
            machines: Vec::new(),
            costars: Vec::new(),
            demands: Vec::new(),
            configuration: Csp1dConfiguration::default(),
            solve_config: None,
        }
    }
}

impl<V: SolveValue> Csp1dProblemBuilder<V> {
    /// 增加产品 / Add a product
    pub fn product(&mut self, product: Product<V>) -> &mut Self {
        self.products.push(product);
        self
    }

    /// 增加产品列表 / Add products
    pub fn products<I>(&mut self, products: I) -> &mut Self
    where
        I: IntoIterator<Item = Product<V>>,
    {
        self.products.extend(products);
        self
    }

    /// 增加物料 / Add a material
    pub fn material(&mut self, material: Material<V>) -> &mut Self {
        self.materials.push(material);
        self
    }

    /// 增加物料列表 / Add materials
    pub fn materials<I>(&mut self, materials: I) -> &mut Self
    where
        I: IntoIterator<Item = Material<V>>,
    {
        self.materials.extend(materials);
        self
    }

    /// 增加设备 / Add a machine
    pub fn machine(&mut self, machine: Machine<V>) -> &mut Self {
        self.machines.push(machine);
        self
    }

    /// 增加设备列表 / Add machines
    pub fn machines<I>(&mut self, machines: I) -> &mut Self
    where
        I: IntoIterator<Item = Machine<V>>,
    {
        self.machines.extend(machines);
        self
    }

    /// 增加配规 / Add a costar
    pub fn costar(&mut self, costar: Costar<V>) -> &mut Self {
        self.costars.push(costar);
        self
    }

    /// 增加配规列表 / Add costars
    pub fn costars<I>(&mut self, costars: I) -> &mut Self
    where
        I: IntoIterator<Item = Costar<V>>,
    {
        self.costars.extend(costars);
        self
    }

    /// 增加需求 / Add a demand
    pub fn demand(&mut self, demand: ProductDemand<V>) -> &mut Self {
        self.demands.push(demand);
        self
    }

    /// 增加需求列表 / Add demands
    pub fn demands<I>(&mut self, demands: I) -> &mut Self
    where
        I: IntoIterator<Item = ProductDemand<V>>,
    {
        self.demands.extend(demands);
        self
    }

    /// 设置列生成配置 / Set column generation configuration
    pub fn configuration(&mut self, configuration: Csp1dConfiguration) -> &mut Self {
        self.configuration = configuration;
        self
    }

    /// 设置一站式求解配置 / Set one-stop solve configuration
    pub fn solve_config(&mut self, solve_config: Csp1dSolveConfig<V>) -> &mut Self {
        self.solve_config = Some(solve_config);
        self
    }

    /// 通过 builder 设置求解配置 / Set solve configuration by builder
    pub fn solve_config_with<F>(&mut self, block: F) -> &mut Self
    where
        F: FnOnce(&mut Csp1dSolveConfigBuilder<V>),
    {
        self.solve_config = Some(csp1d_solve_config(block));
        self
    }

    /// 构建问题定义 / Build problem definition
    pub fn build(&self) -> Csp1dProblem<V> {
        Csp1dProblem {
            products: self.products.clone(),
            materials: self.materials.clone(),
            machines: self.machines.clone(),
            costars: self.costars.clone(),
            demands: self.demands.clone(),
            configuration: self.configuration,
            solve_config: self.solve_config.clone(),
        }
    }
}

/// CSP1D 求解配置 / CSP1D solving configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Csp1dConfiguration {
    pub max_initial_plans: u64,
    pub max_pricing_plans: u64,
    pub iteration_limit: u64,
}

impl Default for Csp1dConfiguration {
    fn default() -> Self {
        Self {
            max_initial_plans: 1024,
            max_pricing_plans: 64,
            iteration_limit: 8,
        }
    }
}

/// CSP1D 一站式求解配置 / CSP1D one-stop solve configuration
#[derive(Debug, Clone)]
pub struct Csp1dSolveConfig<V: SolveValue> {
    pub column_generation: Csp1dConfiguration,
    pub yield_config: Option<YieldModelingConfig<V>>,
    pub waste_config: Option<WasteMinimizationConfig<V>>,
    pub length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    pub top_k_plan_limit: Option<u64>,
    pub allow_partial_solution: bool,
    pub extensions: Vec<Csp1dModelingExtension<V>>,
    pub extension_set: Csp1dExtensionSet<V>,
}

impl<V: SolveValue> Default for Csp1dSolveConfig<V> {
    fn default() -> Self {
        Self {
            column_generation: Csp1dConfiguration::default(),
            yield_config: None,
            waste_config: None,
            length_config: None,
            top_k_plan_limit: None,
            allow_partial_solution: true,
            extensions: Vec::new(),
            extension_set: Csp1dExtensionSet::default(),
        }
    }
}

impl<V: SolveValue> Csp1dSolveConfig<V> {
    pub fn all_extensions(&self) -> Vec<Csp1dModelingExtension<V>> {
        let mut extensions = self.extensions.clone();
        for extension in &self.extension_set.modeling_extensions {
            if !extensions.iter().any(|existing| same_extension(existing, extension)) {
                extensions.push(extension.clone());
            }
        }
        extensions
    }
}

fn same_extension<V: SolveValue>(
    left: &Csp1dModelingExtension<V>,
    right: &Csp1dModelingExtension<V>,
) -> bool {
    left.mode == right.mode
        && match (
            &left.pipeline,
            &right.pipeline,
            &left.context_aware_pipeline,
            &right.context_aware_pipeline,
        ) {
            (Some(left), Some(right), None, None) => Arc::ptr_eq(left, right),
            (None, None, Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None, None, None) => true,
            _ => false,
        }
}

/// CSP1D 求解配置 builder / CSP1D solve configuration builder
#[derive(Clone)]
pub struct Csp1dSolveConfigBuilder<V: SolveValue> {
    column_generation: Csp1dConfiguration,
    yield_config: Option<YieldModelingConfig<V>>,
    waste_config: Option<WasteMinimizationConfig<V>>,
    length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    top_k_plan_limit: Option<u64>,
    allow_partial_solution: bool,
    extensions: Vec<Csp1dModelingExtension<V>>,
    domain_policies: Vec<Arc<dyn Csp1dDomainPolicy<V>>>,
    objective_policies: Vec<Arc<dyn Csp1dObjectivePolicy<V>>>,
    generation_strategies: Vec<Arc<dyn Csp1dGenerationStrategy<V>>>,
    pricing_policies: Vec<Arc<dyn Csp1dPricingPolicy<V>>>,
    flow_policies: Vec<Arc<dyn Csp1dFlowPolicy<V>>>,
    extraction_policies: Vec<Arc<dyn Csp1dExtractionPolicy<V>>>,
}

impl<V: SolveValue> Default for Csp1dSolveConfigBuilder<V> {
    fn default() -> Self {
        Self {
            column_generation: Csp1dConfiguration::default(),
            yield_config: None,
            waste_config: None,
            length_config: None,
            top_k_plan_limit: None,
            allow_partial_solution: true,
            extensions: Vec::new(),
            domain_policies: Vec::new(),
            objective_policies: Vec::new(),
            generation_strategies: Vec::new(),
            pricing_policies: Vec::new(),
            flow_policies: Vec::new(),
            extraction_policies: Vec::new(),
        }
    }
}

impl<V: SolveValue> std::fmt::Debug for Csp1dSolveConfigBuilder<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dSolveConfigBuilder")
            .field("column_generation", &self.column_generation)
            .field("has_yield_config", &self.yield_config.is_some())
            .field("has_waste_config", &self.waste_config.is_some())
            .field("has_length_config", &self.length_config.is_some())
            .field("top_k_plan_limit", &self.top_k_plan_limit)
            .field("allow_partial_solution", &self.allow_partial_solution)
            .field("extensions", &self.extensions.len())
            .field("domain_policies", &self.domain_policies.len())
            .field("objective_policies", &self.objective_policies.len())
            .field("generation_strategies", &self.generation_strategies.len())
            .field("pricing_policies", &self.pricing_policies.len())
            .field("flow_policies", &self.flow_policies.len())
            .field("extraction_policies", &self.extraction_policies.len())
            .finish()
    }
}

impl<V: SolveValue> Csp1dSolveConfigBuilder<V> {
    /// 设置列生成配置 / Set column generation configuration
    pub fn column_generation(&mut self, configuration: Csp1dConfiguration) -> &mut Self {
        self.column_generation = configuration;
        self
    }

    /// 设置列生成上限 / Set column generation limits
    pub fn column_generation_limits(
        &mut self,
        max_initial_plans: u64,
        max_pricing_plans: u64,
        iteration_limit: u64,
    ) -> &mut Self {
        self.column_generation = Csp1dConfiguration {
            max_initial_plans,
            max_pricing_plans,
            iteration_limit,
        };
        self
    }

    /// 设置 yield 配置 / Set yield configuration
    pub fn yield_config(&mut self, config: Option<YieldModelingConfig<V>>) -> &mut Self {
        self.yield_config = config;
        self
    }

    /// 设置 waste 配置 / Set waste configuration
    pub fn waste_config(&mut self, config: Option<WasteMinimizationConfig<V>>) -> &mut Self {
        self.waste_config = config;
        self
    }

    /// 设置 length 配置 / Set length configuration
    pub fn length_config(
        &mut self,
        config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    ) -> &mut Self {
        self.length_config = config;
        self
    }

    /// 设置 Top-K 方案上限 / Set Top-K plan limit
    pub fn top_k_plan_limit(&mut self, limit: Option<u64>) -> &mut Self {
        self.top_k_plan_limit = limit;
        self
    }

    /// 设置是否允许部分结果 / Set whether partial solution is allowed
    pub fn allow_partial_solution(&mut self, enabled: bool) -> &mut Self {
        self.allow_partial_solution = enabled;
        self
    }

    /// 追加建模扩展 / Add a modeling extension
    pub fn extension(&mut self, extension: Csp1dModelingExtension<V>) -> &mut Self {
        self.extensions.push(extension);
        self
    }

    /// 追加建模扩展列表 / Add modeling extensions
    pub fn extensions<I>(&mut self, extensions: I) -> &mut Self
    where
        I: IntoIterator<Item = Csp1dModelingExtension<V>>,
    {
        self.extensions.extend(extensions);
        self
    }

    /// 追加扩展管线 / Add an extension pipeline
    pub fn extension_pipeline(
        &mut self,
        pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>,
    ) -> &mut Self {
        self.extensions.push(Csp1dModelingExtension::new(pipeline));
        self
    }

    /// 追加指定模式的扩展管线 / Add an extension pipeline with mode
    pub fn extension_pipeline_with_mode(
        &mut self,
        pipeline: Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>,
        mode: Csp1dExtensionMode,
    ) -> &mut Self {
        self.extensions.push(Csp1dModelingExtension::with_mode(pipeline, mode));
        self
    }

    /// 追加上下文感知扩展管线 / Add a context-aware extension pipeline
    pub fn context_aware_extension_pipeline<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>
            + Send
            + Sync
            + 'static,
    {
        self.extensions
            .push(Csp1dModelingExtension::context_aware(Arc::new(factory)));
        self
    }

    /// 追加指定模式的上下文感知扩展管线 / Add a context-aware extension pipeline with mode
    pub fn context_aware_extension_pipeline_with_mode<F>(
        &mut self,
        factory: F,
        mode: Csp1dExtensionMode,
    ) -> &mut Self
    where
        F: Fn(&dyn Csp1dModelingContext<V>) -> Arc<dyn Pipeline<MetaModel<f64>> + Send + Sync>
            + Send
            + Sync
            + 'static,
    {
        self.extensions
            .push(Csp1dModelingExtension::context_aware_with_mode(Arc::new(factory), mode));
        self
    }

    /// 追加领域策略 / Add a domain policy
    pub fn domain_policy(&mut self, policy: Arc<dyn Csp1dDomainPolicy<V>>) -> &mut Self {
        self.domain_policies.push(policy);
        self
    }

    /// 追加目标策略 / Add an objective policy
    pub fn objective_policy(&mut self, policy: Arc<dyn Csp1dObjectivePolicy<V>>) -> &mut Self {
        self.objective_policies.push(policy);
        self
    }

    /// 追加生成策略 / Add a generation strategy
    pub fn generation_strategy(&mut self, strategy: Arc<dyn Csp1dGenerationStrategy<V>>) -> &mut Self {
        self.generation_strategies.push(strategy);
        self
    }

    /// 追加定价策略 / Add a pricing policy
    pub fn pricing_policy(&mut self, policy: Arc<dyn Csp1dPricingPolicy<V>>) -> &mut Self {
        self.pricing_policies.push(policy);
        self
    }

    /// 追加流程策略 / Add a flow policy
    pub fn flow_policy(&mut self, policy: Arc<dyn Csp1dFlowPolicy<V>>) -> &mut Self {
        self.flow_policies.push(policy);
        self
    }

    /// 追加提取策略 / Add an extraction policy
    pub fn extraction_policy(&mut self, policy: Arc<dyn Csp1dExtractionPolicy<V>>) -> &mut Self {
        self.extraction_policies.push(policy);
        self
    }

    /// 构建求解配置 / Build solve configuration
    pub fn build(&self) -> Csp1dSolveConfig<V> {
        Csp1dSolveConfig {
            column_generation: self.column_generation,
            yield_config: self.yield_config.clone(),
            waste_config: self.waste_config.clone(),
            length_config: self.length_config.clone(),
            top_k_plan_limit: self.top_k_plan_limit,
            allow_partial_solution: self.allow_partial_solution,
            extensions: self.extensions.clone(),
            extension_set: Csp1dExtensionSet {
                modeling_extensions: Vec::new(),
                domain_policies: self.domain_policies.clone(),
                objective_policies: self.objective_policies.clone(),
                generation_strategies: self.generation_strategies.clone(),
                pricing_policies: self.pricing_policies.clone(),
                flow_policies: self.flow_policies.clone(),
                extraction_policies: self.extraction_policies.clone(),
            },
        }
    }
}

/// 构建 CSP1D 问题 / Build a CSP1D problem
pub fn csp1d_problem<V, F>(block: F) -> Csp1dProblem<V>
where
    V: SolveValue,
    F: FnOnce(&mut Csp1dProblemBuilder<V>),
{
    let mut builder = Csp1dProblemBuilder::default();
    block(&mut builder);
    builder.build()
}

/// 构建 CSP1D 求解配置 / Build a CSP1D solve configuration
pub fn csp1d_solve_config<V, F>(block: F) -> Csp1dSolveConfig<V>
where
    V: SolveValue,
    F: FnOnce(&mut Csp1dSolveConfigBuilder<V>),
{
    let mut builder = Csp1dSolveConfigBuilder::default();
    block(&mut builder);
    builder.build()
}

/// CSP1D 解 / CSP1D solution
#[derive(Debug, Clone)]
pub struct Csp1dSolution<V: SolveValue> {
    pub produce: Produce<V>,
    pub yield_result: Option<crate::domain::r#yield::YieldModelingResult<V>>,
    pub waste_result: Option<crate::domain::wasting_minimization::WasteMinimizationResult<V>>,
    pub length_result: Option<LengthAssignmentResult<V>>,
    pub generated_plans: Vec<CuttingPlan<V>>,
    pub kpi: Csp1dKpi,
    pub render: RenderSchemaDTO,
    pub status: Csp1dSolutionStatus,
    pub failure_message: Option<String>,
    pub top_plans: Vec<CuttingPlan<V>>,
}

/// CSP1D 解分析器 / CSP1D solution analyzer
pub trait Csp1dSolutionAnalyzer<V: SolveValue>: Send + Sync {
    fn analyze(
        &self,
        problem: &Csp1dProblem<V>,
        produce: Produce<V>,
        generated_plans: Vec<CuttingPlan<V>>,
    ) -> Csp1dSolution<V>;
}

/// 默认解分析器 / Default solution analyzer
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultCsp1dSolutionAnalyzer;

impl<V: SolveValue> Csp1dSolutionAnalyzer<V> for DefaultCsp1dSolutionAnalyzer {
    fn analyze(
        &self,
        problem: &Csp1dProblem<V>,
        produce: Produce<V>,
        generated_plans: Vec<CuttingPlan<V>>,
    ) -> Csp1dSolution<V> {
        let selected_batch_count = produce
            .cutting_plans
            .iter()
            .fold(0_u64, |acc, usage| acc.saturating_add(usage.amount));
        let satisfied_demand_count = problem
            .demands
            .len()
            .saturating_sub(produce.unmet_demands.len()) as u64;
        let mut kpi = Csp1dKpi {
            selected_plan_count: produce.cutting_plans.len() as u64,
            selected_batch_count,
            satisfied_demand_count,
            unmet_demand_count: produce.unmet_demands.len() as u64,
            material_usage_count: produce.material_usages.len() as u64,
            machine_usage_count: produce.machine_usages.len() as u64,
            generated_plan_count: generated_plans.len() as u64,
            top_plan_count: 0,
            yield_metric_count: 0,
            waste_metric_count: 0,
            length_metric_count: 0,
            details: BTreeMap::new(),
        };
        let mut kpi_map = BTreeMap::new();
        kpi_map.insert(Csp1dKpiKeys::SelectedPlanCount.to_string(), kpi.selected_plan_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::SelectedBatchCount.to_string(), kpi.selected_batch_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::SatisfiedDemandCount.to_string(), kpi.satisfied_demand_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::UnmetDemandCount.to_string(), kpi.unmet_demand_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::MaterialUsageCount.to_string(), kpi.material_usage_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::MachineUsageCount.to_string(), kpi.machine_usage_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::GeneratedPlanCount.to_string(), kpi.generated_plan_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::TopPlanCount.to_string(), kpi.top_plan_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::YieldMetricCount.to_string(), kpi.yield_metric_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::WasteMetricCount.to_string(), kpi.waste_metric_count.to_string());
        kpi_map.insert(Csp1dKpiKeys::LengthMetricCount.to_string(), kpi.length_metric_count.to_string());
        kpi.details = kpi_map.clone();
        let render = RenderSchemaDTO {
            kpi: kpi_map,
            cutting_plans: produce
                .cutting_plans
                .iter()
                .map(|usage| render_cutting_plan(&usage.plan, usage.amount))
                .collect(),
        };
        Csp1dSolution {
            produce,
            yield_result: None,
            waste_result: None,
            length_result: None,
            generated_plans,
            kpi,
            render,
            status: Csp1dSolutionStatus::Feasible,
            failure_message: None,
            top_plans: Vec::new(),
        }
    }
}
