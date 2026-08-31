//! BLA 服务 / BLA services
//!
//! Bottom-Up-Left-Justified 算法实现，用于二维贪心放置。
//! Bottom-Up-Left-Justified algorithm implementation for 2D greedy placement.

use std::fmt::Debug;
use std::cmp::Ordering;

use ospf_rust_math::algebra::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::geometry::{MetricPoint2, MetricSize2, MetricAabb2};
use crate::infrastructure::packing_shape::ShapeFootprint2;

// ============================================================================
// BlaProjection - BLA 投影 / BLA projection
// ============================================================================

/// BLA 投影 / BLA projection
///
/// 三维物体在二维平面上的投影，用于 BLA 算法放置。
/// 2D projection of a 3D object on a plane, used for BLA algorithm placement.
#[derive(Debug, Clone)]
pub struct BlaProjection<V, U: UnitTrait> {
    /// 投影二维足迹 / 2D footprint
    pub footprint: ShapeFootprint2<V, U>,
    /// 原始三维形状索引 / Original 3D shape index
    pub item_index: usize,
    /// 是否仅允许底部放置 / Whether the item must stay at the bottom
    pub bottom_only: bool,
    /// 原始高度 / Original height
    pub height: Quantity<V, U>,
    /// 重量（用于排序）/ Weight (for sorting)
    pub weight: Quantity<V, U>,
    /// 允许旋转 / Allow rotation
    pub allow_rotation: bool,
}

// ============================================================================
// BlaPlacement - BLA 放置结果 / BLA placement result
// ============================================================================

/// BLA 放置结果 / BLA placement result
///
/// 二维贪心放置后的位置结果。
/// Position result after 2D greedy placement.
#[derive(Debug, Clone)]
pub struct BlaPlacement<V, U: UnitTrait> {
    /// 投影索引 / Projection index
    pub item_index: usize,
    /// 放置位置 / Placement position
    pub position: MetricPoint2<V, U>,
    /// 是否旋转 / Whether rotated
    pub rotated: bool,
}

// ============================================================================
// BlaConfig - BLA 配置 / BLA configuration
// ============================================================================

/// BLA 配置 / BLA configuration
///
/// 控制 Bottom-Up-Left-Justified 算法的行为。
/// Controls the behavior of the Bottom-Up-Left-Justified algorithm.
#[derive(Debug, Clone)]
pub struct BlaConfig {
    /// 启用 X 方向位移 / Enable X displacement
    pub with_displacement_x: bool,
    /// 启用 Y 方向位移 / Enable Y displacement
    pub with_displacement_y: bool,
    /// 启用旋转 / Enable rotation
    pub with_rotation: bool,
}

impl Default for BlaConfig {
    fn default() -> Self {
        Self {
            with_displacement_x: true,
            with_displacement_y: true,
            with_rotation: true,
        }
    }
}

impl BlaConfig {
    /// 创建默认配置 / Create default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// 禁用旋转 / Disable rotation
    pub fn no_rotation(mut self) -> Self {
        self.with_rotation = false;
        self
    }
}

// ============================================================================
// BottomUpLeftJustifiedAlgorithm - BLA 算法 / BLA algorithm
// ============================================================================

/// Bottom-Up-Left-Justified 算法 / Bottom-Up-Left-Justified algorithm
///
/// 二维贪心放置算法，按照底部优先、尺寸和重量排序投影，
/// 依次寻找左下角最靠近原点的可行位置。
///
/// 2D greedy placement algorithm that sorts projections by bottom-only priority,
/// shape size, height, and weight, then finds the bottom-left-most feasible position.
#[derive(Debug, Clone)]
pub struct BottomUpLeftJustifiedAlgorithm<V, U: UnitTrait> {
    /// 容器宽度 / Container width
    pub container_width: Quantity<V, U>,
    /// 容器深度 / Container depth
    pub container_depth: Quantity<V, U>,
    /// 配置 / Configuration
    pub config: BlaConfig,
}

