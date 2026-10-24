//! 物品模型 / Item model
//!
//! 与 BPP3D 的 `ActualItem` 对齐：物品携带装载形状、旋转许可、单件重量与物料件数映射。
//! 二维侧使用 `PackingShape2`（矩形或圆形），面积口径由几何 crate 统一。
//! Mirrors BPP3D `ActualItem`: an item carries its packing shape, rotation allowance, per-piece weight and
//! material amounts. The 2D side uses `PackingShape2` (rectangle or circle); the area convention comes from
//! the geometry crate.

use std::collections::BTreeMap;

use ospf_rust_framework_geometry2d::{
    PackingShape2, PackingShape2Type, Shape2Candidate, candidate_shapes,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;

use super::id::{BatchNo, ItemId};
use super::material::MaterialKey;

// ============================================================================
// AbstractItem - 物品抽象 / Abstract item
// ============================================================================

/// 物品共有能力 / Capabilities shared by every item kind
pub trait AbstractItem<V, U: UnitTrait> {
    /// 物品编号 / Item identifier
    fn id(&self) -> &ItemId;

    /// 物品名称 / Item name
    fn item_name(&self) -> &str;

    /// 装载形状 / Packing shape
    fn shape(&self) -> &PackingShape2<V, U>;

    /// 是否允许旋转 90 度 / Whether rotation by 90 degrees is allowed
    fn allow_rotate(&self) -> bool;

    /// 单件重量 / Per-piece weight
    fn item_weight(&self) -> &Quantity<V, U>;

    /// 物料与件数映射 / Material-to-amount map
    fn material_amounts(&self) -> &BTreeMap<MaterialKey, u64>;

    /// 批次号 / Batch number
    fn batch(&self) -> Option<&BatchNo>;
}

// ============================================================================
// ItemKey - 物品键 / Item key
// ============================================================================

/// 物品键：按编号、形状、旋转许可与批次识别物品 / Item key identifying an item by id, shape, rotation
/// allowance and batch
///
/// 用于需求归并与列去重：形状差异必须参与身份，否则不同尺寸的物品会被错误合并。
/// Used for demand aggregation and column deduplication: the shape must take part in the identity,
/// otherwise items of different sizes would be merged incorrectly.
#[derive(Debug, Clone)]
pub struct ItemKey<V, U: UnitTrait> {
    /// 物品编号 / Item identifier
    pub id: ItemId,
    /// 装载形状 / Packing shape
    pub shape: PackingShape2<V, U>,
    /// 是否允许旋转 / Whether rotation is allowed
    pub allow_rotate: bool,
    /// 批次号 / Batch number
    pub batch: Option<BatchNo>,
}

// ============================================================================
// ActualItem - 实际物品 / Actual item
// ============================================================================

/// 实际物品：板材上真正被排样的物品 / Actual item: the item really placed on a sheet
#[derive(Debug, Clone)]
pub struct ActualItem<V, U: UnitTrait> {
    /// 物品编号 / Item identifier
    pub id: ItemId,
    /// 物品名称 / Item name
    pub name: String,
    /// 装载形状 / Packing shape
    pub shape: PackingShape2<V, U>,
    /// 是否允许旋转 90 度 / Whether rotation by 90 degrees is allowed
    pub allow_rotate: bool,
    /// 单件重量 / Per-piece weight
    pub weight: Quantity<V, U>,
    /// 物料与件数映射 / Material-to-amount map
    pub material_amounts: BTreeMap<MaterialKey, u64>,
    /// 批次号 / Batch number
    pub batch: Option<BatchNo>,
}

impl<V, U: UnitTrait> ActualItem<V, U> {
    /// 创建实际物品 / Create an actual item
    pub fn new(
        id: ItemId,
        name: impl Into<String>,
        shape: PackingShape2<V, U>,
        allow_rotate: bool,
        weight: Quantity<V, U>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            shape,
            allow_rotate,
            weight,
            material_amounts: BTreeMap::new(),
            batch: None,
        }
    }

    /// 设置批次号 / Set the batch number
    pub fn with_batch(mut self, batch: BatchNo) -> Self {
        self.batch = Some(batch);
        self
    }

    /// 追加物料件数 / Append a material amount
    pub fn with_material_amount(mut self, key: MaterialKey, amount: u64) -> Self {
        self.material_amounts.insert(key, amount);
        self
    }
}

impl<V, U> AbstractItem<V, U> for ActualItem<V, U>
where
    V: Clone,
    U: UnitTrait,
{
    fn id(&self) -> &ItemId {
        &self.id
    }

    fn item_name(&self) -> &str {
        &self.name
    }

    fn shape(&self) -> &PackingShape2<V, U> {
        &self.shape
    }

    fn allow_rotate(&self) -> bool {
        self.allow_rotate
    }

    fn item_weight(&self) -> &Quantity<V, U> {
        &self.weight
    }

    fn material_amounts(&self) -> &BTreeMap<MaterialKey, u64> {
        &self.material_amounts
    }

    fn batch(&self) -> Option<&BatchNo> {
        self.batch.as_ref()
    }
}

// ============================================================================
// PatternItem - 图样物品 / Pattern item
// ============================================================================

/// 图样物品：可重复多次的图样块 / Pattern item: a repeated pattern block
///
/// 用于条带与图样候选生成，件数由 `amount` 表达。
/// Used by strip and pattern candidate generation; the count is expressed by `amount`.
#[derive(Debug, Clone)]
pub struct PatternItem<V, U: UnitTrait> {
    /// 内嵌实际物品 / Wrapped actual item
    pub item: ActualItem<V, U>,
    /// 图样内件数 / Amount inside the pattern block
    pub amount: u64,
}

impl<V, U: UnitTrait> PatternItem<V, U> {
    /// 构造一个单件图样 / Build a single-piece pattern
    pub fn single(item: ActualItem<V, U>) -> Self {
        Self { item, amount: 1 }
    }

    /// 构造指定件数的图样 / Build a pattern with the given amount
    pub fn new(item: ActualItem<V, U>, amount: u64) -> Self {
        Self { item, amount }
    }
}

// ============================================================================
// 共享查询 / Shared queries
// ============================================================================

/// 物品键 / Item key of an item
pub fn item_key<V, U, I>(item: &I) -> ItemKey<V, U>
where
    V: Clone,
    U: UnitTrait + Clone,
    I: AbstractItem<V, U>,
{
    ItemKey {
        id: item.id().clone(),
        shape: item.shape().clone(),
        allow_rotate: item.allow_rotate(),
        batch: item.batch().cloned(),
    }
}

/// 按旋转许可展开候选形状 / Candidate shapes honouring the rotation allowance
pub fn item_candidate_shapes<V, U, I>(item: &I) -> Vec<Shape2Candidate<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
    I: AbstractItem<V, U>,
{
    candidate_shapes(item.shape(), item.allow_rotate())
}

/// 物品形状类型 / Shape type of an item
pub fn item_shape_type<V, U, I>(item: &I) -> PackingShape2Type
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
    I: AbstractItem<V, U>,
{
    item.shape().shape_type()
}
