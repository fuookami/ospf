//! 装箱几何守卫 / Packing geometry guard
//!
//! 校验放置集合的纯几何约束：全部位于板材内且两两不重叠。所有函数返回 `Result`，
//! 业务错误以 `Err` 表达，不做 panic。
//! Validates the pure geometric constraints of a placement list: all inside the sheet and pairwise
//! disjoint. Every function returns a `Result`; business errors surface as `Err`, never as a panic.
//!
//! # 刀缝双向检查 / Double-sided kerf check
//!
//! 只有把**任一侧**膨胀后仍相交，才说明真实间隙小于刀缝。单向膨胀会漏掉另一侧的间隙不足。
//! Only when **either side** is inflated and the shapes still intersect is the real gap below the kerf.
//! Inflating one side only misses a narrow gap on the other side.

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Box2Geometry, PackingShape2, Placement2, Sheet2Shape, inflated_by_shape, inside_sheet,
    origin_box, quantity_is_positive,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::validation_error;

// ============================================================================
// 旋转合法性规则 / Rotation legality rule
// ============================================================================

/// 旋转合法性规则 / Rotation legality rule
///
/// 几何层无法判断「物品是否允许旋转」，因此由调用方以本规则注入该知识。
/// The geometry layer cannot know whether an item allows rotation, so callers inject that knowledge.
pub enum PackingRotationRule {
    /// 始终允许旋转：调用方已在上游校验过 / Always allow rotation; the caller validated it upstream
    AllowAll,
    /// 仅允许旋转不变的形状（圆形件、正方形件）/ Allow only rotation-invariant shapes
    SymmetricOnly,
    /// 按物品编号判断，缺失视为允许 / Decide by item identifier; missing items are allowed
    ByItemCatalog(std::collections::BTreeMap<String, bool>),
}

impl PackingRotationRule {
    /// 判断单个放置的旋转是否合法 / Whether the rotation of one placement is legal
    pub fn allow<V, U>(&self, placement: &Placement2<V, U>) -> bool
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        if !placement.rotated {
            return true;
        }
        if is_rotation_invariant(&placement.shape) {
            return true;
        }
        match self {
            Self::AllowAll => true,
            Self::SymmetricOnly => false,
            Self::ByItemCatalog(catalog) => placement
                .item_id
                .as_ref()
                .and_then(|id| catalog.get(id).copied())
                .unwrap_or(true),
        }
    }
}

impl std::fmt::Debug for PackingRotationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllowAll => f.write_str("PackingRotationRule::AllowAll"),
            Self::SymmetricOnly => f.write_str("PackingRotationRule::SymmetricOnly"),
            Self::ByItemCatalog(catalog) => f
                .debug_tuple("PackingRotationRule::ByItemCatalog")
                .field(catalog)
                .finish(),
        }
    }
}

/// 形状是否旋转不变 / Whether the shape is rotation invariant
///
/// 圆形件与正方形件的包围宽高相等，旋转不改变占用空间。
/// A circle or a square has equal bounding width and height, so rotation does not change the footprint.
pub fn is_rotation_invariant<V, U>(shape: &PackingShape2<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    shape.bounding_width().value == shape.bounding_height().value
}

// ============================================================================
// 装箱几何守卫 / Packing geometry guard
// ============================================================================

/// 装箱几何守卫 / Packing geometry guard
#[derive(Debug, Clone, Default)]
pub struct PackingGeometryGuard;

impl PackingGeometryGuard {
    /// 校验放置集合的纯几何约束 / Validate the pure geometric constraints of a placement list
    ///
    /// 依次检查：板材内、旋转合法、两两不重叠（含刀缝双向膨胀）。
    /// Checks, in order: inside the sheet, legal rotation, and pairwise non-overlap with double-sided kerf
    /// inflation.
    pub fn require_sheet_geometry<V, U>(
        sheet: &Sheet2Shape<V, U>,
        placements: &[Placement2<V, U>],
        sheet_name: &str,
        source: &str,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<()>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        Self::require_sheet_geometry_with_rule(
            sheet,
            placements,
            sheet_name,
            source,
            kerf,
            &PackingRotationRule::AllowAll,
        )
    }

    /// 带旋转规则的几何校验 / Geometric validation with an explicit rotation rule
    #[allow(clippy::too_many_arguments)]
    pub fn require_sheet_geometry_with_rule<V, U>(
        sheet: &Sheet2Shape<V, U>,
        placements: &[Placement2<V, U>],
        sheet_name: &str,
        source: &str,
        kerf: Option<&Quantity<V, U>>,
        rotation_rule: &PackingRotationRule,
    ) -> Ret<()>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let sheet_box = origin_box(sheet.width.clone(), sheet.height.clone());
        let mixed_diagnostic = mixed_shape_diagnostic(placements);

