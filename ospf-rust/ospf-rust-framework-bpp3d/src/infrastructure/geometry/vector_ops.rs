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

