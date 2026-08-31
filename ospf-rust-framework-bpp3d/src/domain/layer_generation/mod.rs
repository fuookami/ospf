//! 层生成上下文 / Layer generation context
//!
//! 映射 Kotlin `bpp3d-domain-layer-generation-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-layer-generation-context` submodule.
//!
//! # 核心组件 / Core Components
//!
//! - `LayerGenerationRequest`: 层生成请求 / Layer generation request
//! - `LayerGenerationResult`: 层生成结果 / Layer generation result
//! - `LayerGenerator`: 层生成器 trait / Layer generator trait
//! - `LayerGenerationContext`: 层生成上下文（组合多个生成器）/ Layer generation context (composite of generators)
//! - `LayerGenerationDemandEntry`: 层生成需求条目 / Layer generation demand entry

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use num_traits::ToPrimitive;
use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::bla::BottomUpLeftJustifiedAlgorithm;
use crate::domain::bla::service::{BlaConfig, BlaProjection};
use crate::domain::block_loading::{
    Block, BlockPlacement, ComplexBlockGenerator, DepthFirstSearchAlgorithm,
    MultiLayerHeuristicSearchAlgorithm, SimpleBlockGenerator, SimpleBlockGeneratorConfig,
};
use crate::domain::item::{
    ActualItem, BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dLayerDemandCoverage,
    CylinderShapeContract, ItemId, PackageAttribute, PackageOrientationRuleInput,
    PackagePlacementBottomContext, PackagePlacementStackingInput, PackageShapeSpec,
    PackageStackingInput, PatternConfig, PatternNextPointPolicy, PatternProjectionOrientation,
    PatternStep,
};
use crate::domain::layer_assignment::DemandShadowPriceKey;
use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
use crate::infrastructure::orientation::Orientation;

include!("model.rs");
include!("context.rs");
include!("block_layer_generator.rs");
include!("bl_layer_generator.rs");
include!("circle_packing_layer_generator.rs");
include!("deferred_generators.rs");
include!("pattern_generator.rs");
include!("pile_generator.rs");
include!("historical_generator.rs");
include!("scoring.rs");
include!("horizontal_cylinder_guard.rs");
include!("tests.rs");
