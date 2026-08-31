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
    /// 选中方案数 / Selected plan count
    pub const SelectedPlanCount: &'static str = "selectedPlanCount";
    /// 选中批次数 / Selected batch count
    pub const SelectedBatchCount: &'static str = "selectedBatchCount";
    /// 满足需求数 / Satisfied demand count
    pub const SatisfiedDemandCount: &'static str = "satisfiedDemandCount";
    /// 未满足需求数 / Unmet demand count
    pub const UnmetDemandCount: &'static str = "unmetDemandCount";
    /// 物料使用数 / Material usage count
    pub const MaterialUsageCount: &'static str = "materialUsageCount";
    /// 设备使用数 / Machine usage count
    pub const MachineUsageCount: &'static str = "machineUsageCount";
    /// 生成方案数 / Generated plan count
    pub const GeneratedPlanCount: &'static str = "generatedPlanCount";
    /// Top 方案数 / Top plan count
    pub const TopPlanCount: &'static str = "topPlanCount";
    /// 产出率指标数 / Yield metric count
    pub const YieldMetricCount: &'static str = "yieldMetricCount";
    /// 损耗指标数 / Waste metric count
    pub const WasteMetricCount: &'static str = "wasteMetricCount";
    /// 长度指标数 / Length metric count
    pub const LengthMetricCount: &'static str = "lengthMetricCount";
    /// 解状态 / Solution status
    pub const SolutionStatus: &'static str = "solutionStatus";
    /// 终止原因 / Termination reason
    pub const TerminationReason: &'static str = "terminationReason";
    /// 最终 MILP 状态 / Final MILP status
    pub const FinalMilpStatus: &'static str = "finalMilpStatus";
    /// 部分解是否可用 / Whether partial solution is available
    pub const PartialSolutionAvailable: &'static str = "partialSolutionAvailable";
    /// 失败信息 / Failure message
    pub const FailureMessage: &'static str = "failureMessage";
    /// 列生成终止原因 / Column generation termination reason
    pub const ColumnGenerationTerminationReason: &'static str = "columnGeneration.terminationReason";
    /// 列生成迭代次数 / Column generation iteration count
    pub const ColumnGenerationIterationCount: &'static str = "columnGeneration.iterationCount";
    /// 列生成定价方案数 / Column generation priced plan count
    pub const ColumnGenerationPricedPlanCount: &'static str = "columnGeneration.pricedPlanCount";
    /// 列生成最后 LP 目标值 / Column generation last LP objective
    pub const ColumnGenerationLastLpObjective: &'static str = "columnGeneration.lastLpObjective";
    /// 列生成最后方案数 / Column generation last plan count
    pub const ColumnGenerationLastPlanCount: &'static str = "columnGeneration.lastPlanCount";
    /// 初始生成访问节点数 / Initial generation visited nodes
    pub const InitialGenerationVisitedNodes: &'static str = "initialGeneration.visitedNodes";
    /// 初始生成候选数 / Initial generation generated candidates
    pub const InitialGenerationGeneratedCandidates: &'static str = "initialGeneration.generatedCandidates";
    /// 初始生成接受方案数 / Initial generation accepted plans
    pub const InitialGenerationAcceptedPlans: &'static str = "initialGeneration.acceptedPlans";
    /// 初始生成不可行候选数 / Initial generation infeasible candidates
    pub const InitialGenerationInfeasibleCandidates: &'static str = "initialGeneration.infeasibleCandidates";
    /// 初始生成重复候选数 / Initial generation duplicate candidates
    pub const InitialGenerationDuplicateCandidates: &'static str = "initialGeneration.duplicateCandidates";
    /// 初始生成被支配候选数 / Initial generation dominated candidates
    pub const InitialGenerationDominatedCandidates: &'static str = "initialGeneration.dominatedCandidates";
    /// 初始生成宽度剪枝节点数 / Initial generation width-bound pruned nodes
    pub const InitialGenerationWidthBoundPrunedNodes: &'static str = "initialGeneration.widthBoundPrunedNodes";
    /// 初始生成刀数剪枝节点数 / Initial generation knife-bound pruned nodes
    pub const InitialGenerationKnifeBoundPrunedNodes: &'static str = "initialGeneration.knifeBoundPrunedNodes";
    /// 初始生成长度剪枝条目数 / Initial generation length-bound pruned entries
    pub const InitialGenerationLengthBoundPrunedEntries: &'static str = "initialGeneration.lengthBoundPrunedEntries";
    /// 初始生成物料宽度索引缓存命中数 / Initial generation material width index cache hits
    pub const InitialGenerationMaterialWidthIndexCacheHits: &'static str = "initialGeneration.materialWidthIndexCacheHits";
    /// 初始生成物料切片模板缓存命中数 / Initial generation material slice template cache hits
    pub const InitialGenerationMaterialSliceTemplateCacheHits: &'static str = "initialGeneration.materialSliceTemplateCacheHits";
    /// 初始生成数量缓存命中数 / Initial generation quantity cache hits
    pub const InitialGenerationQuantityCacheHits: &'static str = "initialGeneration.quantityCacheHits";
    /// 初始生成数量缓存未命中数 / Initial generation quantity cache misses
    pub const InitialGenerationQuantityCacheMisses: &'static str = "initialGeneration.quantityCacheMisses";
    /// 初始生成物料切片模板缓存未命中数 / Initial generation material slice template cache misses
    pub const InitialGenerationMaterialSliceTemplateCacheMisses: &'static str = "initialGeneration.materialSliceTemplateCacheMisses";
    /// 初始生成跨工作线程重复候选数 / Initial generation cross-worker duplicate candidates
    pub const InitialGenerationCrossWorkerDuplicateCandidates: &'static str = "initialGeneration.crossWorkerDuplicateCandidates";
    /// 初始生成跨贡献被支配数 / Initial generation cross-contribution dominated
    pub const InitialGenerationCrossContributionDominated: &'static str = "initialGeneration.crossContributionDominated";
    /// 初始生成耗时（毫秒） / Initial generation elapsed milliseconds
    pub const InitialGenerationElapsedMilliseconds: &'static str = "initialGeneration.elapsedMilliseconds";
    /// 初始生成停止原因 / Initial generation stop reason
    pub const InitialGenerationStopReason: &'static str = "initialGeneration.stopReason";
    /// 初始访问节点数（渲染用） / Initial visited nodes (for rendering)
    pub const InitialVisitedNodes: &'static str = "initialVisitedNodes";
    /// 初始候选数（渲染用） / Initial generated candidates (for rendering)
    pub const InitialGeneratedCandidates: &'static str = "initialGeneratedCandidates";
    /// 初始接受方案数（渲染用） / Initial accepted plans (for rendering)
    pub const InitialAcceptedPlans: &'static str = "initialAcceptedPlans";
    /// 初始不可行候选数（渲染用） / Initial infeasible candidates (for rendering)
    pub const InitialInfeasibleCandidates: &'static str = "initialInfeasibleCandidates";
    /// 初始重复候选数（渲染用） / Initial duplicate candidates (for rendering)
    pub const InitialDuplicateCandidates: &'static str = "initialDuplicateCandidates";
    /// 初始被支配候选数（渲染用） / Initial dominated candidates (for rendering)
    pub const InitialDominatedCandidates: &'static str = "initialDominatedCandidates";
    /// 初始宽度剪枝节点数（渲染用） / Initial width-bound pruned nodes (for rendering)
    pub const InitialWidthBoundPrunedNodes: &'static str = "initialWidthBoundPrunedNodes";
    /// 初始刀数剪枝节点数（渲染用） / Initial knife-bound pruned nodes (for rendering)
    pub const InitialKnifeBoundPrunedNodes: &'static str = "initialKnifeBoundPrunedNodes";
    /// 初始长度剪枝条目数（渲染用） / Initial length-bound pruned entries (for rendering)
    pub const InitialLengthBoundPrunedEntries: &'static str = "initialLengthBoundPrunedEntries";
    /// 初始物料宽度索引缓存命中数（渲染用） / Initial material width index cache hits (for rendering)
    pub const InitialMaterialWidthIndexCacheHits: &'static str = "initialMaterialWidthIndexCacheHits";
    /// 初始物料切片模板缓存命中数（渲染用） / Initial material slice template cache hits (for rendering)
    pub const InitialMaterialSliceTemplateCacheHits: &'static str = "initialMaterialSliceTemplateCacheHits";
    /// 初始生成耗时毫秒（渲染用） / Initial generation elapsed milliseconds (for rendering)
    pub const InitialGenerationElapsedMillisecondsRender: &'static str = "initialGenerationElapsedMilliseconds";
    /// 初始生成停止原因（渲染用） / Initial generation stop reason (for rendering)
    pub const InitialGenerationStopReasonRender: &'static str = "initialGenerationStopReason";
    /// 定价生成访问节点数 / Pricing generation visited nodes
    pub const PricingVisitedNodes: &'static str = "pricingGeneration.visitedNodes";
    /// 定价生成候选数 / Pricing generation generated candidates
    pub const PricingGeneratedCandidates: &'static str = "pricingGeneration.generatedCandidates";
    /// 定价生成接受方案数 / Pricing generation accepted plans
    pub const PricingAcceptedPlans: &'static str = "pricingGeneration.acceptedPlans";
    /// 定价生成不可行候选数 / Pricing generation infeasible candidates
    pub const PricingInfeasibleCandidates: &'static str = "pricingGeneration.infeasibleCandidates";
    /// 定价生成重复候选数 / Pricing generation duplicate candidates
    pub const PricingDuplicateCandidates: &'static str = "pricingGeneration.duplicateCandidates";
    /// 定价生成被支配候选数 / Pricing generation dominated candidates
    pub const PricingDominatedCandidates: &'static str = "pricingGeneration.dominatedCandidates";
    /// 定价生成耗时（毫秒） / Pricing generation elapsed milliseconds
    pub const PricingElapsedMilliseconds: &'static str = "pricingGeneration.elapsedMilliseconds";
    /// 定价生成停止原因 / Pricing generation stop reason
    pub const PricingStopReason: &'static str = "pricingGeneration.stopReason";
    /// LP 失败信息 / LP failure message
    pub const LpFailureMessage: &'static str = "lpFailureMessage";
    /// 总切缝宽度 / Total trim width
    pub const TotalTrimWidth: &'static str = "totalTrimWidth";
    /// 总余料 / Total rest material
    pub const TotalRestMaterial: &'static str = "totalRestMaterial";
    /// 超产面积 / Over-production area
    pub const OverProductionArea: &'static str = "overProductionArea";
    /// 超产面积度量 / Over-production area measure
    pub const OverProductionAreaMeasure: &'static str = "overProductionAreaMeasure";
    /// 余料度量 / Rest material measure
    pub const RestMaterialMeasure: &'static str = "restMaterialMeasure";

    /// 物料使用批次数键 / Material usage batch count key
    pub fn materialUsageBatchCount(material_id: &str) -> String {
        format!("materialUsage.{material_id}.batchCount")
    }

    /// 设备产能使用量键 / Machine capacity used key
    pub fn machineCapacityUsed(machine_id: &str) -> String {
        format!("machineCapacityUsed.{machine_id}")
    }

    /// 欠产量键 / Under-production key
    pub fn underProduction(product_id: &str, unit_symbol: &str) -> String {
        format!("underProduction.{product_id}.{unit_symbol}")
    }

    /// 超产量键 / Over-production key
    pub fn overProduction(product_id: &str, unit_symbol: &str) -> String {
        format!("overProduction.{product_id}.{unit_symbol}")
    }

    /// 物料成本键 / Material cost key
    pub fn materialCost(material_id: &str) -> String {
        format!("materialCost.{material_id}")
    }

    /// 已分配长度键 / Assigned length key
    pub fn assignedLength(product_id: &str) -> String {
        format!("assignedLength.{product_id}")
    }

    /// 超长量键 / Over-length key
    pub fn overLength(product_id: &str) -> String {
        format!("overLength.{product_id}")
    }
}