        for (index, placement) in placements.iter().enumerate() {
            let placement_box =
                Box2Geometry::new(placement.x(), placement.y(), placement.shape.clone());
            if !inside_sheet(&placement_box, &sheet_box) {
                return Err(validation_error(format!(
                    "{} 在 {} 的第 {} 个放置越出板材（{}）/ placement {} falls outside sheet {} ({}); {}",
                    source,
                    sheet_name,
                    index,
                    mixed_diagnostic,
                    index,
                    sheet_name,
                    placement_shape_diagnostic(placement),
                    mixed_diagnostic
                )));
            }
            if !rotation_rule.allow(placement) {
                return Err(validation_error(format!(
                    "{} 在 {} 的第 {} 个放置使用了未允许的旋转（{}）/ placement {} in sheet {} uses a rotation the item does not allow ({})",
                    source,
                    sheet_name,
                    index,
                    placement_shape_diagnostic(placement),
                    index,
                    sheet_name,
                    placement_shape_diagnostic(placement)
                )));
            }
        }

        for lhs_index in 0..placements.len() {
            for rhs_index in (lhs_index + 1)..placements.len() {
                if Self::overlapped_with_kerf(
                    &placements[lhs_index],
                    &placements[rhs_index],
                    kerf,
                ) {
                    return Err(validation_error(format!(
                        "{} 在 {} 的第 {} 与第 {} 个放置重叠（{}；{}）/ placements {} and {} in sheet {} overlap ({}; {})",
                        source,
                        sheet_name,
                        lhs_index,
                        rhs_index,
                        placement_shape_diagnostic(&placements[lhs_index]),
                        placement_shape_diagnostic(&placements[rhs_index]),
                        lhs_index,
                        rhs_index,
                        sheet_name,
                        placement_shape_diagnostic(&placements[lhs_index]),
                        placement_shape_diagnostic(&placements[rhs_index])
                    )));
                }
            }
        }
        Ok(())
    }

    /// 两个放置是否重叠（含刀缝双向膨胀）/ Whether two placements overlap, with double-sided kerf inflation
    pub fn overlapped_with_kerf<V, U>(
        lhs: &Placement2<V, U>,
        rhs: &Placement2<V, U>,
        kerf: Option<&Quantity<V, U>>,
    ) -> bool
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        match kerf {
            Some(kerf) if quantity_is_positive(kerf) => {
                // 先把左侧膨胀再比较；仍不重叠时再把右侧膨胀比较。
                // Inflate the left side first; if that does not intersect, inflate the right side instead.
                let forward = Self::box_of(lhs, Some(kerf))
                    .map(|left| left.overlapped(&Self::box_of(rhs, None).unwrap_or_else(|| {
                        Box2Geometry::new(rhs.x(), rhs.y(), rhs.shape.clone())
                    })))
                    .unwrap_or(false);
                if forward {
                    return true;
                }
                Self::box_of(lhs, None)
                    .map(|left| {
                        left.overlapped(&Self::box_of(rhs, Some(kerf)).unwrap_or_else(|| {
                            Box2Geometry::new(rhs.x(), rhs.y(), rhs.shape.clone())
                        }))
                    })
                    .unwrap_or(false)
            }
            _ => {
                let left = Box2Geometry::new(lhs.x(), lhs.y(), lhs.shape.clone());
                let right = Box2Geometry::new(rhs.x(), rhs.y(), rhs.shape.clone());
                left.overlapped(&right)
            }
        }
    }

    /// 构造放置的包围盒，可选按刀缝膨胀 / Build a placement's bounding box, optionally inflated by the kerf
    fn box_of<V, U>(
        placement: &Placement2<V, U>,
        kerf: Option<&Quantity<V, U>>,
    ) -> Option<Box2Geometry<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let shape = match kerf {
            Some(kerf) => inflated_by_shape(&placement.shape, kerf),
            None => placement.shape.clone(),
        };
        Some(Box2Geometry::new(placement.x(), placement.y(), shape))
    }
}

// ============================================================================
// 诊断 / Diagnostics
// ============================================================================

/// 放置形状诊断 / Placement shape diagnostic
pub fn placement_shape_diagnostic<V, U>(placement: &Placement2<V, U>) -> String
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    format!(
        "shape={:?}, rotated={}, w={:?}, h={:?}",
        placement.shape.shape_type(),
        placement.rotated,
        placement.shape.bounding_width().value,
        placement.shape.bounding_height().value
    )
}

/// 混合形状诊断 / Mixed shape diagnostic
///
/// 同时存在圆形与矩形时给出提示，便于区分「几何本身非法」与「该生成器不支撑混合形状」。
/// When circles and rectangles coexist, a hint is emitted so that "geometrically illegal" can be told apart
/// from "this generator does not support mixed shapes".
pub fn mixed_shape_diagnostic<V, U>(placements: &[Placement2<V, U>]) -> String
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let mut circles = 0usize;
    let mut rectangles = 0usize;
    for placement in placements {
        if placement.shape.is_circle() {
            circles += 1;
        } else {
            rectangles += 1;
        }
    }
    if circles > 0 && rectangles > 0 {
        format!("mixed-shape(circle={circles}, rectangle={rectangles})")
    } else {
        String::new()
    }
}
