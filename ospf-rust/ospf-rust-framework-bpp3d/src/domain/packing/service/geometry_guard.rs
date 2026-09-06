// ============================================================================
// PackingGeometryGuard - 装箱几何守卫 / Packing geometry guard
// ============================================================================

/// 重叠容差 / Overlap tolerance
const PACKING_GEOMETRY_OVERLAP_TOLERANCE: f64 = 1e-7;

/// 装箱几何守卫 / Packing geometry guard
///
/// 验证最终装箱结果的几何合法性：
/// 1. 所有物品不超出容器边界
/// 2. 物品之间无重叠（使用真实 footprint：长方体-长方体、圆柱-长方体、同轴圆柱-圆柱）
/// 3. 横向圆柱有足够的支撑覆盖
///
/// Validates geometric correctness of final packing results:
/// 1. All items are within container bounds
/// 2. No overlaps between items (using real footprint: box-box, cylinder-box, same-axis cylinder-cylinder)
/// 3. Horizontal cylinders have sufficient support coverage
#[derive(Debug, Clone, Default)]
pub struct PackingGeometryGuard;

impl PackingGeometryGuard {
    /// 创建守卫 / Create guard
    pub fn new() -> Self {
        Self
    }

    /// 验证已装箱 / Validate a packed bin
    ///
    /// 执行三重验证：边界、重叠（真实 footprint）、横向圆柱支撑。
    /// Performs triple validation: bounds, overlap (real footprint), horizontal cylinder support.
    pub fn validate<V, U>(packed_bin: &PackedBin<V, U>) -> Result<(), Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        let mut errors = Vec::new();

        // 将所有物品转换为标量几何信息用于碰撞检测
        let geometries: Vec<ScalarGeometry> = packed_bin.items.iter()
            .map(|item| scalar_geometry_from_packed_item(item))
            .collect();

        // 1. 检查容器边界
        let bin_w: f64 = packed_bin.bin_type.width.value.clone().into();
        let bin_h: f64 = packed_bin.bin_type.height.value.clone().into();
        let bin_d: f64 = packed_bin.bin_type.depth.value.clone().into();

        for (i, geo) in geometries.iter().enumerate() {
            if geo.max_x > bin_w + PACKING_GEOMETRY_OVERLAP_TOLERANCE {
                errors.push(format!(
                    "Item {} exceeds bin X bound: max_x={:?} > bin_width={:?}",
                    i, geo.max_x, bin_w
                ));
            }
            if geo.max_y > bin_h + PACKING_GEOMETRY_OVERLAP_TOLERANCE {
                errors.push(format!(
                    "Item {} exceeds bin Y bound: max_y={:?} > bin_height={:?}",
                    i, geo.max_y, bin_h
                ));
            }
            if geo.max_z > bin_d + PACKING_GEOMETRY_OVERLAP_TOLERANCE {
                errors.push(format!(
                    "Item {} exceeds bin Z bound: max_z={:?} > bin_depth={:?}",
                    i, geo.max_z, bin_d
                ));
            }
        }

        // 2. 检查物品间重叠（使用真实 footprint，不是 AABB）
        for i in 0..geometries.len() {
            for j in (i + 1)..geometries.len() {
                if geometries[i].overlaps(&geometries[j]) {
                    errors.push(format!(
                        "Items {} and {} overlap",
                        i, j
                    ));
                }
            }
        }

