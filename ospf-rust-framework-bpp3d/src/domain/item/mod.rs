//! 货物领域上下文 / Item domain context
//!
//! 映射 Kotlin `bpp3d-domain-item-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-item-context` submodule.

pub mod model;
pub mod service;

pub use model::{
    ActualItem, Bin, BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dDemandValue,
    Bpp3dLayerDemandCoverage, ContinuousCylinderRadiusSolverPrototype,
    ContinuousRadiusModelComponent, ContinuousRadiusRegistrationPlan, CylinderShapeContract,
    CylinderCapabilityStatus, DemandStatistics, Material, MaterialKey, MaterialType, Package,
    PackageAttribute, PackageShape, PackageShapeSpec, PackingProgram, PatternedItem,
    PatternedItemKey,
};
