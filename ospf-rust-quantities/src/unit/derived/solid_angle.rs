//! Solid angle units - 立体角单位
//! Solid angle units - SI solid angle units
//!
//! 提供立体角量纲的 SI 单位定义，包括球面度等。
//! Provides SI unit definitions for solid angle dimension, including steradian, etc.

use crate::dimension::derived::SolidAngle;
use crate::scale::Scale;
use crate::unit::CTUnit;

// ============================================================================
// 立体角单位 / Solid angle units
// ============================================================================

// 球面度 / Steradian
define_unit!(Steradian, "steradian", "sr", SolidAngle);

// 平方度 / Square degree (1 sr = (180/π)² square degrees)
define_unit!(SquareDegree, "square degree", "deg²", SolidAngle, Scale::from_f64(0.00030461741978670857));
