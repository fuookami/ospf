//! 空间切分搜索算法 / Space-splitting search algorithms
//!
//! 与 BPP3D 的 `DepthFirstSearchAlgorithm` / `MultiLayerHeuristicSearchAlgorithm` 对应，裁剪为二维：
//! 通过递归切分剩余空间逐块放置物品。两者都不使用异常表达失败，返回放置结果。
//! Mirrors BPP3D `DepthFirstSearchAlgorithm` / `MultiLayerHeuristicSearchAlgorithm` reduced to 2D: items are
//! placed block by block by recursively splitting the remaining space. Neither uses exceptions for failure;
//! both return the layout result.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Placement2, QuantityPoint2, candidate_shapes, count_fitting, pitch_of, quantity_is_positive,
    quantity_less_or_equal, quantity_zero_of, scaled_quantity,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::block_loading::model::{Space2, add_quantity, subtract_quantity};
use crate::domain::item::model::ActualItem;

// ============================================================================
// SpaceSplit2Result - 空间切分搜索结果 / Space-splitting search result
// ============================================================================

/// 空间切分搜索结果 / Space-splitting search result
#[derive(Debug, Clone)]
pub struct SpaceSplit2Result<V, U: UnitTrait> {
    /// 放置列表 / Placement list
    pub placements: Vec<Placement2<V, U>>,
    /// 各物品已放置件数 / Placed amount per item
    pub placed_amounts: BTreeMap<String, usize>,
    /// 各物品剩余件数 / Remaining amount per item
    pub rest_amounts: BTreeMap<String, usize>,
    /// 算法来源标记 / Algorithm source marker
    pub source: String,
}

/// 由剩余件数推导已放置件数 / Derive placed amounts from the remaining ones
fn placed_amounts_of<V, U: UnitTrait>(
    items: &[(ActualItem<V, U>, usize)],
    rest: &BTreeMap<String, usize>,
) -> BTreeMap<String, usize> {
    let mut placed = BTreeMap::new();
    for (item, amount) in items {
        let left = rest.get(item.id.as_str()).copied().unwrap_or(0);
        if left < *amount {
            placed.insert(item.id.as_str().to_string(), amount - left);
        }
    }
    placed
}

/// 由物品与件数构造剩余件数表 / Build the remaining-amount table from items and amounts
fn initial_rest<V, U: UnitTrait>(items: &[(ActualItem<V, U>, usize)]) -> BTreeMap<String, usize> {
    let mut rest = BTreeMap::new();
    for (item, amount) in items {
        rest.insert(item.id.as_str().to_string(), *amount);
    }
    rest
}

/// 只保留仍为正的剩余件数 / Keep only positive remaining amounts
fn positive_rest(rest: &BTreeMap<String, usize>) -> BTreeMap<String, usize> {
    rest.iter()
        .filter(|(_, amount)| **amount > 0)
        .map(|(key, amount)| (key.clone(), *amount))
        .collect()
}

// ============================================================================
// DepthFirstSearchAlgorithm2D - 深度优先空间切分 / Depth-first space split
// ============================================================================

/// 深度优先空间切分算法 / Depth-first space-splitting algorithm
///
/// 每次取剩余区域，放入包围面积最大的可放物品，然后沿放置块的外边界切分剩余空间并递归。
/// Each step takes the remaining region, places the largest-placeable item, then splits the remainder along
/// the block boundary and recurses.
#[derive(Debug, Clone, Default)]
pub struct DepthFirstSearchAlgorithm2D;

impl DepthFirstSearchAlgorithm2D {
    /// 创建算法 / Create the algorithm
    pub fn new() -> Self {
        Self
    }

    /// 执行深度优先空间切分 / Run the depth-first space split
    pub fn invoke<V, U>(
        &self,
        space: &Space2<V, U>,
        items: &[(ActualItem<V, U>, usize)],
        max_depth: usize,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<SpaceSplit2Result<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let mut placements = Vec::new();
        let mut rest = initial_rest(items);
        self.descend(space, items, &mut rest, &mut placements, 0, max_depth, kerf);
        Ok(SpaceSplit2Result {
            placements,
            placed_amounts: placed_amounts_of(items, &rest),
            rest_amounts: positive_rest(&rest),
            source: "dfs".to_string(),
        })
    }

