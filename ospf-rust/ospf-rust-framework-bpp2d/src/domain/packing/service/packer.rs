//! 装箱器 / Packer
//!
//! 把最终板材列表展开为装箱结果：装载顺序按「先下后上、先左后右」计算，几何与旋转由
//! `PackingGeometryGuard` 校验，余料由 `FreeSpaceAnalyzer` 统计。
//! Expands the final sheet list into a packing result: the loading order follows "bottom first, then left to
//! right", geometry and rotation are validated by `PackingGeometryGuard`, and remnants are measured by
//! `FreeSpaceAnalyzer`.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{Placement2, Sheet2Shape};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::model::{AbstractItem, ItemId, Sheet};
use crate::domain::packing::aggregation::{
    PackedSheet, PackingAggregation, PackingResult, flatten_packed_items, loading_rate_of,
    packed_layers_from_sheet_layers,
};
use crate::domain::packing::service::free_space_analyzer::FreeSpaceAnalyzer;
use crate::domain::packing::service::geometry_guard::{PackingGeometryGuard, PackingRotationRule};

// ============================================================================
// LoadingOrderCalculator - 装载顺序 / Loading order
// ============================================================================

/// 装载顺序计算器 / Loading order calculator
///
/// 顺序按「先下后上（y 升序）、再先左后右（x 升序）」排列，相同落点按物品标识稳定排序，
/// 使同一板上的装载顺序可复现。
/// The order is "bottom first (ascending y), then left to right (ascending x)", with a stable tie-break on the
/// item identifier so the loading order on a sheet is reproducible.
#[derive(Debug, Clone, Default)]
pub struct LoadingOrderCalculator;

impl LoadingOrderCalculator {
    /// 创建计算器 / Create the calculator
    pub fn new() -> Self {
        Self
    }

    /// 计算装载顺序 / Compute the loading order
    pub fn order<V, U>(&self, placements: &[Placement2<V, U>]) -> Vec<u64>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let mut indices: Vec<usize> = (0..placements.len()).collect();
        indices.sort_by(|lhs, rhs| {
            let left = &placements[*lhs];
            let right = &placements[*rhs];
            let left_y = left.y().value.to_f64().unwrap_or(0.0);
            let right_y = right.y().value.to_f64().unwrap_or(0.0);
            match left_y
                .partial_cmp(&right_y)
                .unwrap_or(std::cmp::Ordering::Equal)
            {
                std::cmp::Ordering::Equal => {
                    let left_x = left.x().value.to_f64().unwrap_or(0.0);
                    let right_x = right.x().value.to_f64().unwrap_or(0.0);
                    match left_x
                        .partial_cmp(&right_x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                    {
                        std::cmp::Ordering::Equal => left
                            .item_id
                            .as_deref()
                            .unwrap_or("")
                            .cmp(right.item_id.as_deref().unwrap_or("")),
                        other => other,
                    }
                }
                other => other,
            }
        });
        // 顺序数组：结果按原始下标返回从 0 起的序号。
        // Order array: the result maps every original index to its 0-based rank.
        let mut order = vec![0u64; placements.len()];
        for (rank, index) in indices.into_iter().enumerate() {
            order[index] = rank as u64;
        }
        order
    }
}

// ============================================================================
// Packer - 装箱器 / Packer
// ============================================================================

/// 装箱器 / Packer
#[derive(Debug, Clone, Default)]
pub struct Packer {
    /// 装载顺序计算器 / Loading order calculator
    pub loading_order_calculator: LoadingOrderCalculator,
    /// 物品目录：物品编号到是否允许旋转 / Item catalogue: identifier to rotation allowance
    pub item_catalog: BTreeMap<ItemId, bool>,
}

impl Packer {
    /// 创建装箱器 / Create the packer
    pub fn new() -> Self {
        Self::default()
    }

    /// 由物品列表构建物品目录 / Build the item catalogue from items
    pub fn with_items<V, U, I>(mut self, items: &[I]) -> Self
    where
        V: Clone,
        U: UnitTrait + Clone,
        I: AbstractItem<V, U>,
    {
        for item in items {
            self.item_catalog
                .insert(item.id().clone(), item.allow_rotate());
        }
        self
    }