/// CSP1D KPI / CSP1D KPI
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Csp1dKpi {
    /// 选中方案数 / Selected plan count
    pub selected_plan_count: u64,
    /// 选中批次数 / Selected batch count
    pub selected_batch_count: u64,
    /// 满足需求数 / Satisfied demand count
    pub satisfied_demand_count: u64,
    /// 未满足需求数 / Unmet demand count
    pub unmet_demand_count: u64,
    /// 物料使用数 / Material usage count
    pub material_usage_count: u64,
    /// 设备使用数 / Machine usage count
    pub machine_usage_count: u64,
    /// 生成方案数 / Generated plan count
    pub generated_plan_count: u64,
    /// Top 方案数 / Top plan count
    pub top_plan_count: u64,
    /// 产出率指标数 / Yield metric count
    pub yield_metric_count: u64,
    /// 损耗指标数 / Waste metric count
    pub waste_metric_count: u64,
    /// 长度指标数 / Length metric count
    pub length_metric_count: u64,
    /// 详细 KPI 映射 / Detailed KPI map
    pub details: BTreeMap<String, String>,
}

/// CSP1D 问题定义 / CSP1D problem definition
#[derive(Debug, Clone)]
pub struct Csp1dProblem<V: SolveValue> {
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
    /// 列生成配置 / Column generation configuration
    pub configuration: Csp1dConfiguration,
    /// 一站式求解配置 / One-stop solve configuration
    pub solve_config: Option<Csp1dSolveConfig<V>>,
}

