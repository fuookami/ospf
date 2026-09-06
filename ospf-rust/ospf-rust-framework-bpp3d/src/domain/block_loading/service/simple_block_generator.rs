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
    /// 最大竖向堆叠层数 / Maximum vertical stack layers
    pub max_stack_layers: Option<u64>,
}

impl Default for SimpleBlockGeneratorConfig {
    fn default() -> Self {
        Self {
            with_rotation: true,
            with_remainder: false,
            max_stack_layers: None,
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
        self.generate_internal(items, amounts, container_size, None)
    }

    /// 生成属性感知简单块 / Generate package-attribute-aware simple blocks
    pub fn generate_with_package_attributes<V, U>(
        &self,
        items: &[ActualItem<V, U>],
        amounts: &[u64],
        container_size: &MetricSize3<V, U>,
        package_attributes: &[Option<&PackageAttribute>],
    ) -> Vec<Block<V, U>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        self.generate_internal(items, amounts, container_size, Some(package_attributes))
    }

    fn generate_internal<V, U>(
        &self,
        items: &[ActualItem<V, U>],
        amounts: &[u64],
        container_size: &MetricSize3<V, U>,
        package_attributes: Option<&[Option<&PackageAttribute>]>,
    ) -> Vec<Block<V, U>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let mut blocks = Vec::new();

