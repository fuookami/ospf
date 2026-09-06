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

