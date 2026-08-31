//! BPP3D typed geometry adapter / BPP3D 类型化几何适配层
//!
//! 本模块基于 Rust 泛型重新设计 typed geometry 类型，不照搬 Kotlin `Quantity*` 类型体系。
//! This module redesigns typed geometry types with Rust generics and must not copy the Kotlin
//! `Quantity*` type hierarchy.
//!
//! # 核心设计 / Core Design
//!
//! - 使用 `Quantity<V, U>` 作为分量类型，保持量纲安全
//! - Uses `Quantity<V, U>` as component types to maintain dimensional safety
//! - 提供与 `ospf_rust_math::geometry` 标量几何的双向转换
//! - Provides bidirectional conversion with `ospf_rust_math::geometry` scalar geometry
//! - 不照搬 Kotlin 迁移期 `QuantityPoint*` 命名
//! - Does not copy Kotlin migration-era `QuantityPoint*` naming
//!
//! # 类型一览 / Type Overview
//!
//! | 类型 | 说明 |
//! |------|------|
//! | `MetricPoint2<V, U>` | 类型化二维点 |
//! | `MetricPoint3<V, U>` | 类型化三维点 |
//! | `MetricVector2<V, U>` | 类型化二维向量 |
//! | `MetricVector3<V, U>` | 类型化三维向量 |
//! | `MetricSize2<V, U>` | 类型化二维尺寸 |
//! | `MetricSize3<V, U>` | 类型化三维尺寸 |
//! | `MetricAabb2<V, U>` | 类型化二维轴对齐包围盒 |
//! | `MetricAabb3<V, U>` | 类型化三维轴对齐包围盒 |
//! | `MetricPlacement2<V, U, S>` | 类型化二维放置 |
//! | `MetricPlacement3<V, U, S>` | 类型化三维放置 |

use std::ops::{Add, Sub};
use num_traits::Zero;
use ospf_rust_math::geometry::{Axis3, Cuboid3, Point2, Point3};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;


include!("geometry/types.rs");
include!("geometry/constructors.rs");
include!("geometry/point_ops.rs");
include!("geometry/vector_ops.rs");
include!("geometry/aabb.rs");
include!("geometry/scalar_conversion.rs");
include!("geometry/tests.rs");
