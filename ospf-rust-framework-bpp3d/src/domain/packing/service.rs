//! 装箱服务 / Packing services
//!
//! 最终装箱、几何守卫和渲染适配。
//! Final packing, geometry guard, and renderer adapter.

use std::collections::HashMap;
use std::fmt::Debug;

use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::{ActualItem, BinLayer, BinType, Bpp3dDemandKey, MaterialKey};
use crate::domain::layer_generation::{LayerBlockTrace, LayerPlacementTrace};
use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::PackingShapeType;
use crate::infrastructure::pwl_approximation::{
    HorizontalCylinderSupportGeometry, horizontal_cylinder_cuboid_support_coverage,
};
use crate::infrastructure::renderer::RenderLoadingPlanDto;

use super::model::{MaterialSummary, PackedBin, PackedItem};

// ============================================================================
// PackingGeometryContract - 装箱几何契约 / Packing geometry contract
// ============================================================================

/// 装箱几何契约 / Packing geometry contract
///
/// 定义装箱几何验证的核心接口。
/// Defines the core interface for packing geometry verification.
pub trait PackingGeometryContract<V, U>: Debug + Send + Sync
where
    V: Debug + Clone + Send + Sync + num_traits::Float + Field + num_traits::FloatConst + PartialOrd + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    /// 验证装箱几何是否合法 / Validate packing geometry
    fn validate(&self, packed_bin: &PackedBin<V, U>) -> Result<(), Vec<String>>;
}

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

// ============================================================================
// LayerPlacementAdapter - 层放置适配器 / Layer placement adapter
// ============================================================================

/// 已知坐标放置 / Known-coordinate placement
#[derive(Debug, Clone)]
pub struct KnownCoordinatePlacement<V, U: UnitTrait> {
    /// 原始物品索引 / Original item index
    pub item_index: usize,
    /// 物品 / Item
    pub item: ActualItem<V, U>,
    /// 坐标 / Position
    pub position: MetricPoint3<V, U>,
    /// 朝向 / Orientation
    pub orientation: Orientation,
}

/// 层放置适配器 / Layer placement adapter
///
/// 将 Rust 版 typed geometry 放置转换为最终装箱模型，不使用 Kotlin `QuantityPlacement*`。
/// Converts Rust typed-geometry placements into final packing models without
/// Kotlin `QuantityPlacement*` migration types.
#[derive(Debug, Clone, Default)]
pub struct LayerPlacementAdapter;

impl LayerPlacementAdapter {
    /// 创建适配器 / Create adapter
    pub fn new() -> Self {
        Self
    }

    /// 转换单个放置 / Convert one placement
    pub fn to_packed_item<V, U>(&self, placement: KnownCoordinatePlacement<V, U>) -> PackedItem<V, U>
    where
        V: Field + Clone + Debug + Send + Sync + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let packing_shape = placement.item.packing_shape();
        PackedItem {
            item_index: placement.item_index,
            item: placement.item,
            position: placement.position,
            orientation: placement.orientation,
            packing_shape,
            loading_order: placement.item_index as u64,
        }
    }

    /// 转换并验证已知坐标箱 / Convert and validate known-coordinate bin
    pub fn to_packed_bin<V, U>(
        &self,
        name: String,
        bin_type: BinType<V, U>,
        batch_no: Option<String>,
        placements: Vec<KnownCoordinatePlacement<V, U>>,
    ) -> Result<PackedBin<V, U>, Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        let items = placements
            .into_iter()
            .map(|placement| self.to_packed_item(placement))
            .collect();
        let packed_bin = PackedBin {
            name,
            bin_type,
            batch_no,
            items,
        };

        PackingGeometryGuard::validate(&packed_bin)?;
        Ok(packed_bin)
    }
}

// ============================================================================
// LayerTraceReplayAdapter - 层 trace 回放适配器 / Layer trace replay adapter
// ============================================================================

