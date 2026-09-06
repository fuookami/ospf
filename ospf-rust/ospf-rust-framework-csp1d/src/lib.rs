//! 一维分切领域框架 / CSP1D domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-csp1d` 的 Rust 迁移。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-csp1d`.
//!
//! 目前提供核心领域模型、生成语义、应用入口与恢复骨架，后续会继续对齐 Kotlin
//! 的完整求解行为与扩展点。
//! The crate currently provides core domain models, generation semantics,
//! application entry points, and recovery skeletons, and will continue to align
//! with the Kotlin solve behavior and extension points.

pub mod application;
pub mod domain;
pub mod infrastructure;

// 领域错误 re-export（避免与 crate 级 Csp1dError 冲突）
// Domain error re-exports (avoids conflict with crate-level Csp1dError)
pub use domain::error::{
    Csp1dCapabilityError as Csp1dDomainCapabilityError, Csp1dError as Csp1dDomainError,
    Csp1dLifecycleError as Csp1dDomainLifecycleError, Csp1dSolvingError as Csp1dDomainSolvingError,
    Csp1dTypeError as Csp1dDomainTypeError,
};

pub use application::model::{
    Csp1dAssignment, Csp1dConfiguration, Csp1dKpi, Csp1dKpiKeys, Csp1dProblem, Csp1dProblemBuilder,
    Csp1dSolution, Csp1dSolutionAnalyzer, Csp1dSolutionStatus, Csp1dSolveConfig,
    Csp1dSolveConfigBuilder, DefaultCsp1dSolutionAnalyzer, csp1d_problem, csp1d_solve_config,
};
pub use application::service::{
    Csp1dColumnGeneration, Csp1dColumnGenerationRecovery, Csp1dColumnGenerationResult,
    Csp1dColumnGenerationTrace, Csp1dFinalMilpStatus, Csp1dIterationRecord, Csp1dLpSolveResult,
    Csp1dMilp, Csp1dMilpSolveResult, Csp1dMilpSolver, Csp1dRecovery,
    Csp1dRecoveryFallbackDisabledException, Csp1dRecoveryInput, Csp1dRecoveryOptions,
    Csp1dRecoveryResult, Csp1dRecoverySolveException, Csp1dRecoveryStatus, Csp1dRecoveryTrace,
    Csp1dSchedule, Csp1dTerminationReason, Csp1dUnsupportedWarmStartAdapter, Csp1dWarmStart,
    Csp1dWarmStartAdapter, Csp1dWarmStartAdapterInput, Csp1dWarmStartAdapterResult,
    Csp1dWarmStartPlanPoolAdapter, Csp1dWarmStartStatus,
};
pub use domain::cutting_plan_generation::{
    CostarFiller, Csp1dCandidateFilter, Csp1dCanonicalKeyOverride, Csp1dDominanceAcceptOverride,
    Csp1dInitialCuttingPlanGenerator, Csp1dIsImprovingJudge, Csp1dPricingBenefitModifier,
    Csp1dPricingCostModifier, Csp1dPricingGenerator, Csp1dPricingInput,
    Csp1dPricingObjectiveConfig, Csp1dWidthFeasibilityCheck, CuttingPlanConstraint,
    CuttingPlanConstraintContext, CuttingPlanGenerationBenchmarkSnapshot,
    CuttingPlanGenerationInput, CuttingPlanGenerationReport, CuttingPlanGenerationStatistics,
    CuttingPlanGenerationStopReason, DFSGenerator, DominanceStrategy, FullSumGenerator,
    GenerationConstraints, GenerationReportMergeOptions, MaxKnifeCountConstraint,
    MaxOverProduceLengthConstraint, MinKnifeCountConstraint, NSameGenerator, NSumGenerator,
    ReducedCostPricingGenerator, SimpleInitialCuttingPlanGenerator, SimplePricingGenerator,
    WidthUpperBoundConstraint, merge_generation_reports, width_feasibility_check_from_policies,
};
pub use domain::length_assignment::{
    DefaultLengthDerivation, LengthAssignment, LengthAssignmentConstraint, LengthAssignmentContext,
    LengthAssignmentInput, LengthAssignmentModelingConfig, LengthAssignmentResult,
    LengthDerivation, LengthSlackAggregation, OverLengthRecord,
};
pub use domain::material::{
    Costar, Csp1dQuantity, Csp1dShadowPriceKey, CuttingPlan, CuttingPlanDemandContribution,
    CuttingPlanProduction, CuttingPlanSlice, DefaultQuantityArithmetic, DemandMode, Machine,
    MachineBatchShadowPriceKey, MachineCapacityShadowPriceKey, Material,
    MaterialUsageShadowPriceKey, Product, ProductDemand, ProductDemandShadowPriceKey,
    ProductLegacyInput, Production, QuantityArithmetic, QuantityRange, RollCountUnit,
    ShadowPriceMap, SheetCountUnit, WidthRange, YieldOverProductionBoundShadowPriceKey,
    convert_solver_value, from_f64, roll_count_unit, shadow_price_key_from_string,
    shadow_price_key_to_string, shadow_price_unit_symbol, sheet_count_unit, to_f64,
};
pub use domain::produce::{
    ContributionKey, Csp1dCGPipeline, Csp1dDefaultShadowPriceMap, Csp1dDomainCalculationContext,
    Csp1dDomainPolicy, Csp1dExtensionMode, Csp1dExtensionSet, Csp1dExtractionPolicy,
    Csp1dFlowContext, Csp1dFlowPolicy, Csp1dGenerationStrategy, Csp1dIncrementalPipeline,
    Csp1dIterativeContext, Csp1dModelContext, Csp1dModelingContext, Csp1dModelingExtension,
    Csp1dModelingMode, Csp1dObjectivePolicy, Csp1dPlanJudgmentContext, Csp1dPricingPolicy,
    Csp1dProduceContext, Csp1dProduceContextBuilder, Csp1dShadowPriceExtractor,
    Csp1dShadowPriceLifecycle, CuttingPlanUsage, DemandConstraintPipeline,
    LengthConstraintPipeline, LengthObjectivePipeline, MachineCapacityUsage,
    MachineConstraintPipeline, MaterialConstraintPipeline, MaterialUsage, Produce,
    ProduceAggregation, ProduceInput, SimpleDomainCalculationContext, WasteObjectivePipeline,
    YieldConstraintPipeline, YieldObjectivePipeline, accept_partial_by_policies,
    allow_recovery_fallback_by_policies, filter_initial_plans_by_policies,
    filter_initial_plans_by_policies_with_context, is_equivalent_by_policies,
    select_termination_by_policies, select_termination_by_policies_with_default,
    should_stop_by_policies,
};
pub use domain::wasting_minimization::{
    ModeledMaterialCost, OverProductionAreaMeasure, RestMaterialMeasure, WasteAggregation,
    WasteAnalysis, WasteMinimizationConfig, WasteMinimizationResult,
};
pub use domain::r#yield::{
    DemandAggregationKey, ModeledOverProduction, ModeledUnderProduction, ProductOutput,
    YieldAggregation, YieldAnalysis, YieldContext, YieldModelingConfig, YieldModelingResult,
    YieldSlackAggregation,
};
pub use infrastructure::dto::{
    RenderCuttingPlanDTO, RenderCuttingPlanProductionDTO, RenderProductionType, RenderSchemaDTO,
};

/// 一维分切错误类型 / CSP1D error type
#[derive(Debug, thiserror::Error)]
pub enum Csp1dError {
    /// 计算错误 / Calculation error
    #[error("calculation error: {message}")]
    Calculation { message: String },

    /// 无效输入 / Invalid input
    #[error("invalid input: {message}")]
    InvalidInput { message: String },

    /// 不支持的操作 / Unsupported operation
    #[error("unsupported operation: {message}")]
    Unsupported { message: String },

    /// 恢复 fallback 被禁用 / Recovery fallback disabled
    #[error("{message}")]
    RecoveryFallbackDisabled {
        message: String,
        trace: crate::application::service::Csp1dRecoveryTrace,
    },
}

/// CSP1D 结果类型 / CSP1D result type
pub type Csp1dResult<T> = Result<T, Csp1dError>;
