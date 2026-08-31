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

// ============================================================================
// MetricPoint2 - 类型化二维点 / Typed 2D point
// ============================================================================

/// 类型化二维点 / Typed two-dimensional point
///
/// 每个分量都是 `Quantity<V, U>`，保证量纲一致性。
/// Each component is a `Quantity<V, U>`, ensuring dimensional consistency.
///
/// # 泛型参数 / Generic Parameters
/// - `V`: 值类型（如 `f64`, `BigDecimal`）
/// - `U`: 单位类型，实现 `UnitTrait`
#[derive(Debug, Clone)]
pub struct MetricPoint2<V, U: UnitTrait> {
    /// X 分量 / X component
    pub x: Quantity<V, U>,
    /// Y 分量 / Y component
    pub y: Quantity<V, U>,
}

// ============================================================================
// MetricPoint3 - 类型化三维点 / Typed 3D point
// ============================================================================

/// 类型化三维点 / Typed three-dimensional point
///
/// # 泛型参数 / Generic Parameters
/// - `V`: 值类型
/// - `U`: 单位类型，实现 `UnitTrait`
#[derive(Debug, Clone)]
pub struct MetricPoint3<V, U: UnitTrait> {
    /// X 分量 / X component
    pub x: Quantity<V, U>,
    /// Y 分量 / Y component
    pub y: Quantity<V, U>,
    /// Z 分量 / Z component
    pub z: Quantity<V, U>,
}

// ============================================================================
// MetricVector2 - 类型化二维向量 / Typed 2D vector
// ============================================================================

/// 类型化二维向量 / Typed two-dimensional vector
#[derive(Debug, Clone)]
pub struct MetricVector2<V, U: UnitTrait> {
    /// X 分量 / X component
    pub x: Quantity<V, U>,
    /// Y 分量 / Y component
    pub y: Quantity<V, U>,
}

// ============================================================================
// MetricVector3 - 类型化三维向量 / Typed 3D vector
// ============================================================================

/// 类型化三维向量 / Typed three-dimensional vector
#[derive(Debug, Clone)]
pub struct MetricVector3<V, U: UnitTrait> {
    /// X 分量 / X component
    pub x: Quantity<V, U>,
    /// Y 分量 / Y component
    pub y: Quantity<V, U>,
    /// Z 分量 / Z component
    pub z: Quantity<V, U>,
}

// ============================================================================
// MetricSize2 - 类型化二维尺寸 / Typed 2D size
// ============================================================================

/// 类型化二维尺寸 / Typed two-dimensional size
///
/// 表示二维空间中的宽度与高度，分量均为正物理量。
/// Represents width and height in 2D space, all components are positive physical quantities.
#[derive(Debug, Clone)]
pub struct MetricSize2<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
}

// ============================================================================
// MetricSize3 - 类型化三维尺寸 / Typed 3D size
// ============================================================================

/// 类型化三维尺寸 / Typed three-dimensional size
///
/// 表示三维空间中的宽度、高度与深度。
/// Represents width, height, and depth in 3D space.
#[derive(Debug, Clone)]
pub struct MetricSize3<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
}

// ============================================================================
// MetricAabb2 - 类型化二维轴对齐包围盒 / Typed 2D AABB
// ============================================================================

/// 类型化二维轴对齐包围盒 / Typed 2D axis-aligned bounding box
///
/// 由最小点（左下角）和尺寸定义。
/// Defined by minimum point (lower-left corner) and size.
#[derive(Debug, Clone)]
pub struct MetricAabb2<V, U: UnitTrait> {
    /// 最小点 / Minimum point
    pub min: MetricPoint2<V, U>,
    /// 尺寸 / Size
    pub size: MetricSize2<V, U>,
}

// ============================================================================
// MetricAabb3 - 类型化三维轴对齐包围盒 / Typed 3D AABB
// ============================================================================

