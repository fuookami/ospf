//! Pressure units - 压力单位
//! Pressure units - SI pressure units
//!
//! 提供压力量纲的 SI 单位定义，包括帕斯卡、千帕、兆帕、巴等。
//! Provides SI unit definitions for pressure dimension, including pascal, kilopascal, megapascal, bar, etc.

use super::area::SquareMeter;
use super::force::Newton;
use crate::dimension::derived::Pressure;
use crate::scale::{Scale, KILO, MEGA};
use crate::unit::{CTUnit, CTUnitDiv};
use once_cell::sync::Lazy;

// ============================================================================
// 压力单位 / Pressure units
// ============================================================================

define_unit_by!(Pascal, "pascal", "Pa", CTUnitDiv<Newton, SquareMeter>);
define_unit!(
    Kilopascal,
    "kilopascal",
    "kPa",
    Pressure,
    &*Pascal::SCALE * &*KILO
);
define_unit!(
    Megapascal,
    "megapascal",
    "MPa",
    Pressure,
    &*Pascal::SCALE * &*MEGA
);
define_unit!(
    Bar,
    "bar",
    "bar",
    Pressure,
    &*Pascal::SCALE * &Scale::from_int(100000)
);
