#![cfg_attr(feature = "nightly", feature(unboxed_closures, fn_traits))]

//! OSPF Rust Core - 运筹建模框架核心模块
//! OSPF Rust Core - Operations Research Modeling Framework Core Module
//!
//! 本模块实现一个运筹学建模框架，支持线性规划 (LP)、混合整数规划 (MIP)、
//! 二次规划 (QP) 等优化问题的建模与求解。
//!
//! This module implements an operations research modeling framework supporting
//! linear programming (LP), mixed integer programming (MIP), quadratic programming (QP),
//! and other optimization problems.
//!
//! # 核心模块 / Core Modules
//!
//! - [`variable`] - 变量系统（Variable System）
//! - [`token`] - Token 系统（Token System）
//! - [`symbol`] - 中间符号系统（Intermediate Symbol System）
//! - [`model`] - 模型系统（Model System）
//!   - [`symbol::flatten`] - 表达式平展系统主路径（Expression Flatten primary path）
//!   - [`model::mechanism`] - 机理模型系统，包含约束（Mechanism Model System, includes constraints）
//!   - [`model::intermediate`] - 中间模型层（Intermediate Model Layer）
//!   - [`model::callback`] - 回调模型层（Callback Model Layer）
//! - [`solver`] - 求解器接口（Solver Interface）

#[cfg(any(
    all(feature = "gurobi10", feature = "gurobi11"),
    all(feature = "gurobi10", feature = "gurobi12"),
    all(feature = "gurobi11", feature = "gurobi12"),
))]
compile_error!("Only one Gurobi version feature can be enabled: gurobi10, gurobi11, or gurobi12.");

pub mod analysis;
pub mod error;
pub mod model;
pub mod prelude;
pub mod solver;
pub mod symbol;
pub mod token;
pub mod variable;

pub use analysis::{
    ACTIVITY_REPORT_SCHEMA_VERSION, ADAPTIVE_PERTURBATION_REPORT_SCHEMA_VERSION, ActivityConfig,
    ActivityEvidence, ActivityGroupSummary, ActivityStatus, ActivitySummary,
    AdaptivePerturbationConfig, AdaptivePerturbationOutcome, AnalysisCacheKind, AnalysisCapability,
    AnalysisCapabilitySupport, AnalysisStatus, BoundSide, ConstraintActivity,
    ConstraintActivityAdapter, ConstraintActivityAnalyzer, ConstraintActivityReport, ConstraintId,
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
    CriticalConstraintAnalysisOptions, CriticalConstraintAnalysisPipeline,
    CriticalConstraintAnalysisSession, DiagnosticSource, FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION,
    FixedIntegerIncumbentScope, FixedIntegerLpModel, FixedIntegerLpScope,
    FixedIntegerLpAdapter, FixedIntegerLpAnalysisAdapter, FixedIntegerLpAnalysisRequest,
    FixedIntegerLpSensitivityAnalyzer, FixedIntegerLpSensitivityCache,
    FixedIntegerLpSensitivityConfig, LocalConstraintSensitivity, LocalConstraintSensitivityReport,
    ObjectiveId, ObjectiveTarget, ObjectiveTargetRelation, PERTURBATION_REPORT_SCHEMA_VERSION,
    PerturbationObservation, PerturbationPolicy, RemovalTestObservation, SensitivityScope,
    SolverModelType, VariableBoundActivity, VariableBoundRef, VariableDomainRef,
    linear_constraint_ids,
};
pub use error::*;
#[allow(ambiguous_glob_reexports)]
pub use model::*;
#[allow(ambiguous_glob_reexports)]
pub use solver::*;
pub use symbol::expression_symbol::*;
pub use symbol::function::*;
pub use symbol::function_symbol::*;
pub use symbol::monomial_cell::*;
pub use token::*;

// 重新导出常用类型
// Re-export commonly used types
pub use variable::*;
