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

