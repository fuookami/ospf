//! 二维物品模型 / 2D item models
//!
//! 物品、板材、层、需求统计、物料与标识类型。全部泛型于 `V` 并保留编译时单位 `U`；
//! 求解器数值类型只出现在 layer-assignment 与应用层。
//! Items, sheets, layers, demand statistics, materials and identifier types. Everything is generic over `V`
//! while keeping the compile-time unit `U`; solver value types appear only in layer-assignment and the
//! application layer.

pub mod demand_statistics;
pub mod id;
pub mod item;
pub mod layer;
pub mod material;
pub mod sheet;

pub use demand_statistics::{
    Bpp2dDemandDomain, Bpp2dDemandEntry, Bpp2dDemandKey, Bpp2dDemandMode, Bpp2dDemandValue,
    DemandModeKey, aggregate_item_amounts,
};
pub use id::{
    BatchNo, ItemId, MaterialNo, SheetTypeId, batch_no, item_id, material_no, sheet_type_id,
    single_batch_no,
};
pub use item::{
    AbstractItem, ActualItem, ItemKey, PatternItem, item_candidate_shapes, item_key, item_shape_type,
};
pub use layer::{Layer, SheetLayer, layer_area, placement_area, same_sheet_type, sheet_type_code};
pub use material::{
    Material, MaterialKey, MaterialType, finished_material, raw_material, semi_finished_material,
};
pub use sheet::{Sheet, SheetType, shape_area, sheet_loading_rate};