/// 层 trace 回放结果 / Layer trace replay result
#[derive(Debug, Clone, Default)]
pub struct LayerTraceReplayResult<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// 层 trace 回放适配器 / Layer trace replay adapter
///
/// 将 layer generation trace 和 coverage fallback 回放为最终已装箱结果。
/// Replays layer generation traces and coverage fallback into final packed bins.
#[derive(Debug, Clone, Default)]
pub struct LayerTraceReplayAdapter {
    placement_adapter: LayerPlacementAdapter,
}

impl LayerTraceReplayAdapter {
    /// 创建适配器 / Create adapter
    pub fn new() -> Self {
        Self {
            placement_adapter: LayerPlacementAdapter::new(),
        }
    }

    /// 回放选中层 / Replay selected layers
    pub fn replay_selected_layers<V, U>(
        &self,
        layers: &[BinLayer<V, U>],
        layer_indices: &[usize],
        items: &[ActualItem<V, U>],
        bins: &[BinType<V, U>],
        block_traces: &HashMap<usize, Vec<LayerBlockTrace<V, U>>>,
        placement_traces: &HashMap<usize, Vec<LayerPlacementTrace<V, U>>>,
    ) -> LayerTraceReplayResult<V, U>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone + Debug + Send + Sync,
    {
        let mut packed_bins = Vec::new();
        let mut diagnostics = Vec::new();

        for (layer_idx, layer) in layers.iter().enumerate() {
            let original_layer_index = layer_indices.get(layer_idx).copied().unwrap_or(layer_idx);
            let Some(bin_type) = layer.bin.clone().or_else(|| bins.first().cloned()) else {
                diagnostics.push(format!("selected layer {} has no bin type", layer_idx));
                continue;
            };
            let placements = if let Some(traces) = block_traces.get(&original_layer_index) {
                placements_from_block_traces(traces, items, &mut diagnostics)
            } else if let Some(traces) = placement_traces.get(&original_layer_index) {
                placements_from_traces(traces, items, &bin_type, &mut diagnostics)
            } else {
                placements_from_coverage(layer, items, layer_idx, &mut diagnostics)
            };
            if placements.is_empty() {
                continue;
            }
            match self.placement_adapter.to_packed_bin(
                format!("selected-layer-{}", layer_idx),
                bin_type,
                None,
                placements,
            ) {
                Ok(packed_bin) => packed_bins.push(packed_bin),
                Err(errors) => diagnostics.extend(errors),
            }
        }

        LayerTraceReplayResult {
            packed_bins,
            diagnostics,
        }
    }
}

fn placements_from_block_traces<V, U>(
    traces: &[LayerBlockTrace<V, U>],
    items: &[ActualItem<V, U>],
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let mut placements = Vec::new();
    for trace in traces {
        let Some(item) = items.get(trace.item_index).cloned() else {
            diagnostics.push(format!(
                "block trace references missing item index {}",
                trace.item_index,
            ));
            continue;
        };
        placements.extend(expand_block_trace(trace, item));
    }
    placements
}

fn placements_from_traces<V, U>(
    traces: &[LayerPlacementTrace<V, U>],
    items: &[ActualItem<V, U>],
    bin_type: &BinType<V, U>,
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let mut placements = Vec::new();
    for trace in traces {
        let Some(item) = items.get(trace.item_index).cloned() else {
            diagnostics.push(format!(
                "placement trace references missing item index {}",
                trace.item_index,
            ));
            continue;
        };
        placements.extend(expand_placement_trace(trace, item, bin_type, diagnostics));
    }
    placements
}

fn expand_block_trace<V, U>(
    trace: &LayerBlockTrace<V, U>,
    item: ActualItem<V, U>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let size = oriented_item_size(&item, trace.orientation);
    let mut placements = Vec::new();
    let mut emitted = 0_u64;
    for z in 0..trace.nz {
        for y in 0..trace.ny {
            for x in 0..trace.nx {
                if emitted >= trace.item_count {
                    return placements;
                }
                placements.push(KnownCoordinatePlacement {
                    item_index: trace.item_index,
                    item: item.clone(),
                    position: MetricPoint3 {
                        x: quantity_from_f64(trace.origin.x.value.clone().into() + size.width.value.clone().into() * x as f64),
                        y: quantity_from_f64(trace.origin.y.value.clone().into() + size.height.value.clone().into() * y as f64),
                        z: quantity_from_f64(trace.origin.z.value.clone().into() + size.depth.value.clone().into() * z as f64),
                    },
                    orientation: trace.orientation,
                });
                emitted += 1;
            }
        }
    }
    placements
}