/// 类型化三维轴对齐包围盒 / Typed 3D axis-aligned bounding box
///
/// 由最小点和尺寸定义。
/// Defined by minimum point and size.
#[derive(Debug, Clone)]
pub struct MetricAabb3<V, U: UnitTrait> {
    /// 最小点 / Minimum point
    pub min: MetricPoint3<V, U>,
    /// 尺寸 / Size
    pub size: MetricSize3<V, U>,
}

// ============================================================================
// MetricPlacement2 - 类型化二维放置 / Typed 2D placement
// ============================================================================

/// 类型化二维放置 / Typed two-dimensional placement
///
/// 位置加形状的组合，表示物体在二维平面上的放置。
/// Combination of position and shape, representing an object placed in 2D space.
///
/// # 泛型参数 / Generic Parameters
/// - `V`: 值类型
/// - `U`: 单位类型
/// - `S`: 形状类型（如 `Rectangle2<Quantity<V, U>>`, `Circle2<Quantity<V, U>>`）
#[derive(Debug, Clone)]
pub struct MetricPlacement2<V, U: UnitTrait, S> {
    /// 位置 / Position
    pub position: MetricPoint2<V, U>,
    /// 形状 / Shape
    pub shape: S,
}

// ============================================================================
// MetricPlacement3 - 类型化三维放置 / Typed 3D placement
// ============================================================================

/// 类型化三维放置 / Typed three-dimensional placement
///
/// 位置加形状的组合，表示物体在三维空间中的放置。
/// Combination of position and shape, representing an object placed in 3D space.
#[derive(Debug, Clone)]
pub struct MetricPlacement3<V, U: UnitTrait, S> {
    /// 位置 / Position
    pub position: MetricPoint3<V, U>,
    /// 形状 / Shape
    pub shape: S,
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<V, U: UnitTrait> MetricPoint2<V, U> {
    /// 创建类型化二维点 / Create a typed 2D point
    pub fn new(x: Quantity<V, U>, y: Quantity<V, U>) -> Self {
        Self { x, y }
    }
}

impl<V, U: UnitTrait> MetricPoint3<V, U> {
    /// 创建类型化三维点 / Create a typed 3D point
    pub fn new(x: Quantity<V, U>, y: Quantity<V, U>, z: Quantity<V, U>) -> Self {
        Self { x, y, z }
    }
}

impl<V, U: UnitTrait> MetricVector2<V, U> {
    /// 创建类型化二维向量 / Create a typed 2D vector
    pub fn new(x: Quantity<V, U>, y: Quantity<V, U>) -> Self {
        Self { x, y }
    }
}

impl<V, U: UnitTrait> MetricVector3<V, U> {
    /// 创建类型化三维向量 / Create a typed 3D vector
    pub fn new(x: Quantity<V, U>, y: Quantity<V, U>, z: Quantity<V, U>) -> Self {
        Self { x, y, z }
    }
}

impl<V, U: UnitTrait> MetricSize2<V, U> {
    /// 创建类型化二维尺寸 / Create a typed 2D size
    pub fn new(width: Quantity<V, U>, height: Quantity<V, U>) -> Self {
        Self { width, height }
    }
}

impl<V, U: UnitTrait> MetricSize3<V, U> {
    /// 创建类型化三维尺寸 / Create a typed 3D size
    pub fn new(width: Quantity<V, U>, height: Quantity<V, U>, depth: Quantity<V, U>) -> Self {
        Self { width, height, depth }
    }
}

impl<V, U: UnitTrait> MetricAabb2<V, U> {
    /// 创建类型化二维包围盒 / Create a typed 2D AABB
    pub fn new(min: MetricPoint2<V, U>, size: MetricSize2<V, U>) -> Self {
        Self { min, size }
    }

