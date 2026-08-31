//! Produce domain model types
//!
//! Contains [`PlanUsageVariablePool`] which wraps [`AppendableVariablePool`] for
//! column-generation lifecycle management of plan variable indices.
//!
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
    to_f64, CuttingPlan, Material, Machine, ProductDemand,
};

/// Plan usage variable pool wrapping [`AppendableVariablePool<usize, UContinuous>`].
///
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
    /// Create a new plan usage variable pool with the given name prefix.
    pub fn new(prefix: &str) -> Self {
        Self {
            inner: AppendableVariablePool::new(prefix),
            plan_variable_indices: Vec::new(),
            plans_iteration_variable_indices: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Clear all tracked variable indices for re-registration.
    pub fn clear(&mut self) {
        self.plan_variable_indices.clear();
        self.plans_iteration_variable_indices.clear();
    }

    /// Set the initial plan variable indices (from external registration).
    pub fn set_initial_indices(&mut self, indices: Vec<usize>) {
        self.plan_variable_indices = indices;
    }

    /// Record a new iteration's variable indices and extend the flat index list.
    ///
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

    /// All plan variable indices ordered by plan position.
    pub fn plan_variable_indices(&self) -> &[usize] {
        &self.plan_variable_indices
    }

    /// Per-iteration variable index batches.
    pub fn plans_iteration_variable_indices(&self) -> &[Vec<usize>] {
        &self.plans_iteration_variable_indices
    }

    /// Variable index for a specific plan position.
    pub fn plan_variable_index(&self, index: usize) -> Option<usize> {
        self.plan_variable_indices.get(index).copied()
    }

    // --- Inner pool access ---

    /// Immutable access to the inner [`AppendableVariablePool`].
    pub fn inner(&self) -> &AppendableVariablePool<usize, UContinuous> {
        &self.inner
    }

    /// Mutable access to the inner [`AppendableVariablePool`].
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

/// Demand expression key for symbol lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DemandExprKey {
    product_id: String,
    unit_symbol: String,
}

/// Explicit intermediate symbol fields for derived plan expressions (Phase 8.3).
///
/// Replaces inline coefficient computation in constraint registration with
/// pre-built symbol combinations for each derived expression type:
///
/// - `demand_fulfillment[demand]`: sum(plan_amount * plan.demand_coefficient)
/// - `material_usage[material]`: sum(plan_amount * plan.material_coefficient)
/// - `machine_batch_usage[machine]`: sum(plan_amount * plan.machine_batch_coefficient)
/// - `machine_capacity_usage[machine]`: sum(plan_amount * plan.machine_capacity_coefficient)
///
/// Each symbol's polynomial maps plan variable indices to their coefficients,
/// enabling constraint registration via `symbol_polynomial()` instead of
/// ad-hoc iteration over cutting plans.
#[derive(Debug, Clone)]
pub struct DerivedPlanExpressionSymbols {
    /// Per-demand expression: sum(plan_amount * demand_contribution)
    demand_fulfillment: LinearExpressionSymbols1<f64>,
    /// Demand keys in positional order (parallel to demand_fulfillment symbols)
    demand_keys: Vec<DemandExprKey>,
    /// Per-material expression: sum(plan_amount * material_match)
    material_usage: LinearExpressionSymbols1<f64>,
    /// Material IDs in positional order
    material_ids: Vec<String>,
    /// Per-machine expression: sum(plan_amount * machine_batch_match)
    machine_batch_usage: LinearExpressionSymbols1<f64>,
    /// Machine IDs in positional order (shared by batch and capacity)
    machine_ids: Vec<String>,
    /// Per-machine expression: sum(plan_amount * machine_capacity_consumption)
    machine_capacity_usage: LinearExpressionSymbols1<f64>,
}

impl DerivedPlanExpressionSymbols {
    /// Build derived expression symbols from current cutting plans and variable pool.
    ///
    /// Each symbol's polynomial is a linear expression over plan variable indices,
    /// with coefficients determined by the domain relationship (demand contribution,
    /// material match, machine batch match, or capacity consumption).
    ///
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
        let material_ids: Vec<String> = materials.iter().map(|m| m.id.clone()).collect();
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
            |_, material| material.id.clone(),
        );

        // machine_batch_usage[machine] = sum(plan_var * machine_batch_match)
        let machine_ids: Vec<String> = machines.iter().map(|m| m.id.clone()).collect();
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
            |_, machine| machine.id.clone(),
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
            |_, machine| machine.id.clone(),
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

    /// Register all expression symbols to the model.
    ///
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

    /// Extract polynomial terms for a specific demand.
    ///
    /// Returns `Vec<(variable_index, coefficient)>` suitable for constraint registration.
    /// Uses product_id and unit_symbol as lookup keys (type-agnostic).
    pub fn demand_terms(&self, product_id: &str, unit_symbol: &str) -> Vec<(usize, f64)> {
        let key = DemandExprKey {
            product_id: product_id.to_string(),
            unit_symbol: unit_symbol.to_string(),
        };
        self.demand_keys
            .iter()
            .position(|k| *k == key)
            .map(|index| extract_terms_from_symbol(&self.demand_fulfillment, index))
            .unwrap_or_default()
    }

    /// Extract polynomial terms for a specific material.
    /// Uses material_id as lookup key (type-agnostic).
    pub fn material_terms(&self, material_id: &str) -> Vec<(usize, f64)> {
        self.material_ids
            .iter()
            .position(|id| id == material_id)
            .map(|index| extract_terms_from_symbol(&self.material_usage, index))
            .unwrap_or_default()
    }

    /// Extract polynomial terms for a specific machine (batch usage).
    /// Uses machine_id as lookup key (type-agnostic).
    pub fn machine_batch_terms(&self, machine_id: &str) -> Vec<(usize, f64)> {
        self.machine_ids
            .iter()
            .position(|id| id == machine_id)
            .map(|index| extract_terms_from_symbol(&self.machine_batch_usage, index))
            .unwrap_or_default()
    }

    /// Extract polynomial terms for a specific machine (capacity usage).
    /// Uses machine_id as lookup key (type-agnostic).
    pub fn machine_capacity_terms(&self, machine_id: &str) -> Vec<(usize, f64)> {
        self.machine_ids
            .iter()
            .position(|id| id == machine_id)
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
