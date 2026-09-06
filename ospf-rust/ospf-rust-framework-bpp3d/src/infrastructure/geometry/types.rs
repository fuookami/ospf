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

