//! Momentum units - 动量单位
//! Momentum units - SI momentum units
//!
//! 提供动量量纲的 SI 单位定义，包括千克米每秒等。
//! Provides SI unit definitions for momentum dimension, including kilogram meter per second, etc.

use crate::unit::{CTUnit, CTUnitDiv, CTUnitMul};
use super::length::Meter;
use super::mass::Kilogram;
use super::time::Second;

// ============================================================================
// 动量单位 / Momentum units
// ============================================================================

define_unit_by!(
    KilogramMeterPerSecond,
    "kilogram meter per second",
    "kg·m/s",
    CTUnitDiv<CTUnitMul<Kilogram, Meter>, Second>
);
