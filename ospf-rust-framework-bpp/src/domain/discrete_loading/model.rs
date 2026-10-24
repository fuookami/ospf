//! 维度无关的装载输入与注册聚合。 / Dimension-independent loading inputs and registration aggregation.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::model::flatten::Linear;
use ospf_rust_core::variable::VariableId;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::UnitTrait;

/// 离散货物，身份不会因尺寸或重量相同而合并。 / Discrete cargo, never merged by equal dimensions or weights.
#[derive(Debug, Clone)]
pub struct LoadingItem<V, U: UnitTrait> {
    /// 货物身份。 / Cargo identity.
    pub id: String,
    /// 单件重量。 / Item weight.
    pub weight: Quantity<V, U>,
    /// 是否必须装载。 / Whether loading is mandatory.
    pub required: bool,
    /// 固定舱位身份。 / Fixed position identity.
    pub fixed_position_id: Option<String>,
}

/// 舱位真实容量与外部固定占用。 / True position capacity and external fixed occupancy.
#[derive(Debug, Clone)]
pub struct LoadingPosition<V, U: UnitTrait> {
    /// 舱位身份。 / Position identity.
    pub id: String,
    /// 最大件数。 / Maximum item count.
    pub max_count: u64,
    /// 最大总重量。 / Maximum total weight.
    pub max_weight: Quantity<V, U>,
    /// 是否有外部固定占用。 / Whether external fixed occupancy exists.
    pub fixed_occupied: bool,
    /// 是否允许决策装载；不影响合法固定装载。 / Allow decision loading, without removing valid fixed loads.
    pub enabled: bool,
}

/// 合法分配的稳定键。 / Stable key of a legal assignment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssignmentKey {
    /// 货物身份。 / Cargo identity.
    pub item_id: String,
    /// 舱位身份。 / Position identity.
    pub position_id: String,
}

/// 非传递互斥边；使用规范化端点消除反向重复。 / Non-transitive conflict edge with canonical endpoints.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionConflict {
    /// 首个舱位。 / First position.
    pub first: String,
    /// 第二个舱位。 / Second position.
    pub second: String,
}

impl PositionConflict {
    /// 规范化互斥边的端点；引用有效性由问题验证。 / Canonicalize edge endpoints; the problem validates references.
    pub fn new(first: impl Into<String>, second: impl Into<String>) -> Self {
        let first = first.into();
        let second = second.into();
        if first <= second { Self { first, second } } else { Self { first: second, second: first } }
    }
}

/// 一种布局允许使用的舱位。 / Positions permitted by one layout.
#[derive(Debug, Clone)]
pub struct LoadingLayout {
    /// 布局身份。 / Layout identity.
    pub id: String,
    /// 允许的舱位集合。 / Permitted positions.
    pub position_ids: BTreeSet<String>,
}

/// 在显式作用域内恰选一种布局。 / Select exactly one layout within an explicit scope.
#[derive(Debug, Clone)]
pub struct LayoutSelectionGroup {
    /// 选择组身份。 / Selection-group identity.
    pub id: String,
    /// 本组管辖的舱位。 / Positions governed by this group.
    pub position_ids: BTreeSet<String>,
    /// 可选布局；允许共享舱位。 / Alternative layouts, with shared positions allowed.
    pub layouts: Vec<LoadingLayout>,
}

/// 稀疏离散装载问题；不补全货物与舱位的笛卡尔积。 / Sparse discrete loading, never completing the cargo/position Cartesian product.
#[derive(Debug, Clone)]
pub struct DiscreteLoadingProblem<V, U: UnitTrait> {
    /// 独立货物。 / Independent cargo items.
    pub items: Vec<LoadingItem<V, U>>,
    /// 舱位。 / Positions.
    pub positions: Vec<LoadingPosition<V, U>>,
    /// 显式合法分配。 / Explicit legal assignments.
    pub allowed_assignments: BTreeSet<AssignmentKey>,
    /// 必须保全的全部互斥边。 / All conflict edges that must be preserved.
    pub conflicts: BTreeSet<PositionConflict>,
    /// 可选整体或区域布局组。 / Optional global or regional layout groups.
    pub layout_groups: Vec<LayoutSelectionGroup>,
    /// 模型重量口径。 / Model weight unit.
    pub weight_unit: U,
}

