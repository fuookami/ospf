//! 装箱聚合与结果 / Packing aggregation and results
//!
//! 装箱结果由「板材 → 层组合 → 层 → 放置」四层结构展开，附带装载顺序、物料汇总与余料。
//! The packing result expands the "sheet → sheet-layer → layer → placement" structure and carries the
//! loading order, the material summary and the remnants.

use std::collections::BTreeMap;

use ospf_rust_framework_geometry2d::Placement2;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::model::{BatchNo, ItemId, Layer, MaterialKey, SheetType, SheetLayer};
use crate::domain::packing::service::free_space_analyzer::Remnant2;

// ============================================================================
// PackedItem - 已装箱物品 / Packed item
// ============================================================================

/// 已装箱物品 / Packed item
#[derive(Debug, Clone)]
pub struct PackedItem<V, U: UnitTrait> {
    /// 放置 / Placement
    pub placement: Placement2<V, U>,
    /// 装载顺序 / Loading order
    pub loading_order: u64,
    /// 物品标识 / Item identifier
    pub item_id: Option<String>,
}

// ============================================================================
// PackedLayer / PackedSheet - 已装箱层与板材 / Packed layer and sheet
// ============================================================================

/// 已装箱层：一层与其展开后的物品 / Packed layer: one layer and its expanded items
#[derive(Debug, Clone)]
pub struct PackedLayer<V, U: UnitTrait> {
    /// 原始层 / Source layer
    pub layer: Layer<V, U>,
    /// 展开后的物品 / Expanded items
    pub items: Vec<PackedItem<V, U>>,
}

/// 已装箱板材 / Packed sheet
#[derive(Debug, Clone)]
pub struct PackedSheet<V, U: UnitTrait> {
    /// 板材名称 / Sheet name
    pub name: String,
    /// 板材类型 / Sheet type
    pub sheet_type: SheetType<V, U>,
    /// 批次号 / Batch number
    pub batch: Option<BatchNo>,
    /// 已装箱层 / Packed layers
    pub layers: Vec<PackedLayer<V, U>>,
    /// 全部物品（扁平） / All items, flattened
    pub items: Vec<PackedItem<V, U>>,
    /// 装载率 / Loading rate
    pub loading_rate: f64,
    /// 余料矩形 / Remnant rectangles
    pub remnants: Vec<Remnant2<V, U>>,
}

impl<V, U> PackedSheet<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 实际占用面积 / Actually occupied area
    pub fn occupied_area(&self) -> f64 {
        self.items
            .iter()
            .map(|item| item.placement.shape.area().value.to_f64().unwrap_or(0.0))
            .sum()
    }

    /// 板材面积 / Sheet area
    pub fn sheet_area(&self) -> f64 {
        self.sheet_type.area().value.to_f64().unwrap_or(0.0)
    }

    /// 余料总面积 / Total remnant area
    pub fn remnant_area(&self) -> f64 {
        self.remnants
            .iter()
            .map(|remnant| remnant.area.value.to_f64().unwrap_or(0.0))
            .sum()
    }
}

// ============================================================================
// MaterialSummary - 物料汇总 / Material summary
// ============================================================================

/// 物料汇总 / Material summary
#[derive(Debug, Clone, Default)]
pub struct MaterialSummary {
    /// 物料到件数的映射 / Material to amount map
    pub amounts: BTreeMap<MaterialKey, u64>,
}

impl MaterialSummary {
    /// 创建空汇总 / Create an empty summary
    pub fn new() -> Self {
        Self::default()
    }

    /// 累加一个物料件数 / Accumulate one material amount
    pub fn add(&mut self, key: MaterialKey, amount: u64) {
        *self.amounts.entry(key).or_insert(0) += amount;
    }

    /// 总件数 / Total amount
    pub fn total(&self) -> u64 {
        self.amounts.values().sum()
    }
}

// ============================================================================
// PackingAggregation / PackingResult - 装箱聚合与结果 / Aggregation and result
// ============================================================================

