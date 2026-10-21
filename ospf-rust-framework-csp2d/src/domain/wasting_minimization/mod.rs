//! 二维余料最小化领域 / 2D waste minimization domain
//!
//! 映射 Kotlin `csp2d-domain-wasting-minimization-context`：余料面积与余料率分析、余料惩罚目标项。
//! Maps the Kotlin `csp2d-domain-wasting-minimization-context` submodule: remnant area and ratio analysis plus
//! waste penalty objective terms.

pub mod context;

pub use context::{Csp2dWasteMetric, Csp2dWasteResult, WasteContext, pattern_waste_ratio};
