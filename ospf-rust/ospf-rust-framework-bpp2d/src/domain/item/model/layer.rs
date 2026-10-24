//! 层模型 / Layer model
//!
//! 对应 BPP3D 的 `BinLayer`：层是一块板材上的排样单元，由若干二维放置组成，能给出件数、实际占用面积
//! 与利用率。`SheetLayer` 组合多个层，对应 BPP3D 的 `Bin`。
//! Mirrors BPP3D `BinLayer`: a layer is a packing unit on one sheet made of 2D placements, exposing amounts,
//! actually occupied area and utilisation. `SheetLayer` combines layers, mirroring BPP3D `Bin`.

use std::collections::BTreeMap;

use ospf_rust_framework_geometry2d::{
    Placement2, Sheet2Shape, quantity_sum, quantity_zero_of, area_of,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;

use super::id::{BatchNo, ItemId, SheetTypeId};
use super::sheet::SheetType;

// ============================================================================
// Layer - 二维层 / 2D layer
// ============================================================================

/// 二维层：一块板材上的一组放置 / 2D layer: a group of placements on a sheet
///
/// `sheet` 参与列身份：同一排布放在不同板材类型上是不同的列。
/// `sheet` takes part in column identity: the same layout on different sheet types is a different column.
#[derive(Debug, Clone)]
pub struct Layer<V, U: UnitTrait> {
    /// 生成迭代 / Generation iteration
    pub iteration: i64,
    /// 生成来源 / Generation source
    pub from: String,
    /// 所属板材类型 / Owning sheet type
    pub sheet: Option<SheetType<V, U>>,
    /// 层的包围形状 / Bounding shape of the layer
    pub shape: Sheet2Shape<V, U>,
    /// 放置列表 / Placement list
    pub units: Vec<Placement2<V, U>>,
}

impl<V, U> Layer<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    /// 创建层 / Create a layer
    pub fn new(
        iteration: i64,
        from: impl Into<String>,
        sheet: Option<SheetType<V, U>>,
        shape: Sheet2Shape<V, U>,
    ) -> Self {
        Self {
            iteration,
            from: from.into(),
            sheet,
            shape,
            units: Vec::new(),
        }
    }

    /// 设置放置列表 / Set the placement list
    pub fn with_units(mut self, units: Vec<Placement2<V, U>>) -> Self {
        self.units = units;
        self
    }

    /// 层内件数 / Amount of items in the layer
    pub fn amount(&self) -> u64 {
        self.units.len() as u64
    }

    /// 层内实际占用面积 / Actually occupied area of the layer
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let areas: Vec<_> = self.units.iter().map(|placement| placement.area()).collect();
        quantity_sum(areas).unwrap_or_else(|| {
            // 空层退回零面积，单位取板材宽度的单位，避免出现无单位量。
            // An empty layer falls back to zero area using the sheet width's unit so no unitless quantity
            // leaks out.
            quantity_zero_of(&self.shape.width).to_runtime()
        })
    }

    /// 层的外接矩形面积 / Enclosing rectangle area of the layer
    pub fn bounding_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        self.shape.area()
    }

    /// 层的件数映射 / Item amount map of the layer
    pub fn item_amounts(&self) -> BTreeMap<ItemId, u64> {
        let mut counter: BTreeMap<ItemId, u64> = BTreeMap::new();
        for placement in &self.units {
            if let Some(item_id) = &placement.item_id {
                *counter.entry(ItemId::new(item_id.clone())).or_insert(0) += 1;
            }
        }
        counter
    }

    /// 层内各形状类型的件数 / Counts per shape type inside the layer
    pub fn shape_counts(
        &self,
    ) -> std::collections::HashMap<ospf_rust_framework_geometry2d::PackingShape2Type, u64> {
        let mut counter: std::collections::HashMap<
            ospf_rust_framework_geometry2d::PackingShape2Type,
            u64,
        > = std::collections::HashMap::new();
        for placement in &self.units {
            *counter.entry(placement.shape.shape_type()).or_insert(0) += 1;
        }
        counter
    }

    /// 层利用率：实际占用面积与外接矩形面积之比 / Layer utilisation: occupied area over enclosing area
    ///
    /// 分母非正时返回 0，避免除零。
    /// Returns 0 when the denominator is non-positive, avoiding division by zero.
    pub fn utilization(&self) -> f64 {
        let bounding = self.bounding_area();
        let bounding_value = bounding.value.to_f64().unwrap_or(0.0);
        if bounding_value <= 0.0 {
            return 0.0;
        }
        let occupied = self.area().value.to_f64().unwrap_or(0.0);
        occupied / bounding_value
    }

    /// 列身份键 / Column identity key
    ///
    /// 板材类型必须参与列身份：同一排布放在不同板材上是不同的列，否则多板材类型会被错误去重。
    /// 键同时纳入来源、包围尺寸与放置集合，避免把不同排布当成同一列。
    /// The sheet type must take part in column identity: the same layout on different sheets is a different
    /// column, otherwise multiple sheet types would be deduplicated incorrectly. The key also covers the
    /// source, the bounding size and the placement set so that different layouts are never treated as one.
    pub fn identity_key(&self) -> String {
        let sheet_code = self
            .sheet
            .as_ref()
            .map(|sheet| sheet.type_code.as_str().to_string())
            .unwrap_or_default();
        let mut placements: Vec<String> = self
            .units
            .iter()
            .map(|placement| {
                format!(
                    "{:?}|{:?}|{}|{:?}",
                    placement.position.x.value,
                    placement.position.y.value,
                    placement.rotated,
                    placement.item_id
                )
            })
            .collect();
        placements.sort();
        format!(
            "{}|{}|{:?}x{:?}|{}",
            sheet_code,
            self.from,
            self.shape.width.value,
            self.shape.height.value,
            placements.join(",")
        )
    }

    /// 面积口径检查：层面积不得超过板材面积 / Area sanity check: layer area must not exceed the sheet area
    ///
    /// 返回超出量（板材面积 − 层面积）；未绑定板材或存在富余时返回正数。
    /// Returns the slack (sheet area − layer area); positive when a sheet is bound and there is room left.
    pub fn sheet_area_slack(&self) -> Option<f64> {
        let sheet = self.sheet.as_ref()?;
        let sheet_area = sheet.area().value.to_f64()?;
        let layer_area = self.area().value.to_f64()?;
        Some(sheet_area - layer_area)
    }
}

