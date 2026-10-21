//! BPP2D 基础设施 / BPP2D infrastructure
//!
//! 提供错误类型与渲染 DTO。二维几何本身位于共享 crate
//! `ospf-rust-framework-geometry2d`，本层只做再导出，避免出现两份几何实现。
//! Provides error types and render DTOs. The 2D geometry itself lives in the shared crate
//! `ospf-rust-framework-geometry2d`; this layer only re-exports it, so no second geometry implementation
//! can drift.

pub mod dto;
pub mod error;

pub use dto::{RenderLayerDto, RenderLoadingPlanDto, RenderPlacementDto, RenderShape2Type, RenderSheetDto};
pub use error::{
    Bpp2dCapabilityError, Bpp2dInternalError, Bpp2dResult, Bpp2dSolvingError, Bpp2dValidationError,
    capability_error, internal_error, solving_error, validation_error,
};

// 几何再导出：使用方只需依赖 bpp2d 即可获得二维几何类型。
// Geometry re-exports: a consumer only needs bpp2d to reach the 2D geometry types.
pub use ospf_rust_framework_geometry2d::{
    Box2Geometry, CirclePackingShape2, Placement2, PackingShape2, PackingShape2Type, QuantityPoint2,
    QuantityVector2, RectanglePackingShape2, Shape2Candidate, Sheet2Geometry, Sheet2Shape,
    area_of, bounding_area, bounding_height, bounding_width, candidate_shapes, circle_shape2,
    count_fitting, inflated_by_placement, inflated_by_shape, inside_sheet, max_x, max_y, min_x,
    min_y, origin_box, pitch_of, point2_of, quantity_greater, quantity_less, quantity_multiply,
    quantity_plus_safe, quantity_sum, quantity_zero_of, rectangle_shape2, scaled_quantity,
    sheet2_shape, vector2_of, zero_spacing_of,
};
