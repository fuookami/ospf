//! 常用建模入口预导入模块
//! Prelude module for common modeling entry points

pub use crate::analysis::{
    ACTIVITY_REPORT_SCHEMA_VERSION, ActivityConfig, ActivityEvidence, ActivityGroupSummary,
    ActivityStatus, ActivitySummary, AdaptivePerturbationConfig, AdaptivePerturbationOutcome,
    AnalysisCacheKind, AnalysisCapability, AnalysisCapabilitySupport, AnalysisStatus, BoundSide,
    CapabilityMatrix, CapabilitySupport, ConstraintActivity, ConstraintActivityAdapter,
    ConstraintActivityAnalyzer, ConstraintActivityReport, ConstraintId,
    CriticalityKind, CriticalityObservation, CriticalityProfile, analyze_criticality_targets,
    build_criticality_profile,
    CONFLICT_REPORT_SCHEMA_VERSION, ConflictAnalysisOptions, ConflictAnalyzer, ConflictCache,
    ConflictExplanation, ConflictGroupSummary, ConflictMinimality, ConflictValidity,
    ConflictVerification, MinimalBlockingSet, MinimalConflictAnalyzer,
    AlternativeImprovementPlan, CorrectionCandidate, CorrectionSet, RelaxabilityPolicy,
    RelaxationCost, alternative_improvement_plans_from_conflict, correction_sets_from_conflict,
    minimal_correction_set, relaxation_recommendations_from_conflict, weighted_correction_set,
    weighted_correction_sets_from_conflict,
    MULTI_TARGET_ANALYSIS_REPORT_SCHEMA_VERSION, MultiTargetAnalysisOptions,
    MultiTargetAnalysisReport, MultiTargetAnalysisResult, MultiTargetAnalyzer, analyze_targets,
    relaxation_plans_for_result,
    ConstraintPerturbationAnalyzer, ConstraintPerturbationCache, ConstraintPerturbationObservation,
    ConstraintPerturbationPolicy, ConstraintPerturbationReport, ConstraintPerturbationAdapter,
    ConstraintPerturbationAnalysisAdapter, ConstraintPerturbationAnalysisRequest,
    ConstraintProgrammingFeature, ConstraintProgrammingFixedIntegerLpAdapter,
    ConstraintProgrammingSupport, CriticalConstraintAnalysisOptions,
    CriticalConstraintAnalysisPipeline, CriticalConstraintAnalysisSession, DiagnosticSource,
    FixedIntegerIncumbentScope, FixedIntegerLpModel, FixedIntegerLpScope,
    FixedIntegerLpAdapter, FixedIntegerLpAnalysisAdapter, FixedIntegerLpAnalysisRequest,
    FixedIntegerLpSensitivityAnalyzer, FixedIntegerLpSensitivityCache,
    FixedIntegerLpSensitivityConfig, LocalConstraintSensitivity, LocalConstraintSensitivityReport,
    ObjectiveId, ObjectiveTarget, ObjectiveTargetRelation, PerturbationObservation,
    PerturbationPolicy, RemovalTestObservation, SensitivityScope, SolverModelType,
    VariableBoundActivity, VariableBoundRef, VariableDomainRef, linear_constraint_ids,
};
pub use crate::error::{
    CoreError, ModelError, Result, SolverError, SolverErrorClass, VariableError,
};
pub use crate::model::basic::{ConstraintPriority, ConstraintPriorityStats};
pub use crate::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, DumpOptions, LPExportableModel,
    LinearElasticBuilder, LinearTriadModel, LinearTriadModelView, ModelFileFormat,
    QuadraticElasticBuilder, QuadraticTetradModel, QuadraticTetradModelView, SparseMatrix,
    SparseVector, dump_batch, dump_lp_batch, dump_opm_batch,
};
pub use crate::model::{
    ConstraintGroup, ConstraintRelation, LinearConstraint, LinearConstraintInput,
    LinearExpressionBuilder, LinearInequality, MetaModel, ModelBuildingStage, ModelBuildingStatus,
    ModelBuildingStatusCallback, Objective, ObjectiveCategory, QuadraticConstraint,
    QuadraticInequality, SubObjective, SymbolicLinearInequality, SymbolicQuadraticInequality,
};
pub use crate::solver::iis::{IISConfig, LinearIISModel};
#[cfg(feature = "async")]
pub use crate::solver::{
    AsyncSolveOptions, AsyncSolver, SolveJoinHandle, SolveReportJoinHandle,
    solve_async_report_with_callback, solve_async_report_with_options, solve_async_with_callback,
    solve_async_with_options, spawn_solve, spawn_solve_report, spawn_solve_report_with_options,
    spawn_solve_with_callback, spawn_solve_with_options,
};
pub use crate::solver::{
    ConfigurableSolver, FeasibleSolverOutput, Flt64MultiSolutionOutput, LinearSolver,
    MultiSolutionOutput, ProblemStatus, QuadraticSolver, SolveOptions, SolveOptionsBuilder,
    SolveProof, SolveReport, SolveReportWithIIS, SolveSolution, SolveStatistics, SolveTrace,
    SolveValue, SolveValueConversionPolicy, Solver, SolverConfig, SolverExt, SolverInfo,
    SolverOutput, SolverOutputWithIIS, SolverStatus, SolvingStatus, SolvingStatusCallback,
    TerminationReason,
};
pub use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
pub use crate::symbol::function::*;
pub use crate::token::{
    AnyVariable, ConcurrentTokenList, ConcurrentTokenTable, Token, TokenList, TokenTable,
};
pub use crate::variable::*;
