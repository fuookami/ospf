//! 二维装箱领域 / 2D packing domain
//!
//! 映射 Kotlin `bpp2d-domain-packing-context`：终态几何校验、装箱器、余料报告与渲染适配。
//! Maps the Kotlin `bpp2d-domain-packing-context` submodule: final geometry validation, the packer, remnant
//! reporting and render adaptation.

pub mod aggregation;
pub mod service;

pub use aggregation::{
    MaterialSummary, PackedItem, PackedLayer, PackedSheet, PackingAggregation, PackingResult,
    area_value, flatten_packed_items, loading_rate_of, packed_layers_from_sheet_layers,
};
pub use service::*;

use ospf_rust_framework_geometry2d::Sheet2Shape;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

/// 装箱上下文 / Packing context
#[derive(Debug, Clone, Default)]
pub struct PackingContext {
    /// 上下文名称 / Context name
    pub name: Option<String>,
}

impl PackingContext {
    /// 创建上下文 / Create the context
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置名称 / Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// 由板材类型构造板材几何 / Build sheet geometry from a sheet type
pub fn sheet_geometry_of<V, U>(
    sheet_type: &crate::domain::item::model::SheetType<V, U>,
) -> Sheet2Shape<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    Sheet2Shape::new(sheet_type.width.clone(), sheet_type.height.clone())
}

/// 零刀缝样本 / Zero kerf sample
pub fn zero_kerf<V, U>(sample: &Quantity<V, U>) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    ospf_rust_framework_geometry2d::quantity_zero_of(sample)
}
