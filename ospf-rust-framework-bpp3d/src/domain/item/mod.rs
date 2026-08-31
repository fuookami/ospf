//! 货物领域上下文 / Item domain context
//!
//! 映射 Kotlin `bpp3d-domain-item-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-item-context` submodule.

pub mod model;
pub mod service;

pub use model::{
    ActualItem, Bin, BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dDemandValue,
    Bpp3dLayerDemandCoverage, BottomDimensionRange, CargoAttributeKey, BinTypeId,
    ContinuousCylinderRadiusSolution, ContinuousCylinderRadiusSolverPrototype,
    ContinuousRadiusModelComponent, ContinuousRadiusModelRegistration,
    ContinuousRadiusObjectivePolicy, ContinuousRadiusWeightFunction,
    ContinuousRadiusRegistrationPlan, ContinuousRadiusVariableRegistration,
    CylinderShapeContract, CylinderCapabilityStatus, DemandStatistics, DeformationAttribute,
    HangingPolicy, ItemId, Material, MaterialKey, MaterialType, Package, PackageAttribute, PackageCategory,
    PackageClassification, PackageOrientationRule, PackageOrientationRuleInput,
    PackagePairStackingRule, PackagePlacementBottomContext, PackagePlacementStackingInput,
    PackagePlacementStackingRule, PackageShape, PackageShapeSpec, PackageStackingInput,
    PackageType, PackingProgram, PackingProgramMaterialValue, PatternConfig, PatternDefinition,
    PatternedItem, PatternedItemKey, PatternNextPointPolicy, PatternProjectionOrientation,
    PatternStep, StackingOnPolicy, WeightAttribute, bin_type_id, item_id,
};