    /// 递归放置 / Recursive placement step
    #[allow(clippy::too_many_arguments)]
    fn descend<V, U>(
        &self,
        space: &Space2<V, U>,
        items: &[(ActualItem<V, U>, usize)],
        rest: &mut BTreeMap<String, usize>,
        placements: &mut Vec<Placement2<V, U>>,
        depth: usize,
        max_depth: usize,
        kerf: Option<&Quantity<V, U>>,
    ) where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        if depth >= max_depth || space.is_degenerate() {
            return;
        }
        // 选剩余需求未满足且包围面积最大的物品。
        // Pick the item with the largest bounding area among those still demanded.
        let pick = items
            .iter()
            .filter(|(item, _)| {
                rest.get(item.id.as_str()).copied().unwrap_or(0) > 0 && !item.shape.is_circle()
            })
            .max_by(|(lhs, _), (rhs, _)| {
                lhs.shape
                    .bounding_area()
                    .value
                    .to_f64()
                    .unwrap_or(0.0)
                    .partial_cmp(&rhs.shape.bounding_area().value.to_f64().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let Some((item, _)) = pick else {
            return;
        };

        let candidate = candidate_shapes(&item.shape, item.allow_rotate)
            .into_iter()
            .filter(|candidate| space.enabled(&candidate.shape))
            .max_by(|lhs, rhs| {
                lhs.shape
                    .bounding_width()
                    .value
                    .partial_cmp(&rhs.shape.bounding_width().value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let Some(candidate) = candidate else {
            return;
        };
        let shape = candidate.shape.clone();
        // 刀缝通过节距体现。 / The kerf is expressed through the pitch.
        let spacing = match kerf {
            Some(kerf) if quantity_is_positive(kerf) => kerf.clone(),
            _ => quantity_zero_of(&shape.bounding_width()),
        };
        let (pitch_x, pitch_y) = pitch_of(&shape, &spacing);
        let cols = count_fitting(&pitch_x, &space.width);
        let rows = count_fitting(&pitch_y, &space.height);
        if cols == 0 || rows == 0 {
            return;
        }

        let mut remaining = rest.get(item.id.as_str()).copied().unwrap_or(0);
        let mut placed_cols = 0usize;
        let mut placed_rows = 0usize;
        'outer: for row in 0..rows {
            for col in 0..cols {
                if remaining == 0 {
                    break 'outer;
                }
                let x = add_quantity(&space.x, &scaled_quantity(&pitch_x, col));
                let y = add_quantity(&space.y, &scaled_quantity(&pitch_y, row));
                placements.push(
                    Placement2::new(shape.clone(), QuantityPoint2 { x, y })
                        .with_rotated(candidate.rotated)
                        .with_item_id(item.id.as_str()),
                );
                remaining -= 1;
                placed_cols = placed_cols.max(col + 1);
                placed_rows = placed_rows.max(row + 1);
            }
        }
        rest.insert(item.id.as_str().to_string(), remaining);

        let used_width = scaled_quantity(&pitch_x, placed_cols);
        let used_height = scaled_quantity(&pitch_y, placed_rows);
        if let Some((_, right_space)) = space.split_x(&used_width) {
            self.descend(&right_space, items, rest, placements, depth + 1, max_depth, kerf);
        }
        // 顶部空间宽度取已用宽度：与 Kotlin 一致，避免与右侧空间重复覆盖。
        // The top space takes the used width, matching Kotlin and avoiding double coverage with the right space.
        if let (Some(top_y), Some(top_height)) = (
            add_quantity(&space.y, &used_height).into(),
            subtract_quantity(&space.height, &used_height),
        ) {
            let top_space = Space2::new(
                space.x.clone(),
                top_y,
                used_width,
                top_height,
            );
            self.descend(&top_space, items, rest, placements, depth + 1, max_depth, kerf);
        }
    }
}

// ============================================================================
// MultiRowHeuristicAlgorithm2D - 多行启发式 / Multi-row heuristic
// ============================================================================

/// 一行的候选选择结果 / Candidate choice for one row
#[derive(Debug, Clone)]
struct RowPick<V, U: UnitTrait> {
    item_id: String,
    shape: ospf_rust_framework_geometry2d::PackingShape2<V, U>,
    rotated: bool,
    cols: usize,
    /// 横向节距 / Horizontal pitch
    pitch_x: Quantity<V, U>,
    /// 纵向节距 / Vertical pitch
    pitch_y: Quantity<V, U>,
}

/// 多行启发式搜索算法 / Multi-row heuristic search algorithm
///
/// 按行推进：每行选择能放入剩余宽度中数量最多的物品，放满一行后按行高向上推进。
/// Advances row by row: each row picks the item fitting the most copies into the remaining width, then moves
/// up by that row's height.
#[derive(Debug, Clone, Default)]
pub struct MultiRowHeuristicAlgorithm2D;

impl MultiRowHeuristicAlgorithm2D {
    /// 创建算法 / Create the algorithm
    pub fn new() -> Self {
        Self
    }

    /// 执行多行启发式搜索 / Run the multi-row heuristic search
    pub fn invoke<V, U>(
        &self,
        space: &Space2<V, U>,
        items: &[(ActualItem<V, U>, usize)],
        max_rows: usize,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<SpaceSplit2Result<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let mut placements = Vec::new();
        let mut rest = initial_rest(items);
        let mut cursor_y = space.y.clone();
        let mut rows = 0usize;

        while rows < max_rows {
            let Some(remaining_height) = space.remaining_height_from(&cursor_y) else {
                break;
            };
            if !quantity_is_positive(&remaining_height) {
                break;
            }
            let pick = items
                .iter()
                .filter(|(item, _)| {
                    rest.get(item.id.as_str()).copied().unwrap_or(0) > 0 && !item.shape.is_circle()
                })
                .filter_map(|(item, _)| {
                    let candidate = candidate_shapes(&item.shape, item.allow_rotate)
                        .into_iter()
                        .filter(|candidate| space.enabled(&candidate.shape))
                        .filter(|candidate| {
                            quantity_less_or_equal(
                                &candidate.shape.bounding_height(),
                                &remaining_height,
                            )
                        })
                        .max_by(|lhs, rhs| {
                            lhs.shape
                                .bounding_width()
                                .value
                                .partial_cmp(&rhs.shape.bounding_width().value)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })?;
                    let spacing = match kerf {
                        Some(kerf) if quantity_is_positive(kerf) => kerf.clone(),
                        _ => quantity_zero_of(&candidate.shape.bounding_width()),
                    };
                    let (pitch_x, pitch_y) = pitch_of(&candidate.shape, &spacing);
                    let cols = count_fitting(&pitch_x, &space.width);
                    if cols == 0 {
                        return None;
                    }
                    Some(RowPick {
                        item_id: item.id.as_str().to_string(),
                        shape: candidate.shape.clone(),
                        rotated: candidate.rotated,
                        cols,
                        pitch_x,
                        pitch_y,
                    })
                })
                .max_by_key(|pick| pick.cols);
            let Some(pick) = pick else {
                break;
            };

            let mut remaining = rest.get(&pick.item_id).copied().unwrap_or(0);
            let mut placed = 0usize;
            for col in 0..pick.cols {
                if remaining == 0 {
                    break;
                }
                let x = add_quantity(&space.x, &scaled_quantity(&pick.pitch_x, col));
                placements.push(
                    Placement2::new(pick.shape.clone(), QuantityPoint2 {
                        x,
                        y: cursor_y.clone(),
                    })
                    .with_rotated(pick.rotated)
                    .with_item_id(pick.item_id.clone()),
                );
                remaining -= 1;
                placed += 1;
            }
            rest.insert(pick.item_id.clone(), remaining);
            if placed == 0 {
                break;
            }
            cursor_y = add_quantity(&cursor_y, &pick.pitch_y);
            rows += 1;
        }

        Ok(SpaceSplit2Result {
            placements,
            placed_amounts: placed_amounts_of(items, &rest),
            rest_amounts: positive_rest(&rest),
            source: "multi-row".to_string(),
        })
    }
}
