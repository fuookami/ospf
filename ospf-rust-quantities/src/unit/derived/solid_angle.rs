//! Solid angle units - 立体角单位
//! Solid angle units - SI solid angle units
//!
//! 提供立体角量纲的 SI 单位定义，包括球面度等。
//! Provides SI unit definitions for solid angle dimension, including steradian, etc.

use crate::dimension::derived::SolidAngle;
use crate::scale::Scale;
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 立体角单位 / Solid angle units
// ============================================================================

define_unit!(Steradian, "steradian", "sr", SolidAngle);
