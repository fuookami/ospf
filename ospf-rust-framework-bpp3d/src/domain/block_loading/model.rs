//! 块装载模型 / Block loading models
//!
//! 定义块、空间和物品视图等核心域模型。
//! Defines core domain models including blocks, spaces, and item views.

use std::fmt::Debug;
use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::PackingShape3;

// ============================================================================
// ItemView - 物品视图 / Item view
// ============================================================================

/// 物品视图 / Item view
///
/// 物品在特定朝向下的尺寸视图，用于块生成。
/// A view of an item under a specific orientation, used for block generation.
#[derive(Debug, Clone)]
pub struct ItemView<V, U: UnitTrait> {
    /// 原始物品索引 / Original item index
    pub item_index: usize,
    /// 朝向 / Orientation
    pub orientation: Orientation,
    /// 视图宽度 / View width
    pub width: Quantity<V, U>,
    /// 视图高度 / View height
    pub height: Quantity<V, U>,
    /// 视图深度 / View depth
    pub depth: Quantity<V, U>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 包装形状 / Packing shape
    pub packing_shape: PackingShape3<V, U>,
}

// ============================================================================
// Block - 块 / Block
// ============================================================================

/// 块放置 / Block placement
///
/// 一个块在容器中的位置信息。
/// Position information of a block in a container.
#[derive(Debug, Clone)]
pub struct BlockPlacement<V, U: UnitTrait> {
    /// 放置位置 / Placement position
    pub position: MetricPoint3<V, U>,
    /// 块 / Block reference index
    pub block_index: usize,
}

/// 简单块 / Simple block
///
/// 由同一物品同一朝向组成的均匀块。
/// A homogeneous block composed of the same item in the same orientation.
#[derive(Debug, Clone)]
pub struct SimpleBlock<V, U: UnitTrait> {
    /// 物品视图 / Item view
    pub item_view: ItemView<V, U>,
    /// X 方向数量 / X direction count
    pub nx: u64,
    /// Y 方向数量 / Y direction count
    pub ny: u64,
    /// Z 方向数量 / Z direction count
    pub nz: u64,
    /// 总物品数 / Total item count
    pub item_count: u64,
    /// 块宽度 / Block width
    pub width: Quantity<V, U>,
    /// 块高度 / Block height
    pub height: Quantity<V, U>,
    /// 块深度 / Block depth
    pub depth: Quantity<V, U>,
    /// 块重量 / Block weight
    pub weight: Quantity<V, U>,
}

impl<V, U> SimpleBlock<V, U>
where
    V: num_traits::Float + Debug + Clone + Send + Sync,
    U: CTUnit + Default + Clone,
{
    /// 从物品视图和方向数量创建简单块 / Create simple block from item view and direction counts
    pub fn from_item_view(item_view: ItemView<V, U>, nx: u64, ny: u64, nz: u64) -> Self {
        let item_count = nx * ny * nz;
        let nx_v = V::from(nx).unwrap();
        let ny_v = V::from(ny).unwrap();
        let nz_v = V::from(nz).unwrap();
        let count_v = V::from(item_count).unwrap();

        let width = Quantity::new_ct(item_view.width.value * nx_v);
        let height = Quantity::new_ct(item_view.height.value * ny_v);
        let depth = Quantity::new_ct(item_view.depth.value * nz_v);
        let weight = Quantity::new_ct(item_view.weight.value * count_v);

        Self {
            item_view,
            nx,
            ny,
            nz,
            item_count,
            width,
            height,
            depth,
            weight,
        }
    }

    /// 块尺寸 / Block size
    pub fn size(&self) -> MetricSize3<V, U> {
        MetricSize3 {
            width: self.width.clone(),
            height: self.height.clone(),
            depth: self.depth.clone(),
        }
    }
}

/// 复杂块 / Complex block
///
/// 由多个简单块组合而成的复合块。
/// A composite block composed of multiple simple blocks.
#[derive(Debug, Clone)]
pub struct ComplexBlock<V, U: UnitTrait> {
    /// 子块放置 / Sub-block placements
    pub blocks: Vec<BlockPlacement<V, U>>,
    /// 子块列表 / Sub-block list
    pub sub_blocks: Vec<SimpleBlock<V, U>>,
    /// 合并轴 / Merge axis
    pub merge_axis: Axis3,
    /// 总宽度 / Total width
    pub width: Quantity<V, U>,
    /// 总高度 / Total height
    pub height: Quantity<V, U>,
    /// 总深度 / Total depth
    pub depth: Quantity<V, U>,
    /// 总重量 / Total weight
    pub weight: Quantity<V, U>,
}

