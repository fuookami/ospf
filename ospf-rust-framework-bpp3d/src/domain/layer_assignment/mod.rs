//! 层分配上下文 / Layer assignment context
//!
//! 映射 Kotlin `bpp3d-domain-layer-assignment-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-layer-assignment-context` submodule.

pub mod model;
pub mod service;

pub use model::{
    Bpp3dModelComponent, ExpressionArray1, SolutionExtractor, VariableArray1, VariableArray2,
};

pub use service::{
    Bpp3dSolverValueAdapter, Bpp3dSolverValueAdapterKind,
    DefaultBpp3dSolverValueAdapter, ScaledBpp3dSolverValueAdapter,
    ImpreciseAssignment, PreciseAssignment,
    Load, Capacity, PreciseLoadCapacity,
    Bpp3dDemandEntry, DemandShadowPriceKey,
    IterativeLayerAssignmentContext, IterativeLayerColumn,
    LayerAggregation, LayerAssignmentAggregation, LayerAssignmentContext,
    SolutionAnalyzer,
};

pub use service::limits::{
    DemandConstraint, DemandAssignmentRef,
    BinCapacityConstraint, BinDepthConstraint,
    BinAmountMinimization, VolumeMinimization, BetterLayerMaximization,
    PreciseAssignmentActivationConstraint, TailBinAssignmentConstraint,
    RestAmountMinimization, TailBinLoadingRateMinimization,
    BinLoadingOrderConstraint, DeferredRegistrationPlan,
};
