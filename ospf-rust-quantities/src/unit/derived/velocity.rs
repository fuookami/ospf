//! Velocity units - 速度单位
//! Velocity units - SI velocity units
//!
//! 提供速度量纲的 SI 单位定义，包括米每秒、千米每小时等。
//! Provides SI unit definitions for velocity dimension, including meter per second, kilometer per hour, etc.

use super::length::{Kilometer, Meter};
use super::time::{Hour, Second};
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitDiv};
use once_cell::sync::Lazy;

// ============================================================================
// 速度单位 / Velocity units
// ============================================================================

define_unit_by!(
    MeterPerSecond,
    "meter per second",
    "m/s",
    CTUnitDiv<Meter, Second>
);
define_unit_by!(
    KilometerPerHour,
    "kilometer per hour",
    "km/h",
    CTUnitDiv<Kilometer, Hour>
);
