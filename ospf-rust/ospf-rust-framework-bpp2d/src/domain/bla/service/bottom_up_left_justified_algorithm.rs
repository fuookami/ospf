//! 底部向上左对齐算法 / Bottom-up left-justified algorithm
//!
//! 与 BPP3D 的 `BottomUpLeftJustifiedAlgorithm` 对应，但收敛为二维：在板材平面上依次放置物品，
//! 每次都选择最低且最靠左的可行点。算法不使用流式返回，也不使用非空断言；圆形件不参与 BLA
//! （圆形件没有「左下推挤」语义），会被显式跳过并在结果中标记来源。
//! Mirrors BPP3D `BottomUpLeftJustifiedAlgorithm` reduced to 2D: items are placed on the sheet plane always
//! choosing the lowest then leftmost feasible point. The algorithm returns a result instead of a stream and
//! avoids non-null assertions. Circle items do not participate in BLA (they have no "push to bottom-left"
//! semantics) and are skipped with an explicit result marker.

use std::cmp::Ordering as CmpOrdering;

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Box2Geometry, Placement2, QuantityPoint2, Sheet2Shape, candidate_shapes, inflated_by_shape,
    quantity_greater, quantity_greater_or_equal, quantity_is_positive, quantity_less,
    quantity_less_or_equal, quantity_zero_of,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::block_loading::model::{add_quantity, placement_inside_space};
use crate::domain::item::model::{ActualItem, ItemId};
use crate::domain::item::model::layer::Layer;

// ============================================================================
// 落点比较 / Position comparison
// ============================================================================

/// 比较两个落点的放置优先级：先比较「先下后左」的加权距离，再比较 y，最后比较 x
/// / Compare two positions by placement priority: the bottom-left weighted distance first, then y, then x
///
/// 无法比较（量纲不一致）时返回 `None`。
/// Returns `None` when the positions are incomparable (dimension mismatch).
pub fn compare_placement_position<V, U>(
    lhs: &QuantityPoint2<V, U>,
    rhs: &QuantityPoint2<V, U>,
    with_weighted_distance: bool,
) -> Option<CmpOrdering>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    if with_weighted_distance {
        // 加权距离：x + y，量纲与坐标一致。
        // Weighted distance: x + y, carrying the coordinate dimension.
        let lhs_distance = add_quantity(&lhs.x, &lhs.y);
        let rhs_distance = add_quantity(&rhs.x, &rhs.y);
        match lhs_distance.value.partial_cmp(&rhs_distance.value)? {
            CmpOrdering::Equal => {}
            other => return Some(other),
        }
    }
    match lhs.y.value.partial_cmp(&rhs.y.value)? {
        CmpOrdering::Equal => {}
        other => return Some(other),
    }
    lhs.x.value.partial_cmp(&rhs.x.value)
}

// ============================================================================
// BlaConfig - 算法配置 / Algorithm configuration
// ============================================================================

/// 算法配置 / Algorithm configuration
#[derive(Debug, Clone)]
pub struct BlaConfig {
    /// 是否允许沿 X 轴向左滑动 / Whether sliding left along X is allowed
    pub with_displacement_x: bool,
    /// 是否允许沿 Y 轴向下滑动 / Whether sliding down along Y is allowed
    pub with_displacement_y: bool,
    /// 是否按加权距离排序候选点 / Whether candidate points are ordered by weighted distance
    pub with_weighted_distance: bool,
}

impl Default for BlaConfig {
    fn default() -> Self {
        Self {
            with_displacement_x: true,
            with_displacement_y: true,
            with_weighted_distance: true,
        }
    }
}

impl BlaConfig {
    /// 是否启用任一方向的滑动 / Whether sliding is enabled in either direction
    pub fn with_displacement(&self) -> bool {
        self.with_displacement_x || self.with_displacement_y
    }
}

// ============================================================================
// Bla2dLayoutResult - BLA 排样结果 / BLA layout result
// ============================================================================