    /// 从点和尺寸创建 / Create from point and size
    pub fn from_point_size(x: Quantity<V, U>, y: Quantity<V, U>, width: Quantity<V, U>, height: Quantity<V, U>) -> Self {
        Self {
            min: MetricPoint2::new(x, y),
            size: MetricSize2::new(width, height),
        }
    }
}

impl<V, U: UnitTrait> MetricAabb3<V, U> {
    /// 创建类型化三维包围盒 / Create a typed 3D AABB
    pub fn new(min: MetricPoint3<V, U>, size: MetricSize3<V, U>) -> Self {
        Self { min, size }
    }

    /// 从点和尺寸创建 / Create from point and size
    pub fn from_point_size(
        x: Quantity<V, U>, y: Quantity<V, U>, z: Quantity<V, U>,
        width: Quantity<V, U>, height: Quantity<V, U>, depth: Quantity<V, U>,
    ) -> Self {
        Self {
            min: MetricPoint3::new(x, y, z),
            size: MetricSize3::new(width, height, depth),
        }
    }
}

impl<V, U: UnitTrait, S> MetricPlacement2<V, U, S> {
    /// 创建类型化二维放置 / Create a typed 2D placement
    pub fn new(position: MetricPoint2<V, U>, shape: S) -> Self {
        Self { position, shape }
    }
}

impl<V, U: UnitTrait, S> MetricPlacement3<V, U, S> {
    /// 创建类型化三维放置 / Create a typed 3D placement
    pub fn new(position: MetricPoint3<V, U>, shape: S) -> Self {
        Self { position, shape }
    }
}

// ============================================================================
// 编译时单位便捷构造 / CTUnit convenience constructors
// ============================================================================

impl<V, U: CTUnit + Default> MetricPoint2<V, U> {
    /// 从裸值创建编译时单位点 / Create CT unit point from raw values
    pub fn from_raw(x: V, y: V) -> Self {
        Self {
            x: Quantity::new_ct(x),
            y: Quantity::new_ct(y),
        }
    }
}

impl<V, U: CTUnit + Default> MetricPoint3<V, U> {
    /// 从裸值创建编译时单位点 / Create CT unit point from raw values
    pub fn from_raw(x: V, y: V, z: V) -> Self {
        Self {
            x: Quantity::new_ct(x),
            y: Quantity::new_ct(y),
            z: Quantity::new_ct(z),
        }
    }
}

impl<V, U: CTUnit + Default> MetricVector2<V, U> {
    /// 从裸值创建编译时单位向量 / Create CT unit vector from raw values
    pub fn from_raw(x: V, y: V) -> Self {
        Self {
            x: Quantity::new_ct(x),
            y: Quantity::new_ct(y),
        }
    }
}

impl<V, U: CTUnit + Default> MetricVector3<V, U> {
    /// 从裸值创建编译时单位向量 / Create CT unit vector from raw values
    pub fn from_raw(x: V, y: V, z: V) -> Self {
        Self {
            x: Quantity::new_ct(x),
            y: Quantity::new_ct(y),
            z: Quantity::new_ct(z),
        }
    }
}

impl<V, U: CTUnit + Default> MetricSize2<V, U> {
    /// 从裸值创建编译时单位尺寸 / Create CT unit size from raw values
    pub fn from_raw(width: V, height: V) -> Self {
        Self {
            width: Quantity::new_ct(width),
            height: Quantity::new_ct(height),
        }
    }
}

impl<V, U: CTUnit + Default> MetricSize3<V, U> {
    /// 从裸值创建编译时单位尺寸 / Create CT unit size from raw values
    pub fn from_raw(width: V, height: V, depth: V) -> Self {
        Self {
            width: Quantity::new_ct(width),
            height: Quantity::new_ct(height),
            depth: Quantity::new_ct(depth),
        }
    }
}

// ============================================================================
// 加减运算 / Add/Sub operations (same unit)
// ============================================================================

impl<V, U: CTUnit + Default> Add for MetricPoint2<V, U>
where
    V: Add<Output = V>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: Quantity::new_ct(self.x.value + rhs.x.value),
            y: Quantity::new_ct(self.y.value + rhs.y.value),
        }
    }
}

