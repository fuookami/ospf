//! 装箱模型 / Packing models
//!
//! 定义最终装箱结果的核心数据模型。
//! Defines core data models for final packing results.

use std::fmt::Debug;

use ospf_rust_math::algebra::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::{ActualItem, BinType, MaterialKey};
use crate::infrastructure::geometry::MetricPoint3;
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::PackingShape3;

// ============================================================================
// PackedItem - 已装箱物品 / Packed item
// ============================================================================

/// 已装箱物品 / Packed item
///
/// 一个物品在最终装箱中的位置和属性。
/// An item's position and attributes in the final packing.
#[derive(Debug, Clone)]
pub struct PackedItem<V, U: UnitTrait> {
    /// 原始物品索引 / Original item index
    pub item_index: usize,
    /// 物品 / Item
    pub item: ActualItem<V, U>,
    /// 放置位置 / Placement position
    pub position: MetricPoint3<V, U>,
    /// 朝向 / Orientation
    pub orientation: Orientation,
    /// 包装形状 / Packing shape
    pub packing_shape: PackingShape3<V, U>,
    /// 装载顺序 / Loading order
    pub loading_order: u64,
}

impl<V, U> PackedItem<V, U>
where
    V: Field + Clone + Debug + Send + Sync + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    /// 实际体积 / Actual volume
    ///
    /// 圆柱使用 πr²h，长方体使用 w*h*d。
    /// Cylinders use πr²h; cuboids use w*h*d.
    pub fn actual_volume(&self) -> Quantity<V, U> {
        self.packing_shape.actual_volume.clone()
    }
}

// ============================================================================
// PackedBin - 已装箱 / Packed bin
// ============================================================================

/// 已装箱 / Packed bin
///
/// 一个箱的完整装箱结果。
/// Complete packing result for a single bin.
#[derive(Debug, Clone)]
pub struct PackedBin<V, U: UnitTrait> {
    /// 箱名称 / Bin name
    pub name: String,
    /// 箱型 / Bin type
    pub bin_type: BinType<V, U>,
    /// 批号 / Batch number
    pub batch_no: Option<String>,
    /// 已装箱物品列表 / Packed item list
    pub items: Vec<PackedItem<V, U>>,
}

impl<V, U> PackedBin<V, U>
where
    V: Field + Clone + Debug + Send + Sync + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    /// 总重量 / Total weight
    pub fn total_weight(&self) -> Quantity<V, U> {
        let mut total = V::zero();
        for item in &self.items {
            total = total + item.item.weight.value.clone();
        }
        Quantity::new_ct(total)
    }

    /// 总实际体积 / Total actual volume
    pub fn total_actual_volume(&self) -> Quantity<V, U> {
        let mut total = V::zero();
        for item in &self.items {
            total = total + item.actual_volume().value.clone();
        }
        Quantity::new_ct(total)
    }
}

// ============================================================================
// MaterialSummary - 物料汇总 / Material summary
// ============================================================================

/// 物料汇总 / Material summary
///
/// 汇总一个物料在所有箱中的使用量。
/// Summary of a material's usage across all bins.
#[derive(Debug, Clone)]
pub struct MaterialSummary {
    /// 物料标识 / Material key
    pub material: MaterialKey,
    /// 使用量 / Amount used
    pub amount: u64,
}

// ============================================================================
// MaterialPackingPlan - 物料装箱计划 / Material packing plan
// ============================================================================

/// 物料装箱计划 / Material packing plan
///
/// 描述一个物料的装箱策略和用量。
/// Describes a material's packing strategy and usage.
#[derive(Debug, Clone)]
pub struct MaterialPackingPlan {
    /// 物料标识 / Material key
    pub material: MaterialKey,
    /// 需求数量 / Required amount
    pub demand: u64,
    /// 已分配数量 / Allocated amount
    pub allocated: u64,
    /// 物料属性 / Material attribute
    pub attribute: MaterialAttribute,
}

/// 物料属性 / Material attribute
///
/// 物料在装箱场景中的关键属性。
/// Key attributes of a material in the packing context.
#[derive(Debug, Clone, Default)]
pub struct MaterialAttribute {
    /// 最大层数 / Maximum stacking layers
    pub max_layer: Option<u64>,
    /// 最大高度 / Maximum stacking height
    pub max_height: Option<f64>,
}

/// 装箱数量 / Material packing numbers
#[derive(Debug, Clone, Default)]
pub struct MaterialPackingNumbers {
    /// 已装箱数量 / Packed count
    pub packed: u64,
    /// 剩余数量 / Remaining count
    pub remaining: u64,
}

// ============================================================================
// PackageSolutionLikeAdapter - 包装解适配器 / Package solution-like adapter
// ============================================================================

/// 包装解适配器 / Package solution-like adapter
///
/// 将装箱结果适配为通用的包装解接口。
/// Adapts packing results to a generic package solution interface.
#[derive(Debug, Clone)]
pub struct PackageSolutionLikeAdapter<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
    /// 物料汇总 / Material summary
    pub material_summaries: Vec<MaterialSummary>,
}

impl<V, U> PackageSolutionLikeAdapter<V, U>
where
    V: Field + Clone + Debug + Send + Sync + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    /// 从装箱结果创建适配器 / Create adapter from packing result
    pub fn from_packing_result(
        packed_bins: Vec<PackedBin<V, U>>,
        material_summaries: Vec<MaterialSummary>,
    ) -> Self {
        Self {
            packed_bins,
            material_summaries,
        }
    }

    /// 已使用箱数量 / Number of used bins
    pub fn bin_count(&self) -> usize {
        self.packed_bins.len()
    }

    /// 总物品数量 / Total item count
    pub fn total_item_count(&self) -> usize {
        self.packed_bins.iter().map(|b| b.items.len()).sum()
    }
}
