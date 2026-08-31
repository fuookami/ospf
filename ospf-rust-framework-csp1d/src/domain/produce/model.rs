//! 产出领域模型类型 / Produce domain model types
//!
//! 包含 [`PlanUsageVariablePool`]，封装 [`AppendableVariablePool`] 用于
//! 列生成生命周期管理计划变量索引。
//! Contains [`PlanUsageVariablePool`] which wraps [`AppendableVariablePool`] for
//! column-generation lifecycle management of plan variable indices.
//!
//! 还包含 [`DerivedPlanExpressionSymbols`]，用于从切割方案和变量池派生的
//! 显式中间符号字段（Phase 8.3）。
//! Also contains [`DerivedPlanExpressionSymbols`] for explicit intermediate symbol
//! fields derived from cutting plans and variable pools (Phase 8.3).

use std::fmt;
use std::marker::PhantomData;

use ospf_rust_core::solver::SolveValue;
use ospf_rust_core::symbol::symbol_combination::LinearExpressionSymbols1;
use ospf_rust_core::symbol::{LinearIntermediateSymbol, flat_map1};
use ospf_rust_core::variable::UContinuous;
use ospf_rust_framework::model::AppendableVariablePool;

use crate::domain::material::{
    to_f64, CuttingPlan, Material, Machine, MaterialId, ProductDemand, ProductId, MachineId,
};

/// 封装 [`AppendableVariablePool<usize, UContinuous>`] 的计划使用量变量池。 / Plan usage variable pool wrapping [`AppendableVariablePool<usize, UContinuous>`].
///
/// 管理计划位置到模型变量索引的映射，
/// 支持列生成生命周期（注册、添加、移除），
/// 同时通过缓存的 `Vec<usize>` 提供向后兼容的索引访问。
/// Manages the mapping from plan positions to model variable indices,
/// supporting column-generation lifecycle (register, add, remove) while
/// providing backward-compatible indexed access via cached `Vec<usize>`.
pub struct PlanUsageVariablePool<V: SolveValue> {
    inner: AppendableVariablePool<usize, UContinuous>,
    plan_variable_indices: Vec<usize>,
    plans_iteration_variable_indices: Vec<Vec<usize>>,
    _phantom: PhantomData<V>,
}

impl<V: SolveValue> PlanUsageVariablePool<V> {
    /// 使用给定名称前缀创建新的计划使用量变量池。 / Create a new plan usage variable pool with the given name prefix.
    pub fn new(prefix: &str) -> Self {
        Self {
            inner: AppendableVariablePool::new(prefix),
            plan_variable_indices: Vec::new(),
            plans_iteration_variable_indices: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// 清除所有已跟踪的变量索引以便重新注册。 / Clear all tracked variable indices for re-registration.
    pub fn clear(&mut self) {
        self.plan_variable_indices.clear();
        self.plans_iteration_variable_indices.clear();
    }

    /// 设置初始计划变量索引（来自外部注册）。 / Set the initial plan variable indices (from external registration).
    pub fn set_initial_indices(&mut self, indices: Vec<usize>) {
        self.plan_variable_indices = indices;
    }

    /// 记录新迭代的变量索引并扩展平面索引列表。
    /// Record a new iteration's variable indices and extend the flat index list.
    ///
    /// 此方法镜像以下模式：
    /// This mirrors the pattern:
    /// ```ignore
    /// self.plan_variable_indices.extend(indices.iter().copied());
    /// self.plans_iteration_variable_indices.push(indices);
    /// ```
    pub fn push_iteration(&mut self, indices: Vec<usize>) {
        self.plan_variable_indices.extend_from_slice(&indices);
        self.plans_iteration_variable_indices.push(indices);
    }

    // --- Backward-compatible accessors ---

    /// 按计划位置排序的所有计划变量索引。 / All plan variable indices ordered by plan position.
    pub fn plan_variable_indices(&self) -> &[usize] {
        &self.plan_variable_indices
    }

    /// 每次迭代的变量索引批次。 / Per-iteration variable index batches.
    pub fn plans_iteration_variable_indices(&self) -> &[Vec<usize>] {
        &self.plans_iteration_variable_indices
    }

    /// 特定计划位置的变量索引。 / Variable index for a specific plan position.
    pub fn plan_variable_index(&self, index: usize) -> Option<usize> {
        self.plan_variable_indices.get(index).copied()
    }

    // --- Inner pool access ---

    /// 对内部 [`AppendableVariablePool`] 的不可变访问。 / Immutable access to the inner [`AppendableVariablePool`].
    pub fn inner(&self) -> &AppendableVariablePool<usize, UContinuous> {
        &self.inner
    }

    /// 对内部 [`AppendableVariablePool`] 的可变访问。 / Mutable access to the inner [`AppendableVariablePool`].
    pub fn inner_mut(&mut self) -> &mut AppendableVariablePool<usize, UContinuous> {
        &mut self.inner
    }
}

impl<V: SolveValue> fmt::Debug for PlanUsageVariablePool<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlanUsageVariablePool")
            .field("count", &self.plan_variable_indices.len())
            .field("iterations", &self.plans_iteration_variable_indices.len())
            .finish()
    }
}

