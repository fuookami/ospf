//! 装箱上下文 / Packing context
//!
//! 映射 Kotlin `bpp3d-domain-packing-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-packing-context` submodule.

pub mod model;
pub mod service;

pub use model::{
    PackedBin, PackedItem, MaterialSummary, MaterialAttribute, MaterialPackingNumbers,
    MaterialPackingPlan, PackageSolutionLikeAdapter,
};

pub use service::{
    PackingGeometryGuard, PackingGeometryContract,
    Packer, PackingResult, PackingAggregation,
    MaterialPacker, PackingRendererAdapter,
    KnownCoordinatePlacement, LayerPlacementAdapter,
    LayerTraceReplayAdapter, LayerTraceReplayResult,
};

use std::collections::HashMap;
use crate::domain::item::MaterialKey;

// ============================================================================
// PackingContext - 装箱上下文 / Packing context
// ============================================================================

/// 装箱上下文 / Packing context
///
/// 装箱过程的附加信息，包含剩余物品和物料。
/// Additional information for the packing process, including remaining items and materials.
#[derive(Debug, Clone, Default)]
pub struct PackingContext {
    /// 剩余物品 / Remaining items (item_index, remaining_amount)
    pub rest_items: Vec<(usize, u64)>,
    /// 剩余物料 / Remaining materials
    pub rest_materials: HashMap<MaterialKey, u64>,
    /// 附加信息 / Additional info
    pub info: HashMap<String, String>,
}