impl<V, U: CTUnit + Default> Sub for MetricPoint2<V, U>
where
    V: Sub<Output = V>,
{
    type Output = MetricVector2<V, U>;

    fn sub(self, rhs: Self) -> Self::Output {
        MetricVector2 {
            x: Quantity::new_ct(self.x.value - rhs.x.value),
            y: Quantity::new_ct(self.y.value - rhs.y.value),
        }
    }
}

impl<V, U: CTUnit + Default> Add for MetricPoint3<V, U>
where
    V: Add<Output = V>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: Quantity::new_ct(self.x.value + rhs.x.value),
            y: Quantity::new_ct(self.y.value + rhs.y.value),
            z: Quantity::new_ct(self.z.value + rhs.z.value),
        }
    }
}

impl<V, U: CTUnit + Default> Sub for MetricPoint3<V, U>
where
    V: Sub<Output = V>,
{
    type Output = MetricVector3<V, U>;

    fn sub(self, rhs: Self) -> Self::Output {
        MetricVector3 {
            x: Quantity::new_ct(self.x.value - rhs.x.value),
            y: Quantity::new_ct(self.y.value - rhs.y.value),
            z: Quantity::new_ct(self.z.value - rhs.z.value),
        }
    }
}

impl<V, U: CTUnit + Default> Add for MetricVector2<V, U>
where
    V: Add<Output = V>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: Quantity::new_ct(self.x.value + rhs.x.value),
            y: Quantity::new_ct(self.y.value + rhs.y.value),
        }
    }
}

impl<V, U: CTUnit + Default> Sub for MetricVector2<V, U>
where
    V: Sub<Output = V>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: Quantity::new_ct(self.x.value - rhs.x.value),
            y: Quantity::new_ct(self.y.value - rhs.y.value),
        }
    }
}

impl<V, U: CTUnit + Default> Add for MetricVector3<V, U>
where
    V: Add<Output = V>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: Quantity::new_ct(self.x.value + rhs.x.value),
            y: Quantity::new_ct(self.y.value + rhs.y.value),
            z: Quantity::new_ct(self.z.value + rhs.z.value),
        }
    }
}

impl<V, U: CTUnit + Default> Sub for MetricVector3<V, U>
where
    V: Sub<Output = V>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: Quantity::new_ct(self.x.value - rhs.x.value),
            y: Quantity::new_ct(self.y.value - rhs.y.value),
            z: Quantity::new_ct(self.z.value - rhs.z.value),
        }
    }
}

// ============================================================================
// 比较运算 / Comparison operations
// ============================================================================

impl<V: PartialEq, U: CTUnit> PartialEq for MetricPoint2<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricPoint3<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricVector2<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricVector3<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricSize2<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricSize3<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height && self.depth == other.depth
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricAabb2<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.size == other.size
    }
}

impl<V: PartialEq, U: CTUnit> PartialEq for MetricAabb3<V, U> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.size == other.size
    }
}

impl<V: Eq, U: CTUnit> Eq for MetricPoint2<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricPoint3<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricVector2<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricVector3<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricSize2<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricSize3<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricAabb2<V, U> {}
impl<V: Eq, U: CTUnit> Eq for MetricAabb3<V, U> {}

// ============================================================================
// AABB 几何查询 / AABB geometry queries
// ============================================================================

impl<V, U: CTUnit + Default> MetricSize3<V, U> {
    /// 沿指定轴的尺寸 / Dimension along the specified axis
    pub fn along(&self, axis: Axis3) -> &Quantity<V, U> {
        match axis {
            Axis3::X => &self.width,
            Axis3::Y => &self.height,
            Axis3::Z => &self.depth,
        }
    }
}