impl<V, U> BottomUpLeftJustifiedAlgorithm<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    /// 创建 BLA 算法 / Create BLA algorithm
    pub fn new(
        container_width: Quantity<V, U>,
        container_depth: Quantity<V, U>,
        config: BlaConfig,
    ) -> Self {
        Self {
            container_width,
            container_depth,
            config,
        }
    }

    /// 执行 BLA 放置 / Execute BLA placement
    ///
    /// 对投影按底部优先、尺寸和重量排序，依次在容器中寻找最左下角的可行位置。
    /// Returns placements for projections that fit, and None for those that don't.
    ///
    /// Sorts projections by bottom-only priority, shape size, height, and weight,
    /// then sequentially finds the bottom-left-most feasible position in the container.
    pub fn invoke(&self, projections: &[BlaProjection<V, U>]) -> Vec<Option<BlaPlacement<V, U>>> {
        let mut sorted_indices: Vec<usize> = (0..projections.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            self.compare_projection(&projections[a], &projections[b])
                .then_with(|| a.cmp(&b))
        });

        let mut placements: Vec<Option<BlaPlacement<V, U>>> = vec![None; projections.len()];
        let mut placed_aabbs: Vec<MetricAabb2<V, U>> = Vec::new();

        for idx in sorted_indices {
            let projection = &projections[idx];
            let (width, depth) = self.footprint_dimensions(&projection.footprint, false);

            // 尝试找到最左下角位置
            if let Some((position, rotated)) = self.find_position(
                &width, &depth, projection, &placed_aabbs,
            ) {
                let aabb = MetricAabb2::new(
                    position.clone(),
                    MetricSize2 { width: width.clone(), height: depth.clone() },
                );
                placed_aabbs.push(aabb);
                placements[idx] = Some(BlaPlacement {
                    item_index: projection.item_index,
                    position,
                    rotated,
                });
            }
        }

        placements
    }

    /// 比较投影装载优先级 / Compare projection loading priority
    fn compare_projection(
        &self,
        lhs: &BlaProjection<V, U>,
        rhs: &BlaProjection<V, U>,
    ) -> Ordering {
        if lhs.bottom_only != rhs.bottom_only {
            return rhs.bottom_only.cmp(&lhs.bottom_only);
        }
        let (lhs_width, lhs_depth) = self.footprint_dimensions(&lhs.footprint, false);
        let (rhs_width, rhs_depth) = self.footprint_dimensions(&rhs.footprint, false);
        rhs_width
            .value
            .partial_cmp(&lhs_width.value)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                rhs_depth
                    .value
                    .partial_cmp(&lhs_depth.value)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                rhs.height
                    .value
                    .partial_cmp(&lhs.height.value)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                rhs.weight
                    .value
                    .partial_cmp(&lhs.weight.value)
                    .unwrap_or(Ordering::Equal)
            })
    }

    /// 获取足迹的宽深尺寸 / Get footprint width and depth dimensions
    fn footprint_dimensions(
        &self,
        footprint: &ShapeFootprint2<V, U>,
        rotated: bool,
    ) -> (Quantity<V, U>, Quantity<V, U>) {
        match footprint {
            ShapeFootprint2::Rectangle { width, depth } => {
                if rotated {
                    (depth.clone(), width.clone())
                } else {
                    (width.clone(), depth.clone())
                }
            }
            ShapeFootprint2::Circle { radius } => {
                // 圆的包围盒为直径 x 直径，旋转不影响
                let diameter = Quantity::new_ct(radius.value.clone() + radius.value.clone());
                (diameter.clone(), diameter)
            }
        }
    }

    /// 寻找可行位置 / Find feasible position
    fn find_position(
        &self,
        width: &Quantity<V, U>,
        depth: &Quantity<V, U>,
        projection: &BlaProjection<V, U>,
        placed: &[MetricAabb2<V, U>],
    ) -> Option<(MetricPoint2<V, U>, bool)> {
        // 先尝试不旋转
        if let Some(pos) = self.find_position_for_dims(width, depth, placed) {
            return Some((pos, false));
        }

        // 如果允许旋转，尝试旋转 90°（仅矩形有效）
        if self.config.with_rotation && projection.allow_rotation {
            if let ShapeFootprint2::Rectangle { .. } = &projection.footprint {
                if let Some(pos) = self.find_position_for_dims(depth, width, placed) {
                    return Some((pos, true));
                }
            }
        }

        None
    }

    /// 在给定尺寸下寻找最左下角可行位置 / Find bottom-left feasible position for given dimensions
    fn find_position_for_dims(
        &self,
        width: &Quantity<V, U>,
        depth: &Quantity<V, U>,
        placed: &[MetricAabb2<V, U>],
    ) -> Option<MetricPoint2<V, U>> {
        // 简化 BLA：使用贪心扫描
        // 检查容器边界
        if width.value > self.container_width.value || depth.value > self.container_depth.value {
            return None;
        }

        // 收集候选 Y 坐标（0 和已放置物体顶部）
        let mut y_candidates = vec![V::zero()];
        for aabb in placed {
            y_candidates.push(aabb.max_y().value.clone());
        }
        y_candidates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        y_candidates.dedup();

        // 对每个 Y 候选，尝试从左到右放置
        for y in &y_candidates {
            let mut x_candidates = vec![V::zero()];
            for aabb in placed {
                // 收集 X 候选：已放置物体右侧
                if *y < aabb.max_y().value && *y >= aabb.min.y.value {
                    x_candidates.push(aabb.max_x().value.clone());
                }
            }
            x_candidates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            x_candidates.dedup();

            for x in &x_candidates {
                let position = MetricPoint2 {
                    x: Quantity::new_ct(x.clone()),
                    y: Quantity::new_ct(y.clone()),
                };

                // 检查是否在容器内
                let x_plus_w = x.clone() + width.value.clone();
                let y_plus_d = y.clone() + depth.value.clone();
                if x_plus_w > self.container_width.value {
                    continue;
                }
                if y_plus_d > self.container_depth.value {
                    break; // Y 太大，后续只会更大
                }

                // 检查是否与已放置物体重叠
                let candidate = MetricAabb2::new(
                    position.clone(),
                    MetricSize2 { width: width.clone(), height: depth.clone() },
                );

                let overlaps = placed.iter().any(|aabb| candidate.overlaps(aabb));
                if !overlaps {
                    return Some(position);
                }
            }
        }

        None
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn bla_single_rectangle_fits() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );

        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Rectangle {
                width: meters(3.0),
                depth: meters(4.0),
            },
            item_index: 0,
            bottom_only: false,
            height: meters(3.0),
            weight: meters(1.0),
            allow_rotation: true,
        }];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        let p = placements[0].as_ref().unwrap();
        assert!(!p.rotated);
        assert_eq!(p.position.x.value, 0.0);
        assert_eq!(p.position.y.value, 0.0);
    }

    #[test]
    fn bla_two_rectangles_no_overlap() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );

        let projections = vec![
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle {
                    width: meters(5.0),
                    depth: meters(5.0),
                },
                item_index: 0,
                bottom_only: false,
                height: meters(5.0),
                weight: meters(2.0),
                allow_rotation: true,
            },
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle {
                    width: meters(5.0),
                    depth: meters(5.0),
                },
                item_index: 1,
                bottom_only: false,
                height: meters(5.0),
                weight: meters(1.0),
                allow_rotation: true,
            },
        ];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        assert!(placements[1].is_some());

        // 两个 5x5 矩形在 10x10 容器中不应重叠
        let p0 = placements[0].as_ref().unwrap();
        let p1 = placements[1].as_ref().unwrap();

        let aabb0 = MetricAabb2::new(p0.position.clone(), MetricSize2 {
            width: meters(5.0),
            height: meters(5.0),
        });
        let aabb1 = MetricAabb2::new(p1.position.clone(), MetricSize2 {
            width: meters(5.0),
            height: meters(5.0),
        });
        assert!(!aabb0.overlaps(&aabb1));
    }

    #[test]
    fn bla_too_large_fails() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(5.0),
            meters(5.0),
            BlaConfig::default(),
        );

        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Rectangle {
                width: meters(6.0),
                depth: meters(6.0),
            },
            item_index: 0,
            bottom_only: false,
            height: meters(6.0),
            weight: meters(1.0),
            allow_rotation: false,
        }];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_none());
    }

    #[test]
    fn bla_circle_fits() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );

        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Circle {
                radius: meters(2.0),
            },
            item_index: 0,
            bottom_only: false,
            height: meters(4.0),
            weight: meters(1.0),
            allow_rotation: false,
        }];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
    }

    #[test]
    fn bla_rotation_used() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(3.0), // 深度受限
            BlaConfig::default(),
        );

        // 2x8 矩形不旋转放不下（深度 8 > 3），但旋转后 8x2 可以（深度 2 < 3）
        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Rectangle {
                width: meters(2.0),
                depth: meters(8.0),
            },
            item_index: 0,
            bottom_only: false,
            height: meters(8.0),
            weight: meters(1.0),
            allow_rotation: true,
        }];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        assert!(placements[0].as_ref().unwrap().rotated);
    }

    #[test]
    fn bla_rotation_disabled() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(3.0),
            BlaConfig::new().no_rotation(),
        );

        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Rectangle {
                width: meters(2.0),
                depth: meters(8.0),
            },
            item_index: 0,
            bottom_only: false,
            height: meters(8.0),
            weight: meters(1.0),
            allow_rotation: true,
        }];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_none()); // 不旋转放不下
    }

    #[test]
    fn bla_container_full() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(5.0),
            meters(5.0),
            BlaConfig::default(),
        );

        // 第一个 5x5 占满容器，第二个放不下
        let projections = vec![
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle {
                    width: meters(5.0),
                    depth: meters(5.0),
                },
                item_index: 0,
                bottom_only: false,
                height: meters(5.0),
                weight: meters(2.0),
                allow_rotation: true,
            },
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle {
                    width: meters(3.0),
                    depth: meters(3.0),
                },
                item_index: 1,
                bottom_only: false,
                height: meters(3.0),
                weight: meters(1.0),
                allow_rotation: true,
            },
        ];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        assert!(placements[1].is_none()); // 容器已满
    }

    #[test]
    fn bla_bottom_only_precedes_weight() {
        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );

        let projections = vec![
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle {
                    width: meters(4.0),
                    depth: meters(4.0),
                },
                item_index: 0,
                bottom_only: false,
                height: meters(4.0),
                weight: meters(10.0),
                allow_rotation: true,
            },
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle {
                    width: meters(4.0),
                    depth: meters(4.0),
                },
                item_index: 1,
                bottom_only: true,
                height: meters(4.0),
                weight: meters(1.0),
                allow_rotation: true,
            },
        ];

        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        assert!(placements[1].is_some());
        assert_eq!(placements[1].as_ref().unwrap().position.x.value, 0.0);
        assert_eq!(placements[1].as_ref().unwrap().position.y.value, 0.0);
        assert_eq!(placements[0].as_ref().unwrap().position.x.value, 4.0);
        assert_eq!(placements[0].as_ref().unwrap().position.y.value, 0.0);
    }
}
