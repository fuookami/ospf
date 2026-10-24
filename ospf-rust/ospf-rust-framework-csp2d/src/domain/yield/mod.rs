//! 二维良率领域 / 2D yield domain
//!
//! 映射 Kotlin `csp2d-domain-yield-context`：欠产与超产分析、超产上界约束。
//! Maps the Kotlin `csp2d-domain-yield-context` submodule: under- and over-production analysis plus the
//! over-production upper bound.

pub mod context;

pub use context::{Csp2dYieldMetric, Csp2dYieldResult, YieldContext};
