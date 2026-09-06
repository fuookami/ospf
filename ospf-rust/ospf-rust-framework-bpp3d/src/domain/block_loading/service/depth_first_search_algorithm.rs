/// 深度优先搜索配置 / Depth-first search configuration
///
/// 控制装载搜索的分支和状态数量。
/// Controls branching and state count for loading search.
#[derive(Debug, Clone)]
pub struct DepthFirstSearchConfig {
    /// 分支数量 / Branch count
    pub branch: usize,
    /// 最大放置数量 / Maximum placement count
    pub max_placements: usize,
    /// 最大搜索状态数量 / Maximum search state count
    pub max_states: usize,
    /// 是否合并相邻空间 / Whether adjacent spaces are merged
    pub merge_spaces: bool,
}

impl Default for DepthFirstSearchConfig {
    fn default() -> Self {
        Self {
            branch: 8,
            max_placements: 256,
            max_states: 4096,
            merge_spaces: true,
        }
    }
}

/// 深度优先搜索算法 / Depth-first search algorithm
///
/// 使用空间分裂、分支剪枝和体积评分生成可行块放置。
/// Uses space splitting, branch pruning, and volume scoring to generate feasible block placements.
#[derive(Debug, Clone)]
pub struct DepthFirstSearchAlgorithm {
    /// 配置 / Configuration
    pub config: DepthFirstSearchConfig,
}

impl DepthFirstSearchAlgorithm {
    /// 创建 DFS 算法 / Create DFS algorithm
    pub fn new(config: DepthFirstSearchConfig) -> Self {
        Self { config }
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec![format!(
            "depth-first search layer loading uses bounded branch-and-bound space splitting: branch={}, max_placements={}, max_states={}, merge_spaces={}",
            self.config.branch,
            self.config.max_placements,
            self.config.max_states,
            self.config.merge_spaces,
        )]
    }

    /// 装载块 / Pack blocks
    pub fn pack<V, U>(
        &self,
        blocks: &[Block<V, U>],
        container_size: &MetricSize3<V, U>,
    ) -> Vec<BlockPlacement<V, U>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        self.pack_candidates(blocks, container_size)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// 生成候选装载方案 / Generate packing candidates
    pub fn pack_candidates<V, U>(
        &self,
        blocks: &[Block<V, U>],
        container_size: &MetricSize3<V, U>,
    ) -> Vec<Vec<BlockPlacement<V, U>>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        if blocks.is_empty() {
            return Vec::new();
        }
        #[derive(Clone)]
        struct SearchState<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
            spaces: Vec<Space<V, U>>,
            placements: Vec<BlockPlacement<V, U>>,
        }

        let mut spaces = vec![Space::new(
            MetricPoint3 {
                x: Quantity::new_ct(V::zero()),
                y: Quantity::new_ct(V::zero()),
                z: Quantity::new_ct(V::zero()),
            },
            container_size.clone(),
        )];
        spaces = normalize_spaces(spaces, self.config.merge_spaces);
        let mut stack = vec![SearchState {
            spaces,
            placements: Vec::new(),
        }];
        let mut candidates = Vec::<Vec<BlockPlacement<V, U>>>::new();
        let mut seen = std::collections::HashSet::new();
        let mut explored = 0usize;
        let branch = self.config.branch.max(1);

        while let Some(state) = stack.pop() {
            explored += 1;
            if !state.placements.is_empty() {
                let signature = placement_signature(&state.placements);
                if seen.insert(signature) {
                    candidates.push(state.placements.clone());
                    candidates.sort_by(|lhs, rhs| {
                        placements_volume(blocks, rhs)
                            .partial_cmp(&placements_volume(blocks, lhs))
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| rhs.len().cmp(&lhs.len()))
                    });
                    candidates.truncate(branch);
                }
            }
            if explored >= self.config.max_states
                || state.placements.len() >= self.config.max_placements
                || state.spaces.is_empty()
            {
                continue;
            }

            let mut space_indices = (0..state.spaces.len()).collect::<Vec<_>>();
            space_indices.sort_by(|lhs, rhs| compare_spaces(&state.spaces[*lhs], &state.spaces[*rhs]));
            let mut next_states = Vec::new();
            for space_index in space_indices.into_iter().take(branch) {
                let space = &state.spaces[space_index];
                let mut block_indices = (0..blocks.len())
                    .filter(|block_index| {
                        !state
                            .placements
                            .iter()
                            .any(|placement| placement.block_index == *block_index)
                            && space.fits(&blocks[*block_index])
                    })
                    .collect::<Vec<_>>();
                block_indices.sort_by(|lhs, rhs| {
                    space_fitness(space, &blocks[*lhs])
                        .partial_cmp(&space_fitness(space, &blocks[*rhs]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| {
                            block_volume(&blocks[*rhs])
                                .partial_cmp(&block_volume(&blocks[*lhs]))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                });
                for block_index in block_indices.into_iter().take(branch) {
                    let mut next_spaces = state.spaces.clone();
                    let used_space = next_spaces.remove(space_index);
                    next_spaces.extend(used_space.place_block(&blocks[block_index]));
                    next_spaces = normalize_spaces(next_spaces, self.config.merge_spaces);
                    let mut next_placements = state.placements.clone();
                    next_placements.push(BlockPlacement {
                        position: used_space.position,
                        block_index,
                    });
                    next_states.push(SearchState {
                        spaces: next_spaces,
                        placements: next_placements,
                    });
                }
            }
            next_states.sort_by(|lhs, rhs| {
                placements_volume(blocks, &rhs.placements)
                    .partial_cmp(&placements_volume(blocks, &lhs.placements))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| rhs.placements.len().cmp(&lhs.placements.len()))
            });
            stack.extend(next_states.into_iter().take(branch * branch).rev());
        }

        candidates.sort_by(|lhs, rhs| {
            placements_volume(blocks, rhs)
                .partial_cmp(&placements_volume(blocks, lhs))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| rhs.len().cmp(&lhs.len()))
        });
        candidates
    }
}

