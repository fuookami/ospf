//! 货架式图案生成器 / Shelf pattern generator
//!
//! 按「行」推进：每行选一种产品，按行高铺满宽度，行高取该行产品的包围高度。这是二维套裁最常用的
//! 构造式启发式，既支持任意排样，也能直接产出断头台切分树（先切条、再切件）。
//! Advances row by row: each row picks one product, fills the width at that product's bounding height. This is
//! the most common constructive heuristic for 2D cutting stock; it supports arbitrary nesting and can also emit
//! a guillotine cut tree directly (strips, then pieces).

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Placement2, QuantityPoint2, count_fitting, quantity_is_positive, quantity_less_or_equal,
    quantity_zero_of,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::material::{Material2, Pattern2, Product2};
use crate::domain::pattern_generation::request::PatternGenerationRequest2;
use crate::infrastructure::cut_tree::{CutLine2, CutTree2};
use crate::infrastructure::cutting_process::CuttingProcess2;
use crate::infrastructure::error::validation_error;
use crate::infrastructure::region::{CutOrientation2, Region2};

/// 一行的选择结果 / The choice made for one row
#[derive(Debug, Clone)]
struct RowPick<V, U: UnitTrait> {
    product_id: String,
    width: Quantity<V, U>,
    height: Quantity<V, U>,
    cols: usize,
    rotated: bool,
}

/// 货架式图案生成器 / Shelf pattern generator
#[derive(Debug, Clone, Default)]
pub struct ShelfPatternGenerator2;

impl ShelfPatternGenerator2 {
    /// 创建生成器 / Create the generator
    pub fn new() -> Self {
        Self
    }

    /// 生成候选图案 / Generate candidate patterns
    ///
    /// 每个候选对应「行数从 1 到上限」的一个前缀排布，便于主问题在覆盖量与余料之间权衡。
    /// Every candidate corresponds to a prefix layout with one to N rows, letting the master problem trade off
    /// coverage against waste.
    pub fn generate_patterns<V, U>(
        &self,
        request: &PatternGenerationRequest2<V, U>,
    ) -> Ret<Vec<Pattern2<V, U>>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        request.validate()?;
        let width_bound = request.region_width_bound();
        let height_bound = request.region_height_bound();
        if !quantity_is_positive(&width_bound) || !quantity_is_positive(&height_bound) {
            return Err(validation_error(
                "料域上界必须为正 / The region bounds must be positive",
            ));
        }

        // 逐行选择：每行取「能放进剩余高度、且可放件数最多」的产品。
        // Row by row: each row takes the product fitting the most copies into the remaining height.
        let mut remaining: Vec<u64> = request.products.iter().map(|(_, amount)| *amount).collect();
        // 用带单位的量值累计已用高度：裸 V 相减会丢失单位，使上界判定退化为永远成立。
        // Accumulate the used height as a quantity: subtracting a bare `V` drops the unit and makes the bound
        // check degenerate into always-true.
        let mut cursor_y = quantity_zero_of(&height_bound);
        let mut rows: Vec<(RowPick<V, U>, Vec<Placement2<V, U>>)> = Vec::new();
        let mut guard = 0usize;

