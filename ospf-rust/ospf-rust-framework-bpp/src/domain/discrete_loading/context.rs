//! 事务注册与独立领域复核。 / Transactional registration and independent domain verification.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_core::error::{CoreError, ModelError, Result};
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{ConstraintRelation, LinearModelValue, MetaModel};
use ospf_rust_core::token::TokenVariableData;
use ospf_rust_core::variable::{VariableId, VariableType, new_standalone_id};
use ospf_rust_quantities::unit::{Kilogram, UnitTrait};
use crate::infrastructure::PackingValueAdapter;
use super::{AssignmentKey, ConflictIndex, DiscreteLoadingProblem, DiscreteLoadingSolution, LoadingAggregation, LoadingCapacityLimit, LoadingConflictLimit};

static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(1);

pub(super) fn invalid(message: impl Into<String>) -> CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

pub(super) fn checked<V: LinearModelValue>(value: V) -> Result<V> {
    if value.is_finite_value() { Ok(value) } else { Err(invalid("nonfinite discrete loading value")) }
}

pub(super) fn count<V: LinearModelValue + FromPrimitive + ToPrimitive>(value: u64) -> Result<V> {
    V::from_u64(value).filter(|converted| converted.to_u64() == Some(value))
        .ok_or_else(|| invalid("loading count cannot be represented exactly in model value domain"))
}

pub(super) fn index<V: LinearModelValue>(model: &MetaModel<V>, variable: VariableId) -> Result<usize> {
    model.tokens().iter().position(|token| token.id() == variable)
        .ok_or_else(|| invalid("loading variable is absent from its registered model"))
}

fn binary<V: LinearModelValue + One>(
    model: &mut MetaModel<V>,
    name: String,
    fixed: Option<bool>,
) -> Result<VariableId> {
    let variable = new_standalone_id();
    model.register_variable_data(TokenVariableData {
        id: variable, index: 0, name, display_name: None, var_type: VariableType::Binary,
        lower_bound: Some(if fixed == Some(true) { V::one() } else { V::zero() }),
        upper_bound: Some(if fixed == Some(false) { V::zero() } else { V::one() }),
    })?;
    Ok(variable)
}

/// 单模型离散装载入口；每次回退或完整求解应创建新 context。
/// Single-model loading entry; create a fresh context for each fallback or complete solve.
pub struct DiscreteLoadingContext<V, U: UnitTrait> {
    problem: DiscreteLoadingProblem<V, U>,
    capacities: BTreeMap<String, V>,
    item_weights: BTreeMap<String, V>,
    conflicts: ConflictIndex,
    aggregation: LoadingAggregation<V>,
    additional_weights: BTreeMap<String, Linear<V>>,
    model: Option<u64>,
    namespace: String,
}

