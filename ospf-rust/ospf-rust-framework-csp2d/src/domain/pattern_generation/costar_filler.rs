//! 副产物填充 / By-product filling
//!
//! 余料里能切出可出售的副产物时，它就不再是纯粹的浪费。本模块把副产物排进图案的余料区域，
//! 只做**不重叠**的贪心放置，且不改动成品件的位置。
//! When the remnant can yield a sellable by-product it is no longer pure waste. This module packs by-products
//! into the pattern's remnant area with a non-overlapping greedy placement and never moves a finished part.
//!
//! 关键约束：
//! 1. 副产物不得与成品件重叠，也不得互相重叠（含刀缝）。
//! 2. 副产物必须落在料域内。
//! 3. 填充只增加副产物，不改变成品覆盖量——副产物不是需求驱动的产出，不能参与需求覆盖计算。
//!
//! Key constraints: a by-product may not overlap a finished part or another by-product (kerf included), it must
//! stay inside the region, and filling only adds by-products — it never changes finished coverage, because
//! by-products are not demand-driven output and must not enter demand coverage.

use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, quantity_sum, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::material::{Costar2, Pattern2};
use crate::infrastructure::region::Region2;

/// 填充结果 / Filling result
#[derive(Debug, Clone)]
pub struct CostarFillingResult<V, U: UnitTrait> {
    /// 放入的副产物放置 / Placed by-products
    pub placements: Vec<Placement2<V, U>>,
    /// 未能放入的副产物 / By-products that did not fit
    pub unplaced: Vec<Costar2<V, U>>,
}

impl<V, U> CostarFillingResult<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 放入数量 / Placed count
    pub fn placed_count(&self) -> usize {
        self.placements.len()
    }

    /// 放入面积 / Placed area
    pub fn placed_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let areas: Vec<_> = self.placements.iter().map(|placement| placement.area()).collect();
        match quantity_sum(areas) {
            Some(total) => total,
            // 空集合时以零面积表示；调用方不应依赖其单位做进一步运算。
            // An empty set reports zero area; callers must not rely on its unit for further arithmetic.
            None => Quantity::new(V::zero(), ospf_rust_quantities::unit::derived::SquareMeter::INSTANT.clone()),
        }
    }
}

/// 副产物填充器 / By-product filler
#[derive(Debug, Clone, Default)]
pub struct CostarFiller;

impl CostarFiller {
    /// 创建填充器 / Create the filler
    pub fn new() -> Self {
        Self
    }

    /// 把副产物填充进图案余料 / Fill by-products into a pattern's remnant
    ///
    /// 按面积降序尝试放置（大件先放，减少碎片化），每个副产物只尝试**不旋转**一次。
    /// 旋转会让副产物的纹理方向改变，是否可旋转属于业务决策，因此这里不做假设。
    /// Attempts placement in descending area order (largest first, less fragmentation) and tries each by-product
    /// exactly once **without rotation**. Rotation would change the grain direction of a by-product, which is a
    /// business decision, so no assumption is made here.
    pub fn fill<V, U>(
        &self,
        pattern: &Pattern2<V, U>,
        costars: &[Costar2<V, U>],
        kerf: &Quantity<V, U>,
    ) -> CostarFillingResult<V, U>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let mut placed: Vec<Placement2<V, U>> = Vec::new();
        let mut unplaced: Vec<Costar2<V, U>> = Vec::new();
        if costars.is_empty() {
            return CostarFillingResult {
                placements: placed,
                unplaced,
            };
        }

        // 大件优先：面积降序。
        // Largest first: descending area.
        let mut ordered: Vec<&Costar2<V, U>> = costars.iter().collect();
        ordered.sort_by(|lhs, rhs| {
            let left = lhs.width.value.to_f64().unwrap_or(0.0) * lhs.height.value.to_f64().unwrap_or(0.0);
            let right = rhs.width.value.to_f64().unwrap_or(0.0) * rhs.height.value.to_f64().unwrap_or(0.0);
            right
                .partial_cmp(&left)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for costar in ordered {
            match self.try_place(pattern, &placed, costar, kerf) {
                Some(placement) => placed.push(placement),
                None => unplaced.push(costar.clone()),
            }
        }
        CostarFillingResult {
            placements: placed,
            unplaced,
        }
    }

    /// 尝试放置一个副产物 / Try to place one by-product
    ///
    /// 用网格候选点扫描料域：从左上角开始，按成品件与已放副产物的「右边界 + 刀缝」「上边界 + 刀缝」生成
    /// 候选点，取第一个不重叠且不越界的位置。
    /// Scans the region with grid candidate points: starting at the top-left corner, candidates are generated from
    /// the "right edge + kerf" and "top edge + kerf" of finished parts and already placed by-products, and the
    /// first non-overlapping, in-bounds position is taken.
    fn try_place<V, U>(
        &self,
        pattern: &Pattern2<V, U>,
        placed: &[Placement2<V, U>],
        costar: &Costar2<V, U>,
        kerf: &Quantity<V, U>,
    ) -> Option<Placement2<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let shape = rectangle_shape2(costar.width.clone(), costar.height.clone()).ok()?;
        let candidates = self.candidate_points(pattern, placed, kerf);
        for (x, y) in candidates {
            let placement = Placement2::new(
                PackingShape2::Rectangle(shape.clone()),
                QuantityPoint2 {
                    x: Quantity::new(x, costar.width.unit.clone()),
                    y: Quantity::new(y, costar.height.unit.clone()),
                },
            )
            .with_item_id(costar.id.as_str().to_string());
            if !fits_in_region(&pattern.region, &placement) {
                continue;
            }
            if overlaps_any(&placement, &pattern.placements, kerf) {
                continue;
            }
            if overlaps_any(&placement, placed, kerf) {
                continue;
            }
            return Some(placement);
        }
        None
    }

