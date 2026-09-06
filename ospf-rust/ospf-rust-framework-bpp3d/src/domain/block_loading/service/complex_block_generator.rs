/// 复杂块生成器配置 / Complex block generator configuration
///
/// 控制沿各轴合并块候选的策略。
/// Controls the strategy for merging block candidates along axes.
#[derive(Debug, Clone)]
pub struct ComplexBlockGeneratorConfig {
    /// 是否沿 X 轴合并 / Whether to merge along the X axis
    pub with_x: bool,
    /// 是否沿 Y 轴合并 / Whether to merge along the Y axis
    pub with_y: bool,
    /// 是否沿 Z 轴合并 / Whether to merge along the Z axis
    pub with_z: bool,
    /// 最大生成数量 / Maximum generated count
    pub max_candidates: usize,
    /// 最大复合深度 / Maximum composite depth
    pub max_depth: usize,
}

impl Default for ComplexBlockGeneratorConfig {
    fn default() -> Self {
        Self {
            with_x: true,
            with_y: true,
            with_z: false,
            max_candidates: 256,
            max_depth: 3,
        }
    }
}

/// 复杂块生成器 / Complex block generator
///
/// 合并兼容块，形成多轮复合候选。
/// Merges compatible blocks into multi-round composite candidates.
#[derive(Debug, Clone)]
pub struct ComplexBlockGenerator {
    /// 配置 / Configuration
    pub config: ComplexBlockGeneratorConfig,
}

impl ComplexBlockGenerator {
    /// 创建复杂块生成器 / Create a complex block generator
    pub fn new(config: ComplexBlockGeneratorConfig) -> Self {
        Self { config }
    }

    /// 创建默认生成器 / Create default generator
    pub fn default_generator() -> Self {
        Self::new(ComplexBlockGeneratorConfig::default())
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec![format!(
            "complex block generation uses multi-round axis merge: with_x={}, with_y={}, with_z={}, max_candidates={}, max_depth={}",
            self.config.with_x,
            self.config.with_y,
            self.config.with_z,
            self.config.max_candidates,
            self.config.max_depth,
        )]
    }

    /// 生成复杂块 / Generate complex blocks
    pub fn generate<V, U>(
        &self,
        amounts: &[u64],
        container_size: &MetricSize3<V, U>,
        simple_blocks: &[Block<V, U>],
        rest_weight: Option<V>,
    ) -> Vec<Block<V, U>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let base_candidates = simple_blocks
            .iter()
            .filter(|block| {
                Self::fits_container(block, container_size)
                    && Self::enough_amounts(block, amounts)
                    && Self::within_weight(block, rest_weight.clone())
            })
            .cloned()
            .collect::<Vec<_>>();
        if base_candidates.is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        let mut frontier = base_candidates.clone();
        let mut seen = std::collections::HashSet::new();
        let depth_limit = self.config.max_depth.max(1);