        while guard < 4096 {
            guard += 1;
            let remaining_height = Quantity::new(
                height_bound.value.clone() - cursor_y.value.clone(),
                height_bound.unit.clone(),
            );
            if !quantity_is_positive(&remaining_height) {
                break;
            }
            let pick = request
                .products
                .iter()
                .enumerate()
                .filter(|(index, _)| remaining[*index] > 0)
                .filter_map(|(index, (product, _))| {
                    let mut best: Option<RowPick<V, U>> = None;
                    for rotated in product.orientations() {
                        let (width, height) = product.size_of(rotated);
                        // 判定必须按「行距 = 高 + 刀缝」而非裸件高：行占用的空间包含行间刀缝，
                        // 只用件高比较会把最后一行挤出料域（实际产出 9 件而料域只装得下 6 件）。
                        // The check must use the row pitch (height plus kerf) rather than the bare height: a row
                        // occupies its kerf too, and comparing the height alone pushes the last row out of the
                        // region (producing 9 pieces in a region that only holds 6).
                        let row_pitch = Quantity::new(
                            height.value.clone() + request.kerf.value.clone(),
                            height.unit.clone(),
                        );
                        if !quantity_less_or_equal(&width, &width_bound)
                            || !quantity_less_or_equal(&row_pitch, &remaining_height)
                        {
                            continue;
                        }
                        // 刀缝通过节距体现：列距 = 宽 + 刀缝。
                        // The kerf is expressed through the pitch: column pitch = width plus kerf.
                        let pitch = Quantity::new(
                            width.value.clone() + request.kerf.value.clone(),
                            width.unit.clone(),
                        );
                        if !quantity_is_positive(&pitch) {
                            continue;
                        }
                        let cols = count_fitting(&pitch, &width_bound);
                        if cols == 0 {
                            continue;
                        }
                        let candidate = RowPick {
                            product_id: product.id.as_str().to_string(),
                            width: width.clone(),
                            height: height.clone(),
                            cols,
                            rotated,
                        };
                        best = match best {
                            Some(existing) if existing.cols >= candidate.cols => Some(existing),
                            _ => Some(candidate),
                        };
                    }
                    best.map(|pick| (index, pick))
                })
                .max_by_key(|(_, pick)| pick.cols);

            let Some((index, pick)) = pick else {
                break;
            };

            let mut placements = Vec::new();
            let mut placed = 0usize;
            for col in 0..pick.cols {
                if remaining[index] == 0 {
                    break;
                }
                let x = V::from(0.0).unwrap_or_else(V::zero)
                    + Quantity::new(
                        pick.width.value.clone() + request.kerf.value.clone(),
                        pick.width.unit.clone(),
                    )
                    .value
                    * V::from(col as f64).unwrap_or_else(V::zero);
                let (width, height) = request.products[index].0.size_of(pick.rotated);
                placements.push(
                    Placement2::new(
                        ospf_rust_framework_geometry2d::PackingShape2::Rectangle(
                            ospf_rust_framework_geometry2d::rectangle_shape2(width, height)
                                .map_err(|error| validation_error(error.to_string()))?,
                        ),
                        QuantityPoint2 {
                            x: Quantity::new(x, width_bound.unit.clone()),
                            y: Quantity::new(cursor_y.value.clone(), width_bound.unit.clone()),
                        },
                    )
                    .with_rotated(pick.rotated)
                    .with_item_id(pick.product_id.clone()),
                );
                remaining[index] -= 1;
                placed += 1;
            }
            if placed == 0 {
                break;
            }
            let row_pitch = Quantity::new(
                pick.height.value.clone() + request.kerf.value.clone(),
                pick.height.unit.clone(),
            );
            cursor_y = Quantity::new(
                cursor_y.value.clone() + row_pitch.value.clone(),
                height_bound.unit.clone(),
            );
            rows.push((pick, placements));
            // `max_candidates` 限制的是**产出图案数**而不是行数：行仍须铺满料域，
            // 否则「最大候选数」这个名字与行为不符，列生成也拿不到足够的候选。
            // `max_candidates` bounds the number of **emitted patterns**, not the rows: rows must still fill the
            // region, otherwise the name and the behaviour disagree and column generation gets too few
            // candidates.
        }

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // 每个前缀行数产出一个候选图案，便于主问题按覆盖量取舍。候选过多时保留覆盖最大的若干个：
        // 前缀越长覆盖越多，因此保留末尾。
        // Emit one candidate per prefix row count so the master can trade coverage against waste. When there are
        // too many candidates keep the largest ones: longer prefixes cover more, so the tail is kept.
        let limit = request.max_candidates.max(1);
        let first_prefix = rows.len().saturating_sub(limit);
        let mut patterns = Vec::with_capacity(rows.len() - first_prefix);
        for prefix in (first_prefix + 1)..=rows.len() {
            let mut placements = Vec::new();
            let mut height = V::zero();
            for (pick, row_placements) in rows.iter().take(prefix) {
                placements.extend(row_placements.iter().cloned());
                height = height.clone() + pick.height.value.clone() + request.kerf.value.clone();
            }
            let region = Region2::new(
                quantity_zero_of(&request.material.width),
                quantity_zero_of(&request.material.width),
                width_bound.clone(),
                Quantity::new(height, height_bound.unit.clone()),
            );
            let mut pattern = Pattern2::new(
                crate::domain::material::pattern_id2(format!(
                    "{}-{}-{}",
                    request.material.id, request.iteration, prefix
                )),
                request.material.clone(),
                region,
                placements,
                request.process,
            );
            // 副产物填充：把可出售的副产物排进余料。填充只增加副产物，不改变成品覆盖量，
            // 因此不会影响需求满足判定。
            // By-product filling: pack sellable by-products into the remnant. Filling only adds by-products and
            // never changes finished coverage, so demand satisfaction is unaffected.
            if !request.costars.is_empty() {
                let filled = crate::domain::pattern_generation::costar_filler::CostarFiller::new()
                    .fill(&pattern, &request.costars, &request.kerf);
                if !filled.placements.is_empty() {
                    pattern = pattern.with_costars(filled.placements);
                }
            }
            if request.process.requires_cut_tree()
                && let Some(tree) = self.build_cut_tree(&pattern, request)
            {
                pattern = pattern.with_cut_tree(tree);
            }
            patterns.push(pattern);
        }
        Ok(patterns)
    }

    /// 为图案构造断头台切分树 / Build the guillotine cut tree of a pattern
    ///
    /// 与 Kotlin 的 `buildRowsTree` 一致：沿 Y 方向逐行切分，每一刀切出最下面一行，其余递归。
    /// 级数规则是「同向刀同级、换向才加一级」，因此这一串横刀全部是第 1 级，行内竖切成件才是第 2 级——
    /// 这正是两级断头台「先切条、再切件」的语义。
    /// Matches Kotlin's `buildRowsTree`: rows are cut one by one along Y, each cut separating the bottom row and
    /// recursing on the rest. The stage rule is "same direction keeps the stage, a direction change adds one", so
    /// this chain of horizontal cuts all stays at stage 1 while vertical cuts inside a row become stage 2 — exactly
    /// the two-stage guillotine semantics of "strips then pieces".
    fn build_cut_tree<V, U>(
        &self,
        pattern: &Pattern2<V, U>,
        request: &PatternGenerationRequest2<V, U>,
    ) -> Option<CutTree2<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        if !request.process.requires_cut_tree() {
            return None;
        }
        // 收集各行上边界（按 Y 升序去重）。
        // Collect the row upper edges in ascending Y order, deduplicated.
        let mut row_tops: Vec<f64> = pattern
            .placements
            .iter()
            .filter_map(|placement| {
                let y = placement.position.y.value.to_f64()?;
                let height = placement.shape.bounding_height().value.to_f64()?;
                Some(y + height)
            })
            .collect();
        row_tops.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
        row_tops.dedup_by(|lhs, rhs| (*lhs - *rhs).abs() < 1e-9);
        if row_tops.is_empty() {
            return None;
        }
        if row_tops.len() == 1 {
            // 只有一行时仍需按工艺级数切分成件：两级工艺下竖切，三级工艺下先横切分块再竖切。
            // With a single row the pieces still have to be cut at the process stages: vertical cuts under a
            // two-stage process, and a block cut before the vertical cuts under a three-stage process.
            return Self::cut_pieces(
                pattern.region.clone(),
                &pattern.placements,
                request.process,
                1,
                &request.kerf,
            );
        }
        Self::cut_rows(
            pattern.region.clone(),
            &row_tops,
            &pattern.placements,
            request.process,
            &request.kerf,
        )
    }

    /// 逐行递归切分 / Recursively split row by row
    ///
    /// 每一刀沿 Y 切出最下面一行（第 1 级），其余部分递归。行内再按工艺级数切分成件：
    /// 两级工艺直接竖切，三级工艺先横切分块（第 2 级）再竖切（第 3 级）。
    /// Each cut separates the bottom row along Y (stage 1) and recurses on the rest. Inside a row the pieces are
    /// cut according to the process: two-stage cuts vertically, while three-stage first cuts blocks (stage 2) and
    /// then cuts pieces vertically (stage 3).
    fn cut_rows<V, U>(
        region: Region2<V, U>,
        row_tops: &[f64],
        placements: &[Placement2<V, U>],
        process: CuttingProcess2,
        kerf: &Quantity<V, U>,
    ) -> Option<CutTree2<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let Some((first, rest)) = row_tops.split_first() else {
            return Some(CutTree2::leaf(region, None));
        };
        if rest.is_empty() {
            return Self::cut_pieces(region, placements, process, 2, kerf);
        }
        let position =
            Quantity::new(V::from(*first).unwrap_or_else(V::zero), region.height.unit.clone());
        let (bottom, upper) = region
            .cut(CutOrientation2::Horizontal, &position, kerf)
            .ok()?;
        // 该行内的放置：按**绝对** Y 落在本行区间内。
        // 不能用 region.y 作下界——递归后 region.y 已随切分上移，而 row_tops 仍是绝对坐标，
        // 两者混用会把后续行误判为空行，导致切分树构造失败。
        // The placements of this row: those whose **absolute** Y falls inside the row band. `region.y` must not
        // be used as the lower bound: after recursion it has moved up with the splits while `row_tops` is still
        // absolute, and mixing the two would wrongly treat later rows as empty and break the cut tree.
        // 本行下边界由 row_tops 的前一项给出，因此需要把当前行在完整序列里的位置一并传入。
        // The row's lower bound comes from the previous entry of `row_tops`, so the current row's position in the
        // full sequence has to be passed in.
        let row_bottom = row_bottom_of(row_tops, *first);
        let row_top = *first;
        let row_placements: Vec<Placement2<V, U>> = placements
            .iter()
            .filter(|placement| {
                let y = placement.position.y.value.to_f64().unwrap_or(f64::NAN);
                y >= row_bottom - 1e-9 && y < row_top - 1e-9
            })
            .cloned()
            .collect();
        let lower = Self::cut_pieces(bottom, &row_placements, process, 2, kerf)?;
        let upper_tree = Self::cut_rows(upper, rest, placements, process, kerf)?;
        CutTree2::cut(
            CutLine2 {
                orientation: CutOrientation2::Horizontal,
                position,
                stage: 1,
            },
            region,
            kerf.clone(),
            lower,
            upper_tree,
        )
        .ok()
    }

    /// 行内切分成件 / Cut the pieces inside a row
    ///
    /// 两级工艺：竖切逐件（第 2 级）。
    /// 三级工艺：先横切分出「块」（第 2 级），再竖切（第 3 级）——对应剪板机「条 → 块 → 件」。
    /// Two-stage: vertical cuts per piece (stage 2). Three-stage: a horizontal block cut (stage 2) followed by
    /// vertical cuts (stage 3), matching the shear workflow "strips, blocks, pieces".
    fn cut_pieces<V, U>(
        region: Region2<V, U>,
        placements: &[Placement2<V, U>],
        process: CuttingProcess2,
        root_stage: usize,
        kerf: &Quantity<V, U>,
    ) -> Option<CutTree2<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        // 按 X 升序排列，逐件竖切。
        // Sort by ascending X and cut the pieces one by one vertically.
        let mut ordered: Vec<&Placement2<V, U>> = placements.iter().collect();
        ordered.sort_by(|lhs, rhs| {
            lhs.position
                .x
                .value
                .to_f64()
                .unwrap_or(0.0)
                .partial_cmp(&rhs.position.x.value.to_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if ordered.is_empty() {
            return Some(CutTree2::leaf(region, None));
        }
        if ordered.len() == 1 && process != CuttingProcess2::GuillotineThreeStage {
            return Some(CutTree2::leaf(region, None));
        }

        // 三级工艺：件高小于行高时，先用横刀切出与件等高的块，把上方余料分离。
        // Three-stage: when the piece is shorter than the row, cut a block as tall as the piece first so the
        // remnant above it is separated.
        let row_height = region.height.value.to_f64().unwrap_or(0.0);
        let piece_height = ordered[0]
            .shape
            .bounding_height()
            .value
            .to_f64()
            .unwrap_or(row_height);
        if process == CuttingProcess2::GuillotineThreeStage && piece_height + 1e-9 < row_height {
            let position =
                Quantity::new(V::from(piece_height).unwrap_or_else(V::zero), region.height.unit.clone());
            let (block, above) = region
                .cut(CutOrientation2::Horizontal, &position, kerf)
                .ok()?;
            let block_tree = Self::cut_vertical(block, &ordered, root_stage + 1, kerf)?;
            return CutTree2::cut(
                CutLine2 {
                    orientation: CutOrientation2::Horizontal,
                    position,
                    stage: root_stage,
                },
                region,
                kerf.clone(),
                block_tree,
                CutTree2::leaf(above, None),
            )
            .ok();
        }
        // 件高等于行高时没有块可切，竖切直接成为当前子树的根刀线，级数必须等于该子树被期望的级数。
        // `root_stage` 由调用方给出：整板直接进件时为 1，位于条带横刀之下时为 2。
        // When the piece is as tall as the row there is no block to cut, so the vertical cuts become the root of
        // this subtree and their stage must equal the stage expected of it. `root_stage` comes from the caller:
        // 1 when the whole region goes straight to pieces, 2 when it sits below a strip cut.
        Self::cut_vertical(region, &ordered, root_stage, kerf)
    }

    /// 竖切逐件 / Cut the pieces one by one vertically
    fn cut_vertical<V, U>(
        region: Region2<V, U>,
        ordered: &[&Placement2<V, U>],
        stage: usize,
        kerf: &Quantity<V, U>,
    ) -> Option<CutTree2<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let Some((piece, rest)) = ordered.split_first() else {
            return Some(CutTree2::leaf(region, None));
        };
        let piece_width = piece.shape.bounding_width().value.to_f64().unwrap_or(0.0);
        // 刀位必须相对**当前**区域左边界计算：递归后区域已随前面的竖切右移，
        // 再用放置的绝对 x 会把刀位算到区域之外。
        // The cut position must be relative to the **current** region's left edge: after recursion the region has
        // already moved right with the preceding vertical cuts, and using the placement's absolute X would put
        // the cut outside the region.
        let region_left = region.x.value.to_f64().unwrap_or(0.0);
        let cut_at = region_left + piece_width;
        let region_right = region.x.value.to_f64().unwrap_or(0.0) + region.width.value.to_f64().unwrap_or(0.0);
        if cut_at + 1e-9 >= region_right {
            // 该件占满剩余宽度，直接成为叶子。
            // The piece fills the remaining width and becomes a leaf.
            return Some(CutTree2::leaf(region, piece.item_id.clone()));
        }
        let position = Quantity::new(V::from(cut_at).unwrap_or_else(V::zero), region.width.unit.clone());
        let (left, right) = region
            .cut(CutOrientation2::Vertical, &position, kerf)
            .ok()?;
        let rest_tree = Self::cut_vertical(right, rest, stage, kerf)?;
        CutTree2::cut(
            CutLine2 {
                orientation: CutOrientation2::Vertical,
                position,
                stage,
            },
            region,
            kerf.clone(),
            CutTree2::leaf(left, piece.item_id.clone()),
            rest_tree,
        )
        .ok()
    }
}

