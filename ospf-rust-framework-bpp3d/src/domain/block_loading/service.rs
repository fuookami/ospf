//! 块装载服务 / Block loading services
//!
//! 简单块生成器和算法实现。
//! Simple block generator and algorithm implementations.

use std::fmt::Debug;
use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Cuboid3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::ActualItem;
use crate::infrastructure::geometry::MetricSize3;
use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::{PackingShape3, PackingShapeType};

use super::model::{Block, ItemView, SimpleBlock};

// ============================================================================
// SimpleBlockGenerator - 简单块生成器 / Simple block generator
// ============================================================================

/// 简单块生成器配置 / Simple block generator configuration
#[derive(Debug, Clone)]
pub struct SimpleBlockGeneratorConfig {
    /// 启用旋转 / Enable rotation
    pub with_rotation: bool,
    /// 启用余量 / Enable remainder blocks
    pub with_remainder: bool,
}

impl Default for SimpleBlockGeneratorConfig {
    fn default() -> Self {
        Self {
            with_rotation: true,
            with_remainder: false,
        }
    }
}

impl SimpleBlockGeneratorConfig {
    /// 创建默认配置 / Create default configuration
    pub fn new() -> Self {
        Self::default()
    }
}

/// 简单块生成器 / Simple block generator
///
/// 为每个物品的每个允许朝向，生成所有可能的简单块。
/// 长方体物品：遍历 X/Y/Z 方向数量。
/// 圆柱物品：生成单物品块（圆柱不堆叠在简单块中）。
///
/// Generates all possible simple blocks for each item under each enabled orientation.
/// Cuboid items: iterates X/Y/Z direction counts.
/// Cylinder items: generates single-item blocks (cylinders are not stacked in simple blocks).
#[derive(Debug, Clone)]
pub struct SimpleBlockGenerator {
    /// 配置 / Configuration
    pub config: SimpleBlockGeneratorConfig,
}

impl SimpleBlockGenerator {
    /// 创建简单块生成器 / Create a simple block generator
    pub fn new(config: SimpleBlockGeneratorConfig) -> Self {
        Self { config }
    }

    /// 创建默认配置的生成器 / Create a generator with default configuration
    pub fn default_generator() -> Self {
        Self::new(SimpleBlockGeneratorConfig::default())
    }

    /// 生成简单块 / Generate simple blocks
    pub fn generate<V, U>(
        &self,
        items: &[ActualItem<V, U>],
        amounts: &[u64],
        container_size: &MetricSize3<V, U>,
    ) -> Vec<Block<V, U>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let mut blocks = Vec::new();

        for (item_idx, item) in items.iter().enumerate() {
            let amount = amounts.get(item_idx).copied().unwrap_or(1);
            let packing_shape = item.packing_shape();

            // 获取允许的朝向
            let orientations: Vec<Orientation> = if item.enabled_orientations.is_empty() {
                vec![Orientation::Upright]
            } else {
                if self.config.with_rotation {
                    item.enabled_orientations.clone()
                } else {
                    item.enabled_orientations.iter()
                        .filter(|o| !o.is_rotated())
                        .copied()
                        .collect()
                }
            };

            for orientation in &orientations {
                let item_view = self.create_item_view(item_idx, item, &packing_shape, *orientation);

                match packing_shape.shape_type {
                    PackingShapeType::Cuboid => {
                        // 长方体：遍历各方向数量
                        let max_nx = Self::max_count(&container_size.width.value, &item_view.width.value);
                        let max_ny = Self::max_count(&container_size.height.value, &item_view.height.value);
                        let max_nz = Self::max_count(&container_size.depth.value, &item_view.depth.value);

                        for nx in 1..=max_nx {
                            for ny in 1..=max_ny {
                                for nz in 1..=max_nz {
                                    let total = nx * ny * nz;
                                    if total > amount && !self.config.with_remainder {
                                        continue;
                                    }
                                    let block = SimpleBlock::from_item_view(
                                        item_view.clone(),
                                        nx, ny, nz,
                                    );
                                    blocks.push(Block::Simple(block));
                                }
                            }
                        }
                    }
                    PackingShapeType::Cylinder => {
                        // 圆柱：简单块只生成单物品（1x1x1）
                        let block = SimpleBlock::from_item_view(
                            item_view.clone(),
                            1, 1, 1,
                        );
                        blocks.push(Block::Simple(block));
                    }
                }
            }
        }

        blocks
    }

    /// 创建物品视图 / Create item view
    fn create_item_view<V, U>(
        &self,
        item_index: usize,
        item: &ActualItem<V, U>,
        packing_shape: &PackingShape3<V, U>,
        orientation: Orientation,
    ) -> ItemView<V, U>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let perm = orientation.to_axis_permutation();
        let cuboid = Cuboid3::new(
            item.width.value.clone(),
            item.height.value.clone(),
            item.depth.value.clone(),
        );
        let permuted = perm.apply_cuboid(&cuboid);

        ItemView {
            item_index,
            orientation,
            width: Quantity::new_ct(permuted.width),
            height: Quantity::new_ct(permuted.height),
            depth: Quantity::new_ct(permuted.depth),
            weight: item.weight.clone(),
            packing_shape: packing_shape.clone(),
        }
    }

    /// 计算方向最大数量 / Calculate maximum count in a direction
    fn max_count<V>(container_dim: &V, item_dim: &V) -> u64
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
    {
        if *item_dim <= V::zero() {
            return 0;
        }
        let count = (*container_dim / *item_dim).floor().to_u64().unwrap_or(0);
        count
    }
}