impl<V, U> DiscreteLoadingContext<V, U>
where
    V: LinearModelValue + One + FromPrimitive + ToPrimitive,
    U: UnitTrait,
{
    /// 归一化并验证全部输入，失败不产生半注册模型。 / Normalize and validate every input without leaving a partially registered model.
    pub fn new(problem: DiscreteLoadingProblem<V, U>, adapter: &dyn PackingValueAdapter<V, U>) -> Result<Self> {
        if problem.weight_unit.dimension_symbol() != Kilogram.dimension_symbol() {
            return Err(invalid("discrete loading requires a physical mass unit"));
        }
        let capacities = problem.positions.iter().map(|position|
            Ok((position.id.clone(), checked(adapter.quantity(&position.max_weight, &problem.weight_unit)?)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let item_weights = problem.items.iter().map(|item|
            Ok((item.id.clone(), checked(adapter.quantity(&item.weight, &problem.weight_unit)?)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let conflicts = ConflictIndex::build(&problem.conflicts);
        let context = Self {
            problem, capacities, item_weights, conflicts, aggregation: LoadingAggregation::default(),
            additional_weights: BTreeMap::new(), model: None,
            namespace: format!("loading_{}", NEXT_CONTEXT.fetch_add(1, Ordering::Relaxed)),
        };
        context.validate()?;
        Ok(context)
    }

    /// 不可变领域输入。 / Immutable domain input.
    pub fn problem(&self) -> &DiscreteLoadingProblem<V, U> { &self.problem }

    /// 注册聚合及扩展建模入口。 / Registration aggregation for extension modeling.
    pub fn aggregation(&self) -> &LoadingAggregation<V> { &self.aggregation }

    /// 保全所有边的共享冲突索引。 / Shared conflict index preserving every edge.
    pub fn conflict_index(&self) -> &ConflictIndex { &self.conflicts }

    /// 验证引用、真实容量、固定货物、布局及非传递冲突。 / Validate references, true capacities, fixed cargo, layouts and non-transitive conflicts.
    pub fn validate(&self) -> Result<()> {
        let item_ids = self.problem.items.iter().map(|item| &item.id).collect::<BTreeSet<_>>();
        let positions = self.problem.positions.iter().map(|position| &position.id).collect::<BTreeSet<_>>();
        if item_ids.len() != self.problem.items.len() || item_ids.iter().any(|id| id.trim().is_empty())
            || positions.len() != self.problem.positions.len() || positions.iter().any(|id| id.trim().is_empty())
            || self.problem.allowed_assignments.iter().any(|key| !item_ids.contains(&key.item_id) || !positions.contains(&key.position_id))
            || self.problem.conflicts.iter().any(|edge| edge.first == edge.second || !positions.contains(&edge.first) || !positions.contains(&edge.second))
        { return Err(invalid("invalid loading identities or references")); }
        let mut groups = BTreeSet::new();
        for group in &self.problem.layout_groups {
            let layouts = group.layouts.iter().map(|layout| &layout.id).collect::<BTreeSet<_>>();
            let union = group.layouts.iter().flat_map(|layout| layout.position_ids.iter()).cloned().collect::<BTreeSet<_>>();
            if group.id.trim().is_empty() || !groups.insert(&group.id) || group.layouts.is_empty()
                || layouts.len() != group.layouts.len() || layouts.iter().any(|id| id.trim().is_empty())
                || group.position_ids.iter().any(|id| !positions.contains(id)) || union != group.position_ids
            { return Err(invalid("invalid loading layout group")); }
        }
        for position in &self.problem.positions {
            if position.max_count == 0 || self.capacities[&position.id] < V::zero() {
                return Err(invalid("loading capacities must be finite and nonnegative, with positive item count"));
            }
        }
        for item in &self.problem.items {
            if self.item_weights[&item.id] < V::zero() || item.fixed_position_id.as_ref().is_some_and(|position|
                !self.problem.allowed_assignments.contains(&AssignmentKey { item_id: item.id.clone(), position_id: position.clone() }))
            { return Err(invalid("invalid loading item weight or fixed assignment")); }
        }
        let fixed = self.problem.positions.iter().filter(|position| position.fixed_occupied).map(|position| position.id.clone())
            .chain(self.problem.items.iter().filter_map(|item| item.fixed_position_id.clone())).collect::<BTreeSet<_>>();
        if self.conflicts.edges().iter().any(|edge| fixed.contains(&edge.first) && fixed.contains(&edge.second)) {
            return Err(invalid("conflicting fixed position occupancy"));
        }
        for group in &self.problem.layout_groups {
            let required = fixed.intersection(&group.position_ids).collect::<BTreeSet<_>>();
            if !group.layouts.iter().any(|layout| required.iter().all(|id| layout.position_ids.contains(*id))) {
                return Err(invalid("no layout can contain all fixed occupied positions"));
            }
        }
        for position in &self.problem.positions {
            let fixed = self.problem.items.iter().filter(|item| item.fixed_position_id.as_ref() == Some(&position.id)).collect::<Vec<_>>();
            let mut weight = V::zero();
            for item in &fixed { weight = checked(weight + self.item_weights[&item.id].clone())?; }
            if fixed.len() as u64 > position.max_count || weight > self.capacities[&position.id] {
                return Err(invalid("fixed cargo exceeds true position capacity"));
            }
        }
        Ok(())
    }

    /// 原子注册共享变量和通用限制；不覆盖外部目标。 / Atomically register shared variables and limits, preserving external objectives.
    pub fn register(&mut self, model: &mut MetaModel<V>) -> Result<()> {
        self.register_with(model, |_, _| Ok(()))
    }

    /// 原子注册并注入扩展变量、行或目标；闭包外部副作用不在回滚范围内。
    /// Atomically register extensions (variables, rows or objective); external closure side effects are outside rollback.
    pub fn register_with<F>(&mut self, model: &mut MetaModel<V>, extension: F) -> Result<()>
    where
        F: FnOnce(&mut MetaModel<V>, &LoadingAggregation<V>) -> Result<()>,
    {
        if self.model.is_some() { return Err(invalid("loading context is already registered")); }
        self.validate()?;
        let mut aggregation = LoadingAggregation::<V>::default();
        model.transaction(|model| -> Result<()> {
            let fixed_positions = self.problem.items.iter().filter_map(|item| item.fixed_position_id.as_ref()).collect::<BTreeSet<_>>();
            for (i, position) in self.problem.positions.iter().enumerate() {
                let fixed = if position.fixed_occupied || fixed_positions.contains(&position.id) { Some(true) }
                    else if !position.enabled { Some(false) } else { None };
                aggregation.occupied.insert(position.id.clone(), binary(model, format!("{}_occupied_{i}", self.namespace), fixed)?);
                aggregation.counts.insert(position.id.clone(), Linear::new(Vec::new(), V::zero()));
                aggregation.weights.insert(position.id.clone(), Linear::new(Vec::new(), V::zero()));
            }
            for item in &self.problem.items { aggregation.loaded.insert(item.id.clone(), Linear::new(Vec::new(), V::zero())); }
            for (i, key) in self.problem.allowed_assignments.iter().enumerate() {
                let item = self.problem.items.iter().find(|item| item.id == key.item_id).ok_or_else(|| invalid("unknown item"))?;
                let position = self.problem.positions.iter().find(|position| position.id == key.position_id).ok_or_else(|| invalid("unknown position"))?;
                let fixed = item.fixed_position_id.as_ref() == Some(&position.id);
                if (item.fixed_position_id.is_some() && !fixed)
                    || ((!position.enabled || self.item_weights[&item.id] > self.capacities[&position.id]) && !fixed)
                { continue; }
                let variable = binary(model, format!("{}_assignment_{i}", self.namespace), fixed.then_some(true))?;
                let column = index(model, variable)?;
                aggregation.assignments.insert(key.clone(), variable);
                for (map, id, coefficient) in [
                    (&mut aggregation.loaded, &item.id, V::one()),
                    (&mut aggregation.counts, &position.id, V::one()),
                    (&mut aggregation.weights, &position.id, self.item_weights[&item.id].clone()),
                ] {
                    let previous = map.get(id).ok_or_else(|| invalid("missing loading expression"))?;
                    let mut terms = previous.monomials().to_vec();
                    terms.push(LinearMonomial::new(coefficient, column));
                    map.insert(id.clone(), Linear::new(terms, V::zero()));
                }
            }
            for (g, group) in self.problem.layout_groups.iter().enumerate() {
                for (l, layout) in group.layouts.iter().enumerate() {
                    aggregation.layouts.insert((group.id.clone(), layout.id.clone()),
                        binary(model, format!("{}_layout_{g}_{l}", self.namespace), None)?);
                }
            }
            LoadingCapacityLimit::register(model, &self.problem, &aggregation, &self.capacities, &self.namespace)?;
            LoadingConflictLimit::register(model, &self.problem, &aggregation, &self.conflicts, &self.namespace)?;
            extension(model, &aggregation)?;
            model.linear_model_snapshot()?;
            Ok(())
        })?;
        self.aggregation = aggregation;
        self.model = Some(model.model_identity());
        Ok(())
    }

    pub(crate) fn require_model(&self, model: &MetaModel<V>) -> Result<()> {
        if self.model == Some(model.model_identity()) { Ok(()) } else { Err(invalid("loading context is unregistered or belongs to another model")) }
    }

    /// 将一个合并附加重量表达式与真实容量、整体占位联动；每舱位只能注册一次。
    /// Link one combined additional-weight expression to true capacity/whole occupancy, once per position.
    pub fn link_additional_weight(
        &mut self,
        model: &mut MetaModel<V>,
        position: &str,
        additional: Linear<V>,
    ) -> Result<()> {
        self.require_model(model)?;
        if self.additional_weights.contains_key(position) { return Err(invalid("additional weight is already linked; combine contributions before registration")); }
        let occupied = *self.aggregation.occupied.get(position).ok_or_else(|| invalid("unknown additional-weight position"))?;
        model.transaction(|model| -> Result<()> {
            let suffix = self.additional_weights.len();
            model.add_linear_polynomial_constraint(additional.clone(), ConstraintRelation::GreaterEqual, V::zero(),
                &format!("{}_additional_nonnegative_{suffix}", self.namespace))?;
            let weight = &self.aggregation.weights[position];
            let mut terms = weight.monomials().to_vec();
            terms.extend_from_slice(additional.monomials());
            terms.push(LinearMonomial::new(-self.capacities[position].clone(), index(model, occupied)?));
            model.add_linear_polynomial_constraint(Linear::new(terms, additional.constant_term().clone()),
                ConstraintRelation::LessEqual, V::zero(), &format!("{}_additional_capacity_{suffix}", self.namespace))?;
            model.linear_model_snapshot()?;
            Ok(())
        })?;
        self.additional_weights.insert(position.to_string(), additional);
        Ok(())
    }

    /// 按本模型 token 提取，并复核全部扩展模型行及领域规则。 / Extract by model-local tokens and validate all extension rows and domain rules.
    pub fn extract_solution(&self, model: &MetaModel<V>, values: &[V], tolerance: &V) -> Result<DiscreteLoadingSolution> {
        self.require_model(model)?;
        let snapshot = model.linear_model_snapshot()?;
        if values.len() != snapshot.variables.len() { return Err(invalid("incomplete loading solution vector")); }
        let by_variable = snapshot.variables.iter().zip(values).map(|(state, value)| (state.variable, value.clone())).collect();
        snapshot.validate_solution(&by_variable, tolerance)?;
        let selected = |variable: VariableId| -> Result<bool> { Ok(values[index(model, variable)?] >= V::one() - tolerance.clone()) };
        let mut solution = DiscreteLoadingSolution::default();
        for (key, variable) in &self.aggregation.assignments {
            if selected(*variable)? && solution.assignments.insert(key.item_id.clone(), key.position_id.clone()).is_some() {
                return Err(invalid("cargo loaded more than once"));
            }
        }
        for (position, variable) in &self.aggregation.occupied {
            if selected(*variable)? { solution.occupied_positions.insert(position.clone()); }
        }
        for ((group, layout), variable) in &self.aggregation.layouts {
            if selected(*variable)? && solution.selected_layouts.insert(group.clone(), layout.clone()).is_some() {
                return Err(invalid("multiple layouts selected in a group"));
            }
        }
        let mut additional_weights = BTreeMap::new();
        for (position, expression) in &self.additional_weights {
            let mut weight = expression.constant_term().clone();
            for term in expression.monomials() {
                weight = checked(weight + checked(term.coefficient().clone()
                    * values.get(term.var_index()).ok_or_else(|| invalid("unknown additional-weight variable"))?.clone())?)?;
            }
            additional_weights.insert(position.clone(), weight);
        }
        self.verify(&solution, &additional_weights, tolerance)?;
        Ok(solution)
    }

    /// 不依赖 solver 的领域复核；严格正附加重量必须整体占位，不用 epsilon 忽略。
    /// Solver-independent validation; any strictly positive additional weight requires occupancy, never ignored by epsilon.
    pub fn verify(
        &self,
        solution: &DiscreteLoadingSolution,
        additional_weights: &BTreeMap<String, V>,
        tolerance: &V,
    ) -> Result<()> {
        self.validate()?;
        if !tolerance.is_finite_value() || tolerance < &V::zero() || tolerance.clone() + tolerance.clone() >= V::one() {
            return Err(invalid("invalid loading verification tolerance"));
        }
        if solution.occupied_positions.iter().any(|id| !self.capacities.contains_key(id))
            || additional_weights.keys().any(|id| !self.capacities.contains_key(id))
            || solution.assignments.iter().any(|(item, position)| !self.problem.allowed_assignments.contains(
                &AssignmentKey { item_id: item.clone(), position_id: position.clone() }))
        { return Err(invalid("unknown position or illegal loading assignment")); }
        if self.problem.items.iter().any(|item| (item.required && !solution.assignments.contains_key(&item.id))
            || item.fixed_position_id.as_ref().is_some_and(|position| solution.assignments.get(&item.id) != Some(position)))
        { return Err(invalid("required or fixed cargo missing")); }
        if self.conflicts.edges().iter().any(|edge| solution.occupied_positions.contains(&edge.first) && solution.occupied_positions.contains(&edge.second)) {
            return Err(invalid("occupied positions conflict"));
        }
        if solution.selected_layouts.keys().collect::<BTreeSet<_>>() != self.problem.layout_groups.iter().map(|group| &group.id).collect() {
            return Err(invalid("incomplete layout selections"));
        }
        for group in &self.problem.layout_groups {
            let layout = group.layouts.iter().find(|layout| layout.id == solution.selected_layouts[&group.id])
                .ok_or_else(|| invalid("unknown selected layout"))?;
            if solution.occupied_positions.intersection(&group.position_ids).any(|id| !layout.position_ids.contains(id)) {
                return Err(invalid("occupied position is absent from selected layout"));
            }
        }
        for position in &self.problem.positions {
            let items = self.problem.items.iter().filter(|item| solution.assignments.get(&item.id) == Some(&position.id)).collect::<Vec<_>>();
            let additional = checked(additional_weights.get(&position.id).cloned().unwrap_or_else(V::zero))?;
            if additional < V::zero() { return Err(invalid("negative additional loading weight")); }
            let mut weight = additional.clone();
            for item in &items { weight = checked(weight + self.item_weights[&item.id].clone())?; }
            if items.len() as u64 > position.max_count || weight > self.capacities[&position.id].clone() + tolerance.clone() {
                return Err(invalid("loading exceeds true position capacity"));
            }
            let occupied = solution.occupied_positions.contains(&position.id);
            if (!items.is_empty() || additional > V::zero() || position.fixed_occupied) && !occupied {
                return Err(invalid("cargo, fixed occupancy or positive additional weight requires whole-position occupancy"));
            }
            let fixed = self.problem.items.iter().any(|item| item.fixed_position_id.as_ref() == Some(&position.id));
            if !position.enabled && ((occupied && !position.fixed_occupied && !fixed)
                || items.iter().any(|item| item.fixed_position_id.as_ref() != Some(&position.id)))
            { return Err(invalid("decision loading in a disabled position")); }
        }
        Ok(())
    }
}
