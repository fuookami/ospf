//! 发光强度单位 / Luminous intensity units
//!
//! 提供发光强度量纲的 SI 单位定义，包括坎德拉等 / Provides SI unit definitions for luminous intensity dimension, including candela, etc

use crate::dimension::derived::LuminousIntensity;
use crate::unit::CTUnit;

// ============================================================================
// 发光强度单位 / Luminous intensity units
// ============================================================================

define_unit!(Candela, "candela", "cd", LuminousIntensity);