impl<V, U: CTUnit + Default> MetricAabb2<V, U>
where
    V: Add<Output = V> + Sub<Output = V> + Zero + Clone + PartialOrd,
{
    /// 最大 X / Maximum X
    pub fn max_x(&self) -> Quantity<V, U> {
        Quantity::new_ct(self.min.x.value.clone() + self.size.width.value.clone())
    }

    /// 最大 Y / Maximum Y
    pub fn max_y(&self) -> Quantity<V, U> {
        Quantity::new_ct(self.min.y.value.clone() + self.size.height.value.clone())
    }

    /// 判断点是否在包围盒内 / Check if point is inside the AABB
    pub fn contains_point(&self, point: &MetricPoint2<V, U>) -> bool {
        point.x.value >= self.min.x.value
            && point.y.value >= self.min.y.value
            && point.x.value < self.max_x().value
            && point.y.value < self.max_y().value
    }

    /// 判断两个包围盒是否重叠 / Check if two AABBs overlap
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.x.value < other.max_x().value
            && self.max_x().value > other.min.x.value
            && self.min.y.value < other.max_y().value
            && self.max_y().value > other.min.y.value
    }
}

impl<V, U: CTUnit + Default> MetricAabb3<V, U>
where
    V: Add<Output = V> + Sub<Output = V> + Zero + Clone + PartialOrd,
{
    /// 最大 X / Maximum X
    pub fn max_x(&self) -> Quantity<V, U> {
        Quantity::new_ct(self.min.x.value.clone() + self.size.width.value.clone())
    }

    /// 最大 Y / Maximum Y
    pub fn max_y(&self) -> Quantity<V, U> {
        Quantity::new_ct(self.min.y.value.clone() + self.size.height.value.clone())
    }

    /// 最大 Z / Maximum Z
    pub fn max_z(&self) -> Quantity<V, U> {
        Quantity::new_ct(self.min.z.value.clone() + self.size.depth.value.clone())
    }

    /// 判断点是否在包围盒内 / Check if point is inside the AABB
    pub fn contains_point(&self, point: &MetricPoint3<V, U>) -> bool {
        point.x.value >= self.min.x.value
            && point.y.value >= self.min.y.value
            && point.z.value >= self.min.z.value
            && point.x.value < self.max_x().value
            && point.y.value < self.max_y().value
            && point.z.value < self.max_z().value
    }

    /// 判断两个包围盒是否重叠 / Check if two AABBs overlap
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.x.value < other.max_x().value
            && self.max_x().value > other.min.x.value
            && self.min.y.value < other.max_y().value
            && self.max_y().value > other.min.y.value
            && self.min.z.value < other.max_z().value
            && self.max_z().value > other.min.z.value
    }
}

// ============================================================================
// 标量几何转换 / Scalar geometry conversion
// ============================================================================

/// 将类型化三维点转换为标量 f64 点 / Convert typed 3D point to scalar f64 point
///
/// 只在求解器适配边界使用，public API 不暴露裸 `f64`。
/// Only used at solver adapter boundaries; public APIs must not expose raw `f64`.
pub fn typed_point3_to_scalar<V, U>(typed: &MetricPoint3<V, U>) -> Point3<f64>
where
    V: Into<f64> + Clone,
    U: CTUnit + Default,
{
    Point3::new(typed.x.value.clone().into(), typed.y.value.clone().into(), typed.z.value.clone().into())
}

/// 将类型化二维点转换为标量 f64 点 / Convert typed 2D point to scalar f64 point
pub fn typed_point2_to_scalar<V, U>(typed: &MetricPoint2<V, U>) -> Point2<f64>
where
    V: Into<f64> + Clone,
    U: CTUnit + Default,
{
    Point2::new(typed.x.value.clone().into(), typed.y.value.clone().into())
}