        // 3. 检查横向圆柱支撑覆盖（复用 infrastructure 的支撑覆盖率算法）
        for (i, _) in geometries.iter().enumerate() {
            if !has_horizontal_cylinder_support_coverage(&geometries, i) {
                let geo = &geometries[i];
                let diagnostic = format!(
                    "shape={}, axis={}",
                    geo.shape_type_str(),
                    geo.cylinder_axis.map_or("none".to_string(), |a| format!("{:?}", a))
                );
                errors.push(format!(
                    "Horizontal cylinder item {} lacks support coverage: {}",
                    i, diagnostic
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ============================================================================
// ScalarGeometry - 标量几何信息（用于碰撞检测）
// ============================================================================

/// 物品标量几何信息（用于真实 footprint 碰撞检测）
/// Item scalar geometry info (for real footprint collision detection)
///
/// 所有计算在 f64 标量域完成，避免泛型 V 的约束问题。
/// All calculations are done in the f64 scalar domain to avoid
/// generic V constraint issues.
#[derive(Debug, Clone)]
struct ScalarGeometry {
    /// 最小 X / Minimum X
    pub min_x: f64,
    /// 最大 X / Maximum X
    pub max_x: f64,
    /// 最小 Y / Minimum Y
    pub min_y: f64,
    /// 最大 Y / Maximum Y
    pub max_y: f64,
    /// 最小 Z / Minimum Z
    pub min_z: f64,
    /// 最大 Z / Maximum Z
    pub max_z: f64,
    /// 形状类型 / Shape type
    pub shape_type: PackingShapeType,
    /// 圆柱轴 / Cylinder axis
    pub cylinder_axis: Option<Axis3>,
    /// 圆柱半径 / Cylinder radius (for footprint collision)
    pub cylinder_radius: Option<f64>,
}

/// 从 PackedItem 提取标量几何信息 / Extract scalar geometry from PackedItem
fn scalar_geometry_from_packed_item<V, U: UnitTrait>(item: &PackedItem<V, U>) -> ScalarGeometry
where
    V: Clone + Debug + Send + Sync + Field + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone,
{
    let min_x: f64 = item.position.x.value.clone().into();
    let min_y: f64 = item.position.y.value.clone().into();
    let min_z: f64 = item.position.z.value.clone().into();
    let bw: f64 = item.packing_shape.bounding_width.value.clone().into();
    let bh: f64 = item.packing_shape.bounding_height.value.clone().into();
    let bd: f64 = item.packing_shape.bounding_depth.value.clone().into();

    let cylinder_radius = item.packing_shape.radius.as_ref()
        .map(|q| -> f64 { q.value.clone().into() });

    ScalarGeometry {
        min_x,
        max_x: min_x + bw,
        min_y,
        max_y: min_y + bh,
        min_z,
        max_z: min_z + bd,
        shape_type: item.packing_shape.shape_type,
        cylinder_axis: item.packing_shape.axis,
        cylinder_radius,
    }
}

impl ScalarGeometry {
    /// 形状类型字符串 / Shape type string
    fn shape_type_str(&self) -> &'static str {
        match self.shape_type {
            PackingShapeType::Cuboid => "cuboid",
            PackingShapeType::Cylinder => "cylinder",
        }
    }

    /// 中心点（指定轴）/ Center on a given axis
    fn center(&self, axis: Axis3) -> f64 {
        match axis {
            Axis3::X => (self.min_x + self.max_x) / 2.0,
            Axis3::Y => (self.min_y + self.max_y) / 2.0,
            Axis3::Z => (self.min_z + self.max_z) / 2.0,
        }
    }

    /// 最小值（指定轴）/ Minimum on a given axis
    fn min_on(&self, axis: Axis3) -> f64 {
        match axis {
            Axis3::X => self.min_x,
            Axis3::Y => self.min_y,
            Axis3::Z => self.min_z,
        }
    }

    /// 最大值（指定轴）/ Maximum on a given axis
    fn max_on(&self, axis: Axis3) -> f64 {
        match axis {
            Axis3::X => self.max_x,
            Axis3::Y => self.max_y,
            Axis3::Z => self.max_z,
        }
    }

    /// 检查是否与另一个几何重叠（使用真实 footprint）
    /// Check if this overlaps with another geometry (using real footprint)
    fn overlaps(&self, rhs: &ScalarGeometry) -> bool {
        let lhs_cylinder = self.shape_type == PackingShapeType::Cylinder;
        let rhs_cylinder = rhs.shape_type == PackingShapeType::Cylinder;

        match (lhs_cylinder, rhs_cylinder) {
            (true, true) => cylinder_cylinder_overlaps(self, rhs),
            (true, false) => cylinder_box_overlaps(self, rhs),
            (false, true) => cylinder_box_overlaps(rhs, self),
            (false, false) => box_box_overlaps(self, rhs),
        }
    }

    /// 转换为横向圆柱支撑几何 / Convert to horizontal cylinder support geometry
    fn to_support_geometry(&self) -> HorizontalCylinderSupportGeometry {
        HorizontalCylinderSupportGeometry {
            min_x: self.min_x,
            max_x: self.max_x,
            min_y: self.min_y,
            max_y: self.max_y,
            min_z: self.min_z,
            max_z: self.max_z,
            is_cylinder: self.shape_type == PackingShapeType::Cylinder,
        }
    }
}

// ============================================================================
// 真实 footprint 碰撞检测 / Real footprint collision detection
// ============================================================================

/// 区间重叠检测 / Interval overlap detection
///
/// 如果两个区间有足够的交集（超过容差），则认为重叠。
/// Two intervals overlap if they have sufficient intersection (exceeding tolerance).
fn interval_overlaps(a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> bool {
    a_min < b_max - PACKING_GEOMETRY_OVERLAP_TOLERANCE
        && a_max > b_min + PACKING_GEOMETRY_OVERLAP_TOLERANCE
}

/// 点到区间距离 / Distance from point to interval
fn distance_to_interval(point: f64, min: f64, max: f64) -> f64 {
    if point < min {
        min - point
    } else if point > max {
        point - max
    } else {
        0.0
    }
}

/// 除指定轴外的其他轴 / Axes except the given axis
fn axes_except(axis: Axis3) -> [Axis3; 2] {
    match axis {
        Axis3::X => [Axis3::Y, Axis3::Z],
        Axis3::Y => [Axis3::X, Axis3::Z],
        Axis3::Z => [Axis3::X, Axis3::Y],
    }
}

/// 长方体-长方体重叠 / Box-box overlap (AABB)
fn box_box_overlaps(lhs: &ScalarGeometry, rhs: &ScalarGeometry) -> bool {
    interval_overlaps(lhs.min_x, lhs.max_x, rhs.min_x, rhs.max_x)
        && interval_overlaps(lhs.min_y, lhs.max_y, rhs.min_y, rhs.max_y)
        && interval_overlaps(lhs.min_z, lhs.max_z, rhs.min_z, rhs.max_z)
}

/// 圆柱-长方体重叠 / Cylinder-box overlap (real footprint)
///
/// 圆柱在径向平面上的足迹是圆形，长方体是矩形。
/// 重叠判定：先检查轴向区间，再检查径向平面圆心到矩形最近点距离 < 半径。
fn cylinder_box_overlaps(cylinder: &ScalarGeometry, box_: &ScalarGeometry) -> bool {
    let axis = cylinder.cylinder_axis.unwrap_or(Axis3::Y);
    let radius = cylinder.cylinder_radius.unwrap_or(0.0);

    // 轴向区间必须重叠
    if !interval_overlaps(cylinder.min_on(axis), cylinder.max_on(axis), box_.min_on(axis), box_.max_on(axis)) {
        return false;
    }

    // 径向平面上的碰撞检测
    let [radial0, radial1] = axes_except(axis);
    let nearest0 = cylinder.center(radial0).clamp(box_.min_on(radial0), box_.max_on(radial0));
    let nearest1 = cylinder.center(radial1).clamp(box_.min_on(radial1), box_.max_on(radial1));
    let delta0 = cylinder.center(radial0) - nearest0;
    let delta1 = cylinder.center(radial1) - nearest1;
    let distance = (delta0 * delta0 + delta1 * delta1).sqrt();
    radius - distance > PACKING_GEOMETRY_OVERLAP_TOLERANCE
}

/// 同轴圆柱-圆柱重叠 / Same-axis cylinder-cylinder overlap
///
/// 同轴圆柱在径向平面上都是圆形，重叠判定：
/// 轴向区间重叠 + 两圆心距离 < 两半径之和。
fn same_axis_cylinder_overlaps(lhs: &ScalarGeometry, rhs: &ScalarGeometry) -> bool {
    let axis = lhs.cylinder_axis.unwrap_or(Axis3::Y);
    let lhs_radius = lhs.cylinder_radius.unwrap_or(0.0);
    let rhs_radius = rhs.cylinder_radius.unwrap_or(0.0);

    if !interval_overlaps(lhs.min_on(axis), lhs.max_on(axis), rhs.min_on(axis), rhs.max_on(axis)) {
        return false;
    }

    let [radial0, radial1] = axes_except(axis);
    let delta0 = lhs.center(radial0) - rhs.center(radial0);
    let delta1 = lhs.center(radial1) - rhs.center(radial1);
    let distance = (delta0 * delta0 + delta1 * delta1).sqrt();
    lhs_radius + rhs_radius - distance > PACKING_GEOMETRY_OVERLAP_TOLERANCE
}

/// 不同轴圆柱-圆柱重叠 / Different-axis cylinder-cylinder overlap
///
/// 两个圆柱轴不同时，在共享轴平面上的截面分别是两个圆。
/// 重叠判定通过截面半径投影计算。
fn different_axis_cylinder_overlaps(lhs: &ScalarGeometry, rhs: &ScalarGeometry) -> bool {
    let lhs_axis = lhs.cylinder_axis.unwrap_or(Axis3::Y);
    let rhs_axis = rhs.cylinder_axis.unwrap_or(Axis3::Y);
    let lhs_radius = lhs.cylinder_radius.unwrap_or(0.0);
    let rhs_radius = rhs.cylinder_radius.unwrap_or(0.0);

    // 共享轴 = 既不是 lhs 轴也不是 rhs 轴的那个轴
    let shared_axis = [Axis3::X, Axis3::Y, Axis3::Z]
        .iter()
        .find(|a| **a != lhs_axis && **a != rhs_axis)
        .unwrap();

    // lhs 轴方向上 rhs 到 lhs 区间的距离
    let lhs_axis_distance = distance_to_interval(
        rhs.center(lhs_axis),
        lhs.min_on(lhs_axis),
        lhs.max_on(lhs_axis),
    );
    // rhs 轴方向上 lhs 到 rhs 区间的距离
    let rhs_axis_distance = distance_to_interval(
        lhs.center(rhs_axis),
        rhs.min_on(rhs_axis),
        rhs.max_on(rhs_axis),
    );

    // 截面半径投影
    let lhs_shared_radius_squared = lhs_radius * lhs_radius - rhs_axis_distance * rhs_axis_distance;
    let rhs_shared_radius_squared = rhs_radius * rhs_radius - lhs_axis_distance * lhs_axis_distance;

    if lhs_shared_radius_squared <= PACKING_GEOMETRY_OVERLAP_TOLERANCE
        || rhs_shared_radius_squared <= PACKING_GEOMETRY_OVERLAP_TOLERANCE
    {
        return false;
    }

    let lhs_shared_radius = lhs_shared_radius_squared.sqrt();
    let rhs_shared_radius = rhs_shared_radius_squared.sqrt();

    interval_overlaps(
        lhs.center(*shared_axis) - lhs_shared_radius,
        lhs.center(*shared_axis) + lhs_shared_radius,
        rhs.center(*shared_axis) - rhs_shared_radius,
        rhs.center(*shared_axis) + rhs_shared_radius,
    )
}

/// 圆柱-圆柱重叠 / Cylinder-cylinder overlap
fn cylinder_cylinder_overlaps(lhs: &ScalarGeometry, rhs: &ScalarGeometry) -> bool {
    let lhs_axis = lhs.cylinder_axis.unwrap_or(Axis3::Y);
    let rhs_axis = rhs.cylinder_axis.unwrap_or(Axis3::Y);

    if lhs_axis == rhs_axis {
        same_axis_cylinder_overlaps(lhs, rhs)
    } else {
        different_axis_cylinder_overlaps(lhs, rhs)
    }
}

/// 检查横向圆柱是否有支撑覆盖 / Check if horizontal cylinder has support coverage
///
/// 复用 infrastructure 的 `horizontal_cylinder_cuboid_support_coverage`。
fn has_horizontal_cylinder_support_coverage(
    geometries: &[ScalarGeometry],
    index: usize,
) -> bool {
    let geo = &geometries[index];

    // 只有圆柱才需要检查
    if geo.shape_type != PackingShapeType::Cylinder {
        return true;
    }

    let axis = geo.cylinder_axis.unwrap_or(Axis3::Y);
    // 竖直圆柱不需要横向支撑检查
    if axis == Axis3::Y {
        return true;
    }
    // 贴地横向圆柱自动通过
    if geo.min_y.abs() <= PACKING_GEOMETRY_OVERLAP_TOLERANCE {
        return true;
    }

    // 复用 infrastructure 的支撑覆盖率算法
    let supports: Vec<HorizontalCylinderSupportGeometry> = geometries
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, g)| g.to_support_geometry())
        .collect();

    horizontal_cylinder_cuboid_support_coverage(
        geo.min_x,
        geo.max_x,
        geo.min_z,
        geo.max_z,
        geo.min_y,
        axis,
        &supports,
        PACKING_GEOMETRY_OVERLAP_TOLERANCE,
    )
}

impl<V, U> PackingGeometryContract<V, U> for PackingGeometryGuard
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    fn validate(&self, packed_bin: &PackedBin<V, U>) -> Result<(), Vec<String>> {
        Self::validate(packed_bin)
    }
}

