//! 完整航空装载的唯一注册入口。 / Single registration entry for complete aviation loading.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::error::{CoreError, ModelError, Result};
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{BinaryVariableItem, UIntegerVariableItem, UContinuousVariableItem, VariableId, VariableRange};
use ospf_rust_framework_bpp::{AssignmentKey, DiscreteLoadingContext, DiscreteLoadingProblem, LoadingInterval,
    LoadingItem, LoadingPosition, PartitionDecision, PartitionLoadingOptions, SameUnitPackingValueAdapter};
use ospf_rust_framework_bpp::application::RegisteredDiscreteLoading;
use ospf_rust_framework_bpp1d::domain::Bpp1dDiscreteLoading;
use ospf_rust_framework_bpp2d::domain::discrete_loading::{Bpp2dDiscreteLoading, LoadingRectangle};
use ospf_rust_framework_geometry2d::Box2Geometry;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Kilogram, Meter};
use super::model_registration::{RegistrationResult, construct_objective, register_derived_variables};
use super::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::service::domain_pipeline::apply_domain_pipeline;
use crate::framework::demo2::infrastructure::dto::{AircraftTypeInput, Demo2Request};

pub(crate) fn invalid(message: impl Into<String>) -> CoreError {
    ModelError::InvalidConstraint(format!("demo2: {}", message.into())).into()
}

fn boxed(error: Box<dyn std::error::Error>) -> CoreError { invalid(error.to_string()) }

pub(crate) fn index(model: &MetaModel, variable: VariableId) -> Result<usize> {
    model.tokens().iter().position(|token| token.id() == variable).ok_or_else(|| invalid("missing registered column"))
}

/// 完整航空上下文；共享装载变量是唯一分配与占位来源。 / Complete aviation context; shared loading owns all assignment and occupancy decisions.
pub(crate) struct Demo2LoadingContext;

impl Demo2LoadingContext {
    pub(crate) fn normalize(mut request: Demo2Request) -> Result<Demo2Request> {
        if request.positions.iter().any(|position| !request.loading.positions.contains_key(&position.name)) {
            return Err(invalid("every position requires explicit geometry and loading order"));
        }
        let adjacent = if request.loading.adjacent_positions.is_empty() {
            request.adjacent_positions.iter().map(|pair| {
                let first = request.positions.get(pair.first).ok_or_else(|| invalid("invalid legacy adjacent position"))?;
                let second = request.positions.get(pair.second).ok_or_else(|| invalid("invalid legacy adjacent position"))?;
                Ok((first.name.clone(), second.name.clone()))
            }).collect::<Result<Vec<_>>>()?
        } else { request.loading.adjacent_positions.clone() };
        request.positions.sort_by_key(|position| {
            let input = &request.loading.positions[&position.name];
            (input.loading_order_scope.clone(), input.loading_order, position.name.clone())
        });
        request.adjacent_positions = adjacent.into_iter().map(|(first, second)| {
            let first = request.positions.iter().position(|position| position.name == first).ok_or_else(|| invalid("unknown adjacent position ID"))?;
            let second = request.positions.iter().position(|position| position.name == second).ok_or_else(|| invalid("unknown adjacent position ID"))?;
            if first == second { return Err(invalid("self adjacency")); }
            Ok(crate::framework::demo2::infrastructure::dto::PositionPair { first, second })
        }).collect::<Result<_>>()?;
        Ok(request)
    }

