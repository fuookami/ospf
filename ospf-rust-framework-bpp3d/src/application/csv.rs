//! BPP3D CSV 数据集加载 / BPP3D CSV dataset loading
//!
//! 该模块只处理 application 协议边界，不注册 MetaModel。
//! This module only handles the application protocol boundary and does not register MetaModel.

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use csv::{ReaderBuilder, StringRecord, Trim};
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

use crate::domain::item::{
    ActualItem, BinLayer, BinType, ContinuousCylinderRadiusSolverPrototype,
    ContinuousRadiusModelComponent, ContinuousRadiusRegistrationPlan,
    ContinuousRadiusWeightFunction, PackageAttribute, PackageShapeSpec, PatternedItemKey,
};
use crate::infrastructure::orientation::Orientation;

const ITEMS_TABLE: &str = "items";
const BINS_TABLE: &str = "bins";
const LAYERS_TABLE: &str = "layers";
const DEPTH_BOUNDARY_POLICY_TABLE: &str = "depth_boundary_policy";
const RADIUS_WEIGHT_FUNCTIONS_TABLE: &str = "radius_weight_functions";

const ITEM_REQUIRED_COLUMNS: &[&str] = &[
    "item_id",
    "name",
    "shape_type",
    "width",
    "height",
    "depth",
    "weight",
    "amount",
];

const ITEM_OPTIONAL_COLUMNS: &[&str] = &[
    "package_code",
    "material_no",
    "material_name",
    "material_weight",
    "radius",
    "radius_min",
    "radius_max",
    "radius_step",
    "radius_weight_function_key",
    "axis",
    "enabled_orientations",
    "pattern_code",
    "allow_mixed_loading",
    "max_stack_layers",
    "package_tags",
];

const BIN_REQUIRED_COLUMNS: &[&str] = &[
    "bin_id",
    "type_code",
    "width",
    "height",
    "depth",
    "capacity",
];

const BIN_OPTIONAL_COLUMNS: &[&str] = &[
    "is_main",
    "batch_no",
];

const LAYER_REQUIRED_COLUMNS: &[&str] = &[
    "layer_id",
    "bin_id",
    "depth",
];

const LAYER_OPTIONAL_COLUMNS: &[&str] = &[
    "iteration",
    "from",
    "z",
];

const DEPTH_POLICY_REQUIRED_COLUMNS: &[&str] = &[
    "field",
    "values",
];

const DEPTH_POLICY_OPTIONAL_COLUMNS: &[&str] = &[];

const RADIUS_WEIGHT_FUNCTION_REQUIRED_COLUMNS: &[&str] = &[
    "key",
    "radius_squared_coefficient",
];

const RADIUS_WEIGHT_FUNCTION_OPTIONAL_COLUMNS: &[&str] = &[
    "intercept",
    "objective_weight",
];


include!("csv/error.rs");
include!("csv/records.rs");
include!("csv/dataset.rs");
include!("csv/request.rs");
include!("csv/materializer.rs");
include!("csv/depth_boundary.rs");
include!("csv/schema_guard.rs");
include!("csv/loader.rs");
include!("csv/kotlin_adapter.rs");
include!("csv/materialize_helpers.rs");
include!("csv/tests.rs");
