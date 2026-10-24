//! 扫描线候选冲突；空间隔离且贴边不重叠。 / Sweep-line conflicts, isolated by space and excluding touching boundaries.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::{Meter, UnitTrait};
use crate::domain::discrete_loading::PositionConflict;
use super::PackingValueAdapter;

/// 物理空间内的纵向区间。 / Longitudinal interval within a physical space.
#[derive(Debug, Clone)]
pub struct LoadingInterval<V, U: UnitTrait> {
    /// 舱位身份。 / Position identity.
    pub id: String,
    /// 甲板或独立空间身份。 / Deck or independent-space identity.
    pub space_id: String,
    /// 起点。 / Start coordinate.
    pub start: Quantity<V, U>,
    /// 终点。 / End coordinate.
    pub end: Quantity<V, U>,
}

/// 可选精确判交，错误必须传播。 / Optional exact intersection refinement whose errors must propagate.
pub type LoadingIntersection<'a> = dyn Fn(&str, &str) -> Result<bool> + 'a;

/// 纵向扫描生成重叠边，可进一步执行二维精确判交。
/// Sweep longitudinal overlaps, optionally refining them with exact 2D intersection.
pub fn loading_conflicts<V: LinearModelValue, U: UnitTrait>(
    intervals: &[LoadingInterval<V, U>],
    unit: &U,
    adapter: &dyn PackingValueAdapter<V, U>,
    intersects: Option<&LoadingIntersection<'_>>,
) -> Result<BTreeSet<PositionConflict>> {
    if unit.dimension_symbol() != Meter.dimension_symbol() {
        return Err(ModelError::InvalidConstraint("loading intervals require a physical length unit".into()).into());
    }
    let mut ids = BTreeSet::new();
    let mut spaces = BTreeMap::<&str, Vec<(&str, V, V)>>::new();
    for interval in intervals {
        let start = adapter.quantity(&interval.start, unit)?;
        let end = adapter.quantity(&interval.end, unit)?;
        if interval.id.trim().is_empty() || interval.space_id.trim().is_empty()
            || !ids.insert(&interval.id) || !start.is_finite_value() || !end.is_finite_value() || start >= end
        {
            return Err(ModelError::InvalidConstraint("invalid or duplicate loading interval".into()).into());
        }
        spaces.entry(&interval.space_id).or_default().push((&interval.id, start, end));
    }
    let mut conflicts = BTreeSet::new();
    for intervals in spaces.values_mut() {
        intervals.sort_by(|first, second| first.1.partial_cmp(&second.1).unwrap_or(Ordering::Equal));
        let mut active: Vec<(&str, V)> = Vec::new();
        for (id, start, end) in intervals {
            active.retain(|(_, end)| *end > *start);
            for (other, _) in &active {
                if intersects.map(|test| test(other, id)).transpose()?.unwrap_or(true) {
                    conflicts.insert(PositionConflict::new(*other, *id));
                }
            }
            active.push((id, end.clone()));
        }
    }
    Ok(conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::derived::Meter;
    use crate::infrastructure::SameUnitPackingValueAdapter;

    fn interval(id: &str, space: &str, start: f64, end: f64) -> LoadingInterval<f64, Meter> {
        LoadingInterval { id: id.into(), space_id: space.into(), start: Quantity::new_ct(start), end: Quantity::new_ct(end) }
    }

    #[test]
    fn sweep_separates_spaces_touching_boundaries_and_nontransitive_conflicts() {
        let intervals = vec![interval("a", "main", 0.0, 2.0), interval("b", "main", 1.0, 3.0),
            interval("c", "main", 2.0, 4.0), interval("other", "lower", 0.0, 4.0)];
        let edges = loading_conflicts(&intervals, &Meter, &SameUnitPackingValueAdapter, None).unwrap();
        assert_eq!(edges, BTreeSet::from([PositionConflict::new("a", "b"), PositionConflict::new("b", "c")]));
        let refined = loading_conflicts(&intervals, &Meter, &SameUnitPackingValueAdapter, Some(&|_, _| Ok(false))).unwrap();
        assert!(refined.is_empty());
        assert!(loading_conflicts(&intervals, &Meter, &SameUnitPackingValueAdapter,
            Some(&|_, _| Err(ModelError::InvalidConstraint("geometry failed".into()).into()))).is_err());
    }

    #[test]
    fn invalid_duplicate_degenerate_or_nonfinite_intervals_fail() {
        for intervals in [vec![interval("a", "main", 1.0, 1.0)], vec![interval("a", "main", f64::NAN, 2.0)],
            vec![interval("a", "main", 0.0, 1.0), interval("a", "other", 0.0, 1.0)]]
        { assert!(loading_conflicts(&intervals, &Meter, &SameUnitPackingValueAdapter, None).is_err()); }
    }
}
