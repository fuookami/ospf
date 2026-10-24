//! 共享 BPP 的二维离散几何适配，不依赖 BPP1D。 / Shared-BPP discrete 2D geometry adapter, independent of BPP1D.

use std::collections::{BTreeMap, BTreeSet};
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::UnitTrait;
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_framework_bpp::{DiscreteLoadingContext, DiscreteLoadingProblem, LoadingInterval, PackingValueAdapter, loading_conflicts};
use ospf_rust_framework_geometry2d::{Box2Geometry, PackingShape2, RectanglePackingShape2};

/// 物理空间内显式声明的矩形舱位。 / Explicit rectangular loading position within a physical space.
#[derive(Debug, Clone)]
pub struct LoadingRectangle<V, U: UnitTrait> {
    /// 舱位身份。 / Position identity.
    pub id: String,
    /// 甲板或独立空间身份。 / Deck or independent-space identity.
    pub space_id: String,
    /// 共享几何内核的矩形边界。 / Rectangle bounds from the shared geometry kernel.
    pub bounds: Box2Geometry<V, U>,
}

/// 二维离散装载工厂，只追加几何冲突，不引入隐式布局选择。 / Discrete 2D loading factory, adding geometry conflicts but never implicit layouts.
pub struct Bpp2dDiscreteLoading;