/// 装箱聚合 / Packing aggregation
#[derive(Debug, Clone, Default)]
pub struct PackingAggregation {
    /// 名称 / Name
    pub name: String,
}

impl PackingAggregation {
    /// 创建聚合 / Create the aggregation
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// 装箱结果 / Packing result
#[derive(Debug, Clone)]
pub struct PackingResult<V, U: UnitTrait> {
    /// 已装箱板材 / Packed sheets
    pub sheets: Vec<PackedSheet<V, U>>,
    /// 物料汇总 / Material summary
    pub material_summary: MaterialSummary,
    /// 全部余料 / All remnants
    pub remnants: Vec<Remnant2<V, U>>,
}

impl<V, U> PackingResult<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 板材数量 / Sheet count
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// 总件数 / Total amount
    pub fn total_amount(&self) -> usize {
        self.sheets.iter().map(|sheet| sheet.items.len()).sum()
    }

    /// 余料数量 / Remnant count
    pub fn remnant_count(&self) -> usize {
        self.remnants.len()
    }

    /// 余料总面积 / Total remnant area
    pub fn remnant_area(&self) -> f64 {
        self.remnants
            .iter()
            .map(|remnant| remnant.area.value.to_f64().unwrap_or(0.0))
            .sum()
    }

    /// 总占用面积 / Total occupied area
    pub fn occupied_area(&self) -> f64 {
        self.sheets.iter().map(|sheet| sheet.occupied_area()).sum()
    }

    /// 总板材面积 / Total sheet area
    pub fn sheet_area(&self) -> f64 {
        self.sheets.iter().map(|sheet| sheet.sheet_area()).sum()
    }

    /// 物品级件数映射 / Item-level amount map
    pub fn item_amounts(&self) -> BTreeMap<ItemId, u64> {
        let mut counter: BTreeMap<ItemId, u64> = BTreeMap::new();
        for sheet in &self.sheets {
            for item in &sheet.items {
                if let Some(id) = &item.item_id {
                    *counter.entry(ItemId::new(id.clone())).or_insert(0) += 1;
                }
            }
        }
        counter
    }
}

/// 由板材与层组合构造已装箱层 / Build packed layers from a sheet's layer combinations
pub fn packed_layers_from_sheet_layers<V, U>(
    sheet_layers: &[SheetLayer<V, U>],
    loading_order_for: impl Fn(&Placement2<V, U>) -> u64,
) -> Vec<PackedLayer<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let mut packed = Vec::new();
    for sheet_layer in sheet_layers {
        for layer in &sheet_layer.layers {
            let items = layer
                .units
                .iter()
                .map(|placement| PackedItem {
                    placement: placement.clone(),
                    loading_order: loading_order_for(placement),
                    item_id: placement.item_id.clone(),
                })
                .collect();
            packed.push(PackedLayer {
                layer: layer.clone(),
                items,
            });
        }
    }
    packed
}

/// 由已装箱层收集全部物品 / Collect every item from the packed layers
pub fn flatten_packed_items<V, U: UnitTrait>(
    layers: &[PackedLayer<V, U>],
) -> Vec<PackedItem<V, U>>
where
    V: Clone,
    U: UnitTrait + Clone,
{
    layers
        .iter()
        .flat_map(|layer| layer.items.iter().cloned())
        .collect()
}

/// 由箱型与面积计算装载率 / Compute the loading rate from the sheet area and the occupied area
pub fn loading_rate_of(occupied_area: f64, sheet_area: f64) -> f64 {
    if sheet_area <= 0.0 {
        return 0.0;
    }
    occupied_area / sheet_area
}

/// 面积量值的浮点视图 / Floating-point view of an area quantity
pub fn area_value<V, U>(area: &Quantity<V, ospf_rust_quantities::unit::Unit>) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + Clone,
{
    area.value.to_f64().unwrap_or(0.0)
}
