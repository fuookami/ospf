//! 物质的量单位 / Amount of substance units
//!
//! 提供物质的量量纲的 SI 单位定义，包括摩尔等 / Provides SI unit definitions for amount of substance dimension, including mole, etc

use crate::dimension::derived::AmountOfSubstance;
use crate::unit::physical_unit::CTUnit;

// ============================================================================
// 物质的量单位 / Amount of substance units
// ============================================================================

define_unit!(Mole, "mole", "mol", AmountOfSubstance);
