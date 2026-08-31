//! BPP3D 应用层 / BPP3D application layer
//!
//! 映射 Kotlin `bpp3d-application` 子模块。
//! Maps the Kotlin `bpp3d-application` submodule.

#[cfg(feature = "serde")]
pub mod csv;
pub mod service;

#[cfg(feature = "serde")]
pub use csv::{
    CsvApplicationMaterializer, CsvApplicationRequestDraft, CsvBinRecord, CsvDataset,
    CsvDatasetError, CsvDatasetLoader, CsvDepthBoundaryPolicy,
    CsvDepthBoundaryPolicyRecord, CsvItemRecord, CsvLayerRecord,
    CsvMaterializedApplicationRequest, CsvSchemaGuard, CsvShapeType,
};
pub use service::{
    ColumnGenerationAlgorithm, ColumnGenerationApplicationFlowResult,
    ColumnGenerationApplicationService, ColumnGenerationApplicationState,
    ColumnGenerationConfig, ColumnGenerationFinalExecution, ColumnGenerationFinalExecutor,
    ColumnGenerationPackingAnalysis, ColumnGenerationPackingAnalyzer,
    ColumnGenerationResult, ColumnGenerationRmpExecution, ColumnGenerationRmpExecutor,
    ColumnGenerationStandardExecutors, ColumnGenerationState, ColumnGenerationStatus,
    DepthBoundaryLayerOrientationPolicy, DepthBoundaryValidationStage,
    MetaModelExecutionDiagnostics, MetaModelExecutorSolveResult, MetaModelFinalExecutor,
    MetaModelFinalExecutorConfig,
    MetaModelRmpExecutor, MetaModelRmpExecutorConfig, MetaModelSolverBackend,
    MockColumnGenerationFinalExecutor, MockColumnGenerationRmpExecutor,
    NoopMetaModelSolverBackend, ObjectiveSense, SolverBackedMetaModelFinalExecutor,
    SolverBackedMetaModelRmpExecutor,
};
#[cfg(feature = "serde")]
pub use service::{
    SolverDatasetFixture, SolverDatasetFixtureManifest, SolverDatasetFixtureManifestEntry,
    SolverDatasetFixtureRunResult, SolverDatasetFixtureSuite, SolverDatasetSuiteRunResult,
};
pub use service::{SolverDatasetSuiteDiagnostics, SolverFeatureMatrixDiagnostics};
pub use crate::domain::packing::{KnownCoordinatePlacement, LayerPlacementAdapter};
#[cfg(not(feature = "async"))]
pub use service::ColumnGenerationSolverMetaModelBackend;