        for (item_idx, item) in items.iter().enumerate() {
            let amount = amounts.get(item_idx).copied().unwrap_or(1);
            let packing_shape = item.packing_shape();
            let package_attribute = package_attributes
                .and_then(|attributes| attributes.get(item_idx).copied().flatten());

            let orientations = self.enabled_orientations_at_space(
                item,
                package_attribute,
                container_size,
            );

            for orientation in &orientations {
                let item_view = self.create_item_view(item_idx, item, *orientation);

                match packing_shape.shape_type {
                    PackingShapeType::Cuboid => {
                        // 长方体：遍历各方向数量
                        let max_nx = Self::max_count(&container_size.width.value, &item_view.width.value);
                        let max_ny = Self::max_count(&container_size.height.value, &item_view.height.value);
                        let max_ny = self.config.max_stack_layers
                            .map(|limit| max_ny.min(limit))
                            .unwrap_or(max_ny);
                        let max_ny = Self::limit_vertical_count(
                            max_ny,
                            &item_view.height.value,
                            package_attribute,
                            item,
                            *orientation,
                        );
                        let Some((min_nz, max_nz)) = Self::depth_count_bounds(
                            &container_size.depth.value,
                            &item_view.depth.value,
                            package_attribute,
                        ) else {
                            continue;
                        };

                        for nx in 1..=max_nx {
                            for ny in 1..=max_ny {
                                for nz in min_nz..=max_nz {
                                    let total = nx * ny * nz;
                                    if total > amount && !self.config.with_remainder {
                                        continue;
                                    }
                                    let block = SimpleBlock::from_item_view(
                                        item_view.clone(),
                                        nx, ny, nz,
                                    );
                                    if !Self::attribute_allows_depth(
                                        &block.depth.value,
                                        package_attribute,
                                    ) {
                                        continue;
                                    }
                                    blocks.push(Block::Simple(block));
                                }
                            }
                        }
                    }
                    PackingShapeType::Cylinder => {
                        // 圆柱：简单块只生成单物品（1x1x1）
                        if amount == 0 {
                            continue;
                        }
                        if Self::limit_vertical_count(
                            1,
                            &item_view.height.value,
                            package_attribute,
                            item,
                            *orientation,
                        ) == 0 {
                            continue;
                        }
                        if !Self::attribute_allows_depth(&item_view.depth.value, package_attribute) {
                            continue;
                        }
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

    fn limit_vertical_count<V>(
        max_count: u64,
        item_height: &V,
        package_attribute: Option<&PackageAttribute>,
        item: &ActualItem<V, impl CTUnit>,
        orientation: Orientation,
    ) -> u64
    where
        V: num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
    {
        let Some(attribute) = package_attribute else {
            return max_count;
        };
        let mut limit = max_count;
        let orientation_enabled = item.enabled_orientations.is_empty()
            || item.enabled_orientations.contains(&orientation);
        if let Some(max_layer) = attribute.max_layer_for_orientation(orientation, orientation_enabled) {
            limit = limit.min(max_layer);
        }
        if let Some(max_stack_layers) = attribute.max_stack_layers {
            limit = limit.min(max_stack_layers);
        }
        if let Some(max_height) = attribute.max_height {
            let Some(item_height) = item_height.to_f64() else {
                return limit;
            };
            if item_height <= 0.0 || max_height <= 0.0 {
                return 0;
            }
            let max_by_height = (max_height / item_height).floor();
            if max_by_height.is_finite() {
                limit = limit.min(max_by_height.max(0.0) as u64);
            }
        }
        limit
    }

    fn depth_count_bounds<V>(
        container_depth: &V,
        item_depth: &V,
        package_attribute: Option<&PackageAttribute>,
    ) -> Option<(u64, u64)>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
    {
        let mut max_count = Self::max_count(container_depth, item_depth);
        let Some(attribute) = package_attribute else {
            return (max_count >= 1).then_some((1, max_count));
        };
        let item_depth = item_depth.to_f64()?;
        if item_depth <= 0.0 {
            return None;
        }
        if let Some(max_depth) = attribute.max_depth {
            if max_depth <= 0.0 {
                return None;
            }
            let max_by_depth = (max_depth / item_depth).floor();
            if max_by_depth.is_finite() {
                max_count = max_count.min(max_by_depth.max(0.0) as u64);
            }
        }
        let min_count = if attribute.min_depth > 0.0 {
            let min_by_depth = (attribute.min_depth / item_depth).ceil();
            if !min_by_depth.is_finite() {
                return None;
            }
            min_by_depth.max(1.0) as u64
        } else {
            1
        };
        (min_count <= max_count).then_some((min_count, max_count))
    }

    fn attribute_allows_depth<V>(
        block_depth: &V,
        package_attribute: Option<&PackageAttribute>,
    ) -> bool
    where
        V: num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
    {
        package_attribute
            .and_then(|attribute| block_depth.to_f64().map(|depth| attribute.enabled_depth(depth)))
            .unwrap_or(true)
    }

    fn enabled_orientations_at_space<V, U>(
        &self,
        item: &ActualItem<V, U>,
        package_attribute: Option<&PackageAttribute>,
        container_size: &MetricSize3<V, U>,
    ) -> Vec<Orientation>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let mut orientations = if item.enabled_orientations.is_empty() {
            vec![Orientation::Upright]
        } else {
            item.enabled_orientations.clone()
        };
        if let Some(attribute) = package_attribute {
            if container_size.height.value <= item.height.value {
                if attribute.enabled_side_on_top() {
                    for orientation in Orientation::ALL
                        .iter()
                        .copied()
                        .filter(|orientation| {
                            orientation.category()
                                == crate::infrastructure::orientation::OrientationCategory::Side
                        })
                    {
                        if !orientations.contains(&orientation) {
                            orientations.push(orientation);
                        }
                    }
                }
                if attribute.enabled_lie_on_top() {
                    for orientation in Orientation::ALL
                        .iter()
                        .copied()
                        .filter(|orientation| {
                            orientation.category()
                                == crate::infrastructure::orientation::OrientationCategory::Lie
                        })
                    {
                        if !orientations.contains(&orientation) {
                            orientations.push(orientation);
                        }
                    }
                }
            }
        }
        orientations
            .into_iter()
            .filter(|orientation| self.config.with_rotation || !orientation.is_rotated())
            .filter(|orientation| {
                package_attribute
                    .map(|attribute| {
                        attribute.enabled_orientation_by_rule(&PackageOrientationRuleInput {
                            orientation: *orientation,
                            space_width: container_size.width.value.to_f64().unwrap_or(f64::INFINITY),
                            space_height: container_size.height.value.to_f64().unwrap_or(f64::INFINITY),
                            space_depth: container_size.depth.value.to_f64().unwrap_or(f64::INFINITY),
                        })
                    })
                    .unwrap_or(true)
            })
            .collect()
    }

    /// 创建物品视图 / Create item view
    fn create_item_view<V, U>(
        &self,
        item_index: usize,
        item: &ActualItem<V, U>,
        orientation: Orientation,
    ) -> ItemView<V, U>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let oriented_shape = item.oriented_packing_shape(orientation);

        ItemView {
            item_index,
            orientation,
            width: oriented_shape.bounding_width.clone(),
            height: oriented_shape.bounding_height.clone(),
            depth: oriented_shape.bounding_depth.clone(),
            weight: item.weight.clone(),
            packing_shape: oriented_shape,
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

