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
    /// 按 `request.bins` 逐箱型展开调用生成器链，再按列身份去重并截取到 max_candidates。
    /// 未指定任何箱型时保持单次直通，使不依赖箱型的生成器（如历史层生成器）仍可工作。
    /// Expands the generator chain per bin type from `request.bins`, then deduplicates by column identity and
    /// truncates to max_candidates. When no bin type is specified the request is passed through once, so
    /// generators that do not depend on a bin type (such as the historical generator) keep working.
    pub fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        let candidate_bins = request.candidate_bins();
        let bin_requests: Vec<LayerGenerationRequest<V, U>> = if candidate_bins.is_empty() {
            vec![request.clone()]
        } else {
            candidate_bins
                .into_iter()
                .map(|bin| LayerGenerationRequest {
                    bin: Some(bin),
                    ..request.clone()
                })
                .collect()
        };

        let mut all_results: Vec<LayerGenerationResult<V, U>> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        for bin_request in bin_requests {
            if deduped.len() >= request.max_candidates {
                break;
            }
            for generator in &self.generators {
                let results = generator.generate(&bin_request);
                all_results.extend(results);
                if deduped.len() >= request.max_candidates {
                    break;
                }
            }
        }

        for result in all_results {
            // 列身份必须包含箱型：同一排布放在不同箱型上是不同的列。
            // Column identity must include the bin type: the same layout on different bin types is a
            // different column.
            if seen.insert(result.layer.identity_key()) {
                deduped.push(result);
            }
            if deduped.len() >= request.max_candidates {
                break;
            }
        }
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