fn placements_from_coverage<V, U>(
    layer: &BinLayer<V, U>,
    items: &[ActualItem<V, U>],
    layer_idx: usize,
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let Some((item_index, item)) = first_covered_item(layer, items) else {
        diagnostics.push(format!("selected layer {} has no matching covered item", layer_idx));
        return Vec::new();
    };
    vec![KnownCoordinatePlacement {
        item_index,
        item,
        position: MetricPoint3 {
            x: quantity_from_f64(0.0),
            y: quantity_from_f64(0.0),
            z: quantity_from_f64(0.0),
        },
        orientation: Orientation::Upright,
    }]
}

fn expand_placement_trace<V, U>(
    trace: &LayerPlacementTrace<V, U>,
    item: ActualItem<V, U>,
    bin_type: &BinType<V, U>,
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let size = oriented_item_size(&item, trace.orientation);
    let max_x = grid_capacity(
        bin_type.width.value.clone().into() - trace.position.x.value.clone().into(),
        size.width.value.clone().into(),
    );
    let max_y = grid_capacity(
        bin_type.height.value.clone().into() - trace.position.y.value.clone().into(),
        size.height.value.clone().into(),
    );
    let max_z = grid_capacity(
        bin_type.depth.value.clone().into() - trace.position.z.value.clone().into(),
        size.depth.value.clone().into(),
    );
    let layer_capacity = max_x.saturating_mul(max_y).saturating_mul(max_z);
    if trace.amount > layer_capacity {
        diagnostics.push(format!(
            "placement trace for item {} truncated from {} to {} placements",
            trace.item_id,
            trace.amount,
            layer_capacity,
        ));
    }
    (0..trace.amount.min(layer_capacity))
        .map(|offset| {
            let x_index = offset % max_x;
            let y_index = (offset / max_x) % max_y;
            let z_index = offset / max_x / max_y;
            KnownCoordinatePlacement {
                item_index: trace.item_index,
                item: item.clone(),
                position: MetricPoint3 {
                    x: quantity_from_f64(
                        trace.position.x.value.clone().into()
                            + size.width.value.clone().into() * x_index as f64,
                    ),
                    y: quantity_from_f64(
                        trace.position.y.value.clone().into()
                            + size.height.value.clone().into() * y_index as f64,
                    ),
                    z: quantity_from_f64(
                        trace.position.z.value.clone().into()
                            + size.depth.value.clone().into() * z_index as f64,
                    ),
                },
                orientation: trace.orientation,
            }
        })
        .collect()
}

fn grid_capacity(available: f64, step: f64) -> u64 {
    if available <= 0.0 || step <= 0.0 {
        return 1;
    }
    (available / step).floor().max(1.0) as u64
}

fn oriented_item_size<V, U>(
    item: &ActualItem<V, U>,
    orientation: Orientation,
) -> MetricSize3<V, U>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let perm = orientation.to_axis_permutation();
    let cuboid = ospf_rust_math::geometry::Cuboid3::new(
        item.width.value.clone().into(),
        item.height.value.clone().into(),
        item.depth.value.clone().into(),
    );
    let permuted = perm.apply_cuboid(&cuboid);
    MetricSize3 {
        width: quantity_from_f64(permuted.width),
        height: quantity_from_f64(permuted.height),
        depth: quantity_from_f64(permuted.depth),
    }
}