impl<V: SolveValue> Csp1dProblem<V> {
    /// 创建问题定义 / Create a problem definition
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
    /// 最大初始方案数 / Maximum initial plan count
    pub max_initial_plans: u64,
    /// 最大定价方案数 / Maximum pricing plan count
    pub max_pricing_plans: u64,
    /// 迭代上限 / Iteration limit
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
    /// 列生成配置 / Column generation configuration
    pub column_generation: Csp1dConfiguration,
    /// 产出率建模配置 / Yield modeling configuration
    pub yield_config: Option<YieldModelingConfig<V>>,
    /// 损耗最小化配置 / Waste minimization configuration
    pub waste_config: Option<WasteMinimizationConfig<V>>,
    /// 长度分配配置 / Length assignment configuration
    pub length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    /// Top-K 方案上限 / Top-K plan limit
    pub top_k_plan_limit: Option<u64>,
    /// 是否允许部分解 / Whether partial solution is allowed
    pub allow_partial_solution: bool,
    /// 建模扩展列表 / Modeling extension list
    pub extensions: Vec<Csp1dModelingExtension<V>>,
    /// 扩展集 / Extension set
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
    /// 合并所有扩展（去重） / Merge all extensions (deduplicated)
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
    /// 产出结果 / Produce result
    pub produce: Produce<V>,
    /// 产出率结果 / Yield modeling result
    pub yield_result: Option<crate::domain::r#yield::YieldModelingResult<V>>,
    /// 损耗结果 / Waste minimization result
    pub waste_result: Option<crate::domain::wasting_minimization::WasteMinimizationResult<V>>,
    /// 长度分配结果 / Length assignment result
    pub length_result: Option<LengthAssignmentResult<V>>,
    /// 生成方案列表 / Generated cutting plans
    pub generated_plans: Vec<CuttingPlan<V>>,
    /// KPI / KPI
    pub kpi: Csp1dKpi,
    /// 渲染数据 / Render data
    pub render: RenderSchemaDTO,
    /// 解状态 / Solution status
    pub status: Csp1dSolutionStatus,
    /// 失败信息 / Failure message
    pub failure_message: Option<String>,
    /// Top 方案列表 / Top cutting plans
    pub top_plans: Vec<CuttingPlan<V>>,
}

/// CSP1D 解分析器 / CSP1D solution analyzer
pub trait Csp1dSolutionAnalyzer<V: SolveValue>: Send + Sync {
    /// 分析解 / Analyze the solution
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