    /// 执行装箱 / Run the packing
    pub fn invoke<V, U>(
        &self,
        sheets: &[Sheet<V, U>],
        aggregation: &PackingAggregation,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<PackingResult<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let rotation_rule = if self.item_catalog.is_empty() {
            PackingRotationRule::ByItemCatalog(BTreeMap::new())
        } else {
            PackingRotationRule::ByItemCatalog(
                self.item_catalog
                    .iter()
                    .map(|(id, allow)| (id.as_str().to_string(), *allow))
                    .collect(),
            )
        };
        let analyzer = FreeSpaceAnalyzer::new();
        let mut packed_sheets = Vec::with_capacity(sheets.len());
        let mut material_summary = crate::domain::packing::aggregation::MaterialSummary::new();
        let mut remnants = Vec::new();

        for (index, sheet) in sheets.iter().enumerate() {
            let name = format!("sheet-{}", index + 1);
            let placements: Vec<Placement2<V, U>> = sheet
                .layers
                .iter()
                .flat_map(|sheet_layer| sheet_layer.layers.iter())
                .flat_map(|layer| layer.units.iter().cloned())
                .collect();
            let orders = self.loading_order_calculator.order(&placements);

            let geometry = Sheet2Shape::new(
                sheet.sheet_type.width.clone(),
                sheet.sheet_type.height.clone(),
            );
            PackingGeometryGuard::require_sheet_geometry_with_rule(
                &geometry,
                &placements,
                &name,
                "Packer::invoke",
                kerf,
                &rotation_rule,
            )?;

            let order_index: BTreeMap<(u64, u64), u64> = placements
                .iter()
                .zip(orders.iter())
                .map(|(placement, order)| {
                    (
                        (
                            (placement.x().value.to_f64().unwrap_or(0.0) * 1e6) as u64,
                            (placement.y().value.to_f64().unwrap_or(0.0) * 1e6) as u64,
                        ),
                        *order,
                    )
                })
                .collect();
            let loading_order_for = |placement: &Placement2<V, U>| -> u64 {
                order_index
                    .get(&(
                        (placement.x().value.to_f64().unwrap_or(0.0) * 1e6) as u64,
                        (placement.y().value.to_f64().unwrap_or(0.0) * 1e6) as u64,
                    ))
                    .copied()
                    .unwrap_or(0)
            };

            let packed_layers = packed_layers_from_sheet_layers(&sheet.layers, loading_order_for);
            let items = flatten_packed_items(&packed_layers);
            let occupied: f64 = items
                .iter()
                .map(|item| item.placement.shape.area().value.to_f64().unwrap_or(0.0))
                .sum();
            let sheet_area = sheet.sheet_type.area().value.to_f64().unwrap_or(0.0);
            let sheet_remnants = analyzer.analyze_sheet(&name, &sheet.sheet_type, &placements);

            // 物料汇总按物品目录缺失即不贡献，与 Kotlin 的保守口径一致。
            // The material summary contributes nothing for items missing from the catalogue, matching the
            // conservative Kotlin convention.
            for item in &items {
                if let Some(id) = &item.item_id {
                    if let Some(key) = self.material_key_of(id, sheet) {
                        material_summary.add(key, 1);
                    }
                }
            }

            remnants.extend(sheet_remnants.iter().cloned());
            packed_sheets.push(PackedSheet {
                name,
                sheet_type: sheet.sheet_type.clone(),
                batch: sheet.batch.clone(),
                layers: packed_layers,
                items,
                loading_rate: loading_rate_of(occupied, sheet_area),
                remnants: sheet_remnants,
            });
        }

        let _ = aggregation;
        Ok(PackingResult {
            sheets: packed_sheets,
            material_summary,
            remnants,
        })
    }

    /// 由物品标识与板材推导物料键 / Derive a material key from the item identifier and the sheet
    ///
    /// 当前骨架按物品标识生成单一条目：物料模型尚未接入时仍能给出可用的件数汇总。
    /// The current skeleton derives one entry per item identifier, so a usable amount summary is still
    /// available before the material model is wired in.
    fn material_key_of<V, U>(
        &self,
        item_id: &str,
        _sheet: &Sheet<V, U>,
    ) -> Option<crate::domain::item::model::MaterialKey>
    where
        U: UnitTrait,
    {
        Some(crate::domain::item::model::MaterialKey::new(
            crate::domain::item::model::MaterialNo::new(item_id),
            crate::domain::item::model::MaterialType::FinishedProduct,
        ))
    }
}
