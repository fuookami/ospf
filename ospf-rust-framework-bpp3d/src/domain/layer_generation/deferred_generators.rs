// ============================================================================
// Pattern/Pile/Historical generators - 延后生成器 / Deferred generators
// ============================================================================

/// 延后层生成器配置 / Deferred layer generator config
#[derive(Debug, Clone)]
pub struct DeferredLayerGeneratorConfig {
    /// 覆盖系数 / Coverage coefficient
    pub coverage_coefficient: f64,
    /// 最大候选数 / Maximum candidate count
    pub max_candidates: usize,
    /// 是否使用现有层提示 / Whether existing layer hints are used
    pub use_existing_layer_hint: bool,
    /// 模式配置 / Pattern config
    pub pattern: PatternConfig,
}

impl Default for DeferredLayerGeneratorConfig {
    fn default() -> Self {
        Self {
            coverage_coefficient: 1.0,
            max_candidates: 1,
            use_existing_layer_hint: true,
            pattern: PatternConfig::default(),
        }
    }
}

/// 模式层生成器 / Pattern layer generator
#[derive(Debug)]
pub struct PatternLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
    /// 生成器配置 / Generator config
    config: DeferredLayerGeneratorConfig,
}

impl PatternLayerGenerator {
    /// 创建生成器 / Create generator
    pub fn new() -> Self {
        Self::with_config(DeferredLayerGeneratorConfig::default())
    }

    /// 使用配置创建生成器 / Create generator with config
    pub fn with_config(config: DeferredLayerGeneratorConfig) -> Self {
        Self {
            name: "pattern_layer_generator".to_string(),
            config,
        }
    }
}

impl Default for PatternLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 堆叠层生成器 / Pile layer generator
#[derive(Debug)]
pub struct PileLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
    /// 生成器配置 / Generator config
    config: DeferredLayerGeneratorConfig,
}

impl PileLayerGenerator {
    /// 创建生成器 / Create generator
    pub fn new() -> Self {
        Self::with_config(DeferredLayerGeneratorConfig::default())
    }

    /// 使用配置创建生成器 / Create generator with config
    pub fn with_config(config: DeferredLayerGeneratorConfig) -> Self {
        Self {
            name: "pile_layer_generator".to_string(),
            config,
        }
    }
}

impl Default for PileLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 历史层生成器 / Historical layer generator
#[derive(Debug)]
pub struct HistoricalLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
    /// 生成器配置 / Generator config
    config: DeferredLayerGeneratorConfig,
}

impl HistoricalLayerGenerator {
    /// 创建生成器 / Create generator
    pub fn new() -> Self {
        Self::with_config(DeferredLayerGeneratorConfig::default())
    }

    /// 使用配置创建生成器 / Create generator with config
    pub fn with_config(config: DeferredLayerGeneratorConfig) -> Self {
        Self {
            name: "historical_layer_generator".to_string(),
            config,
        }
    }
}

