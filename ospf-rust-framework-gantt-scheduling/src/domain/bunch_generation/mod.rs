//! 任务束生成上下文 / Bunch generation context
//!
//! 映射 Kotlin `gantt-scheduling-domain-bunch-generation-context` 子模块。
//! 实现列生成定价问题的图模型、标签算法和束生成器。
//!
//! Maps the Kotlin `gantt-scheduling-domain-bunch-generation-context` submodule.
//! Implements graph model, label algorithm, and bunch generators for column generation pricing.

pub mod label;
pub mod model;
pub mod pricing;
pub mod service;

pub use pricing::{BunchPricingProblem, LabelSettingAlgorithm};
pub use service::{
    BunchFeasibilityPolicy, BunchGenerationConfig, BunchTaskCandidate, CapacityIntermediateValues,
    DefaultBunchFeasibilityPolicy, PlannedTaskBunchGenerator, SlotBasedBunchGenerator,
    SlotBunchPricingRequest, SlotConstraints, UnplannedTaskBunchGenerator,
};
