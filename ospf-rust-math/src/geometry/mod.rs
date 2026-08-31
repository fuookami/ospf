//! 几何实体模块
//! Geometry entities module
//!
//! 本模块提供几何实体的定义和操作，包括：
//! This module provides definitions and operations for geometric entities, including:
//!
//! - [`Point`] - 点（支持任意维度）/ Point (arbitrary dimensions supported)
//! - [`Vector`] - 向量（支持任意维度）/ Vector (arbitrary dimensions supported)
//! - [`Edge`] - 边 / Edge
//! - [`Triangle`] - 三角形 / Triangle
//! - [`Quadrilateral`] - 四边形 / Quadrilateral
//! - [`Circle`] - 圆 / Circle
//! - [`delaunay_triangulate`] - Delaunay 三角剖分 / Delaunay triangulation

pub mod circle;
pub mod delaunay;
pub mod distance;
pub mod edge;
pub mod point;
pub mod quadrilateral;
pub mod triangle;
pub mod vector;

pub use circle::*;
pub use delaunay::*;
pub use distance::*;
pub use edge::*;
pub use point::*;
pub use quadrilateral::*;
pub use triangle::*;
pub use vector::*;
