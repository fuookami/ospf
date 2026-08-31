//! BPP3D 应用层 / BPP3D application layer
//!
//! 映射 Kotlin `bpp3d-application` 子模块。
//! Maps the Kotlin `bpp3d-application` submodule.

#[cfg(feature = "serde")]
pub mod csv;
pub mod report;
pub mod service;

#[cfg(feature = "serde")]
pub use csv::{
    CsvApplicationMaterializer, CsvApplicationRequestDraft, CsvBinRecord, CsvDataset,
    CsvDatasetError, CsvDatasetLoader, CsvDepthBoundaryPolicy,
    CsvDepthBoundaryPolicyRecord, CsvItemRecord, CsvLayerRecord,
    CsvMaterializedApplicationRequest, CsvSchemaGuard, CsvShapeType,
};
pub use report::{
    Bpp3dDemandCoverageReport, Bpp3dErrorCategory, Bpp3dFixtureReport,
    Bpp3dFixtureStatus, Bpp3dPackedBinReport, Bpp3dRunReport,
    Bpp3dRunReportComparison, Bpp3dRunReportDifference,
    Bpp3dRunReportDifferenceSeverity, Bpp3dSelectedLayerReport,
    Bpp3dSolverAvailability, Bpp3dSolverFailure, Bpp3dSolverModelStatus,
    Bpp3dSuiteSummary, FixtureFilter,
};
#[cfg(feature = "serde")]
pub use report::Bpp3dRunReportIoError;
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
    LayerGenerationFixtureQualityReport, LayerGenerationQualityComparison,
    LayerGenerationQualityDifference, LayerGenerationQualityDifferenceSeverity,
    LayerGenerationSourceQualityReport, LayerGenerationSuiteQualityReport,
};
pub use service::{SolverDatasetSuiteDiagnostics, SolverFeatureMatrixDiagnostics};
pub use service::SolverBackendSurveyReport;
pub use crate::domain::packing::{KnownCoordinatePlacement, LayerPlacementAdapter};
#[cfg(not(feature = "async"))]
pub use service::ColumnGenerationSolverMetaModelBackend;