/// 已验证的离散领域解，包含允许的空预留。 / Verified discrete domain solution, including permitted empty reservations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscreteLoadingSolution {
    /// 货物到舱位。 / Cargo-to-position assignments.
    pub assignments: BTreeMap<String, String>,
    /// 已整体占用的舱位。 / Whole occupied positions.
    pub occupied_positions: BTreeSet<String>,
    /// 选择组到布局。 / Layout selections by group.
    pub selected_layouts: BTreeMap<String, String>,
}

/// 单个模型内的注册聚合，表达式使用该模型列序号。 / Single-model registration aggregation with model-local expression indices.
#[derive(Debug, Clone)]
pub struct LoadingAggregation<V> {
    /// 合法分配到变量身份。 / Legal assignments to variable identities.
    pub assignments: BTreeMap<AssignmentKey, VariableId>,
    /// 整体占位变量。 / Whole-position occupancy variables.
    pub occupied: BTreeMap<String, VariableId>,
    /// 选择组与布局到变量。 / Group/layout choices to variables.
    pub layouts: BTreeMap<(String, String), VariableId>,
    /// 舱位件数表达式。 / Position item-count expressions.
    pub counts: BTreeMap<String, Linear<V>>,
    /// 统一单位重量表达式。 / Weight expressions in the common unit.
    pub weights: BTreeMap<String, Linear<V>>,
    /// 每件货物的装载表达式。 / Per-item loading expressions.
    pub loaded: BTreeMap<String, Linear<V>>,
}

impl<V> Default for LoadingAggregation<V> {
    fn default() -> Self {
        Self { assignments: BTreeMap::new(), occupied: BTreeMap::new(), layouts: BTreeMap::new(),
            counts: BTreeMap::new(), weights: BTreeMap::new(), loaded: BTreeMap::new() }
    }
}

/// 保全全部原始边的团覆盖；不会把连通分量误当作团。 / Edge-preserving clique cover, never treating connected components as cliques.
#[derive(Debug, Clone)]
pub struct ConflictIndex {
    edges: BTreeSet<PositionConflict>,
    cliques: Vec<BTreeSet<String>>,
}

impl ConflictIndex {
    /// 确定性构建团覆盖，每个扩张候选必须邻接团中所有端点。 / Build a deterministic cover, extending only by vertices adjacent to every clique member.
    pub fn build(edges: &BTreeSet<PositionConflict>) -> Self {
        let edges = edges.iter().map(|edge| PositionConflict::new(&edge.first, &edge.second)).collect::<BTreeSet<_>>();
        let vertices = edges.iter().flat_map(|edge| [&edge.first, &edge.second]).cloned().collect::<BTreeSet<_>>();
        let mut uncovered = edges.clone();
        let mut cliques = Vec::new();
        while let Some(edge) = uncovered.first().cloned() {
            let mut clique = BTreeSet::from([edge.first, edge.second]);
            for vertex in &vertices {
                if !clique.contains(vertex) && clique.iter().all(|member| edges.contains(&PositionConflict::new(member, vertex))) {
                    clique.insert(vertex.clone());
                }
            }
            uncovered.retain(|edge| !clique.contains(&edge.first) || !clique.contains(&edge.second));
            cliques.push(clique);
        }
        Self { edges, cliques }
    }

    /// 全部规范化原始边。 / Every canonical original edge.
    pub fn edges(&self) -> &BTreeSet<PositionConflict> { &self.edges }

    /// 完整覆盖全部边的团。 / Cliques covering every edge.
    pub fn cliques(&self) -> &[BTreeSet<String>] { &self.cliques }
}