fn first_covered_item<V, U>(
    layer: &BinLayer<V, U>,
    items: &[ActualItem<V, U>],
) -> Option<(usize, ActualItem<V, U>)>
where
    V: Clone + Debug + Send + Sync,
    U: UnitTrait + Debug + Clone + Send + Sync,
{
    layer
        .demand_coverage
        .iter()
        .find_map(|coverage| match &coverage.key {
            Bpp3dDemandKey::Item { id } if coverage.coefficient > 0.0 => {
                items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| &item.id == id)
                    .map(|(index, item)| (index, item.clone()))
            }
            _ => None,
        })
}

fn quantity_from_f64<V, U>(value: f64) -> Quantity<V, U>
where
    V: num_traits::Float,
    U: CTUnit + Default,
{
    Quantity::new_ct(V::from(value).unwrap_or_else(V::zero))
}

// ============================================================================
// Packer - 装箱器 / Packer
// ============================================================================

/// 装箱结果 / Packing result
#[derive(Debug, Clone)]
pub struct PackingResult<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
    /// 物料汇总 / Material summaries
    pub material_summaries: Vec<MaterialSummary>,
    /// 附加信息 / Additional info
    pub info: HashMap<String, String>,
}

/// 装箱聚合 / Packing aggregation
#[derive(Debug, Clone)]
pub struct PackingAggregation<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
}

/// 装箱器 / Packer
///
/// 将最终箱子转换为装箱结果，汇总物料使用情况。
/// Converts final bins into packing results, summarizes material usage.
#[derive(Debug, Clone, Default)]
pub struct Packer;

impl Packer {
    /// 创建装箱器 / Create a packer
    pub fn new() -> Self {
        Self
    }

    /// 执行装箱分析 / Execute packing analysis
    ///
    /// 对已分配的箱子执行最终装箱验证和物料汇总。
    /// Performs final packing validation and material summarization
    /// for assigned bins.
    pub fn invoke<V, U>(&self, packed_bins: Vec<PackedBin<V, U>>) -> PackingResult<V, U>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        let material_summaries = Self::summarize_materials(&packed_bins);

        PackingResult {
            packed_bins,
            material_summaries,
            info: HashMap::new(),
        }
    }

    /// 汇总物料使用 / Summarize material usage
    ///
    /// 从每个物品的 Package.materials 中提取物料使用量。
    /// Extracts material usage from each item's Package.materials.
    fn summarize_materials<V, U>(bins: &[PackedBin<V, U>]) -> Vec<MaterialSummary>
    where
        V: Clone + Debug + Send + Sync + Field + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let mut summary: HashMap<MaterialKey, u64> = HashMap::new();
        for bin in bins {
            for item in &bin.items {
                // 从 package.materials 提取物料使用量
                if let Some(pack) = &item.item.pack {
                    for (material_key, amount) in &pack.materials {
                        let entry = summary.entry(material_key.clone()).or_insert(0);
                        *entry += amount;
                    }
                }
            }
        }
        summary.into_iter()
            .map(|(material, amount)| MaterialSummary { material, amount })
            .collect()
    }
}

// ============================================================================
// MaterialPacker - 物料装箱器 / Material packer
// ============================================================================

/// 物料装箱器 / Material packer
///
/// 对物料维度执行装箱汇总。
/// Performs packing summarization on the material dimension.
#[derive(Debug, Clone, Default)]
pub struct MaterialPacker;

impl MaterialPacker {
    /// 创建物料装箱器 / Create a material packer
    pub fn new() -> Self {
        Self
    }

    /// 汇总物料装箱结果 / Summarize material packing results
    pub fn invoke<V, U>(&self, result: &PackingResult<V, U>) -> Vec<MaterialSummary>
    where
        U: UnitTrait,
    {
        result.material_summaries.clone()
    }
}

// ============================================================================
// PackingRendererAdapter - 装箱渲染适配器 / Packing renderer adapter
// ============================================================================

/// 装箱渲染适配器 / Packing renderer adapter
///
/// 将装箱结果转换为渲染 DTO，输出 actualVolume。
/// Converts packing results to render DTOs, outputting actualVolume.
#[derive(Debug, Clone, Default)]
pub struct PackingRendererAdapter;