impl Default for DepthFirstSearchAlgorithm {
    fn default() -> Self {
        Self::new(DepthFirstSearchConfig::default())
    }
}

fn normalize_spaces<V, U>(spaces: Vec<Space<V, U>>, merge_spaces: bool) -> Vec<Space<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    let mut normalized = spaces
        .into_iter()
        .filter(|space| {
            space.size.width.value > V::zero()
                && space.size.height.value > V::zero()
                && space.size.depth.value > V::zero()
        })
        .collect::<Vec<_>>();
    normalized.sort_by(compare_spaces);
    normalized.dedup_by(|lhs, rhs| {
        lhs.position.x.value == rhs.position.x.value
            && lhs.position.y.value == rhs.position.y.value
            && lhs.position.z.value == rhs.position.z.value
            && lhs.size.width.value == rhs.size.width.value
            && lhs.size.height.value == rhs.size.height.value
            && lhs.size.depth.value == rhs.size.depth.value
    });
    if merge_spaces {
        merge_adjacent_spaces(normalized)
    } else {
        normalized
    }
}

fn merge_adjacent_spaces<V, U>(spaces: Vec<Space<V, U>>) -> Vec<Space<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
    U: CTUnit + Default + Clone,
{
    let mut merged = Vec::<Space<V, U>>::new();
    'outer: for space in spaces {
        for existing in &mut merged {
            let same_y = existing.position.y.value == space.position.y.value
                && existing.size.height.value == space.size.height.value;
            let same_z = existing.position.z.value == space.position.z.value
                && existing.size.depth.value == space.size.depth.value;
            let touches_x = existing.position.x.value + existing.size.width.value == space.position.x.value;
            if same_y && same_z && touches_x {
                existing.size.width = Quantity::new_ct(existing.size.width.value + space.size.width.value);
                continue 'outer;
            }
        }
        merged.push(space);
    }
    merged.sort_by(compare_spaces);
    merged
}

fn compare_spaces<V, U>(lhs: &Space<V, U>, rhs: &Space<V, U>) -> std::cmp::Ordering
where
    V: PartialOrd,
    U: CTUnit + Default + Clone,
{
    lhs.position.z.value
        .partial_cmp(&rhs.position.z.value)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            lhs.position.x.value
                .partial_cmp(&rhs.position.x.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| {
            lhs.position.y.value
                .partial_cmp(&rhs.position.y.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn placement_signature<V, U>(placements: &[BlockPlacement<V, U>]) -> String
where
    V: Debug,
    U: ospf_rust_quantities::unit::concept::UnitTrait,
{
    placements
        .iter()
        .map(|placement| {
            format!(
                "{}@{:?},{:?},{:?}",
                placement.block_index,
                placement.position.x.value,
                placement.position.y.value,
                placement.position.z.value,
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn placements_volume<V, U>(blocks: &[Block<V, U>], placements: &[BlockPlacement<V, U>]) -> V
where
    V: Field + num_traits::Float + Clone,
    U: CTUnit + Default + Clone,
{
    placements.iter().fold(V::zero(), |acc, placement| {
        acc + blocks
            .get(placement.block_index)
            .map(block_volume)
            .unwrap_or_else(V::zero)
    })
}

