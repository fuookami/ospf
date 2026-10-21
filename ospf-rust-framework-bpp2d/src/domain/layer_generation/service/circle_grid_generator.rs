//! 圆形件网格生成器 / Circle grid generator
//!
//! 与 BPP3D 的 circle-packing 策略对应：圆形件不做通用密排优化，而是生成两种规则网格候选——
//! 直角网格（按直径铺格）与六角网格（行距约 `√3/2·d`、奇偶行错半格）。生成后仍用底层几何逐对复核，
//! 复核不通过的候选直接丢弃，不做近似。
//! Mirrors the BPP3D circle-packing strategy: circle items are not optimised by a general dense-packing
//! solver; instead two regular grid candidates are generated — a square grid (cells of one diameter) and a
//! hexagonal grid (row pitch about `√3/2·d` with alternate rows offset by half a cell). Every candidate is
//! then re-validated pairwise against the underlying geometry, and failures are discarded rather than
//! approximated.
//!
//! # 六角行距为什么取 26/30 / Why the hexagonal pitch is 26/30
//!
//! 行距取 `26/30·d`，略大于 `√3/2 ≈ 0.866025`。取偏大是**保守**的：行与行之间比理论值更远，
//! 因此不会产生重叠，同时仍比直角网格更密。这样既避免了对泛型数值类型求平方根，也保持了正确性。
//! The pitch uses `26/30·d`, slightly larger than `√3/2 ≈ 0.866025`. Choosing a larger pitch is
//! **conservative**: rows are farther apart than the theoretical value, so overlap cannot occur while density
//! still beats the square grid. This avoids square roots on a generic numeric type without losing
//! correctness.

use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, QuantityVector2, count_fitting, quantity_greater,
    quantity_is_positive, quantity_zero_of, scaled_quantity,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::model::{ActualItem, SheetType};
use crate::domain::layer_generation::request::add_quantity;

/// 六角网格的最大行数，避免病态输入造成的长时间循环 / Maximum hexagonal rows, guarding pathological input
const MAX_HEX_ROWS: usize = 10_000;

/// 六角行距分子：26 / Hexagonal pitch numerator: 26
const HEX_PITCH_NUMERATOR: usize = 26;

/// 六角行距分母：30（26/30 略大于 √3/2，保守且无重叠）
/// / Hexagonal pitch denominator: 30 (26/30 slightly exceeds √3/2; conservative and overlap-free)
const HEX_PITCH_DENOMINATOR: usize = 30;

// ============================================================================
// CircleGridMode - 圆形网格布局模式 / Circle grid layout mode
// ============================================================================

/// 圆形网格布局模式 / Circle grid layout mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircleGridMode {
    /// 直角网格，按直径铺格 / Square grid using one diameter per cell
    SquareGrid,
    /// 六角网格，行距约 `√3/2·d` 且错行 / Hexagonal grid with a row pitch of about `√3/2·d` and offset rows
    HexGrid,
}

impl CircleGridMode {
    /// 稳定的来源标记 / Stable source marker
    pub fn source(&self) -> &'static str {
        match self {
            Self::SquareGrid => "circle-rect-grid",
            Self::HexGrid => "circle-hex-grid",
        }
    }
}

// ============================================================================
// CircleGridLayout - 圆形网格布局结果 / Circle grid layout result
// ============================================================================

/// 圆形件网格布局结果 / Circle grid layout result
#[derive(Debug, Clone)]
pub struct CircleGridLayout<V, U: UnitTrait> {
    /// 放置列表 / Placement list
    pub placements: Vec<Placement2<V, U>>,
    /// 布局模式 / Layout mode
    pub mode: CircleGridMode,
    /// 物品编号 / Item identifier
    pub item_id: String,
}

// ============================================================================
// CircleGridGenerator - 圆形件网格生成器 / Circle grid generator
// ============================================================================

/// 圆形件网格生成器 / Circle grid generator
#[derive(Debug, Clone, Default)]
pub struct CircleGridGenerator;

impl CircleGridGenerator {
    /// 创建生成器 / Create the generator
    pub fn new() -> Self {
        Self
    }