/// 复杂块生成器占位 / Complex block generator placeholder
///
/// 第一版延后实现。
/// Deferred from first version.
#[derive(Debug, Clone, Default)]
pub struct ComplexBlockGenerator;

impl ComplexBlockGenerator {
    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec!["complex block generation is not implemented".to_string()]
    }
}

/// 深度优先搜索算法占位 / Depth-first search algorithm placeholder
///
/// 第一版延后实现。
/// Deferred from first version.
#[derive(Debug, Clone, Default)]
pub struct DepthFirstSearchAlgorithm;

impl DepthFirstSearchAlgorithm {
    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec!["depth-first search layer loading is not implemented".to_string()]
    }
}

/// 多层启发式搜索算法占位 / Multi-layer heuristic search algorithm placeholder
///
/// 第一版延后实现。
/// Deferred from first version.
#[derive(Debug, Clone, Default)]
pub struct MultiLayerHeuristicSearchAlgorithm;

impl MultiLayerHeuristicSearchAlgorithm {
    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec!["multi-layer heuristic search is not implemented".to_string()]
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_math::geometry::Axis3;
    use crate::domain::item::PackageShapeSpec;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn make_cuboid_item(id: &str, w: f64, h: f64, d: f64, weight: f64) -> ActualItem<f64, Meter> {
        ActualItem {
            id: id.to_string(),
            name: format!("Item {}", id),
            package_code: None,
            pack: None,
            width: meters(w),
            height: meters(h),
            depth: meters(d),
            weight: meters(weight),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }
    }

    fn make_cylinder_item(id: &str, radius: f64, h: f64, axis: Axis3, weight: f64) -> ActualItem<f64, Meter> {
        let diameter = 2.0 * radius;
        ActualItem {
            id: id.to_string(),
            name: format!("Cylinder {}", id),
            package_code: None,
            pack: None,
            width: meters(diameter),
            height: meters(h),
            depth: meters(diameter),
            weight: meters(weight),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis,
                radius: meters(radius),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        }
    }

    #[test]
    fn simple_block_generator_cuboid() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cuboid_item("i1", 2.0, 3.0, 4.0, 1.0)];
        let amounts = vec![10u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 应生成多个块（1x1x1, 2x1x1, 1x2x1, ...）
        assert!(!blocks.is_empty());

        // 验证所有块都是简单块
        for block in &blocks {
            if let Block::Simple(sb) = block {
                assert_eq!(sb.item_view.item_index, 0);
            }
        }
    }

    #[test]
    fn simple_block_generator_single_item() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cuboid_item("i1", 5.0, 5.0, 5.0, 1.0)];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 单物品、5x5x5 在 10x10x10 中，nx <= 2, ny <= 2, nz <= 2
        // 但 amount = 1，所以只能 nx*ny*nz <= 1
        // 只有 1x1x1 满足
        assert_eq!(blocks.len(), 1);
        if let Block::Simple(sb) = &blocks[0] {
            assert_eq!(sb.nx, 1);
            assert_eq!(sb.ny, 1);
            assert_eq!(sb.nz, 1);
        }
    }

    #[test]
    fn simple_block_generator_cylinder_single_item() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cylinder_item("c1", 2.0, 5.0, Axis3::Y, 1.0)];
        let amounts = vec![10u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 圆柱只生成 1x1x1 块
        assert_eq!(blocks.len(), 1);
        if let Block::Simple(sb) = &blocks[0] {
            assert_eq!(sb.nx, 1);
            assert_eq!(sb.ny, 1);
            assert_eq!(sb.nz, 1);
            assert_eq!(sb.item_count, 1);
        }
    }

    #[test]
    fn simple_block_generator_too_large() {
        let generator = SimpleBlockGenerator::default_generator();
        let items = vec![make_cuboid_item("i1", 15.0, 15.0, 15.0, 1.0)];
        let amounts = vec![1u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 物品太大，放不下
        assert!(blocks.is_empty());
    }

    #[test]
    fn simple_block_generator_no_rotation() {
        let config = SimpleBlockGeneratorConfig {
            with_rotation: false,
            with_remainder: false,
        };
        let generator = SimpleBlockGenerator::new(config);

        let items = vec![ActualItem {
            id: "i1".to_string(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright, Orientation::Side],
            shape_spec_override: None,
        }];
        let amounts = vec![10u64];
        let container = MetricSize3 {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
        };

        let blocks = generator.generate(&items, &amounts, &container);

        // 两种朝向（Upright + Side）都应生成块
        assert!(!blocks.is_empty());

        // 验证存在不同朝向的块
        let upright_count = blocks.iter().filter(|b| {
            if let Block::Simple(sb) = b {
                sb.item_view.orientation == Orientation::Upright
            } else {
                false
            }
        }).count();
        let side_count = blocks.iter().filter(|b| {
            if let Block::Simple(sb) = b {
                sb.item_view.orientation == Orientation::Side
            } else {
                false
            }
        }).count();
        assert!(upright_count > 0);
        assert!(side_count > 0);
    }

    #[test]
    fn advanced_block_skeletons_report_diagnostics() {
        assert!(ComplexBlockGenerator
            .diagnostics()[0]
            .contains("complex block generation"));
        assert!(DepthFirstSearchAlgorithm
            .diagnostics()[0]
            .contains("depth-first search"));
        assert!(MultiLayerHeuristicSearchAlgorithm
            .diagnostics()[0]
            .contains("multi-layer heuristic"));
    }
}
