//! 二维物料领域 / 2D material domain
//!
//! 映射 Kotlin `csp2d-domain-material-context`：双模式物料、产品与需求、裁切图案与裁切方案。
//! Maps the Kotlin `csp2d-domain-material-context` submodule: dual-mode materials, products and demands,
//! cutting patterns and cutting plans.

pub mod ids;
pub mod material;
pub mod pattern;
pub mod product;

pub use ids::{
    CostarId2, MachineId2, MaterialId2, PatternId2, PlanId2, ProductId2, costar_id2, machine_id2,
    material_id2, pattern_id2, plan_id2, product_id2,
};
pub use material::{Costar2, Machine2, Material2, Material2Kind};
pub use pattern::{CuttingPlan2, Pattern2, fits_in, remnant_reaches};
pub use product::{DemandMode2, DemandValue2, Product2, ProductDemand2, ProductDemands2};