/// 将类型化三维尺寸转换为标量 f64 长方体 / Convert typed 3D size to scalar f64 cuboid
pub fn typed_size3_to_scalar_cuboid<V, U>(typed: &MetricSize3<V, U>) -> Cuboid3<f64>
where
    V: Into<f64> + Clone,
    U: CTUnit + Default,
{
    Cuboid3::new(
        typed.width.value.clone().into(),
        typed.height.value.clone().into(),
        typed.depth.value.clone().into(),
    )
}

/// 从标量 f64 点构造类型化点 / Construct typed point from scalar f64 point
pub fn scalar_point3_to_typed<V, U>(point: &Point3<f64>) -> MetricPoint3<V, U>
where
    V: From<f64>,
    U: CTUnit + Default,
{
    MetricPoint3 {
        x: Quantity::new_ct(V::from(point.x())),
        y: Quantity::new_ct(V::from(point.y())),
        z: Quantity::new_ct(V::from(point.z())),
    }
}

/// 从标量 f64 点构造类型化二维点 / Construct typed 2D point from scalar f64 point
pub fn scalar_point2_to_typed<V, U>(point: &Point2<f64>) -> MetricPoint2<V, U>
where
    V: From<f64>,
    U: CTUnit + Default,
{
    MetricPoint2 {
        x: Quantity::new_ct(V::from(point.x())),
        y: Quantity::new_ct(V::from(point.y())),
    }
}