    /// 为单个圆形件生成全部网格候选 / Generate every grid candidate for a single circle item
    ///
    /// 非圆形件直接返回空列表；结果按件数降序排列。
    /// Non-circle items return an empty list; results are ordered by descending amount.
    pub fn invoke<V, U>(
        &self,
        item: &ActualItem<V, U>,
        sheet: &SheetType<V, U>,
        kerf: Option<&Quantity<V, U>>,
    ) -> Vec<CircleGridLayout<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let PackingShape2::Circle(circle) = &item.shape else {
            return Vec::new();
        };
        let diameter = circle.diameter();
        if !quantity_is_positive(&diameter) {
            return Vec::new();
        }
        // 圆形件的刀缝同样通过节距体现：节距 = 直径 + kerf。
        // For circles the kerf is also expressed through the pitch: pitch = diameter + kerf.
        let spacing = match kerf {
            Some(kerf) if quantity_is_positive(kerf) => kerf.clone(),
            _ => quantity_zero_of(&diameter),
        };
        let mut layouts = Vec::new();
        if let Some(layout) = self.square_grid(item, &diameter, &spacing, sheet) {
            layouts.push(layout);
        }
        if let Some(layout) = self.hex_grid(item, &diameter, &spacing, sheet) {
            layouts.push(layout);
        }
        layouts.sort_by(|lhs, rhs| rhs.placements.len().cmp(&lhs.placements.len()));
        layouts
    }

    /// 生成直角网格布局 / Generate the square grid layout
    fn square_grid<V, U>(
        &self,
        item: &ActualItem<V, U>,
        diameter: &Quantity<V, U>,
        spacing: &Quantity<V, U>,
        sheet: &SheetType<V, U>,
    ) -> Option<CircleGridLayout<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let pitch = add_quantity(diameter, spacing);
        if !quantity_is_positive(&pitch) {
            return None;
        }
        let cols = count_fitting(&pitch, &sheet.width);
        let rows = count_fitting(&pitch, &sheet.height);
        if cols == 0 || rows == 0 {
            return None;
        }
        let mut placements = Vec::with_capacity(cols * rows);
        for row in 0..rows {
            let y = scaled_quantity(&pitch, row);
            for col in 0..cols {
                let x = scaled_quantity(&pitch, col);
                placements.push(
                    Placement2::new(item.shape.clone(), QuantityPoint2 { x, y: y.clone() })
                        .with_item_id(item.id.as_str()),
                );
            }
        }
        Some(CircleGridLayout {
            placements,
            mode: CircleGridMode::SquareGrid,
            item_id: item.id.as_str().to_string(),
        })
    }

    /// 生成六角网格布局 / Generate the hexagonal grid layout
    ///
    /// 行距与列距都由「直径 + kerf」推导，保证任意相邻件的真实间隙不小于 kerf。
    /// Both the row and column pitch derive from diameter plus kerf, so every pair of neighbours keeps a real
    /// gap of at least the kerf.
    fn hex_grid<V, U>(
        &self,
        item: &ActualItem<V, U>,
        diameter: &Quantity<V, U>,
        spacing: &Quantity<V, U>,
        sheet: &SheetType<V, U>,
    ) -> Option<CircleGridLayout<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let pitch = add_quantity(diameter, spacing);
        if !quantity_is_positive(&pitch) {
            return None;
        }
        let row_pitch = hex_row_pitch(&pitch)?;
        if !quantity_is_positive(&row_pitch) {
            return None;
        }
        let half_pitch = Quantity::new(
            pitch.value.clone() / two_of::<V>(),
            pitch.unit.clone(),
        );

        let mut placements = Vec::new();
        let mut row = 0usize;
        while row < MAX_HEX_ROWS {
            let y = scaled_quantity(&row_pitch, row);
            // 本行最底端加上一个直径仍超出板材高度时停止，避免产出越界放置。
            // Stop when this row's bottom plus one diameter exceeds the sheet height, so no placement can
            // fall outside the sheet.
            if quantity_greater(&add_quantity(&y, &pitch), &sheet.height) {
                break;
            }
            // 奇数行整体右移半个节距，与偶数行交错，使斜向相邻的圆心距离不小于直径。
            // Odd rows shift right by half a pitch, interleaving with even rows so that diagonally adjacent
            // circle centres stay at least one diameter apart.
            let row_offset = if row % 2 == 0 {
                quantity_zero_of(&pitch)
            } else {
                half_pitch.clone()
            };
            let available_width =
                crate::domain::layer_generation::request::subtract_quantity(&sheet.width, &row_offset)
                    .unwrap_or_else(|| quantity_zero_of(&pitch));
            let cols = count_fitting(&pitch, &available_width);
            for col in 0..cols {
                let x = add_quantity(&row_offset, &scaled_quantity(&pitch, col));
                placements.push(
                    Placement2::new(item.shape.clone(), QuantityPoint2 { x, y: y.clone() })
                        .with_item_id(item.id.as_str()),
                );
            }
            row += 1;
        }
        if placements.is_empty() {
            return None;
        }
        Some(CircleGridLayout {
            placements,
            mode: CircleGridMode::HexGrid,
            item_id: item.id.as_str().to_string(),
        })
    }
}

/// 平移一组放置 / Translate a set of placements
pub fn translate_placements<V, U>(
    placements: &[Placement2<V, U>],
    offset: &QuantityVector2<V, U>,
) -> Vec<Placement2<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    placements
        .iter()
        .map(|placement| {
            Placement2::new(
                placement.shape.clone(),
                QuantityPoint2 {
                    x: add_quantity(&placement.x(), &offset.x),
                    y: add_quantity(&placement.y(), &offset.y),
                },
            )
            .with_rotated(placement.rotated)
            .with_item_id_ref(placement.item_id.clone())
        })
        .collect()
}

/// 计算六角网格行距 `26/30·d` / Compute the hexagonal row pitch `26/30·d`
///
/// 以整数比 `26/30` 缩放**节距量**本身，得到 `0.8667·d` 的行距；分母非正时返回 `None`。
/// 注意：不能写成「26d / 30d」，那会得到一个无量纲的 0.8667，只在 `d = 1` 时碰巧正确。
/// Scales the **pitch quantity** by the integer ratio `26/30`, yielding a row pitch of `0.8667·d`; returns
/// `None` when the denominator is non-positive. Note: it must not be written as "26d / 30d", which produces
/// the dimensionless 0.8667 and is only accidentally correct when `d = 1`.
fn hex_row_pitch<V, U>(diameter: &Quantity<V, U>) -> Option<Quantity<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + Clone,
{
    if !quantity_is_positive(diameter) {
        return None;
    }
    let numerator = scaled_quantity(diameter, HEX_PITCH_NUMERATOR);
    let denominator = V::one() + {
        let mut acc = V::zero();
        for _ in 0..(HEX_PITCH_DENOMINATOR - 1) {
            acc = acc + V::one();
        }
        acc
    };
    Some(Quantity::new(
        numerator.value / denominator,
        numerator.unit,
    ))
}

/// 构造数值 `2` / Build the value `2`
fn two_of<V>() -> V
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
{
    V::one() + V::one()
}
