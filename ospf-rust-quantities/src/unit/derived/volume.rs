//! Volume units - 体积单位
//! Volume units - SI volume units
//!
//! 提供体积量纲的 SI 单位定义，包括立方米、升、毫升等。
//! Provides SI unit definitions for volume dimension, including cubic meter, liter, milliliter, etc.

use super::length::Meter;
use crate::dimension::derived::Volume;
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitMul};
use once_cell::sync::Lazy;

// ============================================================================
// 体积单位 / Volume units
// ============================================================================

define_unit_by!(
    CubicMeter,
    "cubic meter",
    "m³",
    CTUnitMul<Meter, CTUnitMul<Meter, Meter>>
);
define_unit!(
    Liter,
    "liter",
    "L",
    Volume,
    Scale::from_int(1) / Scale::from_int(1000)
);
