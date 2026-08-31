//! Energy units - 能量单位
//! Energy units - SI energy units
//!
//! 提供能量量纲的 SI 单位定义，包括焦耳、千焦、兆焦、吉焦、电子伏特、千瓦时等。
//! Provides SI unit definitions for energy dimension, including joule, kilojoule, megajoule, gigajoule, electronvolt, kilowatt-hour, etc.

use super::force::Newton;
use super::length::Meter;
use super::power::Kilowatt;
use super::time::Hour;
use crate::dimension::derived::Energy;
use crate::scale::{Scale, GIGA, KILO, MEGA};
use crate::unit::{CTUnit, CTUnitMul};

// ============================================================================
// 能量单位 / Energy units
// ============================================================================

define_unit_by!(Joule, "joule", "J", CTUnitMul<Newton, Meter>);
define_unit!(
    KiloJoule,
    "kilojoule",
    "kJ",
    Energy,
    &*Joule::SCALE * &*KILO
);
define_unit!(
    MegaJoule,
    "megajoule",
    "MJ",
    Energy,
    &*Joule::SCALE * &*MEGA
);
define_unit!(
    GigaJoule,
    "gigajoule",
    "GJ",
    Energy,
    &*Joule::SCALE * &*GIGA
);
define_unit!(
    ElectronVolt,
    "electronvolt",
    "eV",
    Energy,
    Scale::from_f64(1.602176634e-19)
);
define_unit_by!(KilowattHour, "kilowatt-hour", "kWh", CTUnitMul<Kilowatt, Hour>);
define_unit!(
    MegawattHour,
    "megawatt-hour",
    "MWh",
    Energy,
    &*KilowattHour::SCALE * &*KILO
);
