/// 多层启发式搜索配置 / Multi-layer heuristic search configuration
///
/// 控制层数、序列深度和候选数量。
/// Controls layer count, sequence depth, and candidate count.
#[derive(Debug, Clone)]
pub struct MultiLayerHeuristicSearchConfig {
    /// 最大层数量 / Maximum layer count
    pub max_layers: usize,
    /// 初始块序列深度 / Initial block sequence depth
    pub depth: usize,
    /// 分支数量 / Branch count
    pub branch: usize,
}

impl Default for MultiLayerHeuristicSearchConfig {
    fn default() -> Self {
        Self {
            max_layers: 8,
            depth: 2,
            branch: 128,
        }
    }
}

/// 多层启发式搜索算法 / Multi-layer heuristic search algorithm
///
/// 通过短序列枚举、DFS 放置和体积评分生成多层候选。
/// Generates multi-layer candidates through short sequence enumeration,
/// DFS placement, and volume scoring.
#[derive(Debug, Clone)]
pub struct MultiLayerHeuristicSearchAlgorithm {
    /// 配置 / Configuration
    pub config: MultiLayerHeuristicSearchConfig,
    /// DFS 子策略 / DFS sub-strategy
    pub dfs: DepthFirstSearchAlgorithm,
}

impl MultiLayerHeuristicSearchAlgorithm {
    /// 创建 MLHS 算法 / Create MLHS algorithm
    pub fn new(
        config: MultiLayerHeuristicSearchConfig,
        dfs: DepthFirstSearchAlgorithm,
    ) -> Self {
        Self { config, dfs }
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec![format!(
            "multi-layer heuristic search enumerates bounded DFS candidates: max_layers={}, depth={}, branch={}",
            self.config.max_layers,
            self.config.depth,
            self.config.branch,
        )]
    }

