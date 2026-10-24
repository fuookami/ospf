//! 二维空间区域与块 / 2D space region and block
//!
//! 对应 BPP3D 的 `Space`，裁剪为二维：一个矩形空区域，可按「沿 X 切分」或「沿 Y 切分」递归拆分为
//! 剩余空间，供块生成与空间切分搜索使用。
//! Mirrors BPP3D `Space` reduced to 2D: a rectangular free region that can be recursively split along X or
//! Y into remainder spaces for block generation and space-splitting search.

use std::collections::BTreeMap;

use ospf_rust_framework_geometry2d::{
    Placement2, PackingShape2, quantity_greater_or_equal, quantity_less_or_equal,
    quantity_is_non_positive, quantity_is_positive, quantity_multiply, quantity_zero_of,
    quantity_sum, quantity_plus_safe, quantity_minus_safe,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

// ============================================================================
// Space2 - 二维空间区域 / 2D space region
// ============================================================================

/// 二维空间区域 / 2D space region
#[derive(Debug, Clone)]
pub struct Space2<V, U: UnitTrait> {
    /// 左下角 x 坐标 / Lower-left x coordinate
    pub x: Quantity<V, U>,
    /// 左下角 y 坐标 / Lower-left y coordinate
    pub y: Quantity<V, U>,
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
}

impl<V, U> Space2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建空间区域 / Create a space region
    pub fn new(
        x: Quantity<V, U>,
        y: Quantity<V, U>,
        width: Quantity<V, U>,
        height: Quantity<V, U>,
    ) -> Self {
        Self { x, y, width, height }
    }

    /// 区域面积 / Region area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        quantity_multiply(&self.width, &self.height)
    }

    /// 判断给定包围尺寸能否放入本区域 / Whether a bounding size fits the region
    pub fn enabled_by_size(&self, box_width: &Quantity<V, U>, box_height: &Quantity<V, U>) -> bool {
        quantity_less_or_equal(box_width, &self.width)
            && quantity_less_or_equal(box_height, &self.height)
    }

    /// 判断给定装载形状能否放入本区域 / Whether a packing shape fits the region
    pub fn enabled(&self, shape: &PackingShape2<V, U>) -> bool {
        self.enabled_by_size(&shape.bounding_width(), &shape.bounding_height())
    }

    /// 沿 X 方向切出左侧一块，返回左侧块与剩余区域 / Cut a left-hand piece along X and return the piece plus
    /// the remainder
    pub fn split_x(&self, cut_width: &Quantity<V, U>) -> Option<(Self, Self)> {
        let right_x = quantity_plus_safe(&self.x, cut_width).ok()?;
        let right_width = quantity_minus_safe(&self.width, cut_width).ok()?;
        let left = Self::new(
            self.x.clone(),
            self.y.clone(),
            cut_width.clone(),
            self.height.clone(),
        );
        let right = Self::new(right_x, self.y.clone(), right_width, self.height.clone());
        Some((left, right))
    }

    /// 沿 Y 方向切出底部一块，返回底部块与剩余区域 / Cut a bottom piece along Y and return the piece plus the
    /// remainder
    pub fn split_y(&self, cut_height: &Quantity<V, U>) -> Option<(Self, Self)> {
        let top_y = quantity_plus_safe(&self.y, cut_height).ok()?;
        let top_height = quantity_minus_safe(&self.height, cut_height).ok()?;
        let bottom = Self::new(
            self.x.clone(),
            self.y.clone(),
            self.width.clone(),
            cut_height.clone(),
        );
        let top = Self::new(self.x.clone(), top_y, self.width.clone(), top_height);
        Some((bottom, top))
    }

    /// 区域是否已退化（宽或高非正）/ Whether the region is degenerate (non-positive width or height)
    pub fn is_degenerate(&self) -> bool {
        quantity_is_non_positive(&self.width) || quantity_is_non_positive(&self.height)
    }

    /// 右边界 x 坐标 / Right boundary x coordinate
    pub fn max_x(&self) -> Option<Quantity<V, U>> {
        quantity_plus_safe(&self.x, &self.width).ok()
    }

    /// 上边界 y 坐标 / Upper boundary y coordinate
    pub fn max_y(&self) -> Option<Quantity<V, U>> {
        quantity_plus_safe(&self.y, &self.height).ok()
    }

    /// 剩余高度：从给定 y 到区域上边界 / Remaining height from the given y to the upper boundary
    pub fn remaining_height_from(&self, cursor_y: &Quantity<V, U>) -> Option<Quantity<V, U>> {
        let max_y = self.max_y()?;
        quantity_minus_safe(&max_y, cursor_y).ok()
    }

    /// 原点起算的零宽空间 / A zero-width space anchored at the given sample
    pub fn zero_width_of(&self) -> Quantity<V, U> {
        quantity_zero_of(&self.width)
    }

    /// 坐标系原点 / The coordinate origin
    pub fn origin_point(&self) -> (Quantity<V, U>, Quantity<V, U>) {
        (quantity_zero_of(&self.x), quantity_zero_of(&self.y))
    }
}