    pub(crate) fn context(request: &Demo2Request, mode: Demo2PipelineMode) -> Result<DiscreteLoadingContext<f64, Kilogram>> {
        let cargo_ids = request.cargos.iter().map(|cargo| cargo.name.as_str()).collect::<BTreeSet<_>>();
        let position_ids = request.positions.iter().map(|position| position.name.as_str()).collect::<BTreeSet<_>>();
        if cargo_ids.len() != request.cargos.len() || position_ids.len() != request.positions.len()
            || request.positions.is_empty() || request.cargos.is_empty()
            || request.loading.positions.keys().map(String::as_str).collect::<BTreeSet<_>>() != position_ids
            || request.loading.cargos.keys().any(|id| !cargo_ids.contains(id.as_str()))
        { return Err(invalid("duplicate/empty identities or mismatched loading configuration")); }
        let finite = [request.payload_upper_bound, request.min_payload_ratio, request.max_adjacent_load_gap,
            request.max_cumulative_forward_load, request.max_cumulative_backward_load,
            request.envelope_longitudinal_moment_min, request.envelope_longitudinal_moment_max,
            request.target_longitudinal_moment, request.max_longitudinal_moment_deviation, request.max_lateral_imbalance,
            request.weight_recommendation_objective.balance_priority, request.weight_recommendation_objective.payload_priority];
        if finite.iter().any(|value| !value.is_finite()) || request.payload_upper_bound < 0.0
            || !(0.0..=1.0).contains(&request.min_payload_ratio)
            || request.envelope_longitudinal_moment_min > request.envelope_longitudinal_moment_max
            || [request.max_adjacent_load_gap, request.max_cumulative_forward_load, request.max_cumulative_backward_load,
                request.max_longitudinal_moment_deviation, request.max_lateral_imbalance,
                request.weight_recommendation_objective.balance_priority, request.weight_recommendation_objective.payload_priority]
                .iter().any(|value| *value < 0.0)
            || request.adjacent_positions.iter().any(|pair| pair.first >= request.positions.len() || pair.second >= request.positions.len() || pair.first == pair.second)
        { return Err(invalid("invalid aviation limits or objective priorities")); }
        let mut fixed = BTreeMap::new();
        if request.loading.max_clim.is_some_and(|value| !value.is_finite() || value < 0.0)
            || (request.loading.max_clim.is_some() && matches!(request.aircraft_type, AircraftTypeInput::B737 | AircraftTypeInput::B757))
        { return Err(invalid("CLIM requires a finite nonnegative wide-body limit")); }
        for position in &request.positions {
            if [position.max_weight, position.longitudinal_arm, position.lateral_arm, position.area, position.length]
                .iter().any(|value| !value.is_finite()) || position.max_weight < 0.0 || position.area <= 0.0 || position.length <= 0.0
            { return Err(invalid("invalid position capacity/arm/dimensions")); }
            for id in &position.loaded_items {
                if !cargo_ids.contains(id.as_str()) || fixed.insert(id.clone(), position.name.clone()).is_some() {
                    return Err(invalid("unknown or multiply fixed cargo"));
                }
            }
        }
        let mut items = Vec::new();
        let mut allowed_assignments = BTreeSet::new();
        for cargo in &request.cargos {
            let policy = request.loading.cargos.get(&cargo.name);
            if policy.and_then(|policy| policy.allowed_positions.as_ref()).is_some_and(|ids| ids.iter().any(|id| !position_ids.contains(id.as_str())))
                || policy.and_then(|policy| policy.space_id.as_ref()).is_some_and(|id| !request.loading.positions.values().any(|position| &position.space_id == id))
            { return Err(invalid("unknown allowed position or cargo space")); }
            items.push(LoadingItem { id: cargo.name.clone(), weight: Quantity::new_ct(cargo.weight),
                required: policy.and_then(|policy| policy.required).unwrap_or(mode == Demo2PipelineMode::Predistribution),
                fixed_position_id: fixed.get(&cargo.name).cloned() });
            for position in &request.positions {
                let geometry = &request.loading.positions[&position.name];
                if policy.and_then(|policy| policy.allowed_positions.as_ref()).is_none_or(|ids| ids.contains(&position.name))
                    && policy.and_then(|policy| policy.space_id.as_ref()).is_none_or(|id| *id == geometry.space_id)
                    && (fixed.get(&cargo.name) == Some(&position.name) || geometry.stowage_needed || geometry.adjustment_needed)
                { allowed_assignments.insert(AssignmentKey { item_id: cargo.name.clone(), position_id: position.name.clone() }); }
            }
        }
        let positions = request.positions.iter().map(|position| {
            let input = &request.loading.positions[&position.name];
            LoadingPosition { id: position.name.clone(), max_count: position.max_load_count,
                max_weight: Quantity::new_ct(position.max_weight), fixed_occupied: input.fixed_occupied,
                enabled: input.enabled }
        }).collect();
        let problem = DiscreteLoadingProblem { items, positions, allowed_assignments, conflicts: request.loading.conflicts.clone(),
            layout_groups: request.loading.layout_groups.clone(), weight_unit: Kilogram };
        let mut intervals = Vec::new();
        let mut rectangles = Vec::new();
        for (id, geometry) in &request.loading.positions {
            let [front, back, left, right] = [geometry.front, geometry.back, geometry.left, geometry.right]
                .map(|value| request.loading.length_unit.meters(value));
            if geometry.space_id.trim().is_empty() || geometry.loading_order_scope.trim().is_empty() || [front, back, left, right].iter().any(|value| !value.is_finite())
                || front >= back || left >= right
            { return Err(invalid("explicit finite positive position geometry is required")); }
            intervals.push(LoadingInterval { id: id.clone(), space_id: geometry.space_id.clone(),
                start: Quantity::<_, Meter>::new_ct(front), end: Quantity::new_ct(back) });
            rectangles.push(LoadingRectangle { id: id.clone(), space_id: geometry.space_id.clone(),
                bounds: Box2Geometry::of_rectangle(Quantity::<_, Meter>::new_ct(front), Quantity::new_ct(left),
                    Quantity::new_ct(back - front), Quantity::new_ct(right - left)) });
        }
        match request.aircraft_type {
            AircraftTypeInput::B737 | AircraftTypeInput::B757 => Bpp1dDiscreteLoading::context(problem, &intervals, &Meter,
                &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter),
            AircraftTypeInput::B767 | AircraftTypeInput::B747 => Bpp2dDiscreteLoading::context(problem, &rectangles, &Meter,
                &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter),
            AircraftTypeInput::Unknown => Err(invalid("unsupported aircraft")),
        }
    }

