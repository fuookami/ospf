//! 加速度单位 / Acceleration units
//!
//! 提供加速度量纲的 SI 单位定义，包括米每二次方秒、标准重力加速度等 / Provides SI unit definitions for acceleration dimension, including meter per second squared, standard gravity, etc

use super::length::{Cetimeter, Foot, Inch, Kilometer, Meter};
use super::time::Second;
use crate::dimension::derived::Acceleration;
use crate::scale::Scale;
use crate::unit::physical_unit::CTUnit;
use crate::unit::{CTUnitDiv, CTUnitMul};

// ============================================================================
// SI 加速度单位 / SI acceleration units
// ============================================================================

// 米每二次方秒 / Meter per second squared
define_unit_by!(
    MeterPerSecondSquared,
    "meter per second squared",
    "m/s²",
    CTUnitDiv<Meter, CTUnitMul<Second, Second>>
);

// 厘米每二次方秒 / Centimeter per second squared
define_unit_by!(
    CentimeterPerSecondSquared,
    "centimeter per second squared",
    "cm/s²",
CTUnitDiv<Cetimeter, CTUnitMul<Second, Second>>
);

// 千米每二次方秒 / Kilometer per second squared
define_unit_by!(
    KilometerPerSecondSquared,
    "kilometer per second squared",
    "km/s²",
    CTUnitDiv<Kilometer, CTUnitMul<Second, Second>>
);

// ============================================================================
// 英制加速度单位 / Imperial acceleration units
// ============================================================================

// 英寸每二次方秒 / Inch per second squared
define_unit_by!(
    InchPerSecondSquared,
    "inch per second squared",
    "in/s²",
    CTUnitDiv<Inch, CTUnitMul<Second, Second>>
);

// 英尺每二次方秒 / Foot per second squared
define_unit_by!(
    FootPerSecondSquared,
    "foot per second squared",
    "ft/s²",
    CTUnitDiv<Foot, CTUnitMul<Second, Second>>
);

// ============================================================================
// 特殊加速度单位 / Special acceleration units
// ============================================================================

// 标准重力加速度 / Standard gravity (9.80665 m/s²)
define_unit!(
    StandardGravity,
    "standard gravity",
    "g",
    Acceleration,
    Scale::from_f64(9.80665)
);
