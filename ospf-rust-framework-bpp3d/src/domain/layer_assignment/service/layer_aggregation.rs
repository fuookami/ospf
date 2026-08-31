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
    pub fn add_columns(&mut self, new_layers: Vec<BinLayer<V, U>>) -> Vec<BinLayer<V, U>>
    where
        V: Clone + PartialEq,
    {
        let mut added = Vec::new();
        for layer in new_layers {
            // 简化去重：基于深度和来源
            let is_dup = self.layers.iter().any(|existing| {
                existing.depth.value == layer.depth.value && existing.from == layer.from
            });
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
