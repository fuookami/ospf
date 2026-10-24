//! 二维主问题领域 / 2D master problem domain
//!
//! 映射 Kotlin `csp2d-domain-produce-context`：图案使用量、需求覆盖、物料可用量与目标项。
//! 本层是 csp2d 的建模边界，允许出现求解器数值。
//! Maps the Kotlin `csp2d-domain-produce-context` submodule: pattern usage, demand coverage, material
//! availability and objective terms. This layer is csp2d's modelling boundary and may use solver numbers.

pub mod master_problem;
pub mod pricing;

pub use pricing::{
    Csp2dPricingConfig, Csp2dReducedCost, Csp2dShadowPrices, DEFAULT_REDUCED_COST_TOLERANCE,
    estimate_reduced_cost, estimated_shadow_prices, improving_columns, reduced_cost_of, reduced_costs,
};
pub use master_problem::{
    Csp2dIterationRecord, Csp2dMasterDemand, Csp2dMasterProblem, Csp2dPatternAssignment,
    residual_products,
};