impl<V: SolveValue> Clone for PlanUsageVariablePool<V> {
    fn clone(&self) -> Self {
        Self {
            inner: AppendableVariablePool::new(self.inner.name_prefix()),
            plan_variable_indices: self.plan_variable_indices.clone(),
            plans_iteration_variable_indices: self.plans_iteration_variable_indices.clone(),
            _phantom: PhantomData,
        }
    }
}

// ============================================================================
// DerivedPlanExpressionSymbols - explicit derived symbol fields (Phase 8.3)
// ============================================================================

/// 用于符号查找的需求表达式键。 / Demand expression key for symbol lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DemandExprKey {
    product_id: ProductId,
    unit_symbol: String,
}

/// 派生计划表达式的显式中间符号字段（Phase 8.3）。
/// Explicit intermediate symbol fields for derived plan expressions (Phase 8.3).
///
/// 用预构建的符号组合替代约束注册中的内联系数计算，
/// 为每种派生表达式类型提供：
/// Replaces inline coefficient computation in constraint registration with
/// pre-built symbol combinations for each derived expression type:
///
/// - `demand_fulfillment[demand]`: sum(plan_amount * plan.demand_coefficient) / 需求满足
/// - `material_usage[material]`: sum(plan_amount * plan.material_coefficient) / 物料使用
/// - `machine_batch_usage[machine]`: sum(plan_amount * plan.machine_batch_coefficient) / 机器批次使用
/// - `machine_capacity_usage[machine]`: sum(plan_amount * plan.machine_capacity_coefficient) / 机器产能使用
///
/// 每个符号的多项式将计划变量索引映射到其系数，
/// 通过 `symbol_polynomial()` 实现约束注册，而非对切割方案的临时迭代。
/// Each symbol's polynomial maps plan variable indices to their coefficients,
/// enabling constraint registration via `symbol_polynomial()` instead of
/// ad-hoc iteration over cutting plans.
#[derive(Debug, Clone)]
pub struct DerivedPlanExpressionSymbols {
    /// 每需求表达式：sum(plan_amount * demand_contribution) / Per-demand expression: sum(plan_amount * demand_contribution)
    demand_fulfillment: LinearExpressionSymbols1<f64>,
    /// 按位置排序的需求键（与 demand_fulfillment 符号平行）/ Demand keys in positional order (parallel to demand_fulfillment symbols)
    demand_keys: Vec<DemandExprKey>,
    /// 每物料表达式：sum(plan_amount * material_match) / Per-material expression: sum(plan_amount * material_match)
    material_usage: LinearExpressionSymbols1<f64>,
    /// 按位置排序的物料 ID / Material IDs in positional order
    material_ids: Vec<MaterialId>,
    /// 每机器表达式：sum(plan_amount * machine_batch_match) / Per-machine expression: sum(plan_amount * machine_batch_match)
    machine_batch_usage: LinearExpressionSymbols1<f64>,
    /// 按位置排序的机器 ID（批次和产能共用）/ Machine IDs in positional order (shared by batch and capacity)
    machine_ids: Vec<MachineId>,
    /// 每机器表达式：sum(plan_amount * machine_capacity_consumption) / Per-machine expression: sum(plan_amount * machine_capacity_consumption)
    machine_capacity_usage: LinearExpressionSymbols1<f64>,
}

