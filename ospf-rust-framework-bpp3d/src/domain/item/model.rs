//! 货物领域模型 / Item domain models
//!
//! 映射 Kotlin `bpp3d-domain-item-context` 的核心领域模型。
//! Maps core domain models from Kotlin `bpp3d-domain-item-context`.

use std::collections::HashMap;
use num_traits::ToPrimitive;
use ospf_rust_math::algebra::value_range::IntervalValue;
use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{
    BinaryVariableItem, UContinuousVariableItem, VariableRange,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;
use crate::infrastructure::pwl_approximation::{
    PwlRadiusApproximationConfig, PwlRadiusSquaredApproximation,
};
use crate::infrastructure::orientation::{Orientation, OrientationCategory};
use crate::infrastructure::packing_shape::{PackingShape3, cuboid_packing_shape, cylinder_packing_shape};

include!("model/package_shape.rs");
include!("model/cargo_attribute.rs");
include!("model/material.rs");
include!("model/package.rs");
include!("model/item.rs");
include!("model/pattern.rs");
include!("model/package_attribute.rs");
include!("model/bin.rs");
include!("model/layer.rs");
include!("model/schema.rs");
include!("model/cylinder.rs");
include!("model/continuous_radius.rs");
include!("model/tests.rs");
