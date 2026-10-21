//! 断头台切分树 / Guillotine cut tree
//!
//! 断头台方案不是一组坐标，而是一棵**切分树**：内部结点是贯穿刀，叶子是最终料块。刀线的级数是它的
//! 切割层号：根刀线为第 1 级，其下的刀线必须恰为第 2 级，依此类推。因此 2 级工艺恰好等于「先切条、
//! 再切件」，3 级工艺等于「切条 → 切块 → 切件」。
//!
//! `validate` 会拒绝这些不合法的树：区域切分不一致（含刀缝带）、超出工艺级数上限、子刀线层号不正确。
//!
//! A guillotine layout is not a set of coordinates but a **cut tree**: internal nodes are through cuts and
//! leaves are final pieces. The cut level is the stage number: the root cut is stage 1 and the cuts below it
//! must be exactly stage 2, and so on. A two-stage process therefore equals "strips then pieces" and a
//! three-stage process equals "strips, blocks, pieces".
//!
//! `validate` rejects illegal trees: inconsistent region splits (including the kerf strip), stages beyond the
//! process limit, and incorrect child stages.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::error::validation_error;
use crate::infrastructure::cutting_process::CuttingProcess2;
use crate::infrastructure::region::{CutOrientation2, Region2};

// ============================================================================
// CutLine2 - 一条贯穿刀线 / A single through cut
// ============================================================================

/// 一条贯穿刀线 / A single through cut
#[derive(Debug, Clone)]
pub struct CutLine2<V, U: UnitTrait> {
    /// 刀线方向 / Cut line orientation
    pub orientation: CutOrientation2,
    /// 刀缝起始坐标（竖刀为 x，横刀为 y）/ The coordinate where the kerf strip starts
    pub position: Quantity<V, U>,
    /// 切割层号：1 为切条、2 为切块、3 为切件 / Cut level: 1 strips, 2 blocks, 3 pieces
    pub stage: usize,
}

// ============================================================================
// CutTree2 - 切分树 / Cut tree
// ============================================================================

/// 切分树 / Cut tree
#[derive(Debug, Clone)]
pub enum CutTree2<V, U: UnitTrait> {
    /// 叶子：最终料块 / Leaf: a final piece
    Leaf(Leaf2<V, U>),
    /// 内部结点：一刀与它切出的两个子树 / Internal node: one cut and the two subtrees it produces
    Cut(CutNode2<V, U>),
}

/// 叶子料块 / Leaf piece
#[derive(Debug, Clone)]
pub struct Leaf2<V, U: UnitTrait> {
    /// 料块区域 / Piece region
    pub region: Region2<V, U>,
    /// 工件标识（余料为 `None`）/ Part identifier; `None` for a remnant
    pub part_id: Option<String>,
}

/// 切分结点 / Cut node
#[derive(Debug, Clone)]
pub struct CutNode2<V, U: UnitTrait> {
    /// 本结点的刀缝起始坐标与方向 / Kerf start coordinate and orientation of this node
    pub line: CutLine2<V, U>,
    /// 本结点对应的区域 / The region of this node
    pub region: Region2<V, U>,
    /// 刀缝宽度 / Kerf width
    pub kerf: Quantity<V, U>,
    /// 左侧或下侧子树 / Left or bottom subtree
    pub lower: Box<CutTree2<V, U>>,
    /// 右侧或上侧子树 / Right or top subtree
    pub upper: Box<CutTree2<V, U>>,
    /// 本次切分得到的两个区域，以校验子树是否一致 / The two resulting regions, used to validate the subtrees
    pub split: Option<(Region2<V, U>, Region2<V, U>)>,
}

