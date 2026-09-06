//! 力单位 / Force units
//!
//! 提供力量纲的 SI 单位定义，包括牛顿、千牛、兆牛、千克力等 / Provides SI unit definitions for force dimension, including newton, kilonewton, meganewton, kilogram-force, etc

use super::acceleration::MeterPerSecondSquared;
use super::mass::Kilogram;
use crate::dimension::derived::Force;
use crate::scale::{KILO, MEGA, Scale};
use crate::unit::{CTUnit, CTUnitMul};

// ============================================================================
// SI 力单位 / SI force units
// ============================================================================

define_unit_by!(
    Newton,
    "newton",
    "N",
    CTUnitMul<Kilogram, MeterPerSecondSquared>
);
define_unit!(
    Kilonewton,
    "kilonewton",
    "kN",
    Force,
    &*Newton::SCALE * &*KILO
);
define_unit!(
    Meganewton,
    "meganewton",
    "MN",
    Force,
    &*Newton::SCALE * &*MEGA
);

// ============================================================================
// 公制力单位 / Metric force units
// ============================================================================

// 千克力 / Kilogram force
define_unit!(
    KilogramForce,
    "kilogram force",
    "kgf",
    Force,
    Scale::from_f64(9.80665)
);

// 克力 / Gram force
define_unit!(
    GramForce,
    "gram force",
    "gf",
    Force,
    Scale::from_f64(0.00980665)
);

// 达因 / Dyne (1 g·cm/s²)
define_unit!(Dyne, "dyne", "dyn", Force, Scale::from_f64(0.00001));

// ============================================================================
// 英制力单位 / Imperial force units
// ============================================================================

// 磅力 / Pound force
define_unit!(
    PoundForce,
    "pound force",
    "lbf",
    Force,
    Scale::from_f64(4.4482216152605)
);

// 千磅力 / Kilopound force
define_unit!(
    KilopoundForce,
    "kilopound force",
    "klbf",
    Force,
    Scale::from_f64(4448.2216152605)
);