        for _ in 0..depth_limit {
            let mut next_frontier = Vec::new();
            for lhs in &frontier {
                for rhs in &base_candidates {
                    if results.len() >= self.config.max_candidates {
                        return results;
                    }
                for axis in [Axis3::X, Axis3::Y, Axis3::Z] {
                    if !self.axis_enabled(axis) || !Self::merge_compatible(axis, lhs, rhs) {
                        continue;
                    }
                    let Some(complex) = Self::merge(axis, lhs, rhs) else {
                        continue;
                    };
                    let block = Block::Complex(complex);
                    if !Self::fits_container(&block, container_size)
                        || !Self::enough_amounts(&block, amounts)
                        || !Self::within_weight(&block, rest_weight.clone())
                    {
                        continue;
                    }
                    let signature = Self::block_signature(&block);
                    if !seen.insert(signature) {
                        continue;
                    }
                    next_frontier.push(block.clone());
                    results.push(block);
                    if results.len() >= self.config.max_candidates {
                        return results;
                    }
                }
            }
        }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }
        results
    }

    fn axis_enabled(&self, axis: Axis3) -> bool {
        match axis {
            Axis3::X => self.config.with_x,
            Axis3::Y => self.config.with_y,
            Axis3::Z => self.config.with_z,
        }
    }

    fn merge_compatible<V, U>(
        axis: Axis3,
        lhs: &Block<V, U>,
        rhs: &Block<V, U>,
    ) -> bool
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
        U: CTUnit + Default + Clone,
    {
        match axis {
            Axis3::X => lhs.height().value == rhs.height().value && lhs.depth().value == rhs.depth().value,
            Axis3::Y => lhs.width().value == rhs.width().value && lhs.depth().value == rhs.depth().value,
            Axis3::Z => lhs.width().value == rhs.width().value && lhs.height().value == rhs.height().value,
        }
    }

    fn merge<V, U>(
        axis: Axis3,
        lhs: &Block<V, U>,
        rhs: &Block<V, U>,
    ) -> Option<ComplexBlock<V, U>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
        U: CTUnit + Default + Clone,
    {
        let zero = Quantity::new_ct(V::zero());
        let rhs_position = match axis {
            Axis3::X => MetricPoint3 {
                x: lhs.width().clone(),
                y: zero.clone(),
                z: zero.clone(),
            },
            Axis3::Y => MetricPoint3 {
                x: zero.clone(),
                y: lhs.height().clone(),
                z: zero.clone(),
            },
            Axis3::Z => MetricPoint3 {
                x: zero.clone(),
                y: zero.clone(),
                z: lhs.depth().clone(),
            },
        };
        let width = match axis {
            Axis3::X => Quantity::new_ct(lhs.width().value + rhs.width().value),
            Axis3::Y | Axis3::Z => lhs.width().clone(),
        };
        let height = match axis {
            Axis3::Y => Quantity::new_ct(lhs.height().value + rhs.height().value),
            Axis3::X | Axis3::Z => lhs.height().clone(),
        };
        let depth = match axis {
            Axis3::Z => Quantity::new_ct(lhs.depth().value + rhs.depth().value),
            Axis3::X | Axis3::Y => lhs.depth().clone(),
        };
        let weight = Quantity::new_ct(lhs.weight().value + rhs.weight().value);
        let mut sub_blocks = Vec::new();
        let mut blocks = Vec::new();
        for (position, simple) in Self::flatten_block(lhs, &MetricPoint3 {
            x: zero.clone(),
            y: zero.clone(),
            z: zero.clone(),
        }) {
            let block_index = sub_blocks.len();
            sub_blocks.push(simple);
            blocks.push(BlockPlacement {
                position,
                block_index,
            });
        }
        for (position, simple) in Self::flatten_block(rhs, &rhs_position) {
            let block_index = sub_blocks.len();
            sub_blocks.push(simple);
            blocks.push(BlockPlacement {
                position,
                block_index,
            });
        }
        Some(ComplexBlock {
            blocks,
            sub_blocks,
            merge_axis: axis,
            width,
            height,
            depth,
            weight,
        })
    }

    fn flatten_block<V, U>(
        block: &Block<V, U>,
        offset: &MetricPoint3<V, U>,
    ) -> Vec<(MetricPoint3<V, U>, SimpleBlock<V, U>)>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
        U: CTUnit + Default + Clone,
    {
        match block {
            Block::Simple(simple) => vec![(
                offset.clone(),
                simple.clone(),
            )],
            Block::Complex(complex) => complex
                .blocks
                .iter()
                .filter_map(|placement| {
                    let simple = complex.sub_blocks.get(placement.block_index)?.clone();
                    Some((
                        MetricPoint3 {
                            x: Quantity::new_ct(offset.x.value + placement.position.x.value),
                            y: Quantity::new_ct(offset.y.value + placement.position.y.value),
                            z: Quantity::new_ct(offset.z.value + placement.position.z.value),
                        },
                        simple,
                    ))
                })
                .collect(),
        }
    }

    fn fits_container<V, U>(block: &Block<V, U>, container_size: &MetricSize3<V, U>) -> bool
    where
        V: PartialOrd,
        U: CTUnit + Default + Clone,
    {
        block.width().value <= container_size.width.value
            && block.height().value <= container_size.height.value
            && block.depth().value <= container_size.depth.value
    }

    fn within_weight<V, U>(block: &Block<V, U>, rest_weight: Option<V>) -> bool
    where
        V: PartialOrd,
        U: CTUnit + Default + Clone,
    {
        rest_weight
            .map(|weight| block.weight().value <= weight)
            .unwrap_or(true)
    }

    fn enough_amounts<V, U>(block: &Block<V, U>, amounts: &[u64]) -> bool
    where
        U: CTUnit + Default + Clone,
    {
        let mut used = vec![0u64; amounts.len()];
        for (item_index, amount) in Self::block_amounts(block) {
            let Some(limit) = amounts.get(item_index) else {
                return false;
            };
            used[item_index] += amount;
            if used[item_index] > *limit {
                return false;
            }
        }
        true
    }

    fn block_amounts<V, U>(block: &Block<V, U>) -> Vec<(usize, u64)>
    where
        U: CTUnit + Default + Clone,
    {
        match block {
            Block::Simple(simple) => vec![(simple.item_view.item_index, simple.item_count)],
            Block::Complex(complex) => complex
                .sub_blocks
                .iter()
                .map(|simple| (simple.item_view.item_index, simple.item_count))
                .collect(),
        }
    }

    fn block_signature<V, U>(block: &Block<V, U>) -> String
    where
        V: Debug,
        U: CTUnit + Default + Clone,
    {
        let mut amounts = Self::block_amounts(block);
        amounts.sort_by_key(|(index, _)| *index);
        format!(
            "w={:?};h={:?};d={:?};amounts={:?}",
            block.width().value,
            block.height().value,
            block.depth().value,
            amounts,
        )
    }
}

impl Default for ComplexBlockGenerator {
    fn default() -> Self {
        Self::default_generator()
    }
}