// ============================================================================
// SheetLayer - 板材层组合 / Sheet layer combination
// ============================================================================

/// 板材层组合：某种板材类型加排在其上的多个层 / The sheet plus the layers laid out on it
#[derive(Debug, Clone)]
pub struct SheetLayer<V, U: UnitTrait> {
    /// 板材类型 / Sheet type
    pub sheet_type: SheetType<V, U>,
    /// 组合后的包围形状 / Combined bounding shape
    pub shape: Sheet2Shape<V, U>,
    /// 层列表 / Layer list
    pub layers: Vec<Layer<V, U>>,
    /// 批次号 / Batch number
    pub batch: Option<BatchNo>,
}

impl<V, U> SheetLayer<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    /// 创建板材层组合 / Create a sheet layer combination
    pub fn new(
        sheet_type: SheetType<V, U>,
        shape: Sheet2Shape<V, U>,
        layers: Vec<Layer<V, U>>,
    ) -> Self {
        Self {
            sheet_type,
            shape,
            layers,
            batch: None,
        }
    }

    /// 组合内件数 / Amount inside the combination
    pub fn amount(&self) -> u64 {
        self.layers
            .iter()
            .map(|layer| layer.units.len() as u64)
            .sum()
    }

    /// 组合内实际占用面积 / Actually occupied area of the combination
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let areas: Vec<_> = self.layers.iter().map(|layer| layer.area()).collect();
        quantity_sum(areas)
            .unwrap_or_else(|| quantity_zero_of(&self.shape.width).to_runtime())
    }

    /// 板材装载率 / Sheet loading rate
    pub fn loading_rate(&self) -> f64 {
        let sheet_area = self.sheet_type.area().value.to_f64().unwrap_or(0.0);
        if sheet_area <= 0.0 {
            return 0.0;
        }
        self.area().value.to_f64().unwrap_or(0.0) / sheet_area
    }

    /// 组合内件数映射 / Item amount map of the combination
    pub fn item_amounts(&self) -> BTreeMap<ItemId, u64> {
        let mut counter: BTreeMap<ItemId, u64> = BTreeMap::new();
        for layer in &self.layers {
            for (item_id, amount) in layer.item_amounts() {
                *counter.entry(item_id).or_insert(0) += amount;
            }
        }
        counter
    }

    /// 组合身份键 / Combination identity key
    ///
    /// 由各层列身份键构成，任一层不同即整体不同，与 Kotlin 的层列表等价语义一致。
    /// Built from each layer's column identity key: any differing layer makes the whole combination
    /// different, matching the Kotlin layer-list equality semantics.
    pub fn identity_key(&self) -> String {
        let mut keys: Vec<String> = self.layers.iter().map(|layer| layer.identity_key()).collect();
        keys.sort();
        format!("{}|{}", self.sheet_type.type_code, keys.join(";"))
    }
}

/// 单层面积 / Single layer area
///
/// 转发到几何 crate，保证层面积与放置面积口径一致。
/// Forwards to the geometry crate so layer area matches placement area.
pub fn layer_area<V, U>(layer: &Layer<V, U>) -> Quantity<V, ospf_rust_quantities::unit::Unit>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    layer.area()
}

/// 放置面积 / Placement area
pub fn placement_area<V, U>(
    placement: &Placement2<V, U>,
) -> Quantity<V, ospf_rust_quantities::unit::Unit>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    area_of(&placement.shape)
}

/// 板材类型编码比较辅助 / Sheet type code comparison helper
pub fn same_sheet_type<V, U: UnitTrait>(
    lhs: &SheetType<V, U>,
    rhs: &SheetType<V, U>,
) -> bool {
    lhs.type_code == rhs.type_code
}

/// 板材类型编码 / Sheet type code of an optional sheet type
pub fn sheet_type_code<V, U: UnitTrait>(sheet: Option<&SheetType<V, U>>) -> Option<SheetTypeId> {
    sheet.map(|sheet| sheet.type_code.clone())
}
