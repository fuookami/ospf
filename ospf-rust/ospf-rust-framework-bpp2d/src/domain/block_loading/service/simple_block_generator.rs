//! 简单块生成器 / Simple block generator
//!
//! 与 BPP3D 的 `SimpleBlockGenerator` 对应：把同一种物品按「整行重复」的方式铺满一个矩形区域，形成
//! 规则块。二维侧的块是矩形条带，可直接作为候选层的基础。
//! Mirrors BPP3D `SimpleBlockGenerator`: fills a rectangular region with repeated whole rows of a single
//! item, forming a regular block. In 2D a block is a rectangular strip usable as the basis of a candidate
//! layer.

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Placement2, QuantityPoint2, candidate_shapes, count_fitting, pitch_of, quantity_is_positive,
    quantity_zero_of, scaled_quantity,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::block_loading::model::{Block2, Space2};
use crate::domain::item::model::ActualItem;

// ============================================================================
// SimpleBlockConfig - 简单块配置 / Simple block configuration
// ============================================================================

/// 简单块生成器配置 / Simple block generator configuration
#[derive(Debug, Clone)]
pub struct SimpleBlockConfig {
    /// 是否同时生成旋转排列 / Whether rotated layouts are also generated
    pub with_rotation: bool,
    /// 是否在剩余空间继续生成块 / Whether blocks continue to be generated in the remainder
    pub with_remainder: bool,
}

impl Default for SimpleBlockConfig {
    fn default() -> Self {
        Self {
            with_rotation: true,
            with_remainder: false,
        }
    }
}

// ============================================================================
// SimpleBlockGenerator - 简单块生成器 / Simple block generator
// ============================================================================

/// 简单块生成器 / Simple block generator
#[derive(Debug, Clone, Default)]
pub struct SimpleBlockGenerator;

impl SimpleBlockGenerator {
    /// 创建生成器 / Create the generator
    pub fn new() -> Self {
        Self
    }

    /// 为单个物品在给定区域生成规则块 / Generate regular blocks for a single item inside the region
    ///
    /// 圆形件不参与规则块（无整行重复语义），会被跳过并返回空列表，而不是产出非法块。
    /// 旋转候选同时受 `config.with_rotation` 与物品自身的 `allow_rotate` 约束：Kotlin 只检查前者，
    /// 这会让不允许旋转的物品产出旋转块，因此这里收紧为两者同时成立。
    /// Circle items do not take part in regular blocks (no whole-row repetition semantics); they are skipped
    /// and an empty list is returned instead of an illegal block. Rotated candidates require both
    /// `config.with_rotation` and the item's own `allow_rotate`: Kotlin checks only the former, which lets an
    /// item that disallows rotation yield rotated blocks, so this is tightened to require both.
    pub fn invoke<V, U>(
        &self,
        item: &ActualItem<V, U>,
        space: &Space2<V, U>,
        config: &SimpleBlockConfig,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<Vec<Block2<V, U>>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        if space.is_degenerate() || item.shape.is_circle() {
            return Ok(Vec::new());
        }
        let mut blocks = Vec::new();
        for candidate in candidate_shapes(&item.shape, config.with_rotation && item.allow_rotate) {
            let shape = &candidate.shape;
            // 刀缝通过节距体现：相邻件按「包围尺寸 + kerf」推进。
            // The kerf is expressed through the pitch: neighbours advance by bounding size plus kerf.
            let spacing = match kerf {
                Some(kerf) if quantity_is_positive(kerf) => kerf.clone(),
                _ => quantity_zero_of(&shape.bounding_width()),
            };
            let (pitch_x, pitch_y) = pitch_of(shape, &spacing);
            let cols = count_fitting(&pitch_x, &space.width);
            let rows = count_fitting(&pitch_y, &space.height);
            if cols == 0 || rows == 0 {
                continue;
            }
            let block_width = scaled_quantity(&pitch_x, cols);
            let block_height = scaled_quantity(&pitch_y, rows);
            let mut placements = Vec::with_capacity(cols * rows);
            for row in 0..rows {
                let y = offset(&space.y, &pitch_y, row);
                for col in 0..cols {
                    let x = offset(&space.x, &pitch_x, col);
                    placements.push(
                        Placement2::new(
                            shape.clone(),
                            QuantityPoint2 { x, y: y.clone() },
                        )
                        .with_rotated(candidate.rotated)
                        .with_item_id(item.id.as_str()),
                    );
                }
            }
            blocks.push(Block2::new(
                Space2::new(
                    space.x.clone(),
                    space.y.clone(),
                    block_width.clone(),
                    block_height,
                ),
                placements,
                if candidate.rotated {
                    "simple-block-rotated"
                } else {
                    "simple-block"
                },
            ));
            if !config.with_remainder {
                continue;
            }
            let Some((_, remaining_right)) = space.split_x(&block_width) else {
                continue;
            };
            if !remaining_right.is_degenerate() {
                blocks.extend(self.invoke(item, &remaining_right, config, kerf)?);
            }
        }
        Ok(blocks)
    }
}

/// 按节距整数倍偏移 / Offset by an integer multiple of the pitch
fn offset<V, U>(base: &Quantity<V, U>, pitch: &Quantity<V, U>, times: usize) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + Clone,
{
    let step = scaled_quantity(pitch, times);
    crate::domain::block_loading::model::add_quantity(base, &step)
}
