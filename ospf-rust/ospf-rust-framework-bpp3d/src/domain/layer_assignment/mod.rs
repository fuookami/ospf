//! 层分配上下文 / Layer assignment context
//!
//! 映射 Kotlin `bpp3d-domain-layer-assignment-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-layer-assignment-context` submodule.

pub mod model;
pub mod service;

pub use model::{Bpp3dModelComponent, SolutionExtractor, VariableArray1, VariableArray2};

pub use service::{
    Bpp3dDemandEntry, Bpp3dSolverValueAdapter, Bpp3dSolverValueAdapterKind, Capacity,
    DefaultBpp3dSolverValueAdapter, DemandShadowPriceKey, ImpreciseAssignment,
    IterativeLayerAssignmentContext, IterativeLayerColumn, LayerAggregation,
    LayerAssignmentAggregation, LayerAssignmentContext, Load, PreciseAssignment,
    PreciseLoadCapacity, ScaledBpp3dSolverValueAdapter, SolutionAnalyzer,
    build_linear_expression_symbol, next_bpp3d_symbol_id,
};

pub use service::limits::{
    BetterLayerMaximization, BinAmountMinimization, BinCapacityConstraint, BinDepthConstraint,
    BinLoadingOrderConstraint, DeferredRegistrationPlan, DemandAssignmentRef, DemandConstraint,
    PreciseAssignmentActivationConstraint, RestAmountMinimization, TailBinAssignmentConstraint,
    TailBinLoadingRateMinimization, VolumeMinimization,
};
