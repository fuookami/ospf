//! Velocity units - 速度单位
//! Velocity units - SI velocity units
//!
//! 提供速度量纲的 SI 单位定义，包括米每秒、千米每小时等。
//! Provides SI unit definitions for velocity dimension, including meter per second, kilometer per hour, etc.

use super::length::{
    Cetimeter, FRNauticalMile, Foot, Inch, Kilometer, Meter, Mile, NauticalMile, RUNauticalMile,
    UKNauticalMile, USNauticalMile,
};
use super::time::{Hour, Second};
use crate::dimension::derived::Velocity;
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// SI 速度单位 / SI velocity units
// ============================================================================

define_unit_by!(
    MeterPerSecond,
    "meter per second",
    "m/s",
    CTUnitDiv<Meter, Second>
);

define_unit_by!(
    CentimeterPerSecond,
    "centimeter per second",
    "cm/s",
    CTUnitDiv<Cetimeter, Second>
);

define_unit_by!(
    KilometerPerSecond,
    "kilometer per second",
    "km/s",
    CTUnitDiv<Kilometer, Second>
);
define_unit_by!(
    KilometersPerSecond,
    "kilometers per second",
    "km/s",
    KilometerPerSecond
);

define_unit_by!(
    KilometerPerHour,
    "kilometer per hour",
    "km/h",
    CTUnitDiv<Kilometer, Hour>
);

// ============================================================================
// 英制速度单位 / Imperial velocity units
// ============================================================================

// 英寸每秒 / Inch per second
define_unit_by!(
    InchPerSecond,
    "inch per second",
    "ips",
    CTUnitDiv<Inch, Second>
);

// 英尺每秒 / Foot per second
define_unit_by!(
    FootPerSecond,
    "foot per second",
    "fps",
    CTUnitDiv<Foot, Second>
);

// 英里每小时 / Mile per hour
define_unit_by!(
    MilePerHour,
    "mile per hour",
    "mph",
    CTUnitDiv<Mile, Hour>
);

// ============================================================================
// 特殊速度单位 / Special velocity units
// ============================================================================

// 节 / Knot (nautical mile per hour)
define_unit_by!(
    Knot,
    "knot",
    "kn",
    CTUnitDiv<NauticalMile, Hour>
);
define_unit_by!(
    FRKnot,
    "fr knot",
    "fr.kn",
    CTUnitDiv<FRNauticalMile, Hour>
);
define_unit_by!(
    UKKnot,
    "uk knot",
    "uk.kn",
    CTUnitDiv<UKNauticalMile, Hour>
);
define_unit_by!(
    RUKnot,
    "ru knot",
    "ru.kn",
    CTUnitDiv<RUNauticalMile, Hour>
);
define_unit_by!(
    USKnot,
    "us knot",
    "us.kn",
    CTUnitDiv<USNauticalMile, Hour>
);

// 马赫 / Mach (speed of sound, approximately 340.3 m/s)
define_unit!(Mach, "mach", "ma", Velocity, Scale::from_f64(340.3));

// 光速 / Light speed
define_unit!(
    LightSpeed,
    "light speed",
    "c",
    Velocity,
    Scale::from_f64(299792458.0)
);
