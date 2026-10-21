//! 装箱分析器 / Packing analyzer
//!
//! 由列生成结果构造装箱结果与渲染 DTO，作为应用服务的最后一步。
//! Builds the packing result and the render DTO from the column-generation result, as the final step of the
//! application service.

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::Sheet2Shape;

use crate::application::state::ColumnGenerationResult;
use crate::domain::packing::{
    PackingAggregation, PackingRendererAdapter, PackingResult, Packer,
};

/// 装箱分析器 / Packing analyzer
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationPackingAnalyzer;

impl ColumnGenerationPackingAnalyzer {
    /// 创建分析器 / Create the analyzer
    pub fn new() -> Self {
        Self
    }

    /// 由列生成结果构造装箱结果 / Build the packing result from the column-generation result
    pub fn analyze<V, U>(
        &self,
        result: &ColumnGenerationResult<V, U>,
        kerf: Option<&ospf_rust_quantities::quantity::Quantity<V, U>>,
    ) -> Ret<PackingResult<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        Packer::new().invoke(
            &result.sheets,
            &PackingAggregation::new("bpp2d"),
            kerf,
        )
    }

    /// 由装箱结果生成渲染 DTO / Build the render DTO from the packing result
    pub fn render<V, U>(&self, packing: &PackingResult<V, U>) -> crate::infrastructure::dto::RenderLoadingPlanDto
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        PackingRendererAdapter::new().to_render(packing)
    }

    /// 板材几何 / Sheet geometry of a result
    pub fn sheet_geometry_of<V, U>(result: &ColumnGenerationResult<V, U>) -> Option<Sheet2Shape<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        let sheet = result.sheets.first()?;
        Some(Sheet2Shape::new(
            sheet.sheet_type.width.clone(),
            sheet.sheet_type.height.clone(),
        ))
    }
}
