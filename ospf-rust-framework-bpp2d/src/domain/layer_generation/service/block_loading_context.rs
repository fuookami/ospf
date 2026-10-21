//! 块装载上下文 / Block loading context
//!
//! 对应 BPP3D 的 `BlockLoadingContext`：组合简单块、复杂块与两类空间切分搜索，向候选列生成暴露统一的
//! 块/放置产出。
//! Mirrors BPP3D `BlockLoadingContext`: combines simple blocks, complex blocks and the two space-splitting
//! searches, exposing a unified block/placement output to candidate column generation.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::block_loading::model::{Block2, Space2};
use crate::domain::block_loading::service::{
    ComplexBlockGenerator, DepthFirstSearchAlgorithm2D, MultiRowHeuristicAlgorithm2D,
    SimpleBlockConfig, SimpleBlockGenerator, SpaceSplit2Result,
};
use crate::domain::item::model::ActualItem;

/// 块装载领域上下文 / Block loading domain context
#[derive(Debug, Clone, Default)]
pub struct BlockLoadingContext;

impl BlockLoadingContext {
    /// 创建上下文 / Create the context
    pub fn new() -> Self {
        Self
    }

    /// 生成局部块候选：对每个物品单独生成规则块 / Generate local block candidates: regular blocks per item
    pub fn local_blocks<V, U>(
        &self,
        items: &[(ActualItem<V, U>, usize)],
        space: &Space2<V, U>,
        config: &SimpleBlockConfig,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<Vec<Block2<V, U>>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let generator = SimpleBlockGenerator::new();
        let mut blocks = Vec::new();
        for (item, _) in items {
            blocks.extend(generator.invoke(item, space, config, kerf)?);
        }
        Ok(blocks)
    }

    /// 生成全局块候选：多种物品拼接为异构块 / Generate global block candidates: heterogeneous blocks
    pub fn global_blocks<V, U>(
        &self,
        items: &[(ActualItem<V, U>, usize)],
        space: &Space2<V, U>,
        with_rotation: bool,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<Vec<Block2<V, U>>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        let plain: Vec<ActualItem<V, U>> = items.iter().map(|(item, _)| item.clone()).collect();
        ComplexBlockGenerator::new().invoke(&plain, space, with_rotation, kerf)
    }

    /// 执行 DFS 空间切分 / Run the DFS space split
    pub fn depth_first<V, U>(
        &self,
        space: &Space2<V, U>,
        items: &[(ActualItem<V, U>, usize)],
        max_depth: usize,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<SpaceSplit2Result<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        DepthFirstSearchAlgorithm2D::new().invoke(space, items, max_depth, kerf)
    }

    /// 执行多行启发式搜索 / Run the multi-row heuristic search
    pub fn multi_row<V, U>(
        &self,
        space: &Space2<V, U>,
        items: &[(ActualItem<V, U>, usize)],
        max_rows: usize,
        kerf: Option<&Quantity<V, U>>,
    ) -> Ret<SpaceSplit2Result<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        MultiRowHeuristicAlgorithm2D::new().invoke(space, items, max_rows, kerf)
    }
}