impl PackingRendererAdapter {
    /// 创建渲染适配器 / Create renderer adapter
    pub fn new() -> Self {
        Self
    }

    /// 将装箱结果转换为渲染 DTO / Convert packing result to render DTO
    ///
    /// 所有几何量在 f64 标量域转换，使用 actualVolume。
    /// All geometric quantities are converted in the f64 scalar domain,
    /// using actualVolume for true volume (not bounding cuboid volume).
    pub fn to_render_dto<V, U>(&self, result: &PackingResult<V, U>) -> Vec<RenderLoadingPlanDto>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        result.packed_bins.iter().map(|bin| {
            let items: Vec<crate::infrastructure::renderer::RenderLoadingPlanItemDto> = bin.items.iter().map(|item| {
                let (render_shape_type, render_algo_shape_type, radius, axis) = match item.packing_shape.shape_type {
                    PackingShapeType::Cuboid => (
                        crate::infrastructure::renderer::RenderShapeType::Cuboid,
                        crate::infrastructure::renderer::RenderAlgorithmShapeType::Cuboid,
                        None,
                        None,
                    ),
                    PackingShapeType::Cylinder => {
                        let axis = item.packing_shape.axis.unwrap_or(Axis3::Y);
                        let render_axis = crate::infrastructure::renderer::RenderAxis3::from(axis);
                        // 从 infrastructure::PackingAlgorithmShapeType 获取渲染类型
                        let infra_algo = crate::infrastructure::PackingAlgorithmShapeType::from_cylinder_axis(axis);
                        let algo_type = match infra_algo {
                            crate::infrastructure::PackingAlgorithmShapeType::Cuboid =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::Cuboid,
                            crate::infrastructure::PackingAlgorithmShapeType::VerticalCylinder =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::VerticalCylinder,
                            crate::infrastructure::PackingAlgorithmShapeType::HorizontalCylinderX =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::HorizontalCylinderX,
                            crate::infrastructure::PackingAlgorithmShapeType::HorizontalCylinderZ =>
                                crate::infrastructure::renderer::RenderAlgorithmShapeType::HorizontalCylinderZ,
                        };
                        let r = item.packing_shape.radius.as_ref()
                            .map(|q| -> f64 { q.value.clone().into() });
                        (
                            crate::infrastructure::renderer::RenderShapeType::Cylinder,
                            algo_type,
                            r,
                            Some(render_axis),
                        )
                    }
                };

                crate::infrastructure::renderer::RenderLoadingPlanItemDto {
                    name: item.item.id.clone(),
                    package_type: "default".to_string(),
                    width: item.packing_shape.bounding_width.value.clone().into(),
                    height: item.packing_shape.bounding_height.value.clone().into(),
                    depth: item.packing_shape.bounding_depth.value.clone().into(),
                    x: item.position.x.value.clone().into(),
                    y: item.position.y.value.clone().into(),
                    z: item.position.z.value.clone().into(),
                    weight: item.item.weight.value.clone().into(),
                    loading_order: item.loading_order,
                    shape_type: render_shape_type,
                    algorithm_shape_type: render_algo_shape_type,
                    radius,
                    axis,
                    bounding_width: item.packing_shape.bounding_width.value.clone().into(),
                    bounding_height: item.packing_shape.bounding_height.value.clone().into(),
                    bounding_depth: item.packing_shape.bounding_depth.value.clone().into(),
                    // 使用 actualVolume，不只使用 bounding cuboid volume
                    actual_volume: item.actual_volume().value.clone().into(),
                    info: None,
                }
            }).collect();

            let total_volume: f64 = bin.total_actual_volume().value.clone().into();
            let bin_volume: f64 = bin.bin_type.width.value.clone().into()
                * bin.bin_type.height.value.clone().into()
                * bin.bin_type.depth.value.clone().into();

            crate::infrastructure::renderer::RenderLoadingPlanDto {
                group: "default".to_string(),
                name: bin.name.clone(),
                type_code: bin.bin_type.type_code.clone(),
                width: bin.bin_type.width.value.clone().into(),
                height: bin.bin_type.height.value.clone().into(),
                depth: bin.bin_type.depth.value.clone().into(),
                loading_rate: if bin_volume > 0.0 { total_volume / bin_volume } else { 0.0 },
                weight: bin.total_weight().value.clone().into(),
                volume: total_volume,
                items,
                info: None,
            }
        }).collect()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::model::PackageSolutionLikeAdapter;
    use crate::domain::item::{ActualItem, BinType, PackageShapeSpec, Package, MaterialType};
    use crate::infrastructure::geometry::MetricPoint3;
    use crate::infrastructure::orientation::Orientation;
    use crate::infrastructure::packing_shape::cuboid_packing_shape;
    use ospf_rust_quantities::quantity::Quantity;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn make_bin_type() -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN-10".to_string(),
            is_main: true,
        }
    }

    fn make_cuboid_packed_item(index: usize, x: f64, y: f64, z: f64, w: f64, h: f64, d: f64) -> PackedItem<f64, Meter> {
        PackedItem {
            item_index: index,
            item: crate::domain::item::ActualItem {
                id: format!("item_{}", index),
                name: format!("Item {}", index),
                package_code: None,
                pack: None,
                width: meters(w),
                height: meters(h),
                depth: meters(d),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: None,
            },
            position: MetricPoint3 { x: meters(x), y: meters(y), z: meters(z) },
            orientation: Orientation::Upright,
            packing_shape: cuboid_packing_shape(meters(w), meters(h), meters(d), meters(1.0)),
            loading_order: 0,
        }
    }

    #[test]
    fn packing_geometry_guard_valid_bin() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
                make_cuboid_packed_item(1, 5.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        };
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn packing_geometry_guard_overlap_detected() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
                make_cuboid_packed_item(1, 2.0, 0.0, 0.0, 5.0, 5.0, 5.0), // overlaps with item 0
            ],
        };
        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("overlap")));
    }

    #[test]
    fn packing_geometry_guard_out_of_bounds() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 8.0, 8.0, 8.0, 5.0, 5.0, 5.0), // exceeds 10.0
            ],
        };
        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("exceeds bin")));
    }

    #[test]
    fn packer_produces_result() {
        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        }];
        let packer = Packer::new();
        let result = packer.invoke(packed_bins);
        assert_eq!(result.packed_bins.len(), 1);
    }

    #[test]
    fn packed_item_actual_volume_cuboid() {
        let item = make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 2.0, 3.0, 4.0);
        let volume: f64 = item.actual_volume().value.into();
        assert!((volume - 24.0).abs() < 1e-10);
    }

    #[test]
    fn packed_bin_total_weight() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
                make_cuboid_packed_item(1, 5.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        };
        let weight: f64 = packed_bin.total_weight().value.into();
        assert!((weight - 2.0).abs() < 1e-10);
    }

    #[test]
    fn packed_bin_total_actual_volume() {
        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 2.0, 3.0, 4.0),
            ],
        };
        let vol: f64 = packed_bin.total_actual_volume().value.into();
        assert!((vol - 24.0).abs() < 1e-10);
    }

    #[test]
    fn packing_renderer_adapter_actual_volume() {
        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 2.0, 3.0, 4.0),
            ],
        }];
        let result = PackingResult {
            packed_bins,
            material_summaries: vec![],
            info: HashMap::new(),
        };
        let adapter = PackingRendererAdapter::new();
        let dtos = adapter.to_render_dto(&result);
        assert_eq!(dtos.len(), 1);
        // actualVolume 应为 24.0，不是 bounding cuboid volume
        assert_eq!(dtos[0].items.len(), 1);
        assert!((dtos[0].items[0].actual_volume - 24.0).abs() < 1e-10);
    }

    #[test]
    fn package_solution_like_adapter() {
        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![
                make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0),
            ],
        }];
        let adapter = PackageSolutionLikeAdapter::from_packing_result(packed_bins, vec![]);
        assert_eq!(adapter.bin_count(), 1);
        assert_eq!(adapter.total_item_count(), 1);
    }

    #[test]
    fn horizontal_cylinder_on_floor_accepted() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        let item = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl".to_string(),
                name: "Cylinder".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::X,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0)),
            loading_order: 0,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![item],
        };

        // 贴地横向圆柱应通过验证
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn horizontal_cylinder_on_cuboid_support_accepted() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 横向圆柱放在长方体上方，长方体在径向轴上完全覆盖圆柱底部
        let cuboid = make_cuboid_packed_item(0, 0.0, 0.0, 0.0, 10.0, 2.0, 10.0);

        // X 轴横向圆柱，放在 y=2.0（长方体顶部），径向轴 Z 上包围盒 0..10 被长方体覆盖
        let cylinder = PackedItem {
            item_index: 1,
            item: crate::domain::item::ActualItem {
                id: "cyl_x".to_string(),
                name: "Cylinder X".to_string(),
                package_code: None,
                pack: None,
                width: meters(5.0),
                height: meters(4.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::X,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(2.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0)),
            loading_order: 1,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![cuboid, cylinder],
        };

        // 方体完全支撑横向圆柱 → 应通过验证
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn material_packer_summarizes() {
        let result: PackingResult<f64, Meter> = PackingResult {
            packed_bins: vec![],
            material_summaries: vec![MaterialSummary {
                material: MaterialKey {
                    no: "MAT001".to_string(),
                    material_type: crate::domain::item::MaterialType::RawMaterial,
                },
                amount: 10,
            }],
            info: HashMap::new(),
        };
        let packer = MaterialPacker::new();
        let summaries = packer.invoke(&result);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].amount, 10);
    }

    #[test]
    fn packer_summarize_materials_from_package() {
        // 测试从 Package.materials 提取物料汇总
        let material_key = MaterialKey {
            no: "MAT001".to_string(),
            material_type: MaterialType::RawMaterial,
        };

        let pack = Package {
            code: Some("PKG001".to_string()),
            shape: crate::domain::item::PackageShape {
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(4.0),
                weight: meters(1.0),
                spec: PackageShapeSpec::Cuboid,
            },
            packages: None,
            materials: vec![(material_key.clone(), 5)],
            amount: 1,
        };

        let item = PackedItem {
            item_index: 0,
            item: ActualItem {
                id: "item_0".to_string(),
                name: "Item 0".to_string(),
                package_code: Some("PKG001".to_string()),
                pack: Some(pack),
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: None,
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cuboid_packing_shape(meters(2.0), meters(3.0), meters(4.0), meters(1.0)),
            loading_order: 0,
        };

        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![item],
        }];

        let packer = Packer::new();
        let result = packer.invoke(packed_bins);

        assert_eq!(result.material_summaries.len(), 1);
        assert_eq!(result.material_summaries[0].material.no, "MAT001");
        assert_eq!(result.material_summaries[0].amount, 5);
    }

    #[test]
    fn cylinder_box_real_footprint_no_overlap() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 竖直圆柱 r=2 at (0,0,0)，长方体 at (3.5,0,3.5)
        // 圆心 XZ=(2,2)，矩形(3.5..5.5, 3.5..5.5)，最近点(3.5,3.5)
        // 距离=sqrt(2.25+2.25)≈3.18>r=2 → 不重叠
        let cylinder = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_y".to_string(),
                name: "Cylinder Y".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        let box_ = make_cuboid_packed_item(1, 3.5, 0.0, 3.5, 2.0, 5.0, 2.0);

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![cylinder, box_],
        };

        // 真实 footprint 不重叠
        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn cylinder_cylinder_same_axis_overlap_detected() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 两个竖直圆柱，圆心距离小于半径之和 → overlap
        let cyl0 = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_0".to_string(),
                name: "Cyl 0".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        // 圆心 (3, 3) → 圆心距 ≈1.41 < r0+r1=4 → overlap
        let cyl1 = PackedItem {
            item_index: 1,
            item: crate::domain::item::ActualItem {
                id: "cyl_1".to_string(),
                name: "Cyl 1".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(1.0), y: meters(0.0), z: meters(1.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 1,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: BinType {
                width: meters(20.0),
                height: meters(20.0),
                depth: meters(20.0),
                capacity: meters(8000.0),
                type_code: "BIN-20".to_string(),
                is_main: true,
            },
            batch_no: None,
            items: vec![cyl0, cyl1],
        };

        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("overlap")));
    }

    #[test]
    fn cylinder_cylinder_same_axis_no_overlap() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 两个竖直圆柱，圆心距离大于半径之和 → no overlap
        let cyl0 = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_0".to_string(),
                name: "Cyl 0".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        // 圆心 (6, 6) → 圆心距≈5.66 > r0+r1=4 → no overlap
        let cyl1 = PackedItem {
            item_index: 1,
            item: crate::domain::item::ActualItem {
                id: "cyl_1".to_string(),
                name: "Cyl 1".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(4.0), y: meters(0.0), z: meters(4.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 1,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: BinType {
                width: meters(20.0),
                height: meters(20.0),
                depth: meters(20.0),
                capacity: meters(8000.0),
                type_code: "BIN-20".to_string(),
                is_main: true,
            },
            batch_no: None,
            items: vec![cyl0, cyl1],
        };

        assert!(PackingGeometryGuard::validate(&packed_bin).is_ok());
    }

    #[test]
    fn packing_renderer_adapter_cylinder_item() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 验证圆柱渲染 DTO 包含正确的 actualVolume (πr²h)
        let item = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl".to_string(),
                name: "Cylinder".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(0.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::Y, meters(1.0)),
            loading_order: 0,
        };

        let packed_bins = vec![PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![item],
        }];

        let result = PackingResult {
            packed_bins,
            material_summaries: vec![],
            info: HashMap::new(),
        };

        let adapter = PackingRendererAdapter::new();
        let dtos = adapter.to_render_dto(&result);
        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].items.len(), 1);
        // actualVolume 应为 π*2²*5 = 20π ≈ 62.83
        let expected_volume = std::f64::consts::PI * 4.0 * 5.0;
        assert!((dtos[0].items[0].actual_volume - expected_volume).abs() < 1e-10);
        // 应包含圆柱相关字段
        assert_eq!(dtos[0].items[0].shape_type, crate::infrastructure::renderer::RenderShapeType::Cylinder);
        assert_eq!(dtos[0].items[0].algorithm_shape_type, crate::infrastructure::renderer::RenderAlgorithmShapeType::VerticalCylinder);
        assert!(dtos[0].items[0].radius.is_some());
        assert!((dtos[0].items[0].radius.unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn horizontal_cylinder_hanging_rejected() {
        use crate::infrastructure::packing_shape::cylinder_packing_shape;

        // 横向圆柱不在地面也没有长方体支撑 → 应被拒绝
        // Horizontal cylinder not on floor and no cuboid support → should be rejected
        let cylinder = PackedItem {
            item_index: 0,
            item: crate::domain::item::ActualItem {
                id: "cyl_x".to_string(),
                name: "Cylinder X".to_string(),
                package_code: None,
                pack: None,
                width: meters(5.0),
                height: meters(4.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::X,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
            position: MetricPoint3 { x: meters(0.0), y: meters(5.0), z: meters(0.0) },
            orientation: Orientation::Upright,
            packing_shape: cylinder_packing_shape(meters(2.0), meters(5.0), Axis3::X, meters(1.0)),
            loading_order: 0,
        };

        let packed_bin = PackedBin {
            name: "bin-1".to_string(),
            bin_type: make_bin_type(),
            batch_no: None,
            items: vec![cylinder],
        };

        // 悬空横向圆柱应被拒绝
        let result = PackingGeometryGuard::validate(&packed_bin);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("support coverage")));
    }
}
