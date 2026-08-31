//! 产出聚合 / Produce aggregation

use std::collections::HashSet;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::SolveValue;
use ospf_rust_core::variable::{Continuous, UInteger, VariableRange};

use crate::domain::material::{CuttingPlan, CuttingPlanId, Machine, Material, ProductDemand};

use super::CuttingPlanUsage;
use super::model::{DerivedPlanExpressionSymbols, PlanUsageVariablePool};

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
    /// 已注册的批量表达式符号 / Registered batch expression symbols
    batch_symbols: Option<DerivedPlanExpressionSymbols>,
    plans_iteration: Vec<Vec<CuttingPlan<V>>>,
    registered_ids: HashSet<CuttingPlanId>,
    registered_keys: HashSet<String>,
    variable_pool: PlanUsageVariablePool<V>,
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
            batch_symbols: None,
            plans_iteration: Vec::new(),
            registered_ids: HashSet::new(),
            registered_keys: HashSet::new(),
            variable_pool: PlanUsageVariablePool::new("csp1d_batch"),
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
            batch_symbols: None,
            plans_iteration: Vec::new(),
            registered_ids: HashSet::new(),
            registered_keys: HashSet::new(),
            variable_pool: PlanUsageVariablePool::new("csp1d_batch"),
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
        self.variable_pool.plan_variable_indices()
    }

    /// 每轮新增方案变量索引 / Plan variable indices added per iteration
    pub fn plans_iteration_variable_indices(&self) -> &[Vec<usize>] {
        self.variable_pool.plans_iteration_variable_indices()
    }

    /// 方案变量索引 / Plan variable index
    pub fn plan_variable_index(&self, index: usize) -> Option<usize> {
        self.variable_pool.plan_variable_index(index)
    }

    /// 变量池引用 / Variable pool reference
    pub fn variable_pool(&self) -> &PlanUsageVariablePool<V> {
        &self.variable_pool
    }

    /// 已注册的批量表达式符号 / Registered batch expression symbols
    pub fn batch_symbols(&self) -> Option<&DerivedPlanExpressionSymbols> {
        self.batch_symbols.as_ref()
    }

    /// 重建批量表达式符号 / Rebuild batch expression symbols
    ///
    /// Rebuilds the expression symbols to reflect the current plan state
    /// (new plans added, plans retired, etc.). Does NOT re-register to the
    /// model since symbols are tracked by group ID.
    pub fn rebuild_batch_symbols(&mut self) {
        let symbols = DerivedPlanExpressionSymbols::build(
            &self.cutting_plans,
            &self.variable_pool,
            &self.demands,
            &self.materials,
            &self.machines,
            |plan_index| self.is_plan_active(plan_index),
        );
        self.batch_symbols = Some(symbols);
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
        self.variable_pool.clear();
        let plans = self.cutting_plans.clone();
        self.register_plan_variables(model, &plans).map(|indices| {
            self.variable_pool.set_initial_indices(indices);
        })?;

        // Build and register expression symbols to the model
        let symbols = DerivedPlanExpressionSymbols::build(
            &self.cutting_plans,
            &self.variable_pool,
            &self.demands,
            &self.materials,
            &self.machines,
            |plan_index| self.is_plan_active(plan_index),
        );
        symbols
            .register_symbols(model)
            .map_err(|error| crate::Csp1dError::Calculation {
                message: format!("register expression symbols failed: {error}"),
            })?;
        self.batch_symbols = Some(symbols);

        Ok(())
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
        self.variable_pool.push_iteration(indices);
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

    pub(crate) fn register_plan_variables(
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
