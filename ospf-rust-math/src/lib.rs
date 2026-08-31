// MIT License
//
// Copyright (c) 2024 fuookami
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
