//! 二维 BLA 领域 / 2D BLA domain
//!
//! 映射 Kotlin `bpp2d-domain-bla-context`。圆形件不参与 BLA，会被显式跳过。
//! Maps the Kotlin `bpp2d-domain-bla-context` submodule. Circle items do not take part in BLA and are
//! skipped explicitly.

pub mod service;

pub use service::*;

use ospf_rust_framework_geometry2d::Sheet2Shape;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::model::ActualItem;

/// BLA 领域上下文 / BLA domain context
#[derive(Debug, Clone)]
pub struct BlaContext<V, U: UnitTrait> {
    /// 板材形状 / Sheet shape
    pub sheet: Sheet2Shape<V, U>,
    /// 算法配置 / Algorithm configuration
    pub config: BlaConfig,
    /// 刀缝间距 / Kerf spacing
    pub kerf: Option<Quantity<V, U>>,
}

impl<V, U> BlaContext<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建上下文 / Create the context
    pub fn new(sheet: Sheet2Shape<V, U>, kerf: Option<Quantity<V, U>>) -> Self {
        Self {
            sheet,
            config: BlaConfig::default(),
            kerf,
        }
    }

    /// 执行 BLA 排样 / Run the BLA layout
    pub fn layout(
        &self,
        items: &[(ActualItem<V, U>, usize)],
    ) -> ospf_rust_base::error::Ret<Bla2dLayoutResult<V, U>> {
        BottomUpLeftJustifiedAlgorithm::new(
            self.sheet.clone(),
            self.config.clone(),
            self.kerf.clone(),
        )
        .invoke(items)
    }
}
