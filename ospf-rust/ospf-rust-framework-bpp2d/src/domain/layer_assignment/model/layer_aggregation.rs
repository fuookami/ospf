//! 层聚合 / Layer aggregation
//!
//! 列生成过程中的候选层池，按列身份去重。层身份包含板材类型、来源、包围尺寸与放置集合，
//! 因此同一排布放在不同板材上是不同的列。
//! The candidate layer pool of a column generation run, deduplicated by column identity. The identity covers
//! the sheet type, source, bounding size and placement set, so the same layout on different sheets is a
//! different column.

use crate::domain::item::model::Layer;

/// 层聚合 / Layer aggregation
#[derive(Debug, Clone, Default)]
pub struct LayerAggregation<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 当前层集合 / Current layer set
    layers: Vec<Layer<V, U>>,
    /// 每次迭代新增的层 / Layers added per iteration
    iterations: Vec<Vec<Layer<V, U>>>,
}

impl<V, U> LayerAggregation<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    /// 创建层聚合 / Create a layer aggregation
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            iterations: Vec::new(),
        }
    }

    /// 当前层集合 / Current layer set
    pub fn layers(&self) -> &[Layer<V, U>] {
        &self.layers
    }

    /// 最近一次迭代的层 / Layers of the last iteration
    pub fn last_iteration_layers(&self) -> &[Layer<V, U>] {
        self.iterations.last().map(|layers| layers.as_slice()).unwrap_or(&[])
    }

    /// 添加新层并去重 / Add new layers with deduplication
    ///
    /// 返回本次真正新增的层；身份相同的层被丢弃。
    /// Returns the layers actually added; layers with an existing identity are dropped.
    pub fn add_columns(&mut self, new_layers: Vec<Layer<V, U>>) -> Vec<Layer<V, U>> {
        let mut added = Vec::new();
        for layer in new_layers {
            let identity = layer.identity_key();
            let duplicate = self
                .layers
                .iter()
                .any(|existing| existing.identity_key() == identity);
            if !duplicate {
                self.layers.push(layer.clone());
                added.push(layer);
            }
        }
        if !added.is_empty() {
            self.iterations.push(added.clone());
        }
        added
    }

    /// 层数量 / Layer count
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}
