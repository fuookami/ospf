//! 板材模型 / Sheet model
//!
//! 对应 BPP3D 的 `Bin` / `BinType`：板材类型描述面积产能与承重，板材实例承载已排样的层。
//! Mirrors BPP3D `Bin` / `BinType`: a sheet type describes area capacity and the load limit, while a sheet
//! instance carries the placed layers.

use ospf_rust_framework_geometry2d::{
    Sheet2Shape, area_of, quantity_multiply,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;

use super::id::{BatchNo, SheetTypeId};

// ============================================================================
// SheetType - 板材类型 / Sheet type
// ============================================================================

/// 板材类型描述 / Sheet type descriptor
///
/// 板材类型参与列身份：同一排布放在不同板材类型上是不同的列。
/// The sheet type takes part in column identity: the same layout on different sheet types is a different
/// column.
#[derive(Debug, Clone)]
pub struct SheetType<V, U: UnitTrait> {
    /// 板材宽度 / Sheet width
    pub width: Quantity<V, U>,
    /// 板材高度 / Sheet height
    pub height: Quantity<V, U>,
    /// 最大承重 / Maximum load
    pub max_load: Quantity<V, U>,
    /// 板材类型编码 / Sheet type code
    pub type_code: SheetTypeId,
    /// 是否为主板材 / Whether this is the main sheet type
    pub is_main: bool,
}

impl<V, U> SheetType<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    /// 创建板材类型 / Create a sheet type
    pub fn new(
        width: Quantity<V, U>,
        height: Quantity<V, U>,
        max_load: Quantity<V, U>,
        type_code: SheetTypeId,
    ) -> Self {
        Self {
            width,
            height,
            max_load,
            type_code,
            is_main: false,
        }
    }

    /// 标记为主板材 / Mark as the main sheet type
    pub fn with_main(mut self, is_main: bool) -> Self {
        self.is_main = is_main;
        self
    }

    /// 板材面积 / Sheet area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        quantity_multiply(&self.width, &self.height)
    }

    /// 对应的板材几何 / The corresponding sheet geometry
    pub fn geometry(&self) -> Sheet2Shape<V, U> {
        Sheet2Shape::new(self.width.clone(), self.height.clone())
    }
}

// ============================================================================
// Sheet - 板材实例 / Sheet instance
// ============================================================================

/// 板材实例：某种板材类型加其上排好的层 / Sheet instance: a sheet type plus the layers laid out on it
#[derive(Debug, Clone)]
pub struct Sheet<V, U: UnitTrait> {
    /// 板材类型 / Sheet type
    pub sheet_type: SheetType<V, U>,
    /// 已排样的层 / Placed layers
    pub layers: Vec<crate::domain::item::model::layer::SheetLayer<V, U>>,
    /// 批次号 / Batch number
    pub batch: Option<BatchNo>,
}

impl<V, U> Sheet<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    /// 创建板材实例 / Create a sheet instance
    pub fn new(sheet_type: SheetType<V, U>) -> Self {
        Self {
            sheet_type,
            layers: Vec::new(),
            batch: None,
        }
    }

    /// 设置层列表 / Set the layer list
    pub fn with_layers(
        mut self,
        layers: Vec<crate::domain::item::model::layer::SheetLayer<V, U>>,
    ) -> Self {
        self.layers = layers;
        self
    }

    /// 设置批次号 / Set the batch number
    pub fn with_batch(mut self, batch: BatchNo) -> Self {
        self.batch = Some(batch);
        self
    }

    /// 板材宽度 / Sheet width
    pub fn width(&self) -> Quantity<V, U> {
        self.sheet_type.width.clone()
    }

    /// 板材高度 / Sheet height
    pub fn height(&self) -> Quantity<V, U> {
        self.sheet_type.height.clone()
    }

    /// 板材面积 / Sheet area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        self.sheet_type.area()
    }
}

// ============================================================================
// 用量辅助 / Usage helpers
// ============================================================================

/// 板材装载率 / Sheet loading rate
///
/// 实际占用面积与板材面积之比；板材面积非正时返回 0。
/// The ratio of actually occupied area to sheet area; returns 0 when the sheet area is non-positive.
pub fn sheet_loading_rate<V, U>(
    occupied_area: &Quantity<V, ospf_rust_quantities::unit::Unit>,
    sheet_type: &SheetType<V, U>,
) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    let sheet_area = sheet_type.area();
    if sheet_area.value <= V::from(0.0).unwrap_or_else(V::zero) {
        return 0.0;
    }
    let occupied = occupied_area.value.to_f64().unwrap_or(0.0);
    let total = sheet_area.value.to_f64().unwrap_or(0.0);
    if total <= 0.0 {
        return 0.0;
    }
    occupied / total
}

/// 形状面积 / Shape area
///
/// 转发到几何 crate，保证单件面积口径与装箱、KPI 完全一致。
/// Forwards to the geometry crate so that per-piece area stays identical across packing and KPI.
pub fn shape_area<V, U>(
    shape: &ospf_rust_framework_geometry2d::PackingShape2<V, U>,
) -> Quantity<V, ospf_rust_quantities::unit::Unit>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    area_of(shape)
}
