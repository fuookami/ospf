// ============================================================================
// LayerGenerator - 层生成器 trait / Layer generator trait
// ============================================================================

/// 层生成器 / Layer generator
///
/// 定义层生成的核心接口，所有生成器必须实现此 trait。
/// Defines the core interface for layer generation; all generators must implement this trait.
pub trait LayerGenerator<V, U>: Debug + Send + Sync
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 生成器名称 / Generator name
    fn name(&self) -> &str;

    /// 生成层候选 / Generate layer candidates
    fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>>;
}

// ============================================================================
// LayerGenerationContext - 层生成上下文 / Layer generation context
// ============================================================================

/// 层生成上下文 / Layer generation context
///
/// 组合多个层生成器，按顺序调用并去重。
/// Composes multiple layer generators, invoking them in sequence and deduplicating.
#[derive(Debug)]
pub struct LayerGenerationContext<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 生成器列表 / Generator list
    pub generators: Vec<Box<dyn LayerGenerator<V, U>>>,
}

impl<V, U> LayerGenerationContext<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建空的层生成上下文 / Create empty layer generation context
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
        }
    }

    /// 添加生成器 / Add a generator
    pub fn add_generator(&mut self, generator: Box<dyn LayerGenerator<V, U>>) {
        self.generators.push(generator);
    }

    /// 生成层候选 / Generate layer candidates
    ///
    /// 依次调用所有生成器，收集结果并去重，截取到 max_candidates。
    /// Invokes all generators in sequence, collects results, deduplicates,
    /// and truncates to max_candidates.
    pub fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        let mut all_results: Vec<LayerGenerationResult<V, U>> = Vec::new();

        for generator in &self.generators {
            let results = generator.generate(request);
            all_results.extend(results);
        }

        // 简化去重：基于层深度和来源
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        for result in all_results {
            let key = format!("{:?}_{}", result.layer.depth.value, result.layer.from);
            if seen.insert(key) {
                deduped.push(result);
            }
        }

        // 截取到最大候选数
        deduped.truncate(request.max_candidates);
        deduped
    }
}

impl<V, U> Default for LayerGenerationContext<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

fn unsupported_layer_generation_result<V, U>(
    request: &LayerGenerationRequest<V, U>,
    source: &str,
    message: String,
) -> LayerGenerationResult<V, U>
where
    V: Field + Clone + Debug + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let depth = request
        .bin
        .as_ref()
        .map(|bin| bin.depth.clone())
        .or_else(|| request.items.first().map(|item| item.depth.clone()))
        .unwrap_or_else(|| Quantity::new_ct(V::zero()));
    LayerGenerationResult {
        layer: BinLayer {
            iteration: request.iteration,
            from: source.to_string(),
            bin: request.bin.clone(),
            depth,
            demand_coverage: Vec::new(),
        },
        reduced_cost: None,
        score: None,
        numeric_score: None,
        block_traces: Vec::new(),
        placement_traces: Vec::new(),
        diagnostics: vec![format!(
            "{}: iteration={}, item_count={}, existing_layer_count={}, demand_count={}, has_bin={}",
            message,
            request.iteration,
            request.items.len(),
            request.existing_layers.len(),
            request.demand_entries.len(),
            request.bin.is_some(),
        )],
        source: source.to_string(),
    }
}

