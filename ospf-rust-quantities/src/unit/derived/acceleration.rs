//! Acceleration units - 加速度单位
//! Acceleration units - SI acceleration units
//!
//! 提供加速度量纲的 SI 单位定义，包括米每二次方秒、标准重力加速度等。
//! Provides SI unit definitions for acceleration dimension, including meter per second squared, standard gravity, etc.

use super::time::Second;
use super::velocity::MeterPerSecond;
use crate::dimension::derived::Acceleration;
use crate::scale::Scale;
use crate::unit::physical_unit::CTUnit;
use crate::unit::CTUnitDiv;
use once_cell::sync::Lazy;

// ============================================================================
// 加速度单位 / Acceleration units
// ============================================================================

define_unit_by!(
    MeterPerSecondSquared,
    "meter per second squared",
    "m/s²",
    CTUnitDiv<MeterPerSecond, Second>
);
define_unit!(
    StandardGravity,
    "standard gravity",
    "g",
    Acceleration,
    &*MeterPerSecondSquared::SCALE / &Scale::from_f64(9.80665)
);
