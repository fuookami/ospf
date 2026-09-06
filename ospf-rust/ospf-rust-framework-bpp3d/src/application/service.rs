//! BPP3D 应用服务 / BPP3D application services

#[cfg(feature = "serde")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
#[cfg(feature = "serde")]
use std::path::Path;
use std::time::{Duration, Instant};

use ospf_rust_core::model::meta_model::MetaModel;
use ospf_rust_framework::model::{
    BasicShadowPriceMap, DynamicColumnContext, DynamicModelLifecycle, ShadowPriceKey,
    ShadowPriceMap, extract_shadow_price,
};
#[cfg(not(feature = "async"))]
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};
use ospf_rust_math::algebra::Field;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::{
    ActualItem, BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dLayerDemandCoverage,
    ContinuousCylinderRadiusSolution, ContinuousRadiusModelComponent, PackageAttribute,
};
use crate::domain::layer_assignment::{
    BinAmountMinimization, BinCapacityConstraint, BinDepthConstraint, Bpp3dDemandEntry,
    Bpp3dModelComponent, Capacity, DemandConstraint, DemandShadowPriceKey, ImpreciseAssignment,
    IterativeLayerAssignmentContext, LayerAggregation, LayerAssignmentAggregation,
    LayerAssignmentContext, Load, PreciseAssignment, PreciseAssignmentActivationConstraint,
    SolutionExtractor, VolumeMinimization,
};
use crate::domain::layer_generation::{
    BLGlobalLayerGenerator, BLLocalLayerGenerator, BlockLayerGenerator,
    CirclePackingLayerGenerator, HistoricalLayerGenerator, LayerBlockTrace, LayerGenerationContext,
    LayerGenerationDemandEntry, LayerGenerationRequest, LayerGenerationResult, LayerPlacementTrace,
    PatternLayerGenerator, PileLayerGenerator,
};
use crate::domain::packing::{
    LayerTraceReplayAdapter, PackedBin, Packer, PackingGeometryContract, PackingGeometryGuard,
    PackingRendererAdapter, PackingResult,
};
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::renderer::RenderLoadingPlanDto;

include!("service/config.rs");
include!("service/state.rs");
include!("service/result.rs");
include!("service/dataset_suite.rs");
include!("service/layer_quality.rs");
include!("service/fixture_suite.rs");
include!("service/executor.rs");
include!("service/reporting_helpers.rs");
include!("service/mock_executor.rs");
include!("service/algorithm.rs");
include!("service/standard_executors.rs");
include!("service/depth_boundary.rs");
include!("service/analysis.rs");
include!("service/application_service.rs");
include!("service/continuous_radius.rs");
include!("service/tests.rs");
