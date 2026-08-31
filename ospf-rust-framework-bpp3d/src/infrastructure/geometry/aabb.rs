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