/// 从标量 f64 长方体构造类型化尺寸 / Construct typed size from scalar f64 cuboid
pub fn scalar_cuboid3_to_typed_size<V, U>(cuboid: &Cuboid3<f64>) -> MetricSize3<V, U>
where
    V: From<f64>,
    U: CTUnit + Default,
{
    MetricSize3 {
        width: Quantity::new_ct(V::from(cuboid.width)),
        height: Quantity::new_ct(V::from(cuboid.height)),
        depth: Quantity::new_ct(V::from(cuboid.depth)),
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::derived::Meter;

    type LengthF64 = Quantity<f64, Meter>;

    fn meters(v: f64) -> LengthF64 {
        Quantity::new_ct(v)
    }

    #[test]
    fn metric_point2_from_raw_and_access() {
        let p = MetricPoint2::<f64, Meter>::from_raw(1.0, 2.0);
        assert_eq!(p.x.value, 1.0);
        assert_eq!(p.y.value, 2.0);
    }

    #[test]
    fn metric_point3_from_raw_and_access() {
        let p = MetricPoint3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        assert_eq!(p.x.value, 1.0);
        assert_eq!(p.y.value, 2.0);
        assert_eq!(p.z.value, 3.0);
    }

    #[test]
    fn metric_vector2_add_sub() {
        let v1 = MetricVector2::<f64, Meter>::from_raw(1.0, 2.0);
        let v2 = MetricVector2::<f64, Meter>::from_raw(3.0, 4.0);

        let sum = v1.clone() + v2.clone();
        assert_eq!(sum.x.value, 4.0);
        assert_eq!(sum.y.value, 6.0);

        let diff = v1 - v2;
        assert_eq!(diff.x.value, -2.0);
        assert_eq!(diff.y.value, -2.0);
    }

    #[test]
    fn metric_point3_sub_returns_vector() {
        let p1 = MetricPoint3::<f64, Meter>::from_raw(4.0, 6.0, 8.0);
        let p2 = MetricPoint3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);

        let diff = p1 - p2;
        assert_eq!(diff.x.value, 3.0);
        assert_eq!(diff.y.value, 4.0);
        assert_eq!(diff.z.value, 5.0);
    }

    #[test]
    fn metric_size3_along_axis() {
        let size = MetricSize3::<f64, Meter>::from_raw(2.0, 3.0, 4.0);
        assert_eq!(size.along(Axis3::X).value, 2.0);
        assert_eq!(size.along(Axis3::Y).value, 3.0);
        assert_eq!(size.along(Axis3::Z).value, 4.0);
    }

    #[test]
    fn metric_aabb2_overlaps_and_contains() {
        let aabb = MetricAabb2::from_point_size(
            meters(0.0), meters(0.0),
            meters(4.0), meters(4.0),
        );

        let inside = MetricPoint2::new(meters(1.0), meters(1.0));
        let outside = MetricPoint2::new(meters(5.0), meters(5.0));
        assert!(aabb.contains_point(&inside));
        assert!(!aabb.contains_point(&outside));

        let other = MetricAabb2::from_point_size(
            meters(2.0), meters(2.0),
            meters(4.0), meters(4.0),
        );
        assert!(aabb.overlaps(&other));

        let disjoint = MetricAabb2::from_point_size(
            meters(5.0), meters(5.0),
            meters(2.0), meters(2.0),
        );
        assert!(!aabb.overlaps(&disjoint));
    }

    #[test]
    fn metric_aabb3_overlaps_and_contains() {
        let aabb = MetricAabb3::from_point_size(
            meters(0.0), meters(0.0), meters(0.0),
            meters(4.0), meters(4.0), meters(4.0),
        );

        let inside = MetricPoint3::new(meters(1.0), meters(1.0), meters(1.0));
        assert!(aabb.contains_point(&inside));

        let other = MetricAabb3::from_point_size(
            meters(2.0), meters(2.0), meters(2.0),
            meters(4.0), meters(4.0), meters(4.0),
        );
        assert!(aabb.overlaps(&other));
    }

    #[test]
    fn metric_point2_equality() {
        let p1 = MetricPoint2::<f64, Meter>::from_raw(1.0, 2.0);
        let p2 = MetricPoint2::<f64, Meter>::from_raw(1.0, 2.0);
        let p3 = MetricPoint2::<f64, Meter>::from_raw(1.0, 3.0);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn metric_size3_equality() {
        let s1 = MetricSize3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        let s2 = MetricSize3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        assert_eq!(s1, s2);
    }

    #[test]
    fn scalar_point3_conversion_roundtrip() {
        let scalar = Point3::new(1.0, 2.0, 3.0);
        let typed = scalar_point3_to_typed::<f64, Meter>(&scalar);
        assert_eq!(typed.x.value, 1.0);
        assert_eq!(typed.y.value, 2.0);
        assert_eq!(typed.z.value, 3.0);

        let back = typed_point3_to_scalar(&typed);
        assert_eq!(back.x(), 1.0);
        assert_eq!(back.y(), 2.0);
        assert_eq!(back.z(), 3.0);
    }

    #[test]
    fn scalar_cuboid3_conversion_roundtrip() {
        let scalar = Cuboid3::new(2.0, 3.0, 4.0);
        let typed = scalar_cuboid3_to_typed_size::<f64, Meter>(&scalar);
        assert_eq!(typed.width.value, 2.0);
        assert_eq!(typed.height.value, 3.0);
        assert_eq!(typed.depth.value, 4.0);

        let back = typed_size3_to_scalar_cuboid(&typed);
        assert_eq!(back.width, 2.0);
        assert_eq!(back.height, 3.0);
        assert_eq!(back.depth, 4.0);
    }

    #[test]
    fn scalar_point2_conversion_roundtrip() {
        let scalar = Point2::new(1.0, 2.0);
        let typed = scalar_point2_to_typed::<f64, Meter>(&scalar);
        assert_eq!(typed.x.value, 1.0);
        assert_eq!(typed.y.value, 2.0);

        let back = typed_point2_to_scalar(&typed);
        assert_eq!(back.x(), 1.0);
        assert_eq!(back.y(), 2.0);
    }

    #[test]
    fn metric_placement3_construction() {
        let position = MetricPoint3::<f64, Meter>::from_raw(1.0, 2.0, 3.0);
        let shape = Cuboid3::new(2.0, 3.0, 4.0);
        let placement = MetricPlacement3::new(position, shape);
        assert_eq!(placement.position.x.value, 1.0);
        assert_eq!(placement.shape.width, 2.0);
    }
}