impl Bpp2dDiscreteLoading {
    /// 扫描纵向候选并调用共享二维矩形判交；保留所有显式追加冲突。
    /// Sweep longitudinal candidates and apply shared 2D rectangle intersection, retaining all explicit conflicts.
    pub fn context<V, U, W>(
        mut problem: DiscreteLoadingProblem<V, W>,
        geometry: &[LoadingRectangle<V, U>],
        geometry_unit: &U,
        weight_adapter: &dyn PackingValueAdapter<V, W>,
        geometry_adapter: &dyn PackingValueAdapter<V, U>,
    ) -> Result<DiscreteLoadingContext<V, W>>
    where
        V: LinearModelValue + One + FromPrimitive + ToPrimitive,
        U: UnitTrait + Clone,
        W: UnitTrait,
    {
        let ids = geometry.iter().map(|rectangle| &rectangle.id).collect::<BTreeSet<_>>();
        if ids.len() != geometry.len() || ids != problem.positions.iter().map(|position| &position.id).collect() {
            return Err(ModelError::InvalidConstraint("loading positions and explicit rectangles differ".into()).into());
        }
        let mut normalized = BTreeMap::new();
        let mut intervals = Vec::new();
        for rectangle in geometry {
            let PackingShape2::Rectangle(shape) = &rectangle.bounds.shape else {
                return Err(ModelError::InvalidConstraint("discrete loading requires explicit rectangle bounds".into()).into());
            };
            let x = geometry_adapter.quantity(&rectangle.bounds.x, geometry_unit)?;
            let y = geometry_adapter.quantity(&rectangle.bounds.y, geometry_unit)?;
            let width = geometry_adapter.quantity(&shape.width, geometry_unit)?;
            let height = geometry_adapter.quantity(&shape.height, geometry_unit)?;
            let max_x = x.clone() + width.clone();
            let max_y = y.clone() + height.clone();
            if [&x, &y, &width, &height, &max_x, &max_y].iter().any(|value| !value.is_finite_value())
                || width <= V::zero() || height <= V::zero() || max_x <= x || max_y <= y
            {
                return Err(ModelError::InvalidConstraint("invalid finite positive discrete rectangle extents".into()).into());
            }
            intervals.push(LoadingInterval { id: rectangle.id.clone(), space_id: rectangle.space_id.clone(),
                start: Quantity::new(x.clone(), geometry_unit.clone()), end: Quantity::new(max_x, geometry_unit.clone()) });
            normalized.insert(rectangle.id.clone(), Box2Geometry {
                x: Quantity::new(x, geometry_unit.clone()), y: Quantity::new(y, geometry_unit.clone()),
                shape: PackingShape2::Rectangle(RectanglePackingShape2 {
                    width: Quantity::new(width, geometry_unit.clone()), height: Quantity::new(height, geometry_unit.clone()),
                }),
            });
        }
        let intersects = |first: &str, second: &str| normalized[first].rectangle_overlapped(&normalized[second])
            .ok_or_else(|| ModelError::InvalidConstraint("normalized loading rectangles are incompatible".into()).into());
        problem.conflicts.extend(loading_conflicts(&intervals, geometry_unit, geometry_adapter, Some(&intersects))?);
        DiscreteLoadingContext::new(problem, weight_adapter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_framework_bpp::{LoadingPosition, PositionConflict, SameUnitPackingValueAdapter};
    use ospf_rust_quantities::unit::derived::{Kilogram, Meter};

    fn rectangle(id: &str, space: &str, x: f64, y: f64) -> LoadingRectangle<f64, Meter> {
        LoadingRectangle { id: id.into(), space_id: space.into(), bounds: Box2Geometry::of_rectangle(
            Quantity::new_ct(x), Quantity::new_ct(y), Quantity::new_ct(2.0), Quantity::new_ct(2.0)) }
    }

    fn problem(ids: &[&str]) -> DiscreteLoadingProblem<f64, Kilogram> {
        DiscreteLoadingProblem {
            items: vec![], positions: ids.iter().map(|id| LoadingPosition {
                id: id.to_string(), max_count: 1, max_weight: Quantity::new_ct(10.0), fixed_occupied: false, enabled: true,
            }).collect(), allowed_assignments: BTreeSet::new(), conflicts: BTreeSet::new(), layout_groups: vec![], weight_unit: Kilogram,
        }
    }

    #[test]
    fn rectangles_refine_longitudinal_candidates_separate_decks_and_preserve_explicit_edges() {
        let geometry = vec![rectangle("a", "main", 0.0, 0.0), rectangle("b", "main", 1.0, 1.0),
            rectangle("touch", "main", 2.0, 0.0), rectangle("side", "main", 0.0, 3.0), rectangle("lower", "lower", 0.0, 0.0)];
        let mut problem = problem(&["a", "b", "touch", "side", "lower"]);
        problem.conflicts.insert(PositionConflict::new("side", "lower"));
        let context = Bpp2dDiscreteLoading::context(problem, &geometry, &Meter,
            &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
        assert_eq!(context.conflict_index().edges(), &BTreeSet::from([
            PositionConflict::new("a", "b"), PositionConflict::new("b", "touch"), PositionConflict::new("side", "lower"),
        ]));
        assert!(context.problem().layout_groups.is_empty());
    }

    #[test]
    fn mismatched_duplicate_degenerate_and_nonfinite_geometry_is_rejected() {
        for geometry in [vec![rectangle("missing", "main", 0.0, 0.0)],
            vec![rectangle("a", "main", 0.0, 0.0), rectangle("a", "other", 0.0, 0.0)],
            vec![rectangle("a", "main", f64::INFINITY, 0.0)]]
        {
            assert!(Bpp2dDiscreteLoading::context(problem(&["a"]), &geometry, &Meter,
                &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).is_err());
        }
    }

    fn native_overlap<V>(epsilon: V)
    where
        V: LinearModelValue + One + FromPrimitive + ToPrimitive,
    {
        let zero = V::zero();
        let one = V::one();
        let rectangles = [LoadingRectangle { id: "a".into(), space_id: "main".into(), bounds: Box2Geometry::of_rectangle(
            Quantity::<_, Meter>::new_ct(zero.clone()), Quantity::new_ct(zero.clone()), Quantity::new_ct(one.clone()), Quantity::new_ct(one.clone())) },
            LoadingRectangle { id: "b".into(), space_id: "main".into(), bounds: Box2Geometry::of_rectangle(
                Quantity::new_ct(one.clone() - epsilon), Quantity::new_ct(zero), Quantity::new_ct(one.clone()), Quantity::new_ct(one.clone())) }];
        let problem = DiscreteLoadingProblem {
            items: vec![], positions: ["a", "b"].into_iter().map(|id| LoadingPosition {
                id: id.into(), max_count: 1, max_weight: Quantity::<_, Kilogram>::new_ct(one.clone()), fixed_occupied: false, enabled: true,
            }).collect(), allowed_assignments: BTreeSet::new(), conflicts: BTreeSet::new(), layout_groups: vec![], weight_unit: Kilogram,
        };
        let context = Bpp2dDiscreteLoading::context(problem, &rectangles, &Meter, &SameUnitPackingValueAdapter, &SameUnitPackingValueAdapter).unwrap();
        assert_eq!(context.conflict_index().edges(), &BTreeSet::from([PositionConflict::new("a", "b")]));
    }

    #[test]
    fn decimal_and_rational_geometry_keep_overlaps_smaller_than_f64_precision() {
        use std::str::FromStr;
        native_overlap(bigdecimal::BigDecimal::from_str("0.00000000000000000000000000001").unwrap());
        native_overlap(num_rational::BigRational::new(1.into(), 10u64.pow(18).into()));
    }

    #[test]
    fn runtime_geometry_converts_mixed_length_units_and_rejects_wrong_dimensions() {
        use ospf_rust_framework_bpp::RuntimePackingValueAdapter;
        use ospf_rust_quantities::unit::{CTUnit, Centimeter};
        let meter = Meter::INSTANT;
        let centimeter = Centimeter::INSTANT;
        let kilogram = Kilogram::INSTANT;
        let geometry = [LoadingRectangle { id: "a".into(), space_id: "main".into(), bounds: Box2Geometry::of_rectangle(
            Quantity::new(0.0, centimeter.clone()), Quantity::new(0.0, centimeter.clone()), Quantity::new(100.0, centimeter.clone()), Quantity::new(100.0, centimeter.clone())) },
            LoadingRectangle { id: "b".into(), space_id: "main".into(), bounds: Box2Geometry::of_rectangle(
                Quantity::new(0.5, meter.clone()), Quantity::new(0.5, meter.clone()), Quantity::new(1.0, meter.clone()), Quantity::new(1.0, meter.clone())) }];
        let input = problem(&["a", "b"]);
        let context = Bpp2dDiscreteLoading::context(input.clone(), &geometry, &meter, &SameUnitPackingValueAdapter, &RuntimePackingValueAdapter).unwrap();
        assert_eq!(context.conflict_index().edges(), &BTreeSet::from([PositionConflict::new("a", "b")]));
        assert!(Bpp2dDiscreteLoading::context(input, &geometry, &kilogram, &SameUnitPackingValueAdapter, &RuntimePackingValueAdapter).is_err());
    }
}
