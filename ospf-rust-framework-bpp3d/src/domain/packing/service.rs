//! 装箱服务 / Packing services
//!
//! 最终装箱、几何守卫和渲染适配。
//! Final packing, geometry guard, and renderer adapter.

use std::collections::HashMap;
use std::fmt::Debug;

use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::{ActualItem, BinLayer, BinType, Bpp3dDemandKey, MaterialKey};
use crate::domain::layer_generation::{LayerBlockTrace, LayerPlacementTrace};
use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::PackingShapeType;
use crate::infrastructure::pwl_approximation::{
    HorizontalCylinderSupportGeometry, horizontal_cylinder_cuboid_support_coverage,
};
use crate::infrastructure::renderer::RenderLoadingPlanDto;

use super::model::{MaterialSummary, PackedBin, PackedItem};

include!("service/geometry_contract.rs");
include!("service/geometry_guard.rs");
include!("service/layer_placement_adapter.rs");
include!("service/layer_trace_replay_adapter.rs");
include!("service/packer.rs");
include!("service/material_packer.rs");
include!("service/renderer_adapter.rs");
include!("service/tests.rs");
