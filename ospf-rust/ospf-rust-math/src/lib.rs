//! # ospf-rust-math
//!
//! 数学函数库
//! Mathematical functions library
//!
//! ## 几何模块 / Geometry Module
//!
//! 提供 2D/3D 几何实体和操作：
//! Provides 2D/3D geometric entities and operations:
//!
//! - [`Point`] / [`Point2`] / [`Point3`] - 点实体 / Point entities
//! - [`Vector`] / [`Vector2`] / [`Vector3`] - 向量实体 / Vector entities
//! - [`Edge`] / [`Edge2`] / [`Edge3`] - 边实体 / Edge entities
//! - [`Triangle`] / [`Triangle2`] / [`Triangle3`] - 三角形实体 / Triangle entities
//! - [`Quadrilateral`] / [`Quadrilateral2`] - 四边形实体 / Quadrilateral entities
//! - [`Circle`] / [`Circle2`] / [`Sphere3`] - 圆/球实体 / Circle/Sphere entities
//! - [`delaunay_triangulate`] - Delaunay 三角剖分 / Delaunay triangulation

pub mod algebra;
pub mod chaotic;
pub mod combinatorics;
pub mod fractal;
pub mod geometry;
pub mod operator;
pub mod ordinary;
pub mod symbol;
pub mod trivalent;

pub use algebra::*;
pub use chaotic::*;
pub use combinatorics::*;
pub use fractal::*;
pub use geometry::*;
pub use operator::*;
pub use symbol::*;
pub use trivalent::*;