    pub(crate) fn register(request: &Demo2Request, mode: Demo2PipelineMode) -> Result<RegisteredDiscreteLoading<f64, Kilogram>> {
        let normalized = Self::normalize(request.clone())?;
        let request = &normalized;
        let mut loading = Self::context(request, mode)?;
        let mut model = MetaModel::new("demo2_complete_loading");
        loading.register(&mut model)?;
        // 旧管线边界只读稀疏决策；非法配对统一映射到固定零列，绝不创建另一套决策。
        // The legacy pipeline boundary reads sparse decisions; illegal pairs share one fixed-zero column, never a second decision model.
        let zero = model.register_variable(UContinuousVariableItem::auto("demo2_illegal_assignment_zero"))?;
        model.set_variable_range_by_index(zero, VariableRange::bounded(0.0, 0.0))?;
        let x_idx = Self::assignments(request, &loading, &model, zero)?;
        let occupied = request.positions.iter().map(|position| index(&model, loading.aggregation().occupied[&position.name])).collect::<Result<Vec<_>>>()?;
        let mut derived = register_derived_variables(request, &mut model, &x_idx, &occupied).map_err(boxed)?;
        for (p, position) in request.positions.iter().enumerate() {
            let input = &request.loading.positions[&position.name];
            let actual = model.register_variable(BinaryVariableItem::auto(&format!("demo2_actual_loaded_{p}")))?;
            let mut count = x_idx.iter().map(|cargo| (cargo[p], 1.0)).collect::<Vec<_>>();
            // 固定占位可以只是空预留；实际装载仅由实际货物推导。
            // Fixed occupancy may be an empty reservation; actual loading derives only from actual cargo.
            count.push((actual, -1.0));
            model.add_linear_constraint(&count, ConstraintRelation::GreaterEqual, 0.0, &format!("demo2_actual_loaded_lower_{p}"))?;
            let mut upper = x_idx.iter().map(|cargo| (cargo[p], 1.0)).collect::<Vec<_>>();
            upper.push((actual, -(position.max_load_count as f64)));
            model.add_linear_constraint(&upper, ConstraintRelation::LessEqual, 0.0, &format!("demo2_actual_loaded_upper_{p}"))?;
            let mut additional = Vec::new();
            for (kind, bounds) in [("predicate", input.predicate_weight), ("recommended",
                if mode == Demo2PipelineMode::WeightRecommendation { input.recommended_weight } else { None })]
            {
                if let Some((lower, upper)) = bounds {
                    if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || upper < lower || upper > position.max_weight {
                        return Err(invalid("invalid additional-weight range"));
                    }
                    let name = format!("demo2_{kind}_weight_{p}");
                    let column = if kind == "recommended" { model.register_variable(UIntegerVariableItem::auto(&name))? }
                        else { model.register_variable(UContinuousVariableItem::auto(&name))? };
                    model.set_variable_range_by_index(column, VariableRange::bounded(0.0, upper))?;
                    let mut minimum = vec![(column, 1.0), (derived.estimate_loaded_idx[p], -lower)];
                    if kind == "recommended" && (input.stowage_needed || input.adjustment_needed) {
                        minimum.push((actual, lower));
                        model.add_linear_constraint(&[(column, 1.0), (actual, position.max_weight)], ConstraintRelation::LessEqual,
                            position.max_weight, &format!("demo2_recommended_excludes_actual_{p}"))?;
                    }
                    model.add_linear_constraint(&minimum,
                        ConstraintRelation::GreaterEqual, 0.0, &format!("demo2_{kind}_min_{p}"))?;
                    additional.push((column, 1.0));
                }
            }
            if !additional.is_empty() {
                loading.link_additional_weight(&mut model, &position.name, Linear::new(additional.iter()
                    .map(|(column, value)| LinearMonomial::new(*value, *column)).collect(), 0.0))?;
                let total = model.register_variable(UContinuousVariableItem::auto(&format!("demo2_total_weight_{p}")))?;
                let mut terms = vec![(total, 1.0), (derived.estimate_load_weight_idx[p], -1.0)];
                terms.extend(additional.iter().map(|(column, value)| (*column, -*value)));
                model.add_linear_constraint(&terms, ConstraintRelation::Equal, 0.0, &format!("demo2_total_weight_definition_{p}"))?;
                model.set_variable_range_by_index(total, VariableRange::bounded(0.0, position.max_weight))?;
                derived.estimate_load_weight_idx[p] = total;
            }
        }
        let z = if mode != Demo2PipelineMode::FullLoad { Some(model.register_variable(UContinuousVariableItem::auto("demo2_max_deviation"))?) } else { None };
        if let Some(z) = z {
            let total = request.cargos.iter().map(|cargo| cargo.weight).sum::<f64>();
            let capacity = request.positions.iter().map(|position| position.max_weight).sum::<f64>();
            let bound = total.max(capacity).next_up();
            if !bound.is_finite() { return Err(invalid("deviation capacity overflow")); }
            model.set_variable_range_by_index(z, VariableRange::bounded(0.0, bound))?;
        }
        let registration = RegistrationResult { x_idx, z, estimate_load_weight_idx: derived.estimate_load_weight_idx,
            estimate_loaded_idx: derived.estimate_loaded_idx, loaded_idx: derived.loaded_idx };
        construct_objective(request, &mut model, &registration, mode).map_err(boxed)?;
        apply_domain_pipeline(mode, &mut model, request, &registration.x_idx, z, &registration.estimate_load_weight_idx,
            &registration.estimate_loaded_idx, &registration.loaded_idx).map_err(boxed)?;
        if mode == Demo2PipelineMode::WeightRecommendation {
            super::recommendation::register(request, &mut model, &registration.estimate_load_weight_idx)?;
        }
        RegisteredDiscreteLoading::new(loading, model)
    }