impl<V, U> CutTree2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 构造叶子 / Build a leaf
    pub fn leaf(region: Region2<V, U>, part_id: Option<String>) -> Self {
        Self::Leaf(Leaf2 { region, part_id })
    }

    /// 构造切分结点 / Build a cut node
    ///
    /// 切分区域由 `Region2::cut` 推导，避免调用方自行计算导致不一致。
    /// The split regions are derived by `Region2::cut` so callers cannot introduce an inconsistency.
    pub fn cut(
        line: CutLine2<V, U>,
        region: Region2<V, U>,
        kerf: Quantity<V, U>,
        lower: CutTree2<V, U>,
        upper: CutTree2<V, U>,
    ) -> Ret<Self> {
        let split = region.cut(line.orientation, &line.position, &kerf)?;
        Ok(Self::Cut(CutNode2 {
            line,
            region,
            kerf,
            lower: Box::new(lower),
            upper: Box::new(upper),
            split: Some(split),
        }))
    }

    /// 本结点的区域 / The region of this node
    pub fn region(&self) -> &Region2<V, U> {
        match self {
            Self::Leaf(leaf) => &leaf.region,
            Self::Cut(node) => &node.region,
        }
    }

    /// 本结点的工序级数 / The process stage of this node
    ///
    /// 叶子返回 0（不是刀线），内部结点返回它自己刀线的级数。
    /// A leaf returns 0 (it is not a cut); an internal node returns its own cut's stage.
    pub fn stage(&self) -> usize {
        match self {
            Self::Leaf(_) => 0,
            Self::Cut(node) => node.line.stage,
        }
    }

    /// 收集全部叶子 / Collect every leaf
    pub fn leaves(&self) -> Vec<&Leaf2<V, U>> {
        let mut leaves = Vec::new();
        self.collect_leaves(&mut leaves);
        leaves
    }

    fn collect_leaves<'a>(&'a self, leaves: &mut Vec<&'a Leaf2<V, U>>) {
        match self {
            Self::Leaf(leaf) => leaves.push(leaf),
            Self::Cut(node) => {
                node.lower.collect_leaves(leaves);
                node.upper.collect_leaves(leaves);
            }
        }
    }

    /// 收集全部刀线，按深度优先顺序，即下刀顺序 / Collect every cut line in depth-first order, i.e. the cut
    /// sequence
    pub fn cut_lines(&self) -> Vec<&CutLine2<V, U>> {
        let mut lines = Vec::new();
        self.collect_cut_lines(&mut lines);
        lines
    }

    fn collect_cut_lines<'a>(&'a self, lines: &mut Vec<&'a CutLine2<V, U>>) {
        match self {
            Self::Leaf(_) => {}
            Self::Cut(node) => {
                lines.push(&node.line);
                node.lower.collect_cut_lines(lines);
                node.upper.collect_cut_lines(lines);
            }
        }
    }

    /// 树的深度（叶子为 0）/ Tree depth (a leaf is 0)
    pub fn depth(&self) -> usize {
        match self {
            Self::Leaf(_) => 0,
            Self::Cut(node) => 1 + node.lower.depth().max(node.upper.depth()),
        }
    }

    /// 校验切分树是否与工艺一致 / Validate the cut tree against the process
    ///
    /// 拒绝三类不合法：区域切分不一致、超出工艺级数上限、子刀线层号不是父级 +1。
    /// Rejects three kinds: inconsistent region splits, stages beyond the process limit, and child cut stages
    /// that are not parent + 1.
    pub fn validate(&self, process: CuttingProcess2) -> Ret<()> {
        self.validate_node(process, 1)
    }

    fn validate_node(&self, process: CuttingProcess2, expected_stage: usize) -> Ret<()> {
        match self {
            Self::Leaf(_) => Ok(()),
            Self::Cut(node) => {
                if node.line.stage != expected_stage {
                    return Err(validation_error(format!(
                        "刀线层号不正确：期望第 {expected_stage} 级，实际第 {} 级 / Wrong cut stage: expected {}, got {}",
                        node.line.stage, expected_stage, node.line.stage
                    )));
                }
                if !process.allows_stage(node.line.stage) {
                    return Err(validation_error(format!(
                        "刀线层号 {} 超出工艺 {:?} 的上限 {} / cut stage {} exceeds the limit {} of process {:?}",
                        node.line.stage,
                        process,
                        process.max_stage(),
                        node.line.stage,
                        process.max_stage(),
                        process
                    )));
                }
                // 区域切分一致性：重新切一次，子区域必须与切分结果**完全相等**。
                // 只做包含判定会放过「子树比切出的区域小」这种错误树，因此这里要求相等。
                // Region split consistency: cutting again must reproduce the child regions **exactly**. A mere
                // containment check would accept a tree whose subtree is smaller than the split region.
                let (lower_region, upper_region) =
                    node.region.cut(node.line.orientation, &node.line.position, &node.kerf)?;
                if !same_region(&lower_region, node.lower.region()) {
                    return Err(validation_error(
                        "下侧子树区域与切分结果不一致 / The lower subtree region does not match the split result",
                    ));
                }
                if !same_region(&upper_region, node.upper.region()) {
                    return Err(validation_error(
                        "上侧子树区域与切分结果不一致 / The upper subtree region does not match the split result",
                    ));
                }
                node.lower.validate_child(process, &node.line)?;
                node.upper.validate_child(process, &node.line)
            }
        }
    }

    /// 校验子结点：级数随刀向耦合，叶子允许提前结束
    /// / Validate a child: the stage is coupled to the cut orientation and a leaf may end early
    ///
    /// 级数规则与 Kotlin 一致：**同向刀属于同一级**（同一批平行刀），**换向才进入下一级**。
    /// 因此把整板切成多条同向条带时，所有条带刀线都是第 1 级；条带内再竖切成件才是第 2 级。
    /// The stage rule matches Kotlin: **cuts in the same direction share a stage** (one family of parallel
    /// cuts) and **only a direction change advances the stage**. Cutting a sheet into several strips therefore
    /// keeps every strip cut at stage 1, while vertical cuts inside a strip become stage 2.
    fn validate_child(&self, process: CuttingProcess2, parent: &CutLine2<V, U>) -> Ret<()> {
        match self {
            Self::Leaf(_) => Ok(()),
            Self::Cut(node) => {
                let expected = if node.line.orientation == parent.orientation {
                    parent.stage
                } else {
                    parent.stage + 1
                };
                self.validate_node(process, expected)
            }
        }
    }

    /// 叶子总面积 / Total leaf area
    pub fn leaf_area(&self) -> f64 {
        self.leaves()
            .iter()
            .map(|leaf| leaf.region.area().value.to_f64().unwrap_or(0.0))
            .sum()
    }

    /// 叶子数量 / Leaf count
    pub fn leaf_count(&self) -> usize {
        self.leaves().len()
    }
}

/// 判断两个区域是否完全相等 / Whether two regions are exactly equal
///
/// 比较四个边界值；浮点比较使用极小容差，避免同一运算路径的末位差异被误判为不等。
/// Compares the four boundary values; a tiny tolerance avoids treating last-bit differences from the same
/// arithmetic path as inequality.
pub fn same_region<V, U>(lhs: &Region2<V, U>, rhs: &Region2<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let close = |left: &Quantity<V, U>, right: &Quantity<V, U>| -> bool {
        match (left.value.to_f64(), right.value.to_f64()) {
            (Some(left), Some(right)) => (left - right).abs() < 1e-9,
            _ => false,
        }
    };
    close(&lhs.x, &rhs.x)
        && close(&lhs.y, &rhs.y)
        && close(&lhs.width, &rhs.width)
        && close(&lhs.height, &rhs.height)
}

/// 判断是否完全落在区域内 / Whether a region lies fully inside another
pub fn is_inside<V, U>(inner: &Region2<V, U>, outer: &Region2<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    outer.contains(inner)
}

/// 判断区域是否有正面积 / Whether a region has a positive area
pub fn has_positive_area<V, U>(region: &Region2<V, U>, sample: &Quantity<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let _ = sample;
    !region.is_degenerate()
}