/// 块枚举 / Block enum
///
/// 统一简单块和复杂块。
/// Unifies simple blocks and complex blocks.
#[derive(Debug, Clone)]
pub enum Block<V, U: UnitTrait> {
    /// 简单块 / Simple block
    Simple(SimpleBlock<V, U>),
    /// 复杂块 / Complex block
    Complex(ComplexBlock<V, U>),
}

impl<V, U> Block<V, U>
where
    U: UnitTrait,
{
    /// 块宽度 / Block width
    pub fn width(&self) -> &Quantity<V, U> {
        match self {
            Block::Simple(b) => &b.width,
            Block::Complex(b) => &b.width,
        }
    }

    /// 块高度 / Block height
    pub fn height(&self) -> &Quantity<V, U> {
        match self {
            Block::Simple(b) => &b.height,
            Block::Complex(b) => &b.height,
        }
    }

    /// 块深度 / Block depth
    pub fn depth(&self) -> &Quantity<V, U> {
        match self {
            Block::Simple(b) => &b.depth,
            Block::Complex(b) => &b.depth,
        }
    }

    /// 块重量 / Block weight
    pub fn weight(&self) -> &Quantity<V, U> {
        match self {
            Block::Simple(b) => &b.weight,
            Block::Complex(b) => &b.weight,
        }
    }
}

// ============================================================================
// Space - 空间 / Space
// ============================================================================

/// 空间 / Space
///
/// 容器中可供放置块的剩余空间。
/// Remaining space in a container available for block placement.
#[derive(Debug, Clone)]
pub struct Space<V, U: UnitTrait> {
    /// 空间位置 / Space position
    pub position: MetricPoint3<V, U>,
    /// 空间尺寸 / Space dimensions
    pub size: MetricSize3<V, U>,
}

impl<V, U> Space<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    /// 创建空间 / Create a space
    pub fn new(position: MetricPoint3<V, U>, size: MetricSize3<V, U>) -> Self {
        Self { position, size }
    }

    /// 检查块是否适合此空间 / Check if a block fits in this space
    pub fn fits(&self, block: &Block<V, U>) -> bool {
        block.width().value <= self.size.width.value
            && block.height().value <= self.size.height.value
            && block.depth().value <= self.size.depth.value
    }

    /// 放置块后的剩余子空间 / Remaining sub-spaces after placing a block
    ///
    /// 简化的三空间分解：右侧、上方、前方。
    /// Simplified three-space decomposition: right, top, front.
    pub fn place_block(&self, block: &Block<V, U>) -> Vec<Space<V, U>> {
        let mut spaces = Vec::new();

        let bw = block.width().value.clone();
        let bh = block.height().value.clone();
        let bd = block.depth().value.clone();
        let sw = self.size.width.value.clone();
        let sh = self.size.height.value.clone();
        let sd = self.size.depth.value.clone();
        let px = self.position.x.value.clone();
        let py = self.position.y.value.clone();
        let pz = self.position.z.value.clone();

        // 右侧空间 (X+)
        if bw < sw {
            spaces.push(Space::new(
                MetricPoint3 {
                    x: Quantity::new_ct(px.clone() + bw.clone()),
                    y: Quantity::new_ct(py.clone()),
                    z: Quantity::new_ct(pz.clone()),
                },
                MetricSize3 {
                    width: Quantity::new_ct(sw.clone() - bw.clone()),
                    height: Quantity::new_ct(sh.clone()),
                    depth: Quantity::new_ct(sd.clone()),
                },
            ));
        }

        // 上方空间 (Y+)
        if bh < sh {
            spaces.push(Space::new(
                MetricPoint3 {
                    x: Quantity::new_ct(px.clone()),
                    y: Quantity::new_ct(py.clone() + bh.clone()),
                    z: Quantity::new_ct(pz.clone()),
                },
                MetricSize3 {
                    width: Quantity::new_ct(sw.clone()),
                    height: Quantity::new_ct(sh.clone() - bh.clone()),
                    depth: Quantity::new_ct(sd.clone()),
                },
            ));
        }

        // 前方空间 (Z+)
        if bd < sd {
            spaces.push(Space::new(
                MetricPoint3 {
                    x: Quantity::new_ct(px),
                    y: Quantity::new_ct(py),
                    z: Quantity::new_ct(pz + bd.clone()),
                },
                MetricSize3 {
                    width: Quantity::new_ct(sw),
                    height: Quantity::new_ct(sh),
                    depth: Quantity::new_ct(sd - bd),
                },
            ));
        }

        spaces
    }
}

// ============================================================================
// BlockLoadingContext - 块装载上下文 / Block loading context
// ============================================================================

