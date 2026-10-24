//! 余料分析器 / Free space analyser
//!
//! 按水平切片扣减已用区域，输出互不重叠的自由矩形。圆形件使用外接正方形，保守但安全。
//! Subtracts the used regions by horizontal slices and emits mutually disjoint free rectangles. Circle items
//! use their enclosing square, which is conservative but safe.
//!
//! # 范围 / Scope
//!
//! 这是**余料报告**，不含跨工单余料库存与优先级复用。
//! This is a **remnant report**; cross-order remnant inventory and priority reuse are out of scope.

use ospf_rust_framework_geometry2d::{Box2Geometry, Placement2, origin_box};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::model::SheetType;

// ============================================================================
// Remnant2 - 余料矩形 / Remnant rectangle
// ============================================================================

/// 板材上的一块剩余矩形 / A remaining rectangle on a sheet
#[derive(Debug, Clone)]
pub struct Remnant2<V, U: UnitTrait> {
    /// 板材名称 / Sheet name
    pub sheet_name: String,
    /// 自由矩形 / Free rectangle
    pub rectangle: Box2Geometry<V, U>,
    /// 自由矩形面积 / Free rectangle area
    pub area: Quantity<V, ospf_rust_quantities::unit::Unit>,
}

// ============================================================================
// FreeSpaceAnalyzer - 余料分析器 / Remnant analyser
// ============================================================================

/// 余料分析器 / Remnant analyser
#[derive(Debug, Clone, Default)]
pub struct FreeSpaceAnalyzer;

impl FreeSpaceAnalyzer {
    /// 创建分析器 / Create the analyser
    pub fn new() -> Self {
        Self
    }

    /// 分析单块板材的余料 / Analyse the remnants of a single sheet
    ///
    /// 无放置时整块板材即一块余料；否则按放置的上边界做水平切片，每片内扣减已用宽度区间。
    /// Without placements the whole sheet is one remnant; otherwise the sheet is sliced horizontally at every
    /// placement's upper boundary and the used x-intervals are subtracted within each slice.
    pub fn analyze_sheet<V, U>(
        &self,
        name: &str,
        sheet: &SheetType<V, U>,
        placements: &[Placement2<V, U>],
    ) -> Vec<Remnant2<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let width = sheet.width.value.to_f64().unwrap_or(0.0);
        let height = sheet.height.value.to_f64().unwrap_or(0.0);
        if width <= 0.0 || height <= 0.0 {
            return Vec::new();
        }

        if placements.is_empty() {
            let whole = origin_box(sheet.width.clone(), sheet.height.clone());
            return vec![Remnant2 {
                sheet_name: name.to_string(),
                area: whole.width().to_runtime() * whole.height().to_runtime(),
                rectangle: whole,
            }];
        }

        // 收集水平切分点：板材底、所有放置的上下边界、板材顶。
        // Collect the horizontal cut points: sheet bottom, every placement's bottom and top, sheet top.
        let mut cuts: Vec<f64> = vec![0.0, height];
        for placement in placements {
            let Some(bottom) = placement.y().value.to_f64() else {
                continue;
            };
            let Some(top) = placement.max_y().ok().and_then(|top| top.value.to_f64()) else {
                continue;
            };
            cuts.push(bottom.clamp(0.0, height));
            cuts.push(top.clamp(0.0, height));
        }
        cuts.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
        cuts.dedup_by(|lhs, rhs| (*lhs - *rhs).abs() < 1e-9);

        let mut remnants = Vec::new();
        for window in cuts.windows(2) {
            let bottom = window[0];
            let top = window[1];
            if top - bottom <= 1e-9 {
                continue;
            }
            // 与本水平切片相交的放置，按左边界排序后扣减其 x 区间。
            // Placements intersecting this slice, ordered by their left edge, with their x-interval subtracted.
            let mut used: Vec<(f64, f64)> = placements
                .iter()
                .filter_map(|placement| {
                    let place_bottom = placement.y().value.to_f64()?;
                    let place_top = placement.max_y().ok()?.value.to_f64()?;
                    if place_top <= bottom + 1e-9 || place_bottom >= top - 1e-9 {
                        return None;
                    }
                    let left = placement.x().value.to_f64()?;
                    let right = placement.max_x().ok()?.value.to_f64()?;
                    Some((left.clamp(0.0, width), right.clamp(0.0, width)))
                })
                .collect();
            used.sort_by(|lhs, rhs| lhs.0.partial_cmp(&rhs.0).unwrap_or(std::cmp::Ordering::Equal));

            let mut cursor = 0.0f64;
            let mut free_intervals: Vec<(f64, f64)> = Vec::new();
            for (left, right) in used {
                if left > cursor + 1e-9 {
                    free_intervals.push((cursor, left));
                }
                cursor = cursor.max(right);
            }
            if cursor < width - 1e-9 {
                free_intervals.push((cursor, width));
            }

            for (left, right) in free_intervals {
                let free_width = right - left;
                if free_width <= 1e-9 {
                    continue;
                }
                let rectangle = Box2Geometry::of_rectangle(
                    Quantity::new(V::from(left).unwrap_or_else(V::zero), sheet.width.unit.clone()),
                    Quantity::new(V::from(bottom).unwrap_or_else(V::zero), sheet.height.unit.clone()),
                    Quantity::new(V::from(free_width).unwrap_or_else(V::zero), sheet.width.unit.clone()),
                    Quantity::new(V::from(top - bottom).unwrap_or_else(V::zero), sheet.height.unit.clone()),
                );
                remnants.push(Remnant2 {
                    sheet_name: name.to_string(),
                    area: rectangle.width().to_runtime() * rectangle.height().to_runtime(),
                    rectangle,
                });
            }
        }
        // 按面积降序，方便调用方优先复用大块余料。
        // Sorted by descending area so callers can reuse the largest remnants first.
        remnants.sort_by(|lhs, rhs| {
            rhs.area
                .value
                .to_f64()
                .unwrap_or(0.0)
                .partial_cmp(&lhs.area.value.to_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        remnants
    }

    /// 汇总余料统计：数量与总面积 / Aggregate remnant statistics: count and total area
    pub fn summarize<V, U>(remnants: &[Remnant2<V, U>]) -> (usize, f64)
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let total: f64 = remnants
            .iter()
            .map(|remnant| remnant.area.value.to_f64().unwrap_or(0.0))
            .sum();
        (remnants.len(), total)
    }
}
