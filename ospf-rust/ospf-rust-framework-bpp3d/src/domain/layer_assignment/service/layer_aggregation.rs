// ============================================================================
// LayerAggregation - 层聚合 / Layer aggregation
// ============================================================================

/// 层聚合 / Layer aggregation
///
/// 管理列生成过程中的层集合。
/// Manages the layer set during column generation.
#[derive(Debug, Clone)]
pub struct LayerAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 所有层 / All layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 每次迭代新增的层 / Layers added per iteration
    pub iterations: Vec<Vec<BinLayer<V, U>>>,
}

impl<V, U> LayerAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建层聚合 / Create a layer aggregation
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            iterations: Vec::new(),
        }
    }

    /// 添加新层（带去重）/ Add new layers (with deduplication)
    ///
    /// 去重按列身份键进行：箱型、来源、深度与需求覆盖共同参与，与层生成上下文的去重口径一致。
    /// Deduplication uses the column identity key: bin type, source, depth and demand coverage all participate,
    /// matching the deduplication口径 of the layer generation context.
    pub fn add_columns(&mut self, new_layers: Vec<BinLayer<V, U>>) -> Vec<BinLayer<V, U>>
    where
        V: Clone + PartialEq,
    {
        let mut added = Vec::new();
        for layer in new_layers {
            let identity = layer.identity_key();
            let is_dup = self
                .layers
                .iter()
                .any(|existing| existing.identity_key() == identity);
            if !is_dup {
                self.layers.push(layer.clone());
                added.push(layer);
            }
        }
        if !added.is_empty() {
            self.iterations.push(added.clone());
        }
        added
    }

    /// 获取上一次迭代新增的层 / Get layers added in the last iteration
    pub fn last_iteration_layers(&self) -> &[BinLayer<V, U>] {
        self.iterations.last().map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl<V, U> Default for LayerAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