    /// 生成候选点 / Generate candidate points
    fn candidate_points<V, U>(
        &self,
        pattern: &Pattern2<V, U>,
        placed: &[Placement2<V, U>],
        kerf: &Quantity<V, U>,
    ) -> Vec<(V, V)>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let zero = V::zero();
        let mut points = vec![(zero.clone(), zero.clone())];
        let kerf_value = kerf.value.clone();
        for placement in pattern.placements.iter().chain(placed.iter()) {
            let x = placement.position.x.value.clone();
            let y = placement.position.y.value.clone();
            let width = placement.shape.bounding_width().value.clone();
            let height = placement.shape.bounding_height().value.clone();
            // 每个已占位置的右边界与上边界都成为候选起点。
            // The right and top edges of every occupied position become candidate origins.
            points.push((x.clone() + width.clone() + kerf_value.clone(), y.clone()));
            points.push((x.clone(), y.clone() + height.clone() + kerf_value.clone()));
            points.push((
                x.clone() + width + kerf_value.clone(),
                y.clone() + height + kerf_value.clone(),
            ));
        }
        points.sort_by(|lhs, rhs| {
            let left = lhs.1.to_f64().unwrap_or(0.0);
            let right = rhs.1.to_f64().unwrap_or(0.0);
            left.partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let left = lhs.0.to_f64().unwrap_or(0.0);
                    let right = rhs.0.to_f64().unwrap_or(0.0);
                    left.partial_cmp(&right).unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        points.dedup_by(|lhs, rhs| {
            (lhs.0.to_f64().unwrap_or(f64::NAN) - rhs.0.to_f64().unwrap_or(f64::MAX)).abs() < 1e-9
                && (lhs.1.to_f64().unwrap_or(f64::NAN) - rhs.1.to_f64().unwrap_or(f64::MAX)).abs() < 1e-9
        });
        points
    }
}

/// 判断放置是否完全落在料域内 / Whether a placement lies fully inside the region
fn fits_in_region<V, U>(region: &Region2<V, U>, placement: &Placement2<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let left = placement.position.x.value.to_f64().unwrap_or(f64::NAN);
    let top = placement.position.y.value.to_f64().unwrap_or(f64::NAN);
    let width = placement.shape.bounding_width().value.to_f64().unwrap_or(f64::NAN);
    let height = placement.shape.bounding_height().value.to_f64().unwrap_or(f64::NAN);
    let region_left = region.x.value.to_f64().unwrap_or(f64::NAN);
    let region_top = region.y.value.to_f64().unwrap_or(f64::NAN);
    let region_right = region_left + region.width.value.to_f64().unwrap_or(f64::NAN);
    let region_bottom = region_top + region.height.value.to_f64().unwrap_or(f64::NAN);
    left >= region_left - 1e-9
        && top >= region_top - 1e-9
        && left + width <= region_right + 1e-9
        && top + height <= region_bottom + 1e-9
}

/// 判断放置是否与一组放置重叠（含刀缝）/ Whether a placement overlaps a set of placements, kerf included
///
/// 刀缝必须**双侧**膨胀：只膨胀一侧会漏判「刚好差一个刀缝」的相邻情况。
/// The kerf must inflate **both** sides: inflating only one side misses the "exactly one kerf apart" neighbour.
fn overlaps_any<V, U>(
    candidate: &Placement2<V, U>,
    others: &[Placement2<V, U>],
    kerf: &Quantity<V, U>,
) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let kerf_value = kerf.value.to_f64().unwrap_or(0.0);
    let candidate_left = candidate.position.x.value.to_f64().unwrap_or(f64::NAN);
    let candidate_top = candidate.position.y.value.to_f64().unwrap_or(f64::NAN);
    let candidate_right = candidate_left + candidate.shape.bounding_width().value.to_f64().unwrap_or(f64::NAN);
    let candidate_bottom = candidate_top + candidate.shape.bounding_height().value.to_f64().unwrap_or(f64::NAN);

    for other in others {
        let left = other.position.x.value.to_f64().unwrap_or(f64::NAN) - kerf_value / 2.0;
        let top = other.position.y.value.to_f64().unwrap_or(f64::NAN) - kerf_value / 2.0;
        let right = other.position.x.value.to_f64().unwrap_or(f64::NAN)
            + other.shape.bounding_width().value.to_f64().unwrap_or(f64::NAN)
            + kerf_value / 2.0;
        let bottom = other.position.y.value.to_f64().unwrap_or(f64::NAN)
            + other.shape.bounding_height().value.to_f64().unwrap_or(f64::NAN)
            + kerf_value / 2.0;
        // 严格相交才算重叠：相切（仅接触边界）不算。
        // Only strict intersection counts as overlap; tangency (merely touching the boundary) does not.
        if candidate_left < right
            && candidate_right > left
            && candidate_top < bottom
            && candidate_bottom > top
        {
            return true;
        }
    }
    false
}