/// 块装载上下文 / Block loading context
///
/// 管理块生成和装载的过程。
/// Manages the block generation and loading process.
#[derive(Debug, Clone, Default)]
pub struct BlockLoadingContext;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::packing_shape::cuboid_packing_shape;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn simple_block_from_item_view() {
        let item_view = ItemView {
            item_index: 0,
            orientation: Orientation::Upright,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            packing_shape: cuboid_packing_shape(meters(2.0), meters(3.0), meters(4.0), meters(1.0)),
        };

        let block = SimpleBlock::from_item_view(item_view, 2, 3, 1);
        assert_eq!(block.nx, 2);
        assert_eq!(block.ny, 3);
        assert_eq!(block.nz, 1);
        assert_eq!(block.item_count, 6);
        assert_eq!(block.width.value, 4.0);  // 2 * 2.0
        assert_eq!(block.height.value, 9.0); // 3 * 3.0
        assert_eq!(block.depth.value, 4.0);  // 1 * 4.0
        assert_eq!(block.weight.value, 6.0); // 6 * 1.0
    }

    #[test]
    fn block_enum_accessors() {
        let item_view = ItemView {
            item_index: 0,
            orientation: Orientation::Upright,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            packing_shape: cuboid_packing_shape(meters(2.0), meters(3.0), meters(4.0), meters(1.0)),
        };

        let block = Block::Simple(SimpleBlock::from_item_view(item_view, 1, 1, 1));
        assert_eq!(block.width().value, 2.0);
        assert_eq!(block.height().value, 3.0);
        assert_eq!(block.depth().value, 4.0);
        assert_eq!(block.weight().value, 1.0);
    }

    #[test]
    fn space_fits_block() {
        let space = Space::new(
            MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            MetricSize3 { width: meters(10.0), height: meters(10.0), depth: meters(10.0) },
        );

        let item_view = ItemView {
            item_index: 0,
            orientation: Orientation::Upright,
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            weight: meters(1.0),
            packing_shape: cuboid_packing_shape(meters(5.0), meters(5.0), meters(5.0), meters(1.0)),
        };
        let block = Block::Simple(SimpleBlock::from_item_view(item_view, 1, 1, 1));
        assert!(space.fits(&block));
    }

    #[test]
    fn space_does_not_fit_block() {
        let space = Space::new(
            MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            MetricSize3 { width: meters(3.0), height: meters(10.0), depth: meters(10.0) },
        );

        let item_view = ItemView {
            item_index: 0,
            orientation: Orientation::Upright,
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            weight: meters(1.0),
            packing_shape: cuboid_packing_shape(meters(5.0), meters(5.0), meters(5.0), meters(1.0)),
        };
        let block = Block::Simple(SimpleBlock::from_item_view(item_view, 1, 1, 1));
        assert!(!space.fits(&block));
    }

    #[test]
    fn space_place_block_subspaces() {
        let space = Space::new(
            MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            MetricSize3 { width: meters(10.0), height: meters(10.0), depth: meters(10.0) },
        );

        let item_view = ItemView {
            item_index: 0,
            orientation: Orientation::Upright,
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            weight: meters(1.0),
            packing_shape: cuboid_packing_shape(meters(5.0), meters(5.0), meters(5.0), meters(1.0)),
        };
        let block = Block::Simple(SimpleBlock::from_item_view(item_view, 1, 1, 1));
        let sub_spaces = space.place_block(&block);

        // 应产生 3 个子空间：右侧、上方、前方
        assert_eq!(sub_spaces.len(), 3);

        // 右侧空间
        assert_eq!(sub_spaces[0].position.x.value, 5.0);
        assert_eq!(sub_spaces[0].size.width.value, 5.0);

        // 上方空间
        assert_eq!(sub_spaces[1].position.y.value, 5.0);
        assert_eq!(sub_spaces[1].size.height.value, 5.0);

        // 前方空间
        assert_eq!(sub_spaces[2].position.z.value, 5.0);
        assert_eq!(sub_spaces[2].size.depth.value, 5.0);
    }

    #[test]
    fn space_place_block_exact_fit() {
        let space = Space::new(
            MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            MetricSize3 { width: meters(5.0), height: meters(5.0), depth: meters(5.0) },
        );

        let item_view = ItemView {
            item_index: 0,
            orientation: Orientation::Upright,
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            weight: meters(1.0),
            packing_shape: cuboid_packing_shape(meters(5.0), meters(5.0), meters(5.0), meters(1.0)),
        };
        let block = Block::Simple(SimpleBlock::from_item_view(item_view, 1, 1, 1));
        let sub_spaces = space.place_block(&block);

        // 精确匹配不产生子空间
        assert!(sub_spaces.is_empty());
    }
}