// ============================================================================
// Block2 - 块 / Block
// ============================================================================

/// 块：一组放置连同其包围区域 / Block: a group of placements together with their bounding region
#[derive(Debug, Clone)]
pub struct Block2<V, U: UnitTrait> {
    /// 块的包围区域 / Bounding region of the block
    pub space: Space2<V, U>,
    /// 块内放置 / Placements inside the block
    pub placements: Vec<Placement2<V, U>>,
    /// 生成来源 / Generation source
    pub source: String,
}

impl<V, U> Block2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建块 / Create a block
    pub fn new(space: Space2<V, U>, placements: Vec<Placement2<V, U>>, source: impl Into<String>) -> Self {
        Self {
            space,
            placements,
            source: source.into(),
        }
    }

    /// 块内件数 / Amount inside the block
    pub fn amount(&self) -> usize {
        self.placements.len()
    }

    /// 块内实际占用面积 / Actually occupied area inside the block
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let areas: Vec<_> = self
            .placements
            .iter()
            .map(|placement| placement.area())
            .collect();
        quantity_sum(areas).unwrap_or_else(|| quantity_zero_of(&self.space.width).to_runtime())
    }

    /// 块的利用率 / Block utilisation
    ///
    /// 包围区域面积非正时返回 0，避免除零。
    /// Returns 0 when the bounding area is non-positive, avoiding division by zero.
    pub fn utilization(&self) -> f64 {
        let space_area = self.space.area().value.to_f64().unwrap_or(0.0);
        if space_area <= 0.0 {
            return 0.0;
        }
        self.area().value.to_f64().unwrap_or(0.0) / space_area
    }

    /// 块内件数映射 / Item amount map inside the block
    pub fn item_amounts(&self) -> BTreeMap<String, usize> {
        let mut counter: BTreeMap<String, usize> = BTreeMap::new();
        for placement in &self.placements {
            if let Some(item_id) = &placement.item_id {
                *counter.entry(item_id.clone()).or_insert(0) += 1;
            }
        }
        counter
    }
}

/// 空间边界与放置边界的包含判定 / Whether a placement lies inside a space region
///
/// 使用闭区间：贴边属于在内，与「相切不算重叠」并不冲突。
/// Uses closed intervals: touching the boundary counts as inside, which does not conflict with
/// "tangency is not overlap".
pub fn placement_inside_space<V, U>(
    placement: &Placement2<V, U>,
    space: &Space2<V, U>,
) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let (Some(max_x), Some(max_y)) = (placement.max_x().ok(), placement.max_y().ok()) else {
        return false;
    };
    let (Some(space_max_x), Some(space_max_y)) = (space.max_x(), space.max_y()) else {
        return false;
    };
    quantity_greater_or_equal(&placement.x(), &space.x)
        && quantity_greater_or_equal(&placement.y(), &space.y)
        && quantity_less_or_equal(&max_x, &space_max_x)
        && quantity_less_or_equal(&max_y, &space_max_y)
}

/// 区域是否为正面积 / Whether the region has a positive area
pub fn space_is_usable<V, U>(space: &Space2<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    quantity_is_positive(&space.width) && quantity_is_positive(&space.height)
}