/// BLA 排样结果 / BLA layout result
#[derive(Debug, Clone)]
pub struct Bla2dLayoutResult<V, U: UnitTrait> {
    /// 放置列表 / Placement list
    pub placements: Vec<Placement2<V, U>>,
    /// 被跳过的圆形件编号 / Identifiers of skipped circle items
    pub skipped_circle_items: Vec<ItemId>,
    /// 无法放置的物品编号 / Identifiers of items that could not be placed
    pub skipped_unfitted_items: Vec<ItemId>,
}

impl<V, U: UnitTrait> Bla2dLayoutResult<V, U> {
    /// 是否全部放置成功 / Whether everything was placed successfully
    pub fn fully_placed(&self) -> bool {
        self.skipped_unfitted_items.is_empty() && self.skipped_circle_items.is_empty()
    }
}

// ============================================================================
// BottomUpLeftJustifiedAlgorithm - 底部向上左对齐 / Bottom-up left-justified
// ============================================================================

/// 底部向上左对齐算法 / Bottom-up left-justified algorithm
#[derive(Debug, Clone)]
pub struct BottomUpLeftJustifiedAlgorithm<V, U: UnitTrait> {
    /// 板材形状 / Sheet shape
    pub sheet: Sheet2Shape<V, U>,
    /// 算法配置 / Algorithm configuration
    pub config: BlaConfig,
    /// 刀缝间距 / Kerf spacing
    pub kerf: Option<Quantity<V, U>>,
}

