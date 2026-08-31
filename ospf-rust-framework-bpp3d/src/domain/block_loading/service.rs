//! 块装载服务 / Block loading services
//!
//! 简单块生成器和算法实现。
//! Simple block generator and algorithm implementations.

use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::physical_unit::CTUnit;
use std::fmt::Debug;

use crate::domain::item::{ActualItem, PackageAttribute, PackageOrientationRuleInput};
use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::PackingShapeType;

use super::model::{Block, BlockPlacement, ComplexBlock, ItemView, SimpleBlock, Space};

include!("service/simple_block_generator.rs");
include!("service/complex_block_generator.rs");
include!("service/depth_first_search_algorithm.rs");
include!("service/multi_layer_heuristic_search_algorithm.rs");
include!("service/tests.rs");
