//! 渲染适配 / Render adaptation
//!
//! 把装箱结果转换为渲染 DTO。圆形件输出半径，矩形件输出宽高，与 Kotlin 的
//! `PackingRendererAdapter` 口径一致。
//! Converts a packing result into render DTOs. Circle items emit a radius while rectangles emit width and
//! height, matching the Kotlin `PackingRendererAdapter` convention.

use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::packing::aggregation::PackingResult;
use crate::infrastructure::dto::{
    RenderLayerDto, RenderLoadingPlanDto, RenderPlacementDto, RenderShape2Type, RenderSheetDto,
};

/// 渲染适配器 / Render adapter
#[derive(Debug, Clone, Default)]
pub struct PackingRendererAdapter;

impl PackingRendererAdapter {
    /// 创建适配器 / Create the adapter
    pub fn new() -> Self {
        Self
    }

    /// 转换装箱结果为渲染 DTO / Convert a packing result into a render DTO
    pub fn to_render<V, U>(&self, result: &PackingResult<V, U>) -> RenderLoadingPlanDto
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let sheets = result
            .sheets
            .iter()
            .map(|sheet| RenderSheetDto {
                sheet_type_code: sheet.sheet_type.type_code.as_str().to_string(),
                width: sheet.sheet_type.width.value.to_f64().unwrap_or(0.0),
                height: sheet.sheet_type.height.value.to_f64().unwrap_or(0.0),
                area: sheet.sheet_area(),
                loading_rate: sheet.loading_rate,
                layers: sheet
                    .layers
                    .iter()
                    .map(|packed_layer| RenderLayerDto {
                        source: packed_layer.layer.from.clone(),
                        sheet_type_code: packed_layer
                            .layer
                            .sheet
                            .as_ref()
                            .map(|sheet_type| sheet_type.type_code.as_str().to_string()),
                        width: packed_layer.layer.shape.width.value.to_f64().unwrap_or(0.0),
                        height: packed_layer.layer.shape.height.value.to_f64().unwrap_or(0.0),
                        area: packed_layer.layer.area().value.to_f64().unwrap_or(0.0),
                        amount: packed_layer.layer.units.len() as u64,
                        placements: packed_layer
                            .items
                            .iter()
                            .map(|item| Self::placement_dto(&item.placement))
                            .collect(),
                    })
                    .collect(),
            })
            .collect();

        RenderLoadingPlanDto {
            sheets,
            total_amount: result.total_amount() as u64,
            total_area: result.occupied_area(),
            total_waste_area: result.remnant_area(),
        }
    }

    /// 转换单个放置 / Convert a single placement
    fn placement_dto<V, U>(
        placement: &ospf_rust_framework_geometry2d::Placement2<V, U>,
    ) -> RenderPlacementDto
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let shape_type = RenderShape2Type::from(placement.shape.shape_type());
        let radius = match &placement.shape {
            ospf_rust_framework_geometry2d::PackingShape2::Circle(circle) => {
                circle.radius.value.to_f64()
            }
            _ => None,
        };
        RenderPlacementDto {
            item_id: placement.item_id.clone(),
            x: placement.x().value.to_f64().unwrap_or(0.0),
            y: placement.y().value.to_f64().unwrap_or(0.0),
            width: placement.shape.bounding_width().value.to_f64().unwrap_or(0.0),
            height: placement.shape.bounding_height().value.to_f64().unwrap_or(0.0),
            radius,
            shape_type,
            rotated: placement.rotated,
        }
    }
}
