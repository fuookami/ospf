//! Stress units - 应力单位
//! Stress units - SI stress units
//!
//! 提供应力量纲的 SI 单位定义，应力与压力同量纲，包括帕斯卡、兆帕等。
//! Provides SI unit definitions for stress dimension, stress has the same dimension as pressure, including pascal, megapascal, etc.

use super::area::SquareMeter;
use super::force::Newton;
use crate::dimension::derived::Pressure;
// Stress has same dimension as pressure
use crate::scale::{Scale, KILO, MEGA};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 应力单位 / Stress units
// ============================================================================

define_unit_by!(
    PascalStress,
    "pascal",
    "Pa",
    CTUnitDiv<Newton, SquareMeter>
);
define_unit!(
    KilopascalStress,
    "kilopascal",
    "kPa",
    Pressure,
    &*PascalStress::SCALE * &*KILO
);
define_unit!(
    MegapascalStress,
    "megapascal",
    "MPa",
    Pressure,
    &*PascalStress::SCALE * &*MEGA
);
define_unit!(
    PoundForcePerSquareInch,
    "pound-force per square inch",
    "psi",
    Pressure,
    Scale::from_f64(6894.757)
);
define_unit!(
    KilogramForcePerSquareCentimeter,
    "kilogram-force per square centimeter",
    "kgf/cm²",
    Pressure,
    Scale::from_f64(98066.5)
);