impl Default for HistoricalLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn block_loading_layer_candidates<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    strategy: &str,
    indexed_items: &[(usize, ActualItem<V, U>)],
    use_global_search: bool,
    config: &DeferredLayerGeneratorConfig,
) -> Vec<LayerGenerationResult<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    if indexed_items.is_empty() {
        return vec![unsupported_layer_generation_result(
            request,
            generator_name,
            format!("{} layer generation requires at least one item", strategy),
        )];
    }
    let Some(bin) = request.bin.clone() else {
        return vec![unsupported_layer_generation_result(
            request,
            generator_name,
            format!("{} layer generation requires a target bin", strategy),
        )];
    };
    let container_size = MetricSize3 {
        width: bin.width.clone(),
        height: bin.height.clone(),
        depth: bin.depth.clone(),
    };
    let local_items = indexed_items
        .iter()
        .map(|(_, item)| item.clone())
        .collect::<Vec<_>>();
    let amounts = indexed_items
        .iter()
        .map(|(index, item)| item_remaining_amount(request, *index, &item.id))
        .collect::<Vec<_>>();
    let package_attributes = indexed_items
        .iter()
        .map(|(_, item)| request.package_attributes.get(&item.id))
        .collect::<Vec<_>>();
    let simple_blocks = SimpleBlockGenerator::new(
        SimpleBlockGeneratorConfig {
            max_stack_layers: config.pattern.single_pile_limit(),
            ..Default::default()
        },
    )
    .generate_with_package_attributes(
        &local_items,
        &amounts,
        &container_size,
        &package_attributes,
    )
    .into_iter()
    .filter(|block| block_allows_package_rule_policy(
        request,
        indexed_items,
        block,
        &bin,
    ))
    .collect::<Vec<_>>();
    if simple_blocks.is_empty() {
        return Vec::new();
    }
    let complex_blocks = ComplexBlockGenerator::default()
        .generate(&amounts, &container_size, &simple_blocks, Some(bin.capacity.value));
    let mut block_table = simple_blocks
        .into_iter()
        .chain(complex_blocks)
        .collect::<Vec<_>>();
    if use_global_search {
        block_table.sort_by(|lhs, rhs| {
            block_volume_score(rhs)
                .partial_cmp(&block_volume_score(lhs))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    rhs.weight()
                        .value
                        .partial_cmp(&lhs.weight().value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
    } else {
        block_table.sort_by(|lhs, rhs| {
            block_unit_count(rhs)
                .cmp(&block_unit_count(lhs))
                .then_with(|| {
                    block_volume_score(rhs)
                        .partial_cmp(&block_volume_score(lhs))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
    }
    let max_candidates = request.max_candidates.min(config.max_candidates.max(1));
    let placement_sets = if use_global_search {
        MultiLayerHeuristicSearchAlgorithm::default()
            .pack_layer_candidates(&block_table, &container_size)
            .into_iter()
            .flat_map(|candidate| candidate.into_iter())
            .collect::<Vec<_>>()
    } else {
        DepthFirstSearchAlgorithm::default()
            .pack_candidates(&block_table, &container_size)
    };
    let mut results = placement_sets
        .into_iter()
        .filter_map(|placements| {
            placement_set_to_layer_result(
                request,
                generator_name,
                strategy,
                &bin,
                &block_table,
                indexed_items,
                placements,
                config.coverage_coefficient,
            )
        })
        .collect::<Vec<_>>();
    rank_layer_results(&mut results);
    results.truncate(max_candidates);
    results
}

fn package_rule_policy_allows_orientation<V, U>(
    request: &LayerGenerationRequest<V, U>,
    item: &ActualItem<V, U>,
    attribute: Option<&PackageAttribute>,
    orientation: Orientation,
    bin: &BinType<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let input = PackageOrientationRuleInput {
        orientation,
        space_width: bin.width.value.to_f64().unwrap_or(f64::INFINITY),
        space_height: bin.height.value.to_f64().unwrap_or(f64::INFINITY),
        space_depth: bin.depth.value.to_f64().unwrap_or(f64::INFINITY),
    };
    request.package_rule_policy.as_ref().allows_orientation(
        item,
        attribute,
        orientation,
        &input,
    )
}

fn block_allows_package_rule_policy<V, U>(
    request: &LayerGenerationRequest<V, U>,
    indexed_items: &[(usize, ActualItem<V, U>)],
    block: &Block<V, U>,
    bin: &BinType<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    match block {
        Block::Simple(simple) => {
            simple_block_allows_package_rule_policy(request, indexed_items, simple, bin)
        }
        Block::Complex(complex) => complex.sub_blocks.iter().all(|simple| {
            simple_block_allows_package_rule_policy(request, indexed_items, simple, bin)
        }),
    }
}

fn simple_block_allows_package_rule_policy<V, U>(
    request: &LayerGenerationRequest<V, U>,
    indexed_items: &[(usize, ActualItem<V, U>)],
    simple: &crate::domain::block_loading::SimpleBlock<V, U>,
    bin: &BinType<V, U>,
) -> bool
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst + ToPrimitive,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let Some((_, item)) = indexed_items.get(simple.item_view.item_index) else {
        return false;
    };
    let attribute = request.package_attributes.get(&item.id);
    package_rule_policy_allows_orientation(
        request,
        item,
        attribute,
        simple.item_view.orientation,
        bin,
    )
}

