//! 二维装箱领域框架 / BPP2D domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-bpp2d` 的 Rust 迁移：矩形与圆形件的二维装箱、
//! 刀缝、余料报告与多板材类型加权目标。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-bpp2d`: 2D packing of rectangular
//! and circular items, kerf, remnant reporting and weighted objectives across multiple sheet types.
//!
//! 二维几何位于共享 crate `ospf-rust-framework-geometry2d`，与 CSP2D 套裁共用。
//! The 2D geometry lives in the shared crate `ospf-rust-framework-geometry2d` and is shared with CSP2D.

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use infrastructure::{
    Bpp2dCapabilityError, Bpp2dInternalError, Bpp2dResult, Bpp2dSolvingError, Bpp2dValidationError,
    RenderLayerDto, RenderLoadingPlanDto, RenderPlacementDto, RenderShape2Type, RenderSheetDto,
    capability_error, internal_error, solving_error, validation_error,
};