impl<V, U> BottomUpLeftJustifiedAlgorithm<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建算法 / Create the algorithm
    pub fn new(sheet: Sheet2Shape<V, U>, config: BlaConfig, kerf: Option<Quantity<V, U>>) -> Self {
        Self { sheet, config, kerf }
    }

    /// 执行排样 / Execute the layout
    ///
    /// 圆形件被跳过；无法放置的物品记入 `skipped_unfitted_items`，不视为异常。
    /// Circle items are skipped; unplaceable items are recorded in `skipped_unfitted_items` rather than
    /// treated as errors.
    pub fn invoke(&self, items: &[(ActualItem<V, U>, usize)]) -> Ret<Bla2dLayoutResult<V, U>> {
        let mut placements: Vec<Placement2<V, U>> = Vec::new();
        let mut skipped_circle_items = Vec::new();
        let mut skipped_unfitted_items = Vec::new();

        for (item, amount) in items {
            if item.shape.is_circle() {
                skipped_circle_items.push(item.id.clone());
                continue;
            }
            for _ in 0..*amount {
                match self.place_item(item, &placements) {
                    Some(placement) => placements.push(placement),
                    None => skipped_unfitted_items.push(item.id.clone()),
                }
            }
        }

        Ok(Bla2dLayoutResult {
            placements,
            skipped_circle_items,
            skipped_unfitted_items,
        })
    }

    /// 排样并转换为一整块候选层 / Lay out and convert into one candidate layer
    ///
    /// 结果为空时返回 `None`，避免产出零件数的空列。
    /// Returns `None` for an empty layout so no zero-amount empty column is produced.
    pub fn layer(
        &self,
        items: &[(ActualItem<V, U>, usize)],
        iteration: i64,
        source: &str,
        sheet_type: Option<crate::domain::item::model::SheetType<V, U>>,
    ) -> Ret<Option<Layer<V, U>>> {
        let result = self.invoke(items)?;
        if result.placements.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            Layer::new(iteration, source, sheet_type, self.sheet.clone())
                .with_units(result.placements),
        ))
    }

    /// 放置单个物品 / Place a single item
    fn place_item(
        &self,
        item: &ActualItem<V, U>,
        placed: &[Placement2<V, U>],
    ) -> Option<Placement2<V, U>> {
        for candidate in candidate_shapes(&item.shape, item.allow_rotate) {
            let shape = candidate.shape.clone();
            let feasible = self.feasible_points(placed);
            for point in feasible {
                let placement = Placement2::new(shape.clone(), point)
                    .with_rotated(candidate.rotated)
                    .with_item_id(item.id.as_str());
                if self.is_feasible(&placement, placed) {
                    return Some(placement);
                }
            }
        }
        None
    }

    /// 计算所有候选落点 / Compute all candidate positions
    ///
    /// 候选点由已放置矩形的「左上角」与「右下角」派生；为空时给出原点。
    /// Candidates derive from the upper-left and lower-right corners of placed rectangles, falling back to
    /// the origin.
    fn feasible_points(&self, placed: &[Placement2<V, U>]) -> Vec<QuantityPoint2<V, U>> {
        let mut points: Vec<QuantityPoint2<V, U>> = Vec::new();
        let push_unique = |points: &mut Vec<QuantityPoint2<V, U>>, point: QuantityPoint2<V, U>| {
            let duplicate = points.iter().any(|existing| {
                existing.x.value == point.x.value && existing.y.value == point.y.value
            });
            if !duplicate {
                points.push(point);
            }
        };

        for placement in placed {
            let (Ok(max_x), Ok(max_y)) = (placement.max_x(), placement.max_y()) else {
                continue;
            };
            push_unique(
                &mut points,
                QuantityPoint2 {
                    x: placement.x(),
                    y: max_y.clone(),
                },
            );
            push_unique(
                &mut points,
                QuantityPoint2 {
                    x: max_x.clone(),
                    y: placement.y(),
                },
            );
            if self.config.with_displacement() {
                for point in self.displaced_points(placement, &max_x, &max_y, placed) {
                    push_unique(&mut points, point);
                }
            }
        }

        let origin_x = quantity_zero_of(&self.sheet.width);
        let origin_y = quantity_zero_of(&self.sheet.height);
        if points.is_empty() {
            return vec![QuantityPoint2 {
                x: origin_x,
                y: origin_y,
            }];
        }

        // 仅保留落在板材内的候选点；全部越界时退回原点。
        // Keep only candidate points inside the sheet; fall back to the origin when all are outside.
        let inside: Vec<_> = points
            .into_iter()
            .filter(|point| {
                quantity_greater_or_equal(&point.x, &origin_x)
                    && quantity_greater_or_equal(&point.y, &origin_y)
                    && quantity_less_or_equal(&point.x, &self.sheet.width)
                    && quantity_less_or_equal(&point.y, &self.sheet.height)
            })
            .collect();
        let mut usable = if inside.is_empty() {
            vec![QuantityPoint2 {
                x: origin_x,
                y: origin_y,
            }]
        } else {
            inside
        };
        usable.sort_by(|lhs, rhs| {
            compare_placement_position(lhs, rhs, self.config.with_weighted_distance)
                .unwrap_or(CmpOrdering::Equal)
        });
        usable
    }

    /// 由既有放置沿 X/Y 轴滑动得到额外候选点 / Derive extra candidate positions by sliding along X and Y
    fn displaced_points(
        &self,
        placement: &Placement2<V, U>,
        max_x: &Quantity<V, U>,
        max_y: &Quantity<V, U>,
        placed: &[Placement2<V, U>],
    ) -> Vec<QuantityPoint2<V, U>> {
        let mut points = Vec::new();
        if self.config.with_displacement_x {
            points.push(QuantityPoint2 {
                x: self.point_on_x(placement, placed),
                y: max_y.clone(),
            });
        }
        if self.config.with_displacement_y {
            points.push(QuantityPoint2 {
                x: max_x.clone(),
                y: self.point_on_y(placement, placed),
            });
        }
        points
    }

    /// 沿 X 轴向左滑动到最近的可行位置 / Slide left along X to the nearest feasible position
    fn point_on_x(
        &self,
        placement: &Placement2<V, U>,
        placed: &[Placement2<V, U>],
    ) -> Quantity<V, U> {
        let placement_max_y = placement.max_y().unwrap_or_else(|_| placement.y());
        let mut best = quantity_zero_of(&placement.x());
        for other in placed {
            if std::ptr::eq(other, placement) {
                continue;
            }
            let (Ok(other_max_x), Ok(other_max_y)) = (other.max_x(), other.max_y()) else {
                continue;
            };
            // 仅考虑在 Y 方向与本放置重叠、且完全位于其左侧的放置。
            // Only consider placements overlapping in Y and lying fully to the left.
            let overlaps_in_y =
                quantity_less(&other.y(), &placement_max_y) && quantity_greater(&other_max_y, &placement.y());
            let lies_left = quantity_less_or_equal(&other_max_x, &placement.x());
            if overlaps_in_y && lies_left && quantity_greater(&other_max_x, &best) {
                best = other_max_x;
            }
        }
        best
    }

    /// 沿 Y 轴向下滑动到最近的可行位置 / Slide down along Y to the nearest feasible position
    fn point_on_y(
        &self,
        placement: &Placement2<V, U>,
        placed: &[Placement2<V, U>],
    ) -> Quantity<V, U> {
        let mut best = quantity_zero_of(&placement.y());
        let placement_max_x = placement.max_x().unwrap_or_else(|_| placement.x());
        for other in placed {
            if std::ptr::eq(other, placement) {
                continue;
            }
            let (Ok(other_max_x), Ok(other_max_y)) = (other.max_x(), other.max_y()) else {
                continue;
            };
            // 仅考虑在 X 方向与本放置重叠、且完全位于其下方的放置。
            // Only consider placements overlapping in X and lying fully below.
            let overlaps_in_x =
                quantity_less(&other.x(), &placement_max_x) && quantity_greater(&other_max_x, &placement.x());
            let lies_below = quantity_less_or_equal(&other_max_y, &placement.y());
            if overlaps_in_x && lies_below && quantity_greater(&other_max_y, &best) {
                best = other_max_y;
            }
        }
        best
    }

    /// 判断放置是否可行：在板材内且与既有放置不重叠 / Whether a placement is feasible: inside the sheet and
    /// not overlapping existing placements
    fn is_feasible(&self, placement: &Placement2<V, U>, placed: &[Placement2<V, U>]) -> bool {
        let space = crate::domain::block_loading::model::Space2::new(
            quantity_zero_of(&self.sheet.width),
            quantity_zero_of(&self.sheet.height),
            self.sheet.width.clone(),
            self.sheet.height.clone(),
        );
        if !placement_inside_space(placement, &space) {
            return false;
        }
        for other in placed {
            // 刀缝双向检查：任一侧膨胀后相交，说明真实间隙小于 kerf。
            // Kerf is checked in both directions: an intersection after inflating either side means the real
            // gap is below the kerf.
            let overlapped = match &self.kerf {
                Some(kerf) if quantity_is_positive(kerf) => {
                    let forward = Self::box_of(placement, Some(kerf))
                        .map(|inflated| inflated.overlapped(&Self::box_of(other, None).unwrap_or_else(|| {
                            Box2Geometry::of_rectangle(
                                other.x(),
                                other.y(),
                                other.width(),
                                other.height(),
                            )
                        })))
                        .unwrap_or(false);
                    if forward {
                        true
                    } else {
                        let backward = Self::box_of(placement, None)
                            .map(|plain| {
                                inflated_by_shape(&other.shape, kerf);
                                plain.overlapped(
                                    &Self::box_of(other, Some(kerf)).unwrap_or_else(|| {
                                        Box2Geometry::of_rectangle(
                                            other.x(),
                                            other.y(),
                                            other.width(),
                                            other.height(),
                                        )
                                    }),
                                )
                            })
                            .unwrap_or(false);
                        backward
                    }
                }
                _ => Self::box_of(placement, None)
                    .zip(Self::box_of(other, None))
                    .map(|(lhs, rhs)| lhs.overlapped(&rhs))
                    .unwrap_or(false),
            };
            if overlapped {
                return false;
            }
        }
        true
    }

    /// 构造放置的包围盒，可选按刀缝膨胀 / Build a placement's bounding box, optionally inflated by the kerf
    fn box_of(
        placement: &Placement2<V, U>,
        kerf: Option<&Quantity<V, U>>,
    ) -> Option<Box2Geometry<V, U>> {
        let shape = match kerf {
            Some(kerf) => inflated_by_shape(&placement.shape, kerf),
            None => placement.shape.clone(),
        };
        Some(Box2Geometry::new(
            placement.x(),
            placement.y(),
            shape,
        ))
    }
}