impl<V, U> crate::domain::pattern_generation::request::PatternGenerator2<V, U>
    for ShelfPatternGenerator2
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "shelf"
    }

    fn generate(&self, request: &PatternGenerationRequest2<V, U>) -> Vec<Pattern2<V, U>> {
        self.generate_patterns(request).unwrap_or_default()
    }
}

/// 由行上边界序列求该行的下边界 / The row's lower bound derived from the sequence of row upper edges
///
/// 首行的下边界是 0；其余行取序列中前一项。这样行区间严格是 `[下边界, 上边界)`，不会把相邻行算进来。
/// The first row starts at 0; every other row takes the previous entry. The row band is therefore exactly
/// `[lower, upper)` and never swallows an adjacent row.
fn row_bottom_of(row_tops: &[f64], upper: f64) -> f64 {
    let position = row_tops
        .iter()
        .position(|top| (*top - upper).abs() < 1e-9);
    match position {
        Some(0) | None => 0.0,
        Some(index) => row_tops[index - 1],
    }
}

/// 由物料与产品快速构造一个货架式图案 / Quickly build one shelf pattern from a material and products
pub fn shelf_pattern<V, U>(
    material: &Material2<V, U>,
    products: &[(Product2<V, U>, u64)],
    kerf: Quantity<V, U>,
    process: CuttingProcess2,
) -> Ret<Option<Pattern2<V, U>>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let request = PatternGenerationRequest2::new(material.clone(), products.to_vec(), kerf)
        .with_process(process);
    let patterns = ShelfPatternGenerator2::new().generate_patterns(&request)?;
    Ok(patterns.into_iter().last())
}
