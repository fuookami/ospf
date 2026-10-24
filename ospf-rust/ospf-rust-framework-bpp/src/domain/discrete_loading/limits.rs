//! 可复用约束族，application 不直接拼接领域行。 / Reusable constraint families, never assembled in applications.

use std::collections::BTreeMap;
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_core::error::Result;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{ConstraintRelation, LinearModelValue, MetaModel};
use ospf_rust_quantities::unit::UnitTrait;
use super::context::{count, index};
use super::{ConflictIndex, DiscreteLoadingProblem, LoadingAggregation};

/// 货物唯一性、真实容量与整体占位限制。 / Cargo uniqueness, true capacity and whole-position occupancy limits.
pub struct LoadingCapacityLimit;

impl LoadingCapacityLimit {
    /// 注册每件货物唯一性和每舱位有限容量联动。 / Register cargo uniqueness and finite position-capacity links.
    pub fn register<V, U>(
        model: &mut MetaModel<V>,
        problem: &DiscreteLoadingProblem<V, U>,
        aggregation: &LoadingAggregation<V>,
        capacities: &BTreeMap<String, V>,
        namespace: &str,
    ) -> Result<()>
    where
        V: LinearModelValue + One + FromPrimitive + ToPrimitive,
        U: UnitTrait,
    {
        for (i, item) in problem.items.iter().enumerate() {
            model.add_linear_polynomial_constraint(aggregation.loaded[&item.id].clone(),
                if item.required || item.fixed_position_id.is_some() { ConstraintRelation::Equal } else { ConstraintRelation::LessEqual },
                V::one(), &format!("{namespace}_item_{i}"))?;
        }
        for (i, position) in problem.positions.iter().enumerate() {
            let active = index(model, aggregation.occupied[&position.id])?;
            let count_bound = position.max_count.min(aggregation.assignments.keys().filter(|key| key.position_id == position.id).count() as u64);
            for (label, expression, bound) in [
                ("count", &aggregation.counts[&position.id], count(count_bound)?),
                ("weight", &aggregation.weights[&position.id], capacities[&position.id].clone()),
            ] {
                let mut terms = expression.monomials().to_vec();
                terms.push(LinearMonomial::new(-bound, active));
                model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), ConstraintRelation::LessEqual,
                    V::zero(), &format!("{namespace}_{label}_{i}"))?;
            }
        }
        Ok(())
    }
}

/// 保全边的互斥团与显式布局选择。 / Edge-preserving conflict cliques and explicit layout selection.
pub struct LoadingConflictLimit;

impl LoadingConflictLimit {
    /// 注册互斥和布局约束，不传播冲突的传递闭包。 / Register conflicts and layouts without taking a transitive closure.
    pub fn register<V: LinearModelValue + One, U: UnitTrait>(
        model: &mut MetaModel<V>,
        problem: &DiscreteLoadingProblem<V, U>,
        aggregation: &LoadingAggregation<V>,
        conflicts: &ConflictIndex,
        namespace: &str,
    ) -> Result<()> {
        for (i, clique) in conflicts.cliques().iter().enumerate() {
            let terms = clique.iter().map(|position|
                Ok(LinearMonomial::new(V::one(), index(model, aggregation.occupied[position])?)))
                .collect::<Result<Vec<_>>>()?;
            model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), ConstraintRelation::LessEqual,
                V::one(), &format!("{namespace}_clique_{i}"))?;
        }
        for (g, group) in problem.layout_groups.iter().enumerate() {
            let choices = group.layouts.iter().map(|layout| {
                Ok(LinearMonomial::new(V::one(), index(model, aggregation.layouts[&(group.id.clone(), layout.id.clone())])?))
            }).collect::<Result<Vec<_>>>()?;
            model.add_linear_polynomial_constraint(Linear::new(choices, V::zero()), ConstraintRelation::Equal,
                V::one(), &format!("{namespace}_layout_choose_{g}"))?;
            for (p, position) in group.position_ids.iter().enumerate() {
                let mut terms = vec![LinearMonomial::new(V::one(), index(model, aggregation.occupied[position])?)];
                for layout in &group.layouts {
                    if layout.position_ids.contains(position) {
                        terms.push(LinearMonomial::new(-V::one(),
                            index(model, aggregation.layouts[&(group.id.clone(), layout.id.clone())])?));
                    }
                }
                model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), ConstraintRelation::LessEqual,
                    V::zero(), &format!("{namespace}_layout_member_{g}_{p}"))?;
            }
        }
        Ok(())
    }
}