    /// 生成多层放置 / Generate multi-layer placements
    pub fn pack_layers<V, U>(
        &self,
        blocks: &[Block<V, U>],
        container_size: &MetricSize3<V, U>,
    ) -> Vec<Vec<BlockPlacement<V, U>>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        self.pack_layer_candidates(blocks, container_size)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// 生成多层候选 / Generate multi-layer candidates
    pub fn pack_layer_candidates<V, U>(
        &self,
        blocks: &[Block<V, U>],
        container_size: &MetricSize3<V, U>,
    ) -> Vec<Vec<Vec<BlockPlacement<V, U>>>>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        if blocks.is_empty() {
            return Vec::new();
        }
        let mut ordered_blocks = (0..blocks.len()).collect::<Vec<_>>();
        ordered_blocks.sort_by(|lhs, rhs| {
            block_volume(&blocks[*rhs])
                .partial_cmp(&block_volume(&blocks[*lhs]))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    blocks[*rhs]
                        .weight()
                        .value
                        .partial_cmp(&blocks[*lhs].weight().value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        let mut candidate_sequences = Vec::new();
        enumerate_block_sequences(
            &ordered_blocks,
            self.config.depth.max(1),
            self.config.branch.max(1),
            &mut Vec::new(),
            &mut candidate_sequences,
        );
        if candidate_sequences.is_empty() {
            candidate_sequences.push(ordered_blocks);
        }

        let mut candidates = candidate_sequences
            .into_iter()
            .take(self.config.branch.max(1))
            .flat_map(|sequence| {
                let sequenced_blocks = sequence
                    .iter()
                    .filter_map(|index| blocks.get(*index).cloned())
                    .collect::<Vec<_>>();
                self.dfs
                    .pack_candidates(&sequenced_blocks, container_size)
                    .into_iter()
                    .filter_map(move |placements| {
                        let mapped = placements
                            .into_iter()
                            .filter_map(|placement| {
                                sequence.get(placement.block_index).map(|original| BlockPlacement {
                                    position: placement.position,
                                    block_index: *original,
                                })
                            })
                            .collect::<Vec<_>>();
                        (!mapped.is_empty()).then_some(group_placements_by_depth(mapped, self.config.max_layers))
                    })
            })
            .collect::<Vec<_>>();

        let full_dfs_candidates = self
            .dfs
            .pack_candidates(blocks, container_size)
            .into_iter()
            .map(|placements| group_placements_by_depth(placements, self.config.max_layers));
        candidates.extend(full_dfs_candidates);
        candidates.sort_by(|lhs, rhs| {
            layer_candidate_volume(blocks, rhs)
                .partial_cmp(&layer_candidate_volume(blocks, lhs))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| layer_candidate_count(rhs).cmp(&layer_candidate_count(lhs)))
        });
        candidates.dedup_by(|lhs, rhs| layer_candidate_signature(lhs) == layer_candidate_signature(rhs));
        candidates.truncate(self.config.max_layers.max(1));
        candidates
    }
}

impl Default for MultiLayerHeuristicSearchAlgorithm {
    fn default() -> Self {
        Self::new(
            MultiLayerHeuristicSearchConfig::default(),
            DepthFirstSearchAlgorithm::default(),
        )
    }
}

fn block_volume<V, U>(block: &Block<V, U>) -> V
where
    V: Field + num_traits::Float + Clone,
    U: CTUnit + Default + Clone,
{
    block.width().value * block.height().value * block.depth().value
}

fn enumerate_block_sequences(
    indices: &[usize],
    depth: usize,
    limit: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    if output.len() >= limit {
        return;
    }
    if current.len() == depth || current.len() == indices.len() {
        output.push(current.clone());
        return;
    }
    for index in indices {
        if current.contains(index) {
            continue;
        }
        current.push(*index);
        enumerate_block_sequences(indices, depth, limit, current, output);
        current.pop();
        if output.len() >= limit {
            break;
        }
    }
}

fn group_placements_by_depth<V, U>(
    mut placements: Vec<BlockPlacement<V, U>>,
    max_layers: usize,
) -> Vec<Vec<BlockPlacement<V, U>>>
where
    V: PartialOrd + Clone,
    U: CTUnit + Default + Clone,
{
    placements.sort_by(|lhs, rhs| {
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
    });
    let mut layers = Vec::<Vec<BlockPlacement<V, U>>>::new();
    for placement in placements {
        let layer_key = placement.position.z.value.clone();
        if let Some(layer) = layers.iter_mut().find(|layer| {
            layer
                .first()
                .is_some_and(|first| first.position.z.value == layer_key)
        }) {
            layer.push(placement);
        } else if layers.len() < max_layers.max(1) {
            layers.push(vec![placement]);
        }
    }
    layers
}

fn layer_candidate_volume<V, U>(blocks: &[Block<V, U>], layers: &[Vec<BlockPlacement<V, U>>]) -> V
where
    V: Field + num_traits::Float + Clone,
    U: CTUnit + Default + Clone,
{
    layers.iter().fold(V::zero(), |acc, layer| {
        acc + placements_volume(blocks, layer)
    })
}

fn layer_candidate_count<V, U>(layers: &[Vec<BlockPlacement<V, U>>]) -> usize
where
    U: ospf_rust_quantities::unit::concept::UnitTrait,
{
    layers.iter().map(Vec::len).sum()
}

fn layer_candidate_signature<V, U>(layers: &[Vec<BlockPlacement<V, U>>]) -> String
where
    V: Debug,
    U: ospf_rust_quantities::unit::concept::UnitTrait,
{
    layers
        .iter()
        .map(|layer| placement_signature(layer))
        .collect::<Vec<_>>()
        .join("||")
}

fn space_fitness<V, U>(space: &Space<V, U>, block: &Block<V, U>) -> V
where
    V: Field + num_traits::Float + Clone,
    U: CTUnit + Default + Clone,
{
    (space.size.width.value - block.width().value)
        + (space.size.height.value - block.height().value)
        + (space.size.depth.value - block.depth().value)
}