    pub(crate) fn assignments(
        request: &Demo2Request,
        loading: &DiscreteLoadingContext<f64, Kilogram>,
        model: &MetaModel,
        zero: usize,
    ) -> Result<Vec<Vec<usize>>> {
        request.cargos.iter().map(|cargo| request.positions.iter().map(|position| loading.aggregation().assignments
            .get(&AssignmentKey { item_id: cargo.name.clone(), position_id: position.name.clone() })
            .map_or(Ok(zero), |variable| index(model, *variable))).collect()).collect()
    }

    pub(crate) fn partitions(request: &Demo2Request, registration: &RegisteredDiscreteLoading<f64, Kilogram>) -> Result<PartitionLoadingOptions<f64>> {
        let model = registration.model();
        let mut extra_decisions = Vec::new();
        let mut refinable_resources = BTreeSet::new();
        for (p, position) in request.positions.iter().enumerate() {
            for kind in ["predicate", "recommended"] {
                let name = format!("demo2_{kind}_weight_{p}");
                if let Some(token) = model.tokens().iter().find(|token| token.name() == name) {
                    let owners = request.loading.partitions.iter().filter(|partition| partition.position_ids.contains(&position.name)).collect::<Vec<_>>();
                    if owners.len() != 1 { return Err(invalid("additional resource needs unique explicit partition ownership")); }
                    extra_decisions.push(PartitionDecision { id: name, partition_id: owners[0].id.clone(), variable: token.id() });
                    refinable_resources.insert(token.id());
                }
            }
        }
        Ok(PartitionLoadingOptions { partitions: request.loading.partitions.clone(), extra_decisions, refinable_resources,
            tolerance: request.loading.column_generation.tolerance })
    }
}
