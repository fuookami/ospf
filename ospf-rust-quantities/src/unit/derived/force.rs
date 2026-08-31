//! Force units - 力单位
//! Force units - SI force units
//!
//! 提供力量纲的 SI 单位定义，包括牛顿、千牛、兆牛、千克力等。
//! Provides SI unit definitions for force dimension, including newton, kilonewton, meganewton, kilogram-force, etc.

use super::acceleration::MeterPerSecondSquared;
use super::mass::Kilogram;
use crate::dimension::derived::Force;
use crate::scale::{Scale, KILO, MEGA};
use crate::unit::{CTUnit, CTUnitMul};
use once_cell::sync::Lazy;

// ============================================================================
// 力单位 / Force units
// ============================================================================

define_unit_by!(
    Newton,
    "newton",
    "N",
    CTUnitMul<Kilogram, MeterPerSecondSquared>
);
define_unit!(
    KiloNewton,
    "kilonewton",
    "kN",
    Force,
    &*Newton::SCALE * &*KILO
);
define_unit!(
    MegaNewton,
    "meganewton",
    "MN",
    Force,
    &*Newton::SCALE * &*MEGA
);
define_unit!(
    KilogramForce,
    "kilogram force",
    "kgf",
    Force,
    &*Newton::SCALE * &Scale::from_f64(9.80665)
);
