//! 货物领域上下文 / Item domain context
//!
//! 映射 Kotlin `bpp3d-domain-item-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-item-context` submodule.

pub mod model;
pub mod service;

pub use model::{
    ActualItem, Bin, BinLayer, BinType, BinTypeId, BottomDimensionRange, Bpp3dDemandKey,
    Bpp3dDemandMode, Bpp3dDemandValue, Bpp3dLayerDemandCoverage, CargoAttributeKey,
    ContinuousCylinderRadiusSolution, ContinuousCylinderRadiusSolverPrototype,
    ContinuousRadiusModelComponent, ContinuousRadiusModelRegistration,
    ContinuousRadiusObjectivePolicy, ContinuousRadiusRegistrationPlan,
    ContinuousRadiusVariableRegistration, ContinuousRadiusWeightFunction, CylinderCapabilityStatus,
    CylinderShapeContract, DeformationAttribute, DemandStatistics, HangingPolicy, ItemId, Material,
    MaterialKey, MaterialType, Package, PackageAttribute, PackageCategory, PackageClassification,
    PackageOrientationRule, PackageOrientationRuleInput, PackagePairStackingRule,
    PackagePlacementBottomContext, PackagePlacementStackingInput, PackagePlacementStackingRule,
    PackageShape, PackageShapeSpec, PackageStackingInput, PackageType, PackingProgram,
    PackingProgramMaterialValue, PatternConfig, PatternDefinition, PatternNextPointPolicy,
    PatternProjectionOrientation, PatternStep, PatternedItem, PatternedItemKey, StackingOnPolicy,
    WeightAttribute, bin_type_id, item_id,
};
