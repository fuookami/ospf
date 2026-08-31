//! 主问题产出与扩展点 / Master problem output and extension points

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use ospf_rust_core::model::{
    LinearObjectiveInput, MetaModel, ObjectiveCategory,
};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    from_f64, to_f64, Csp1dQuantity, CuttingPlan,
    Material, Machine, ProductDemand,
};
use crate::domain::length_assignment::{
    DefaultLengthDerivation, LengthAssignment, LengthAssignmentContext, LengthAssignmentInput,
    LengthAssignmentModelingConfig, LengthAssignmentResult, LengthSlackAggregation,
    OverLengthRecord,
};
use crate::domain::r#yield::{
    ModeledOverProduction, ModeledUnderProduction, YieldAnalysis,
    YieldModelingConfig, YieldModelingResult, YieldSlackAggregation,
};
use crate::domain::wasting_minimization::{
    ModeledMaterialCost, WasteAggregation, WasteAnalysis,
    WasteMinimizationConfig, WasteMinimizationResult,
};

use super::aggregation::ProduceAggregation;
use super::pipeline::{
    Csp1dCGPipeline, Csp1dIncrementalPipeline, LengthObjectivePipeline,
    YieldObjectivePipeline, WasteObjectivePipeline,
    DemandConstraintPipeline, MaterialConstraintPipeline,
    MachineConstraintPipeline, YieldConstraintPipeline,
    LengthConstraintPipeline,
};
use super::shadow_price::Csp1dShadowPriceLifecycle;
use super::{
    CuttingPlanUsage, MachineCapacityUsage, MaterialUsage, Produce,
    Csp1dModelingContext,
    Csp1dModelingMode, Csp1dObjectivePolicy,
    SimpleDomainCalculationContext,
};

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
    ) -> crate::Csp1dResult<crate::domain::material::ShadowPriceMap<V>>;
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
    pub(crate) yield_config: Option<YieldModelingConfig<V>>,
    pub(crate) waste_config: Option<WasteMinimizationConfig<V>>,
    pub(crate) length_config: Option<LengthAssignmentModelingConfig<V>>,
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
    ) -> crate::Csp1dResult<crate::domain::material::ShadowPriceMap<V>> {
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
        self.produce.rebuild_batch_symbols();
        self.rebuild_builtin_constraint_pipelines();
        Self::register_constraint_pipelines(model, &self.constraint_pipelines)
    }

    fn register_yield_variables(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        let Some(r#yield) = self.r#yield.as_mut() else {
            return Ok(());
        };
        r#yield.variables.clear();
        for demand_index in 0..r#yield.demands.len() {
            let demand = &r#yield.demands[demand_index];
            if r#yield.needs_under_production(demand) {
                r#yield
                    .variables
                    .register_under(demand_index, model)
                    .map_err(|error| crate::Csp1dError::Calculation {
                        message: format!("register under-production variable failed: {error}"),
                    })?;
            } else {
                r#yield.variables.push_under_none();
            }
            if r#yield.needs_over_production(demand) {
                r#yield
                    .variables
                    .register_over(demand_index, model)
                    .map_err(|error| crate::Csp1dError::Calculation {
                        message: format!("register over-production variable failed: {error}"),
                    })?;
            } else {
                r#yield.variables.push_over_none();
            }
        }
        Ok(())
    }

    fn register_length_variables(&mut self, model: &mut MetaModel<f64>) -> crate::Csp1dResult<()> {
        let Some(length) = self.length.as_mut() else {
            return Ok(());
        };
        length.variables.clear();
        for demand_index in 0..length.demands.len() {
            let demand = &length.demands[demand_index];
            if length.needs_assigned_length(demand) {
                length
                    .variables
                    .register_assigned(demand_index, model)
                    .map_err(|error| crate::Csp1dError::Calculation {
                        message: format!("register assigned-length variable failed: {error}"),
                    })?;
            } else {
                length.variables.push_assigned_none();
            }
            if length.needs_over_length(demand) {
                length
                    .variables
                    .register_over(demand_index, model)
                    .map_err(|error| crate::Csp1dError::Calculation {
                        message: format!("register over-length variable failed: {error}"),
                    })?;
            } else {
                length.variables.push_over_none();
            }
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
            .unwrap_or_else(|| super::extraction::analyze_yield(&produce, &self.produce.demands));
        Some(YieldModelingResult {
            analysis,
        })
    }

    /// 提取 waste 结果 / Extract waste result
    pub fn extract_waste_result(&self, model: &MetaModel<f64>) -> Option<WasteMinimizationResult<V>> {
        let config = self.waste_config.as_ref()?;
        let produce = self.extract_solution(model).ok()?;
        let yield_analysis = super::extraction::analyze_yield(&produce, &self.produce.demands);
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
                .variables
                .under_index(demand_index)
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
                .variables
                .over_index(demand_index)
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
            outputs: super::extraction::analyze_yield(produce, &self.produce.demands).outputs,
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
                .variables
                .assigned_index(demand_index)
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
                .variables
                .over_index(demand_index)
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

pub(crate) struct ModeledLengthExtraction<V: SolveValue> {
    pub(crate) result: LengthAssignmentResult<V>,
    pub(crate) has_assigned_values: bool,
    pub(crate) has_over_values: bool,
}
