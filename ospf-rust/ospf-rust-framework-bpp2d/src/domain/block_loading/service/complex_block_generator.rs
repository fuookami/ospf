//! 复杂块生成器 / Complex block generator
//!
//! 与 BPP3D 的 `ComplexBlockGenerator` 对应：把尺寸不同的多种物品按行拼接为异构块，块内每行使用同一
//! 物品，行高取该行物品的包围高度。二维侧不做支撑校验（平面排样无堆叠语义）。
//! Mirrors BPP3D `ComplexBlockGenerator`: stitches different items into a heterogeneous block row by row,
//! each row using one item and taking that item's bounding height. 2D needs no support validation because
//! plane layout has no stacking semantics.

use std::collections::BTreeSet;

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Placement2, QuantityPoint2, candidate_shapes, count_fitting, pitch_of, quantity_is_positive,
    quantity_less_or_equal, quantity_max_of, quantity_zero_of, scaled_quantity,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::block_loading::model::{Block2, Space2, add_quantity};
use crate::domain::item::model::ActualItem;

/// 复杂块生成器 / Complex block generator
#[derive(Debug, Clone, Default)]
pub struct ComplexBlockGenerator;

impl ComplexBlockGenerator {
    /// 创建生成器 / Create the generator
    pub fn new() -> Self {
        Self
    }

    /// 按给定物品顺序逐行拼接异构块 / Stitch a heterogeneous block row by row in the given item order
    ///
    /// 每行选一个物品，取在该区域内「包围宽度最大」的可旋转候选，铺满整行后按节距向上的下一行继续。
    /// 圆形件不参与（无整行重复语义）。
    /// Each row picks one item and its widest rotatable candidate fitting the region, fills the row, then
    /// advances by the pitch. Circle items do not take part (no whole-row repetition semantics).
    pub fn invoke<V, U>(
        &self,
        items: &[ActualItem<V, U>],
        space: &Space2<V, U>,
        with_rotation: bool,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<Vec<Block2<V, U>>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        if space.is_degenerate() || items.is_empty() {
            return Ok(Vec::new());
        }
        let mut placements = Vec::new();
        let mut sources: BTreeSet<String> = BTreeSet::new();
        let mut cursor_y = space.y.clone();
        let mut used_width = quantity_zero_of(&space.width);

        for item in items {
            if item.shape.is_circle() {
                continue;
            }
            let Some(remaining_height) = space.remaining_height_from(&cursor_y) else {
                break;
            };
            if !quantity_is_positive(&remaining_height) {
                break;
            }
            // 选在该区域内可放、且行高不超过剩余高度的最宽候选。
            // Pick the widest candidate that fits the region and whose height fits the remaining height.
            let candidate = candidate_shapes(&item.shape, with_rotation)
                .into_iter()
                .filter(|candidate| space.enabled(&candidate.shape))
                .filter(|candidate| {
                    quantity_less_or_equal(&candidate.shape.bounding_height(), &remaining_height)
                })
                .max_by(|lhs, rhs| {
                    lhs.shape
                        .bounding_width()
                        .value
                        .partial_cmp(&rhs.shape.bounding_width().value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            let Some(candidate) = candidate else {
                continue;
            };

            let shape = candidate.shape.clone();
            // 刀缝通过节距体现。 / The kerf is expressed through the pitch.
            let spacing = match kerf {
                Some(kerf) if quantity_is_positive(kerf) => kerf.clone(),
                _ => quantity_zero_of(&shape.bounding_width()),
            };
            let (pitch_x, pitch_y) = pitch_of(&shape, &spacing);
            let cols = count_fitting(&pitch_x, &space.width);
            if cols == 0 {
                continue;
            }
            let row_width = scaled_quantity(&pitch_x, cols);
            for col in 0..cols {
                let x = add_quantity(&space.x, &scaled_quantity(&pitch_x, col));
                placements.push(
                    Placement2::new(
                        shape.clone(),
                        QuantityPoint2 {
                            x,
                            y: cursor_y.clone(),
                        },
                    )
                    .with_rotated(candidate.rotated)
                    .with_item_id(item.id.as_str()),
                );
            }
            used_width = quantity_max_of(used_width, row_width);
            sources.insert(if candidate.rotated {
                "complex-block-rotated".to_string()
            } else {
                "complex-block".to_string()
            });
            cursor_y = add_quantity(&cursor_y, &pitch_y);
        }

        if placements.is_empty() {
            return Ok(Vec::new());
        }
        let Some(height) = crate::domain::block_loading::model::subtract_quantity(&cursor_y, &space.y)
        else {
            return Ok(Vec::new());
        };
        let block = Block2::new(
            Space2::new(
                space.x.clone(),
                space.y.clone(),
                used_width,
                height,
            ),
            placements,
            sources.into_iter().collect::<Vec<_>>().join("+"),
        );
        Ok(vec![block])
    }
}
