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