impl DerivedPlanExpressionSymbols {
    /// 从当前切割方案和变量池构建派生表达式符号。
    /// Build derived expression symbols from current cutting plans and variable pool.
    ///
    /// 每个符号的多项式是计划变量索引上的线性表达式，
    /// 系数由领域关系（需求贡献、物料匹配、机器批次匹配或产能消耗）决定。
    /// Each symbol's polynomial is a linear expression over plan variable indices,
    /// with coefficients determined by the domain relationship (demand contribution,
    /// material match, machine batch match, or capacity consumption).
    ///
    /// `is_active` 闭包决定哪些计划索引参与表达式，
    /// 支持在列生成期间正确过滤已退役的计划。
    /// The `is_active` closure determines which plan indices contribute to the
    /// expressions, enabling proper filtering of retired plans during column generation.
    pub fn build<V: SolveValue>(
        cutting_plans: &[CuttingPlan<V>],
        variable_pool: &PlanUsageVariablePool<V>,
        demands: &[ProductDemand<V>],
        materials: &[Material<V>],
        machines: &[Machine<V>],
        is_active: impl Fn(usize) -> bool,
    ) -> Self {
        // demand_fulfillment[demand] = sum(plan_var * demand_contribution)
        let demand_keys: Vec<DemandExprKey> = demands
            .iter()
            .map(|d| DemandExprKey {
                product_id: d.product.id.clone(),
                unit_symbol: d.quantity.unit.symbol().to_string(),
            })
            .collect();
        let demand_fulfillment = flat_map1(
            "csp1d_demand_expr",
            demands,
            |demand| {
                let monomials: Vec<_> = cutting_plans
                    .iter()
                    .enumerate()
                    .filter_map(|(plan_index, plan)| {
                        if !is_active(plan_index) {
                            return None;
                        }
                        let var_index = variable_pool.plan_variable_index(plan_index)?;
                        let coefficient: f64 = plan
                            .demand_contributions
                            .iter()
                            .filter(|c| {
                                c.product.id == demand.product.id
                                    && c.quantity.unit == demand.quantity.unit
                            })
                            .filter_map(|c| to_f64(&c.quantity.value))
                            .sum();
                        (coefficient != 0.0)
                            .then_some(ospf_rust_core::symbol::flatten::LinearMonomial::new(coefficient, var_index))
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, demand| format!("{}_{}", demand.product.id, demand.quantity.unit.symbol()),
        );

        // material_usage[material] = sum(plan_var * material_match)
        let material_ids: Vec<MaterialId> = materials.iter().map(|m| m.id.clone()).collect();
        let material_usage = flat_map1(
            "csp1d_material_expr",
            materials,
            |material| {
                let monomials: Vec<_> = cutting_plans
                    .iter()
                    .enumerate()
                    .filter_map(|(plan_index, plan)| {
                        if !is_active(plan_index) {
                            return None;
                        }
                        let var_index = variable_pool.plan_variable_index(plan_index)?;
                        (plan.material.id == material.id)
                            .then_some(ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, var_index))
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, material| material.id.to_string(),
        );

        // machine_batch_usage[machine] = sum(plan_var * machine_batch_match)
        let machine_ids: Vec<MachineId> = machines.iter().map(|m| m.id.clone()).collect();
        let machine_batch_usage = flat_map1(
            "csp1d_machine_batch_expr",
            machines,
            |machine| {
                let monomials: Vec<_> = cutting_plans
                    .iter()
                    .enumerate()
                    .filter_map(|(plan_index, plan)| {
                        if !is_active(plan_index) {
                            return None;
                        }
                        let var_index = variable_pool.plan_variable_index(plan_index)?;
                        (plan.machine_id.as_deref() == Some(machine.id.as_str()))
                            .then_some(ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, var_index))
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, machine| machine.id.to_string(),
        );

        // machine_capacity_usage[machine] = sum(plan_var * capacity_consumption)
        let machine_capacity_usage = flat_map1(
            "csp1d_machine_capacity_expr",
            machines,
            |machine| {
                let monomials: Vec<_> = cutting_plans
                    .iter()
                    .enumerate()
                    .filter_map(|(plan_index, plan)| {
                        if !is_active(plan_index) {
                            return None;
                        }
                        let var_index = variable_pool.plan_variable_index(plan_index)?;
                        if plan.machine_id.as_deref() != Some(machine.id.as_str()) {
                            return None;
                        }
                        let consumption = plan.capacity_consumption.as_ref()?;
                        let capacity = machine.capacity.as_ref()?;
                        if consumption.unit != capacity.unit {
                            return None;
                        }
                        let coefficient = to_f64(&consumption.value)?;
                        (coefficient != 0.0)
                            .then_some(ospf_rust_core::symbol::flatten::LinearMonomial::new(coefficient, var_index))
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, machine| machine.id.to_string(),
        );

        Self {
            demand_fulfillment,
            demand_keys,
            material_usage,
            material_ids,
            machine_batch_usage,
            machine_ids,
            machine_capacity_usage,
        }
    }

    /// 将所有表达式符号注册到模型。
    /// Register all expression symbols to the model.
    ///
    /// 注册每个 `LinearExpressionSymbols1` 组合，使符号
    /// 作为中间符号在模型中被跟踪，而非在项提取后被丢弃。
    /// Registers each `LinearExpressionSymbols1` combination so the symbols
    /// are tracked as intermediate symbols in the model rather than being
    /// discarded after term extraction.
    pub fn register_symbols(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> ospf_rust_core::error::Result<()> {
        model.add_symbol_combination(&self.demand_fulfillment)?;
        model.add_symbol_combination(&self.material_usage)?;
        model.add_symbol_combination(&self.machine_batch_usage)?;
        model.add_symbol_combination(&self.machine_capacity_usage)?;
        Ok(())
    }

    /// 提取特定需求的多项式项。
    /// Extract polynomial terms for a specific demand.
    ///
    /// 返回适合约束注册的 `Vec<(variable_index, coefficient)>`。
    /// 使用 product_id 和 unit_symbol 作为查找键（类型无关）。
    /// Returns `Vec<(variable_index, coefficient)>` suitable for constraint registration.
    /// Uses product_id and unit_symbol as lookup keys (type-agnostic).
    pub fn demand_terms(&self, product_id: &str, unit_symbol: &str) -> Vec<(usize, f64)> {
        let key = DemandExprKey {
            product_id: product_id.into(),
            unit_symbol: unit_symbol.to_string(),
        };
        self.demand_keys
            .iter()
            .position(|k| *k == key)
            .map(|index| extract_terms_from_symbol(&self.demand_fulfillment, index))
            .unwrap_or_default()
    }

    /// 提取特定物料的多项式项。 / Extract polynomial terms for a specific material.
    /// 使用 material_id 作为查找键（类型无关）。 / Uses material_id as lookup key (type-agnostic).
    pub fn material_terms(&self, material_id: &str) -> Vec<(usize, f64)> {
        self.material_ids
            .iter()
            .position(|id| id.as_str() == material_id)
            .map(|index| extract_terms_from_symbol(&self.material_usage, index))
            .unwrap_or_default()
    }

    /// 提取特定机器的多项式项（批次使用）。 / Extract polynomial terms for a specific machine (batch usage).
    /// 使用 machine_id 作为查找键（类型无关）。 / Uses machine_id as lookup key (type-agnostic).
    pub fn machine_batch_terms(&self, machine_id: &str) -> Vec<(usize, f64)> {
        self.machine_ids
            .iter()
            .position(|id| id.as_str() == machine_id)
            .map(|index| extract_terms_from_symbol(&self.machine_batch_usage, index))
            .unwrap_or_default()
    }

    /// 提取特定机器的多项式项（产能使用）。 / Extract polynomial terms for a specific machine (capacity usage).
    /// 使用 machine_id 作为查找键（类型无关）。 / Uses machine_id as lookup key (type-agnostic).
    pub fn machine_capacity_terms(&self, machine_id: &str) -> Vec<(usize, f64)> {
        self.machine_ids
            .iter()
            .position(|id| id.as_str() == machine_id)
            .map(|index| extract_terms_from_symbol(&self.machine_capacity_usage, index))
            .unwrap_or_default()
    }
}

/// 从符号提取 `(variable_index, coefficient)` 项。
/// Extract `(variable_index, coefficient)` terms from a symbol at the given index.
///
/// 注意：这是从已注册 `LinearExpressionSymbol` 展开的系数，
/// 不是独立的裸系数。符号是源，这里只是 `add_linear_constraint` API 的适配层。
/// Note: These are coefficients expanded from registered `LinearExpressionSymbol`s,
/// not independent raw coefficients. The symbols are the source of truth;
/// this is the adapter layer for `add_linear_constraint` API.
fn extract_terms_from_symbol(
    symbols: &LinearExpressionSymbols1<f64>,
    index: usize,
) -> Vec<(usize, f64)> {
    let poly = symbols[index].to_linear_polynomial();
    poly.monomials()
        .iter()
        .map(|m| (m.var_index(), *m.coefficient()))
        .collect()
}
